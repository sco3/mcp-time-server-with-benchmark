// Axum handlers for MCP server
use axum::{Json, response::Response, http::{StatusCode, header}, body::Body};
use serde_json::{Value, json};
use chrono::{Utc, DateTime};

use crate::rpc::{JsonRpcRequest, JsonRpcErrorResponse, JsonRpcRequestWithParams, JsonRpcResponse, ToolCallParams, InitializeParams, JsonRpcRequestWithoutParams, JsonRpcNotification};
use crate::tools::get_tools_description_json;

#[allow(clippy::unused_async)]
pub async fn mcp_handler(Json(request_value): Json<Value>) -> Response {
    let id = request_value.get("id").cloned().unwrap_or(Value::Null);
    let request: Result<JsonRpcRequest, _> = serde_json::from_value(request_value);
    match request {
        Ok(JsonRpcRequest::WithParams(req)) => handle_request_with_params(req).await,
        Ok(JsonRpcRequest::WithoutParams(req)) => handle_request_without_params(req).await,
        Ok(JsonRpcRequest::Notification(req)) => handle_notification(req).await,
        Err(_) => {
            let error = JsonRpcErrorResponse::new(id, -32700, "Parse error".to_string());
            create_jsonrpc_response(&serde_json::to_value(error).unwrap())
        }
    }
}

#[allow(clippy::unused_async)]
pub async fn handle_request_with_params(req: JsonRpcRequestWithParams) -> Response {
    match req.method.as_str() {
        "initialize" => process_init(&req),
        "tools/list" => {
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: json!({ "tools": get_tools_description_json() }),
            };
            create_jsonrpc_response(&serde_json::to_value(response).unwrap())
        }
        "tools/call" => {
            let params: Result<ToolCallParams, _> = serde_json::from_value(req.params);
            if let Ok(tool_params) = params {
                if tool_params.name == "get_system_time" {
                    #[derive(serde::Deserialize, Debug)]
                    struct ToolArguments {
                        #[serde(default)]
                        timezone: String,
                    }
                    let args: Result<ToolArguments, _> = serde_json::from_value(tool_params.arguments);
                    if let Ok(args) = args {
                        if !args.timezone.is_empty() && args.timezone.to_uppercase() != "UTC" {
                            let error = JsonRpcErrorResponse::new(
                                req.id.clone(),
                                -32602,
                                "Invalid params: only 'UTC' timezone is supported".to_string(),
                            );
                            return create_jsonrpc_response(&serde_json::to_value(error).unwrap());
                        }
                        let now: DateTime<Utc> = Utc::now();
                        let time_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
                        let response = JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": time_str
                                    }
                                ],
                                "isError": false
                            }),
                        };
                        create_jsonrpc_response(&serde_json::to_value(response).unwrap())
                    } else {
                        let error = JsonRpcErrorResponse::new(
                            req.id,
                            -32602,
                            "Invalid params for get_system_time".to_string(),
                        );
                        create_jsonrpc_response(&serde_json::to_value(error).unwrap())
                    }
                } else {
                    let error = JsonRpcErrorResponse::new(req.id, -32601, "Method not found".to_string());
                    create_jsonrpc_response(&serde_json::to_value(error).unwrap())
                }
            } else {
                let error = JsonRpcErrorResponse::new(
                    req.id,
                    -32602,
                    "Invalid params for tools/call".to_string(),
                );
                create_jsonrpc_response(&serde_json::to_value(error).unwrap())
            }
        }
        _ => {
            let error = JsonRpcErrorResponse::new(req.id, -32601, "Method not found".to_string());
            create_jsonrpc_response(&serde_json::to_value(error).unwrap())
        }
    }
}

pub fn process_init(req: &JsonRpcRequestWithParams) -> Response {
    let params: Result<InitializeParams, _> = serde_json::from_value(req.params.clone());
    match params {
        Ok(_params) => {
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id.clone(),
                result: serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "mcp-time-server",
                        "version": "0.1.0"
                    }
                }),
            };
            create_jsonrpc_response(&serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            let error = JsonRpcErrorResponse::new(
                req.id.clone(),
                -32602,
                format!("Invalid params for initialize: {e}"),
            );
            create_jsonrpc_response(&serde_json::to_value(error).unwrap())
        }
    }
}

#[allow(clippy::unused_async)]
pub async fn handle_request_without_params(req: JsonRpcRequestWithoutParams) -> Response {
    if req.method.as_str() == "tools/list" {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: json!({ "tools": get_tools_description_json() }),
        };
        create_jsonrpc_response(&serde_json::to_value(response).unwrap())
    } else {
        let error = JsonRpcErrorResponse::new(req.id, -32601, "Method not found".to_string());
        create_jsonrpc_response(&serde_json::to_value(error).unwrap())
    }
}

#[allow(clippy::unused_async)]
pub async fn handle_notification(_req: JsonRpcNotification) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

// Helper function (copy from main.rs):
pub fn create_jsonrpc_response(json_response: &serde_json::Value) -> Response {
    match serde_json::to_string(json_response) {
        Ok(json_string) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_string))
            .unwrap_or_else(|_| Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(r#"{"error":"failed to build response"}"#))
                .unwrap_or_else(|_| Response::new(Body::from("{\"error\":\"critical failure\"}")))),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(r#"{"error":"failed to serialize response"}"#))
            .unwrap_or_else(|_| Response::new(Body::from("{\"error\":\"critical failure\"}"))),
    }
}
