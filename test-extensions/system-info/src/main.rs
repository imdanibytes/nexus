//! Test host extension: system-info
//!
//! JSON-RPC 2.0 over stdin/stdout. Three operations:
//! - `get_info`      — low risk, no scope  → tests basic permission gate
//! - `read_env`      — low risk, scoped    → tests scope enforcement
//! - `shutdown_host` — high risk, no scope → tests runtime approval dialog

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

        let response = handle_request(&request);

        let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap());
        let _ = stdout.flush();

        if request.method == "shutdown" {
            break;
        }
    }
}

fn handle_request(req: &JsonRpcRequest) -> JsonRpcResponse {
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
    let input = req.params.get("input").cloned().unwrap_or(Value::Object(Default::default()));

    let result = match operation {
        "get_info" => op_get_info(),
        "read_env" => op_read_env(&input),
        "shutdown_host" => op_shutdown_host(),
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
            error: Some(JsonRpcError {
                code: -32000,
                message: msg,
            }),
            id: req.id,
        },
    }
}

/// Low risk, no scope — just returns system information.
fn op_get_info() -> Result<Value, String> {
    let sys = sysinfo::System::new_all();

    Ok(serde_json::json!({
        "os": sysinfo::System::name().unwrap_or_default(),
        "os_version": sysinfo::System::os_version().unwrap_or_default(),
        "hostname": sysinfo::System::host_name().unwrap_or_default(),
        "cpu_count": sys.cpus().len(),
        "total_memory_mb": sys.total_memory() / 1_048_576,
        "used_memory_mb": sys.used_memory() / 1_048_576,
        "uptime_secs": sysinfo::System::uptime(),
    }))
}

/// Low risk, scoped by `var_name` — reads an environment variable.
/// Tests scope enforcement: first call to a new var_name triggers approval.
fn op_read_env(input: &Value) -> Result<Value, String> {
    let var_name = input
        .get("var_name")
        .and_then(|v| v.as_str())
        .ok_or("missing required field: var_name")?;

    match std::env::var(var_name) {
        Ok(val) => Ok(serde_json::json!({
            "var_name": var_name,
            "value": val,
            "found": true,
        })),
        Err(_) => Ok(serde_json::json!({
            "var_name": var_name,
            "value": null,
            "found": false,
        })),
    }
}

/// High risk, no scope — pretends to do something dangerous.
/// Tests that the runtime approval dialog fires.
fn op_shutdown_host() -> Result<Value, String> {
    Ok(serde_json::json!({
        "message": "Just kidding! This is a test operation.",
        "would_have_shutdown": false,
    }))
}
