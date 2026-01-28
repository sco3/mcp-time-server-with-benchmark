use indexmap::IndexMap;
use serde_json::Value;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::bench::config::{AppCommand, BenchConfig};
use crate::bench::logger::Logger;
use crate::bench::parallel::execute_parallel_step;
use crate::bench::stats::print_statistics;
use crate::bench::timing::{get_cpu_time_ns, ns_to_ms_str, StepTimings};

async fn run_benchmark_steps(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    config: &BenchConfig,
    timings: &mut IndexMap<String, StepTimings>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    for step in &config.steps {
        // Check if this step should be executed in parallel
        if let (Some(concurrency), Some(tasks)) = (step.concurrency, step.tasks) {
            execute_parallel_step(stdin, reader, step, concurrency, tasks, timings, logger).await?;
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
