use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::config::BenchConfig;
use crate::stats::{format_duration, Stats};
use crate::tracker::TaskTracker;

pub struct BenchRunner {
    config: BenchConfig,
    command_args: Vec<String>,
    _log_file: Option<String>,
    _parallel: bool,
    iterations: usize,
}

impl BenchRunner {
    pub fn new(
        config: BenchConfig,
        command_args: Vec<String>,
        log_file: Option<String>,
        parallel: bool,
        iterations: usize,
    ) -> Self {
        Self {
            config,
            command_args,
            _log_file: log_file,
            _parallel: parallel,
            iterations,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting benchmark with {} iterations", self.iterations);
        
        for iteration in 1..=self.iterations {
            info!("=== Iteration {}/{} ===", iteration, self.iterations);
            self.run_single_iteration(iteration).await?;
        }
        
        info!("Benchmark completed successfully");
        Ok(())
    }

    async fn run_single_iteration(&self, _iteration: usize) -> Result<()> {
        // Spawn the process
        let mut child = self.spawn_process()?;
        
        let mut stdin = child.stdin.take().context("Failed to get stdin")?;
        let stdout = child.stdout.take().context("Failed to get stdout")?;
        
        // Create task tracker
        let tracker = Arc::new(TaskTracker::new(self.config.timeout_seconds));
        
        // Create channels for communication
        let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Value>();
        
        // Spawn stdout reader
        let tracker_clone = tracker.clone();
        let reader_handle = tokio::spawn(async move {
            Self::read_responses(stdout, response_tx, tracker_clone).await
        });
        
        // Spawn timeout checker
        let tracker_clone = tracker.clone();
        let timeout_handle = tokio::spawn(async move {
            Self::check_timeouts(tracker_clone).await
        });
        
        // Run benchmark steps
        let mut next_id = 1u64;
        let mut step_results: HashMap<String, Vec<Duration>> = HashMap::new();
        let mut step_failed: HashMap<String, usize> = HashMap::new();
        
        for step in &self.config.steps {
            info!("Executing step: {}", step.name);
            
            let tasks = step.tasks;
            let mut sent_count = 0;
            
            for task_idx in 0..tasks {
                let payload = self.prepare_payload(&step.payload, next_id);
                
                // Track it BEFORE sending to get accurate send time
                if step.bench {
                    tracker.add_task(next_id, payload.clone(), step.name.clone()).await;
                }
                
                // Send payload
                self.send_payload(&mut stdin, &payload).await?;
                
                sent_count += 1;
                next_id += 1;
                
                if task_idx % 100 == 0 && task_idx > 0 {
                    debug!("Sent {}/{} tasks for step '{}'", task_idx, tasks, step.name);
                }
            }
            
            info!("Sent {} tasks for step '{}'", sent_count, step.name);
            
            // Wait for responses if benchmarking
            if step.bench {
                let step_name = step.name.clone();
                let mut received = 0;
                let expected = tasks;
                
                while received < expected {
                    match time::timeout(Duration::from_secs(5), response_rx.recv()).await {
                        Ok(Some(response)) => {
                            if let Some(id) = response.get("id").and_then(|v| v.as_u64()) {
                                if let Some((duration, resp_step_name)) = tracker.complete_task(id).await {
                                    if resp_step_name == step_name {
                                        step_results.entry(step_name.clone())
                                            .or_insert_with(Vec::new)
                                            .push(duration);
                                        received += 1;
                                        
                                        if received % 100 == 0 {
                                            debug!("Received {}/{} responses for step '{}'", received, expected, step_name);
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            warn!("Response channel closed");
                            break;
                        }
                        Err(_) => {
                            // Timeout waiting for response
                            let pending = tracker.pending_count().await;
                            if pending == 0 {
                                break;
                            }
                            debug!("Waiting for responses, {} pending", pending);
                        }
                    }
                }
                
                info!("Received {}/{} responses for step '{}'", received, expected, step_name);
            } else {
                // For non-benchmark steps, just wait a bit
                time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        // Wait a bit for any remaining responses
        time::sleep(Duration::from_secs(2)).await;
        
        // Check for expired tasks
        let expired = tracker.cleanup_expired_tasks().await;
        for (id, step_name) in expired {
            warn!("Task {} for step '{}' timed out", id, step_name);
            *step_failed.entry(step_name).or_insert(0) += 1;
        }
        
        // Print statistics
        self.print_statistics(&step_results, &step_failed);
        
        // Cleanup
        drop(reader_handle);
        drop(timeout_handle);
        
        // Kill the child process
        let _ = child.kill().await;
        
        Ok(())
    }

    fn spawn_process(&self) -> Result<Child> {
        if self.command_args.is_empty() {
            anyhow::bail!("No command specified");
        }
        
        let program = &self.command_args[0];
        let args = &self.command_args[1..];
        
        info!("Spawning process: {} {:?}", program, args);
        
        let child = Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .context("Failed to spawn process")?;
        
        Ok(child)
    }

    async fn read_responses(
        stdout: ChildStdout,
        tx: mpsc::UnboundedSender<Value>,
        _tracker: Arc<TaskTracker>,
    ) {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<Value>(&line) {
                Ok(response) => {
                    info!("RECEIVED: {}", line);
                    if tx.send(response).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to parse response: {} - {}", e, line);
                }
            }
        }
    }

    async fn check_timeouts(tracker: Arc<TaskTracker>) {
        let mut interval = time::interval(Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            let expired = tracker.cleanup_expired_tasks().await;
            
            if !expired.is_empty() {
                warn!("Cleaned up {} expired tasks", expired.len());
            }
        }
    }

    async fn send_payload(&self, stdin: &mut ChildStdin, payload: &Value) -> Result<()> {
        let json_str = serde_json::to_string(payload)?;
        info!("SENT: {}", json_str);
        stdin.write_all(json_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    fn prepare_payload(&self, template: &Value, id: u64) -> Value {
        let mut payload = template.clone();
        
        // Set the id if the payload has an id field
        if payload.is_object() {
            if let Some(obj) = payload.as_object_mut() {
                if obj.contains_key("id") {
                    obj.insert("id".to_string(), Value::Number(id.into()));
                }
            }
        }
        
        payload
    }

    fn print_statistics(
        &self,
        step_results: &HashMap<String, Vec<Duration>>,
        step_failed: &HashMap<String, usize>,
    ) {
        println!("\n=== Benchmark Results ===\n");
        
        for step in &self.config.steps {
            if !step.bench {
                continue;
            }
            
            println!("Step: {}", step.name);
            
            let failed = step_failed.get(&step.name).copied().unwrap_or(0);
            
            if let Some(durations) = step_results.get(&step.name) {
                let stats = Stats::calculate(durations.clone()).with_failed_count(failed);
                
                println!("  Total tasks:      {}", stats.total_tasks);
                println!("  Successful:       {}", stats.successful_tasks);
                println!("  Failed:           {}", stats.failed_tasks);
                
                if let Some(median) = stats.median {
                    println!("  Median:           {}", format_duration(median));
                }
                
                if let Some(p99) = stats.percentile_99 {
                    println!("  99th percentile:  {}", format_duration(p99));
                }
                
                if let Some(std_dev) = stats.std_deviation {
                    println!("  Std deviation:    {}", format_duration(std_dev));
                }
                
                if let Some(min) = stats.min {
                    println!("  Min:              {}", format_duration(min));
                }
                
                if let Some(max) = stats.max {
                    println!("  Max:              {}", format_duration(max));
                }
            } else if failed > 0 {
                println!("  Total tasks:      {}", failed);
                println!("  Successful:       0");
                println!("  Failed:           {}", failed);
            }
            
            println!();
        }
    }
}

// Made with Bob
