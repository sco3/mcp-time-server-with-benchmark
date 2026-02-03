use axum::{
    body::Body,
    http::{header, StatusCode},
    response::Response,
    routing::post,
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;

// --- Clap Argument Parsing ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the TLS certificate file
    #[arg(long)]
    tls_cert: Option<PathBuf>,
    /// Path to the TLS key file
    #[arg(long)]
    tls_key: Option<PathBuf>,
}

// --- JSON-RPC Request Structures ---

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum JsonRpcRequest {
    WithParams(JsonRpcRequestWithParams),
    WithoutParams(JsonRpcRequestWithoutParams),
    Notification(JsonRpcNotification),
}

#[derive(Deserialize, Debug)]
struct JsonRpcRequestWithParams {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    params: Value,
}

#[derive(Deserialize, Debug)]
struct JsonRpcRequestWithoutParams {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
}

#[derive(Deserialize, Debug)]
struct JsonRpcNotification {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    params: Option<Value>,
}

#[derive(Deserialize, Debug)]
struct InitializeParams {
    #[allow(dead_code)]
    #[serde(rename = "protocolVersion")]
    protocol_version: String,
    #[allow(dead_code)]
    capabilities: Value,
    #[allow(dead_code)]
    #[serde(rename = "clientInfo")]
    client_info: Value,
}

#[derive(Deserialize, Debug)]
struct ToolCallParams {
    name: String,
    arguments: Value,
}

// --- JSON-RPC Response Structures ---

#[derive(Serialize, Debug)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: Value,
    result: T,
}

#[allow(dead_code)]
#[derive(Serialize, Debug)]
struct TimeResult {
    #[serde(rename = "systemTime")]
    system_time: String,
}

// --- JSON-RPC Error Structures ---

#[derive(Serialize, Debug)]
struct JsonRpcErrorResponse {
    jsonrpc: String,
    id: Value,
    error: ErrorObject,
}

#[derive(Serialize, Debug)]
struct ErrorObject {
    code: i32,
    message: String,
}

impl JsonRpcErrorResponse {
    fn new(id: Value, code: i32, message: String) -> Self {
        JsonRpcErrorResponse {
            jsonrpc: "2.0".to_string(),
            id,
            error: ErrorObject { code, message },
        }
    }
}

// Helper function to create an Axum Response with JSON-RPC content
fn create_jsonrpc_response(json_response: &serde_json::Value) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&json_response).unwrap()))
        .unwrap()
}

// --- Axum Handler ---
#[allow(clippy::unused_async)]
async fn mcp_handler(Json(request_value): Json<Value>) -> Response {
    let request: Result<JsonRpcRequest, _> = serde_json::from_value(request_value.clone());

    match request {
        Ok(JsonRpcRequest::WithParams(req)) => handle_request_with_params(req).await,
        Ok(JsonRpcRequest::WithoutParams(req)) => handle_request_without_params(req).await,
        Ok(JsonRpcRequest::Notification(req)) => handle_notification(req).await,
        Err(_) => {
            let id = request_value.get("id").cloned().unwrap_or(Value::Null);
            let error = JsonRpcErrorResponse::new(id, -32700, "Parse error".to_string());
            create_jsonrpc_response(&serde_json::to_value(error).unwrap())
        }
    }
}
#[allow(clippy::unused_async)]
async fn handle_request_with_params(req: JsonRpcRequestWithParams) -> Response {
    match req.method.as_str() {
        "initialize" => process_init(&req),
        "tools/list" => {
            // tools/list can be called with or without params
            let tools = serde_json::json!([
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
            ]);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: serde_json::json!({ "tools": tools }),
            };
            create_jsonrpc_response(&serde_json::to_value(response).unwrap())
        }
        "tools/call" => {
            let params: Result<ToolCallParams, _> = serde_json::from_value(req.params);
            if let Ok(tool_params) = params {
                if tool_params.name == "get_system_time" {
                    #[derive(Deserialize, Debug)]
                    struct ToolArguments {
                        #[serde(default)]
                        timezone: String,
                    }

                    let args: Result<ToolArguments, _> =
                        serde_json::from_value(tool_params.arguments);
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
                            result: serde_json::json!({
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
                    let error =
                        JsonRpcErrorResponse::new(req.id, -32601, "Method not found".to_string());
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

fn process_init(req: &JsonRpcRequestWithParams) -> Response {
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
async fn handle_request_without_params(req: JsonRpcRequestWithoutParams) -> Response {
    if req.method.as_str() == "tools/list" {
        let tools = serde_json::json!([
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
        ]);
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: serde_json::json!({ "tools": tools }),
        };
        create_jsonrpc_response(&serde_json::to_value(response).unwrap())
    } else {
        let error = JsonRpcErrorResponse::new(req.id, -32601, "Method not found".to_string());
        create_jsonrpc_response(&serde_json::to_value(error).unwrap())
    }
}
#[allow(clippy::unused_async)]
async fn handle_notification(_req: JsonRpcNotification) -> Response {
    // Notifications don't require a response, but we return 200 OK with empty body
    // to satisfy HTTP transport requirements
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

// --- Main Function ---

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Build our application with routes for both /mcp and /mcp/
    // This ensures compatibility with wrapper.py which adds trailing slashes
    let app = Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/mcp/", post(mcp_handler));

    // Run our app with hyper on localhost:3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    if let (Some(cert_path), Some(key_path)) = (args.tls_cert, args.tls_key) {
        println!("MCP server listening on https://{addr}");
        let config = RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .unwrap();
        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        println!("MCP server listening on http://{addr}");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}
