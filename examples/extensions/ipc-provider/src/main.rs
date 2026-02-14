//! IPC test provider extension.
//!
//! Simple data store that other extensions call via IPC.
//! Operations:
//! - `get_record` — returns a record by key
//! - `list_keys`  — returns all available keys

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: u64,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                    id: 0,
                };
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let response = handle(&request);
        let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap());
        let _ = stdout.flush();

        if request.method == "shutdown" {
            break;
        }
    }
}

fn handle(req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            result: Some(serde_json::json!({ "ready": true })),
            error: None,
            id: req.id,
        },
        "shutdown" => JsonRpcResponse {
            jsonrpc: "2.0",
            result: Some(serde_json::json!({})),
            error: None,
            id: req.id,
        },
        "execute" => handle_execute(req),
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Unknown method: {}", req.method),
            }),
            id: req.id,
        },
    }
}

fn handle_execute(req: &JsonRpcRequest) -> JsonRpcResponse {
    let operation = req.params.get("operation").and_then(|v| v.as_str()).unwrap_or("");
    let input = req.params.get("input").cloned().unwrap_or_default();

    let result = match operation {
        "get_record" => op_get_record(&input),
        "list_keys" => op_list_keys(),
        _ => Err(format!("Unknown operation: {}", operation)),
    };

    match result {
        Ok(data) => JsonRpcResponse {
            jsonrpc: "2.0",
            result: Some(serde_json::json!({
                "success": true,
                "data": data,
                "message": null
            })),
            error: None,
            id: req.id,
        },
        Err(msg) => JsonRpcResponse {
            jsonrpc: "2.0",
            result: None,
            error: Some(JsonRpcError { code: -32000, message: msg }),
            id: req.id,
        },
    }
}

fn op_get_record(input: &Value) -> Result<Value, String> {
    let key = input.get("key").and_then(|v| v.as_str()).ok_or("missing required field: key")?;

    // Hardcoded test data
    let record = match key {
        "alpha" => serde_json::json!({ "key": "alpha", "value": 42, "label": "The answer" }),
        "beta" => serde_json::json!({ "key": "beta", "value": 7, "label": "Lucky number" }),
        "gamma" => serde_json::json!({ "key": "gamma", "value": 100, "label": "Century" }),
        _ => return Err(format!("Record not found: {}", key)),
    };

    Ok(record)
}

fn op_list_keys() -> Result<Value, String> {
    Ok(serde_json::json!(["alpha", "beta", "gamma"]))
}
