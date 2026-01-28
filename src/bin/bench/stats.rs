use indexmap::IndexMap;
use crate::bench::timing::{StepTimings, calculate_median, calculate_p99};
use crate::bench::logger::Logger;

pub fn print_statistics(timings: &IndexMap<String, StepTimings>, logger: &Logger) {
    logger.log(&format!("\n{}", "=".repeat(100)));
    logger.log("Statistics Summary");
    logger.log(&"=".repeat(100));
    logger.log(&format!("{:<25} | {:<15} | {:<15} | {:<15} | {:<15}",
             "Step Name", "Wall Med (ms)", "Wall P99 (ms)", "CPU Med (ms)", "CPU P99 (ms)"));
    logger.log(&"-".repeat(100));
    
    for (step_name, step_timings) in timings {
        if step_timings.wall_times.is_empty() {
            continue;
        }
        
        let mut wall_sorted = step_timings.wall_times.clone();
        wall_sorted.sort_unstable();
        let wall_median = calculate_median(&wall_sorted) / 1_000_000.0;
        let wall_p99 = calculate_p99(&wall_sorted) / 1_000_000.0;
        
        let mut cpu_sorted = step_timings.cpu_times.clone();
        cpu_sorted.sort_unstable();
        let cpu_median = calculate_median(&cpu_sorted) / 1_000_000.0;
        let cpu_p99 = calculate_p99(&cpu_sorted) / 1_000_000.0;
        
        logger.log(&format!("{:<25} | {:<15.3} | {:<15.3} | {:<15.3} | {:<15.3}",
                 step_name, wall_median, wall_p99, cpu_median, cpu_p99));
    }
    logger.log(&"=".repeat(100));
}

// Made with Bob
