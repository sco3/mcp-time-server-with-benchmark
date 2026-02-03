// JSON-RPC types and error helpers
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum JsonRpcRequest {
    WithParams(JsonRpcRequestWithParams),
    WithoutParams(JsonRpcRequestWithoutParams),
    Notification(JsonRpcNotification),
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonRpcRequestWithParams {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Value,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonRpcRequestWithoutParams {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: Value,
    #[serde(rename = "clientInfo")]
    pub client_info: Value,
}

#[derive(Deserialize, Debug)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

#[derive(Serialize, Debug)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: Value,
    pub result: T,
}

#[derive(Serialize, Debug)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub error: ErrorObject,
}

#[derive(Serialize, Debug)]
pub struct ErrorObject {
    pub code: i32,
    pub message: String,
}

impl JsonRpcErrorResponse {
    pub fn new(id: Value, code: i32, message: String) -> Self {
        JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject { code, message },
        }
    }
}
