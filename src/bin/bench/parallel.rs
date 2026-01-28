use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tokio::io::{AsyncWriteExt, BufReader};

use crate::bench::config::Step;
use crate::bench::logger::Logger;
use crate::bench::timing::{get_cpu_time_ns, ns_to_ms_str, StepTimings, calculate_median, calculate_p99};

pub async fn execute_parallel_step(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    step: &Step,
    concurrency: usize,
    tasks: usize,
    timings: &mut IndexMap<String, StepTimings>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    logger.log(&format!("\nExecuting {} tasks with concurrency limit of {}", tasks, concurrency));
    logger.log(&format!("{:<25} | {:<10} | {:<15} | {:<15}", "Step Name", "Status", "Wall (ms)", "CPU (ms)"));
    logger.log(&"-".repeat(80));
    
    // Track the overall start time for calculating total wall clock time
    let overall_start = Instant::now();
    
    // Track pending requests: ID -> (step_name, start_wall, start_cpu)
    let mut pending_requests: HashMap<i64, (String, Instant, i64)> = HashMap::new();
    
    // Base ID for generating unique request IDs
    let base_id = step.payload.get("id").and_then(|v| v.as_i64()).unwrap_or(1000);
    
    let mut tasks_sent = 0;
    let mut tasks_completed = 0;
    
    // Send initial batch up to concurrency limit
    while tasks_sent < tasks && tasks_sent < concurrency {
        let task_num = tasks_sent;
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
        
        pending_requests.insert(request_id, (step_name, start_wall, start_cpu));
        tasks_sent += 1;
    }
    
    // Process responses and send more requests as slots become available
    while tasks_completed < tasks {
        if let Some(line) = reader.next_line().await? {
            let resp: Value = serde_json::from_str(&line)?;
            
            if let Some(resp_id) = resp.get("id").and_then(|v| v.as_i64()) {
                if let Some((step_name, start_wall, start_cpu)) = pending_requests.remove(&resp_id) {
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
                    
                    tasks_completed += 1;
                    
                    // Send next request if there are more tasks to send
                    if tasks_sent < tasks {
                        let task_num = tasks_sent;
                        let step_name = format!("{} #{}", step.name, task_num + 1);
                        let request_id = base_id + task_num as i64;
                        
                        let mut task_payload = step.payload.clone();
                        if let Some(obj) = task_payload.as_object_mut() {
                            obj.insert("id".to_string(), Value::Number(request_id.into()));
                        }
                        
                        let start_wall = Instant::now();
                        let start_cpu = get_cpu_time_ns();
                        
                        stdin.write_all(format!("{}\n", task_payload).as_bytes()).await?;
                        
                        pending_requests.insert(request_id, (step_name, start_wall, start_cpu));
                        tasks_sent += 1;
                    }
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
            
            logger.log(&format!("\nSummary for '{}' ({} tasks, concurrency: {}):", step.name, tasks, concurrency));
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

// Made with Bob
