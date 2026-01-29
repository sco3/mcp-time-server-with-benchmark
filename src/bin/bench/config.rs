use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BenchConfig {
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    #[serde(default)]
    pub steps: Vec<Step>,
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Step {
    pub name: String,
    
    #[serde(default)]
    pub bench: bool,
    
    #[serde(default = "default_tasks")]
    pub tasks: usize,
    
    pub payload: Value,
}

fn default_tasks() -> usize {
    1
}

impl BenchConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read config file")?;
        
        let config: BenchConfig = toml::from_str(&content)
            .context("Failed to parse TOML config")?;
        
        Ok(config)
    }
}

// Made with Bob
