use clap::Parser;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    silent: Option<bool>,
    #[arg(long)]
    log_file: Option<String>,
    #[arg(short, long)]
    server: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    server_args: Vec<String>,
    #[arg(short, long, default_value = "bench.toml")]
    config: String,
}

#[derive(Deserialize)]
struct Config {
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    name: String,
    #[serde(default)]
    bench: bool,
    #[serde(default = "default_batch")]
    batch: usize,
    #[serde(default = "default_tasks")]
    tasks: usize,
    payload: serde_json::Value,
}

fn default_batch() -> usize { 1 }
fn default_tasks() -> usize { 1 }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_content = fs::read_to_string(&args.config)?;
    let config: Config = toml::from_str(&config_content)?;

    // Запуск процесса
    let mut child = Command::new(&args.server)
        .args(&args.server_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    // Карта для трекинга RTT: ID -> Instant
    let pending_requests = Arc::new(Mutex::new(HashMap::new()));
    let latencies = Arc::new(Mutex::new(Vec::new()));

    // Фоновая задача на чтение ответов
    let pending_clone = Arc::clone(&pending_requests);
    let latencies_clone = Arc::clone(&latencies);
    let silent = args.silent.unwrap_or(false);

    tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            let now = Instant::now();
            if let Ok(response) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(id) = response.get("id").and_then(|i| i.as_u64()) {
                    let mut pending = pending_clone.lock().await;
                    if let Some(start) = pending.remove(&id) {
                        let mut lats = latencies_clone.lock().await;
                        lats.push(now.duration_since(start));
                    }
                }
            }
        }
    });

    for step in config.steps {
        if !silent { println!("Executing Step: '{}'...", step.name); }
        
        let step_start = Instant::now();
        latencies.lock().await.clear(); // Сброс для каждого шага
        
        let mut sent = 0;
        let mut current_id_base: u64 = (sent as u64) + 100; // Уникальные ID для шага

        while sent < step.tasks {
            // Контроль окна (batch)
            while pending_requests.lock().await.len() >= step.batch {
                tokio::task::yield_now().await;
            }

            let mut msg = step.payload.clone();
            let id = current_id_base + sent as u64;
            if msg.is_object() && step.bench {
                msg.as_object_mut().unwrap().insert("id".to_string(), id.into());
            }

            let payload = serde_json::to_string(&msg)? + "\n";
            
            pending_requests.lock().await.insert(id, Instant::now());
            stdin.write_all(payload.as_bytes()).await?;
            sent += 1;

            // Flush раз в батч или в конце
            if sent % step.batch == 0 || sent == step.tasks {
                stdin.flush().await?;
            }
        }

        // Ждем завершения всех задач в шаге
        while !pending_requests.lock().await.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let elapsed = step_start.elapsed();
        if step.bench {
            print_step_stats(&step.name, &*latencies.lock().await, elapsed, step.tasks);
        }
    }

    child.kill().await?;
    Ok(())
}

fn print_step_stats(name: &str, latencies: &[Duration], total_time: Duration, tasks: usize) {
    if latencies.is_empty() { return; }
    
    let mut hist = Histogram::<u64>::new_with_bounds(1, 1_000_000_000, 3).unwrap();
    for lat in latencies {
        hist.record(lat.as_nanos() as u64).unwrap();
    }

    let rps = tasks as f64 / total_time.as_secs_f64();
    println!("---");
    println!("Step '{}' stats:", name);
    println!("  Median: {:.3}ms", hist.value_at_quantile(0.5) as f64 / 1_000_000.0);
    println!("  P95:    {:.3}ms", hist.value_at_quantile(0.95) as f64 / 1_000_000.0);
    println!("  P99:    {:.3}ms", hist.value_at_quantile(0.99) as f64 / 1_000_000.0);
    println!("  RPS:    {:.2}", rps);
    println!("---");
}