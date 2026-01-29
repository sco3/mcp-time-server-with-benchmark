use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Stats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub median: Option<Duration>,
    pub percentile_99: Option<Duration>,
    pub std_deviation: Option<Duration>,
    pub min: Option<Duration>,
    pub max: Option<Duration>,
}

impl Stats {
    pub fn calculate(mut durations: Vec<Duration>) -> Self {
        let total_tasks = durations.len();
        
        if durations.is_empty() {
            return Stats {
                total_tasks: 0,
                successful_tasks: 0,
                failed_tasks: 0,
                median: None,
                percentile_99: None,
                std_deviation: None,
                min: None,
                max: None,
            };
        }
        
        durations.sort();
        
        let median = calculate_percentile(&durations, 50.0);
        let percentile_99 = calculate_percentile(&durations, 99.0);
        let std_deviation = calculate_std_deviation(&durations);
        let min = durations.first().copied();
        let max = durations.last().copied();
        
        Stats {
            total_tasks,
            successful_tasks: total_tasks,
            failed_tasks: 0,
            median: Some(median),
            percentile_99: Some(percentile_99),
            std_deviation: Some(std_deviation),
            min,
            max,
        }
    }
    
    pub fn with_failed_count(mut self, failed: usize) -> Self {
        self.failed_tasks = failed;
        self.total_tasks = self.successful_tasks + failed;
        self
    }
}

fn calculate_percentile(sorted_durations: &[Duration], percentile: f64) -> Duration {
    if sorted_durations.is_empty() {
        return Duration::from_secs(0);
    }
    
    let index = (percentile / 100.0 * (sorted_durations.len() - 1) as f64).round() as usize;
    sorted_durations[index]
}

fn calculate_std_deviation(durations: &[Duration]) -> Duration {
    if durations.len() <= 1 {
        return Duration::from_secs(0);
    }
    
    let mean_nanos: f64 = durations.iter()
        .map(|d| d.as_nanos() as f64)
        .sum::<f64>() / durations.len() as f64;
    
    let variance: f64 = durations.iter()
        .map(|d| {
            let diff = d.as_nanos() as f64 - mean_nanos;
            diff * diff
        })
        .sum::<f64>() / durations.len() as f64;
    
    Duration::from_nanos(variance.sqrt() as u64)
}

pub fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1000 {
        format!("{}μs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2}ms", micros as f64 / 1000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

// Made with Bob
