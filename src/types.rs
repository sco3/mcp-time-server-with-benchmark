// Shared types for mcp-time-server
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TimeResult {
    #[serde(rename = "systemTime")]
    pub system_time: String,
}
