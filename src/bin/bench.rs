use clap::Parser;
use indexmap::IndexMap;

#[path = "bench/mod.rs"]
mod bench;

use bench::config::AppCommand;
use bench::logger::Logger;
use bench::runner::{run_benchmark, run_benchmark_persistent};
use bench::stats::print_statistics;
use bench::timing::StepTimings;

/// Benchmark tool for MCP stdio wrapper
#[derive(Parser, Debug)]
#[command(name = "bench")]
#[command(about = "Run benchmarks against an MCP server", long_about = None)]
struct Args {
    /// Path to the benchmark configuration file (TOML format)
    #[arg(value_name = "BENCH_CONFIG")]
    bench_config: String,

    /// Number of times to run the benchmark
    #[arg(short, long, default_value_t = 1)]
    iterations: usize,

    /// Reuse the same child process across all iterations instead of spawning a new one each time
    #[arg(short, long)]
    persistent: bool,

    /// Path to log file for saving error messages and results
    #[arg(long, value_name = "LOG_FILE")]
    log_file: Option<String>,

    /// Path to the binary to benchmark
    #[arg(value_name = "BIN_PATH")]
    bin_path: String,

    /// Arguments to pass to the binary
    #[arg(value_name = "ARGS", trailing_var_arg = true)]
    bin_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Create logger
    let logger = Logger::new(args.log_file.clone())?;
    
    let target = AppCommand {
        bin: args.bin_path,
        args: args.bin_args,
    };
    
    if args.persistent {
        // Persistent mode: reuse the same child process across all iterations
        run_benchmark_persistent(target, &args.bench_config, args.iterations, &logger).await?;
    } else {
        // Default mode: spawn a new child process for each iteration
        let mut timings: IndexMap<String, StepTimings> = IndexMap::new();
        
        for iteration in 1..=args.iterations {
            if args.iterations > 1 {
                logger.log(&format!("\n{}", "=".repeat(80)));
                logger.log(&format!("Iteration {}/{}", iteration, args.iterations));
                logger.log(&format!("{}\n", "=".repeat(80)));
            }
            
            run_benchmark(target.clone(), &args.bench_config, &mut timings, &logger).await?;
        }
        
        if args.iterations > 1 {
            logger.log(&format!("\n{}", "=".repeat(80)));
            logger.log(&format!("Completed {} iterations", args.iterations));
            logger.log(&"=".repeat(80));
            
            // Print statistics
            print_statistics(&timings, &logger);
        }
    }
    
    Ok(())
}

// Made with Bob
