use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct BenchConfig {
    pub steps: Vec<Step>,
}

#[derive(Deserialize)]
pub struct Step {
    pub name: String,
    pub payload: Value,
    #[serde(default = "default_bench")]
    pub bench: bool,
    #[serde(default)]
    pub tasks: Option<usize>,
}

fn default_bench() -> bool {
    true
}

#[derive(Deserialize, Clone)]
pub struct AppCommand {
    pub bin: String,
    pub args: Vec<String>,
}

// Made with Bob
