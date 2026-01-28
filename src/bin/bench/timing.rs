// For CPU time measurement
pub fn get_cpu_time_ns() -> i64 {
    unsafe {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts);
        ts.tv_sec * 1_000_000_000 + ts.tv_nsec
    }
}

// Convert nanoseconds to milliseconds with decimal precision
pub fn ns_to_ms_str(ns: u128) -> String {
    let ms = ns as f64 / 1_000_000.0;
    format!("{:.3}", ms)
}

// Calculate median from a sorted vector
pub fn calculate_median(sorted_values: &[u128]) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let len = sorted_values.len();
    if len.is_multiple_of(2) {
        (sorted_values[len / 2 - 1] + sorted_values[len / 2]) as f64 / 2.0
    } else {
        sorted_values[len / 2] as f64
    }
}

// Calculate 99th percentile from a sorted vector
pub fn calculate_p99(sorted_values: &[u128]) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let len = sorted_values.len();
    let index = ((len as f64 * 0.99).ceil() as usize).saturating_sub(1).min(len - 1);
    sorted_values[index] as f64
}

// Structure to hold timing data for a step
#[derive(Default)]
pub struct StepTimings {
    pub wall_times: Vec<u128>,
    pub cpu_times: Vec<u128>,
}

// Made with Bob
