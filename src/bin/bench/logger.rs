use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};

// Logger for writing to both console and file
pub struct Logger {
    file: Option<Arc<Mutex<File>>>,
}

impl Logger {
    pub fn new(log_file: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let file = if let Some(path) = log_file {
            let f = File::create(path)?;
            Some(Arc::new(Mutex::new(f)))
        } else {
            None
        };
        Ok(Logger { file })
    }

    pub fn log(&self, message: &str) {
        // Print to console
        println!("{}", message);
        
        // Write to file if enabled
        if let Some(file) = &self.file {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{}", message);
            }
        }
    }

    pub fn log_error(&self, step_name: &str, error_msg: &str) {
        let message = format!("ERROR in {}: {}", step_name, error_msg);
        self.log(&message);
    }

    pub fn log_response(&self, step_name: &str, response: &Value) {
        let message = format!("Response for {}: {}", step_name, serde_json::to_string_pretty(response).unwrap_or_else(|_| "Invalid JSON".to_string()));
        
        // Print to console (abbreviated)
        if response.get("error").is_some() {
            println!("ERROR Response for {}", step_name);
        }
        
        // Write full response to file if enabled
        if let Some(file) = &self.file {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{}", message);
            }
        }
    }
}

// Made with Bob
