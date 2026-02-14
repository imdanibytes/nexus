//! IPC test consumer extension.
//!
//! Calls ipc-provider via IPC to fetch data it doesn't own.
//! Operations:
//! - `fetch_record`    — calls provider's get_record via IPC, returns the result
//! - `fetch_all_keys`  — calls provider's list_keys via IPC
//! - `aggregate`       — calls get_record for multiple keys, sums their values
//! - `discover`        — calls list_extensions to see what's available

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

/// Counter for IPC request IDs (starts at 1000 to avoid collisions with host IDs).
static mut IPC_ID: u64 = 1000;

fn next_ipc_id() -> u64 {
    unsafe {
        IPC_ID += 1;
        IPC_ID
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            _ => {}
        }
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
                let _ = writeln!(out, "{}", serde_json::to_string(&resp).unwrap());
                let _ = out.flush();
                continue;
            }
        };

        let is_shutdown = request.method == "shutdown";
        let response = handle(&request, &mut reader, &mut out);
        let _ = writeln!(out, "{}", serde_json::to_string(&response).unwrap());
        let _ = out.flush();

        if is_shutdown {
            break;
        }
    }
}

fn handle(
    req: &JsonRpcRequest,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> JsonRpcResponse {
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
        "execute" => handle_execute(req, reader, writer),
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

fn handle_execute(
    req: &JsonRpcRequest,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> JsonRpcResponse {
    let operation = req.params.get("operation").and_then(|v| v.as_str()).unwrap_or("");
    let input = req.params.get("input").cloned().unwrap_or_default();

    let result = match operation {
        "fetch_record" => op_fetch_record(&input, reader, writer),
        "fetch_all_keys" => op_fetch_all_keys(reader, writer),
        "aggregate" => op_aggregate(&input, reader, writer),
        "discover" => op_discover(reader, writer),
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

/// Make an IPC call to another extension via the host.
/// Writes the request to stdout, reads the response from stdin.
fn ipc_call(
    extension_id: &str,
    operation: &str,
    input: Value,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Value, String> {
    let id = next_ipc_id();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "call_extension",
        "params": {
            "extension_id": extension_id,
            "operation": operation,
            "input": input,
        },
        "id": id,
    });

    writeln!(writer, "{}", serde_json::to_string(&request).unwrap())
        .map_err(|e| format!("IPC write failed: {}", e))?;
    writer.flush().map_err(|e| format!("IPC flush failed: {}", e))?;

    // Read response from host
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("IPC read failed: {}", e))?;

    let resp: Value = serde_json::from_str(&line)
        .map_err(|e| format!("IPC parse failed: {}", e))?;

    if let Some(err) = resp.get("error") {
        return Err(format!(
            "IPC error: {}",
            err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
        ));
    }

    Ok(resp.get("result").cloned().unwrap_or(Value::Null))
}

/// Make a list_extensions IPC call.
fn ipc_list_extensions(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Vec<Value>, String> {
    let id = next_ipc_id();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "list_extensions",
        "params": {},
        "id": id,
    });

    writeln!(writer, "{}", serde_json::to_string(&request).unwrap())
        .map_err(|e| format!("list_extensions write failed: {}", e))?;
    writer.flush().map_err(|e| format!("list_extensions flush failed: {}", e))?;

    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("list_extensions read failed: {}", e))?;

    let resp: Value = serde_json::from_str(&line)
        .map_err(|e| format!("list_extensions parse failed: {}", e))?;

    if let Some(err) = resp.get("error") {
        return Err(format!(
            "list_extensions error: {}",
            err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
        ));
    }

    resp.get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .ok_or_else(|| "list_extensions returned non-array".into())
}

/// Fetch a single record from ipc-provider.
fn op_fetch_record(
    input: &Value,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Value, String> {
    let key = input.get("key").and_then(|v| v.as_str()).ok_or("missing required field: key")?;

    let result = ipc_call(
        "ipc-provider",
        "get_record",
        serde_json::json!({ "key": key }),
        reader,
        writer,
    )?;

    Ok(serde_json::json!({
        "source": "ipc-provider",
        "operation": "get_record",
        "record": result,
    }))
}

/// Fetch all available keys from ipc-provider.
fn op_fetch_all_keys(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Value, String> {
    let result = ipc_call(
        "ipc-provider",
        "list_keys",
        serde_json::json!({}),
        reader,
        writer,
    )?;

    Ok(serde_json::json!({
        "source": "ipc-provider",
        "operation": "list_keys",
        "keys": result,
    }))
}

/// Fetch multiple records and sum their values.
/// Tests making multiple sequential IPC calls in one execute.
fn op_aggregate(
    input: &Value,
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Value, String> {
    let keys = input.get("keys")
        .and_then(|v| v.as_array())
        .ok_or("missing required field: keys (array of strings)")?;

    let mut total: i64 = 0;
    let mut records = Vec::new();

    for key_val in keys {
        let key = key_val.as_str().ok_or("keys must be strings")?;
        let result = ipc_call(
            "ipc-provider",
            "get_record",
            serde_json::json!({ "key": key }),
            reader,
            writer,
        )?;

        if let Some(val) = result.get("data").and_then(|d| d.get("value")).and_then(|v| v.as_i64()) {
            total += val;
        }
        records.push(result);
    }

    Ok(serde_json::json!({
        "keys_requested": keys.len(),
        "records": records,
        "total_value": total,
    }))
}

/// List all available extensions via IPC.
fn op_discover(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
) -> Result<Value, String> {
    let extensions = ipc_list_extensions(reader, writer)?;

    Ok(serde_json::json!({
        "available_extensions": extensions.len(),
        "extensions": extensions,
    }))
}
