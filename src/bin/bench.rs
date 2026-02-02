use clap::{ArgAction, Parser};
use crossbeam_channel::{unbounded, Receiver, Sender};
use serde::{Deserialize, Serialize};
// Import serde_json for JSON serialization
use statistical::{median, standard_deviation};
use std::f64;
use std::fs::File;
use std::io::BufWriter;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to a log file for detailed request/response logging
    #[arg(long)]
    log_file: Option<String>,
    /// Path to the MCP server executable
    #[arg(short, long)]
    server: String,
    /// Arguments to pass to the server executable (everything after --server <exe>)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    server_args: Vec<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    silent: bool,
}

#[derive(Debug, Deserialize)]
struct BenchConfig {
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize, Clone)]
struct Step {
    name: String,
    bench: bool,
    payload: serde_json::Value,
    tasks: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct LogEntry {
    id: u64,
    step_name: String,
    request: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// Helper function to calculate a percentile
fn calculate_percentile(data: &mut [f64], percentile: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let rank = percentile / 100.0 * (data.len() as f64 - 1.0);
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil().abs() as usize;
    let weight = rank - lower_index as f64;
    if lower_index == upper_index {
        data[lower_index]
    } else {
        data[lower_index] * (1.0 - weight) + data[upper_index] * weight
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let toml_content = fs::read_to_string("bench.toml")?;
    let bench_config: BenchConfig = toml::from_str(&toml_content).unwrap();

    let mut server = Command::new(&args.server)
        .args(&args.server_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Redirect stderr to null to suppress server's own logs
        .spawn()?;

    let mut stdin = server.stdin.take().unwrap();
    let stdout = server.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let (tx, rx): (Sender<String>, Receiver<String>) = unbounded();

    thread::spawn(move || {
        for line in reader.lines() {
            tx.send(line.unwrap()).unwrap();
        }
    });

    // Initialize log_writer
    let mut log_writer: Option<BufWriter<File>> = None;
    if let Some(log_file_path) = &args.log_file {
        let file = File::create(log_file_path)?;
        log_writer = Some(BufWriter::new(file));
    }

    let mut request_id_counter: u64 = 1;

    for step in bench_config.steps {
        let mut durations: Vec<f64> = Vec::new();
        let num_tasks = step.tasks.unwrap_or(1);
        let step_start_time = Instant::now();
        for _ in 0..num_tasks {
            let mut payload = step.payload.clone();
            if payload.get("id").is_some() {
                payload["id"] = serde_json::Value::from(request_id_counter);
            }

            let request_str = serde_json::to_string(&payload).unwrap();

            if step.bench {
                let start_time = Instant::now();

                writeln!(stdin, "{}", &request_str).unwrap();
                stdin.flush().unwrap();

                // Wait for a response if an id is present
                if payload.get("id").is_some() {
                    loop {
                        let response_str = rx.recv_timeout(Duration::from_secs(5)).unwrap();
                        if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&response_str)
                        {
                            if response.id == Some(request_id_counter) {
                                let duration = start_time.elapsed();
                                let micros = duration.as_micros();
                                let ms = micros as f64 / 1000.0;
                                durations.push(ms); // Collect duration

                                if let Some(writer) = &mut log_writer {
                                    let log_entry = LogEntry {
                                        id: request_id_counter,
                                        step_name: step.name.clone(),
                                        request: payload.clone(),
                                        response: Some(
                                            serde_json::from_str(&response_str).unwrap(),
                                        ),
                                        duration_ms: Some(ms),
                                        message: None,
                                    };
                                    serde_json::to_writer(&mut *writer, &log_entry)?;
                                    writeln!(writer)?;
                                }
                                if !args.silent {
                                    println!(
                                        "#{} \"{}\" {:.3}ms", // New console output format
                                        request_id_counter, step.name, ms
                                    );
                                }
                                break;
                            }
                        }
                    }
                } else {
                    if let Some(writer) = &mut log_writer {
                        let log_entry = LogEntry {
                            id: request_id_counter,
                            step_name: step.name.clone(),
                            request: payload.clone(),
                            response: None,
                            duration_ms: None,
                            message: Some(format!("{} sent.", step.name)),
                        };
                        serde_json::to_writer(&mut *writer, &log_entry)?;
                        writeln!(writer)?;
                    }
                    println!(
                        "#{} \"{}\" sent", // New console output format
                        request_id_counter, step.name
                    );
                }
            } else {
                writeln!(stdin, "{}", &request_str).unwrap();
                stdin.flush().unwrap();
                if let Some(writer) = &mut log_writer {
                    let log_entry = LogEntry {
                        id: request_id_counter,
                        step_name: step.name.clone(),
                        request: payload.clone(),
                        response: None,
                        duration_ms: None,
                        message: Some(format!("{} sent.", step.name)),
                    };
                    serde_json::to_writer(&mut *writer, &log_entry)?;
                    writeln!(writer)?;
                }
                println!("#{} \"{}\" sent", request_id_counter, step.name); // New console output format
            }

            if payload.get("id").is_some() {
                request_id_counter += 1;
            }
        }

        // Print stats at the end of each step
        if step.bench && !durations.is_empty() {
            let step_total_time = step_start_time.elapsed();
            let total_seconds = step_total_time.as_secs_f64();
            let rps = num_tasks as f64 / total_seconds;

            let mut sorted_durations = durations.clone(); // Clone for sorting

            if durations.len() == 1 {
                // For single data point, median and p99 are the same value, std_dev is 0
                println!(
                    "Step '{}' stats: Median: {:.3}ms, P99: {:.3}ms, StdDev: 0.000ms, RPS: {:.2} (single sample)",
                    step.name, durations[0], durations[0], rps
                );
            } else {
                let median_val = median(&durations);
                let p99 = calculate_percentile(&mut sorted_durations, 99.0);
                let std_dev = standard_deviation(&durations, None);
                println!(
                    "Step '{}' stats: Median: {:.3}ms, P99: {:.3}ms, StdDev: {:.3}ms, RPS: {:.2}",
                    step.name, median_val, p99, std_dev, rps
                );
            }
        }
    }

    server.kill()?;
    Ok(())
}
