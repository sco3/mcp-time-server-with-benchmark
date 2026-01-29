mod config;
mod runner;
mod stats;
mod tracker;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use config::BenchConfig;
use runner::BenchRunner;

#[derive(Parser, Debug)]
#[command(name = "bench")]
#[command(about = "Benchmark utility for MCP servers", long_about = None)]
struct Args {
    /// Path to the benchmark configuration file (TOML)
    config_file: PathBuf,

    /// Enable parallel execution mode
    #[arg(short = 'p', long)]
    parallel: bool,

    /// Number of iterations to run
    #[arg(short = 'i', long, default_value = "1")]
    iterations: usize,

    /// Log file path
    #[arg(long = "log-file")]
    log_file: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long = "log-level", default_value = "info")]
    log_level: String,

    /// Command and arguments to execute (after --)
    #[arg(last = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // Setup logging with optional file output
    if let Some(log_file_path) = &args.log_file {
        use std::fs::OpenOptions;
        use tracing_subscriber::fmt::writer::MakeWriterExt;
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)
            .context("Failed to open log file")?;
        
        let subscriber = FmtSubscriber::builder()
            .with_max_level(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .with_ansi(false)
            .with_writer(file.and(std::io::stdout))
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .context("Failed to set tracing subscriber")?;
    } else {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .with_ansi(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .context("Failed to set tracing subscriber")?;
    }

    // Load configuration
    let config = BenchConfig::from_file(&args.config_file)
        .context("Failed to load benchmark configuration")?;

    // Validate command
    if args.command.is_empty() {
        anyhow::bail!("No command specified. Use -- followed by the command to execute.");
    }

    // Create and run benchmark
    let runner = BenchRunner::new(
        config,
        args.command,
        args.log_file,
        args.parallel,
        args.iterations,
    );

    runner.run().await?;

    Ok(())
}

// Made with Bob
