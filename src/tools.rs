// Tool descriptions and core tool logic for time
use serde_json::json;

pub fn get_tools_description_json() -> serde_json::Value {
    json!([
        {
            "name": "get_system_time",
            "description": "Get current system time in specified timezone",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone name (e.g., 'America/New_York', 'Europe/London'). Defaults to UTC"
                    }
                }
            },
            "annotations": {
                "title": "Get System Time",
                "readOnlyHint": true,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": false
            }
        }
    ])
}
