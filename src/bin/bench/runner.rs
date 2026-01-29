use indexmap::IndexMap;
use serde_json::Value;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::bench::config::{AppCommand, BenchConfig, Step};
use crate::bench::logger::Logger;
use crate::bench::stats::print_statistics;
use crate::bench::timing::{get_cpu_time_ns, ns_to_ms_str, StepTimings, calculate_median, calculate_p99};

async fn run_benchmark_steps(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    config: &BenchConfig,
    timings: &mut IndexMap<String, StepTimings>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    for step in &config.steps {
        // Check if this step should be executed multiple times
        if let Some(tasks) = step.tasks {
            execute_multiple_tasks(stdin, reader, step, tasks, timings, logger).await?;
            continue;
        }
        
        let req_id = step.payload.get("id").and_then(|v: &Value| v.as_i64());
        
        // Check if benchmarking is disabled for this step
        if !step.bench {
            // Send the JSON payload without timing
            stdin.write_all(format!("{}\n", step.payload).as_bytes()).await?;
            logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", step.name, "SENT", "N/A", "N/A"));
            
            // If it's a request (has ID), still need to consume the response
            if req_id.is_some() {
                while let Some(line) = reader.next_line().await? {
                    let resp: Value = serde_json::from_str(&line)?;
                    if resp.get("id").and_then(|v| v.as_i64()) == req_id {
                        break;
                    }
                }
            }
            continue;
        }
        
        // Benchmarking enabled - measure timing
        let start_wall = Instant::now();
        let start_cpu = get_cpu_time_ns();

        // Send the JSON payload exactly as defined in TOML
        stdin.write_all(format!("{}\n", step.payload).as_bytes()).await?;

        if let Some(current_req_id) = req_id {
            // Wait for a response with a matching ID
            while let Some(line) = reader.next_line().await? {
                let resp: Value = serde_json::from_str(&line)?;
                if resp.get("id").and_then(|v| v.as_i64()) == Some(current_req_id) {
                    let wall_duration_ns = start_wall.elapsed().as_nanos();
                    let cpu_duration_ns = get_cpu_time_ns() - start_cpu;
                    let status = if resp.get("error").is_some() { "ERROR" } else { "OK" };

                    let wall_str = ns_to_ms_str(wall_duration_ns);
                    let cpu_str = ns_to_ms_str(cpu_duration_ns as u128);

                    logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", step.name, status, wall_str, cpu_str));
                    
                    // Log error details if present
                    if let Some(error) = resp.get("error") {
                        logger.log_error(&step.name, &error.to_string());
                    }
                    
                    // Log full response to file
                    logger.log_response(&step.name, &resp);
                    
                    // Collect timing data
                    let step_timing = timings.entry(step.name.clone()).or_default();
                    step_timing.wall_times.push(wall_duration_ns);
                    step_timing.cpu_times.push(cpu_duration_ns as u128);
                    
                    break;
                }
            }
        } else {
            // No ID: this is a notification. It's fire and forget.
            let wall_duration_ns = start_wall.elapsed().as_nanos();
            let cpu_duration_ns = get_cpu_time_ns() - start_cpu;
            let wall_str = ns_to_ms_str(wall_duration_ns);
            let cpu_str = ns_to_ms_str(cpu_duration_ns as u128);
            logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", step.name, "SENT", wall_str, cpu_str));
            
            // Collect timing data
            let step_timing = timings.entry(step.name.clone()).or_default();
            step_timing.wall_times.push(wall_duration_ns);
            step_timing.cpu_times.push(cpu_duration_ns as u128);
        }
    }

    Ok(())
}

async fn execute_multiple_tasks(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    step: &Step,
    tasks: usize,
    timings: &mut IndexMap<String, StepTimings>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    logger.log(&format!("\nExecuting {} tasks sequentially", tasks));
    logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", "Step Name", "Status", "Wall (ms)", "CPU (ms)"));
    logger.log(&"-".repeat(80));
    
    let overall_start = Instant::now();
    let base_id = step.payload.get("id").and_then(|v| v.as_i64()).unwrap_or(1000);
    
    for task_num in 0..tasks {
        let step_name = format!("{} #{}", step.name, task_num + 1);
        let request_id = base_id + task_num as i64;
        
        // Create a modified payload with unique ID
        let mut task_payload = step.payload.clone();
        if let Some(obj) = task_payload.as_object_mut() {
            obj.insert("id".to_string(), Value::Number(request_id.into()));
        }
        
        let start_wall = Instant::now();
        let start_cpu = get_cpu_time_ns();
        
        // Send the request
        stdin.write_all(format!("{}\n", task_payload).as_bytes()).await?;
        
        // Wait for response
        while let Some(line) = reader.next_line().await? {
            let resp: Value = serde_json::from_str(&line)?;
            
            if let Some(resp_id) = resp.get("id").and_then(|v| v.as_i64()) {
                if resp_id == request_id {
                    let wall_duration_ns = start_wall.elapsed().as_nanos();
                    let cpu_duration_ns = get_cpu_time_ns() - start_cpu;
                    let status = if resp.get("error").is_some() { "ERROR" } else { "OK" };
                    
                    let wall_str = ns_to_ms_str(wall_duration_ns);
                    let cpu_str = ns_to_ms_str(cpu_duration_ns as u128);
                    
                    logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", step_name, status, wall_str, cpu_str));
                    
                    // Log error details if present
                    if let Some(error) = resp.get("error") {
                        logger.log_error(&step_name, &error.to_string());
                    }
                    
                    // Log full response to file
                    logger.log_response(&step_name, &resp);
                    
                    // Collect timing data
                    let step_timing = timings.entry(step.name.clone()).or_default();
                    step_timing.wall_times.push(wall_duration_ns);
                    step_timing.cpu_times.push(cpu_duration_ns as u128);
                    
                    break;
                }
            }
        }
    }
    
    // Calculate total wall clock time
    let total_wall_time_ms = overall_start.elapsed().as_secs_f64() * 1000.0;
    let avg_time_per_task_ms = total_wall_time_ms / tasks as f64;
    
    // Print summary statistics for all tasks
    if let Some(step_timing) = timings.get(&step.name) {
        if !step_timing.wall_times.is_empty() {
            logger.log(&"-".repeat(80));
            
            let mut wall_sorted = step_timing.wall_times.clone();
            wall_sorted.sort_unstable();
            let wall_median = calculate_median(&wall_sorted) / 1_000_000.0;
            let wall_p99 = calculate_p99(&wall_sorted) / 1_000_000.0;
            let wall_min = wall_sorted[0] as f64 / 1_000_000.0;
            let wall_max = wall_sorted[wall_sorted.len() - 1] as f64 / 1_000_000.0;
            
            let mut cpu_sorted = step_timing.cpu_times.clone();
            cpu_sorted.sort_unstable();
            let cpu_median = calculate_median(&cpu_sorted) / 1_000_000.0;
            let cpu_p99 = calculate_p99(&cpu_sorted) / 1_000_000.0;
            let cpu_min = cpu_sorted[0] as f64 / 1_000_000.0;
            let cpu_max = cpu_sorted[cpu_sorted.len() - 1] as f64 / 1_000_000.0;
            
            logger.log(&format!("\nSummary for '{}' ({} tasks):", step.name, tasks));
            logger.log(&format!("  Wall time: min={:.3}ms, median={:.3}ms, p99={:.3}ms, max={:.3}ms",
                     wall_min, wall_median, wall_p99, wall_max));
            logger.log(&format!("  CPU time:  min={:.3}ms, median={:.3}ms, p99={:.3}ms, max={:.3}ms",
                     cpu_min, cpu_median, cpu_p99, cpu_max));
            logger.log(&format!("\n  Total wall clock time: {:.3}ms", total_wall_time_ms));
            logger.log(&format!("  Average time per task: {:.3}ms (total wall clock / {} tasks)", avg_time_per_task_ms, tasks));
        }
    }
    
    Ok(())
}

pub async fn run_benchmark(
    target: AppCommand,
    bench_path: &str,
    timings: &mut IndexMap<String, StepTimings>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the benchmark sequence
    let bench_str = std::fs::read_to_string(bench_path)?;
    let config: BenchConfig = toml::from_str(&bench_str)?;

    // Spawn the MCP process
    let mut child = Command::new(&target.bin)
        .args(&target.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap()).lines();

    logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", "Step Name", "Status", "Wall (ms)", "CPU (ms)"));
    logger.log(&"-".repeat(80));

    run_benchmark_steps(&mut stdin, &mut reader, &config, timings, logger).await?;

    child.kill().await?;
    Ok(())
}

pub async fn run_benchmark_persistent(
    target: AppCommand,
    bench_path: &str,
    iterations: usize,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the benchmark sequence
    let bench_str = std::fs::read_to_string(bench_path)?;
    let config: BenchConfig = toml::from_str(&bench_str)?;

    // Spawn the MCP process once
    let mut child = Command::new(&target.bin)
        .args(&target.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap()).lines();

    let mut timings: IndexMap<String, StepTimings> = IndexMap::new();

    // Run benchmark multiple times with the same process
    for iteration in 1..=iterations {
        if iterations > 1 {
            logger.log(&format!("\n{}", "=".repeat(80)));
            logger.log(&format!("Iteration {}/{}", iteration, iterations));
            logger.log(&format!("{}\n", "=".repeat(80)));
        }

        logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", "Step Name", "Status", "Wall (ms)", "CPU (ms)"));
        logger.log(&"-".repeat(80));

        run_benchmark_steps(&mut stdin, &mut reader, &config, &mut timings, logger).await?;
    }

    if iterations > 1 {
        logger.log(&format!("\n{}", "=".repeat(80)));
        logger.log(&format!("Completed {} iterations", iterations));
        logger.log(&"=".repeat(80));
        
        // Print statistics
        print_statistics(&timings, logger);
    }

    child.kill().await?;
    Ok(())
}

// Made with Bob
