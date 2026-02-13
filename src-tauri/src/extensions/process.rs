use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capability::Capability;
use super::manifest::ExtensionManifest;
use super::{Extension, ExtensionError, OperationDef, OperationResult};

/// JSON-RPC 2.0 request.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC 2.0 response (success or error).
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: u64,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// A host extension backed by a child process, communicating via JSON-RPC over stdio.
pub struct ProcessExtension {
    manifest: ExtensionManifest,
    /// Leaked strings so we can return &str from trait methods.
    id_str: String,
    display_name_str: String,
    description_str: String,
    /// The child process handle + IO, protected by a mutex for thread safety.
    child: Mutex<Option<ProcessHandle>>,
    binary_path: PathBuf,
    next_id: AtomicU64,
}

struct ProcessHandle {
    process: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl ProcessExtension {
    /// Create a new ProcessExtension from a manifest and binary path.
    /// The process is NOT started yet — call `start()` to spawn it.
    pub fn new(manifest: ExtensionManifest, binary_path: PathBuf) -> Self {
        Self {
            id_str: manifest.id.clone(),
            display_name_str: manifest.display_name.clone(),
            description_str: manifest.description.clone(),
            manifest,
            child: Mutex::new(None),
            binary_path,
            next_id: AtomicU64::new(0),
        }
    }

    /// Spawn the child process and send the `initialize` message.
    pub fn start(&self) -> Result<(), ExtensionError> {
        let mut child = Command::new(&self.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ExtensionError::Other(format!(
                "Failed to spawn extension '{}': {}",
                self.manifest.id, e
            )))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| ExtensionError::Other("Failed to capture stdin".into()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| ExtensionError::Other("Failed to capture stdout".into()))?;

        let mut handle = ProcessHandle {
            process: child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        };

        // Send initialize message
        let init_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "initialize".to_string(),
            params: serde_json::json!({
                "extension_id": self.manifest.id,
                "version": self.manifest.version,
            }),
            id: init_id,
        };

        send_request(&mut handle.stdin, &init_request)?;
        let response = read_response(&mut handle.stdout)?;

        if let Some(err) = response.error {
            return Err(ExtensionError::Other(format!(
                "Extension '{}' initialization failed: {}",
                self.manifest.id, err.message
            )));
        }

        let mut guard = self.child.lock().expect("process lock poisoned");
        *guard = Some(handle);

        log::info!("Started extension process: {}", self.manifest.id);
        Ok(())
    }

    /// Send a `shutdown` message and wait for the process to exit.
    pub fn stop(&self) -> Result<(), ExtensionError> {
        let mut guard = self.child.lock().expect("process lock poisoned");
        if let Some(mut handle) = guard.take() {
            let shutdown_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let shutdown_request = JsonRpcRequest {
                jsonrpc: "2.0",
                method: "shutdown".to_string(),
                params: serde_json::json!({}),
                id: shutdown_id,
            };

            // Best-effort: send shutdown and wait briefly
            let _ = send_request(&mut handle.stdin, &shutdown_request);
            let _ = read_response(&mut handle.stdout);

            // Give the process 5 seconds to exit, then kill it
            match handle.process.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    match handle.process.try_wait() {
                        Ok(Some(_)) => {}
                        _ => {
                            log::warn!("Extension '{}' did not exit gracefully, killing", self.manifest.id);
                            let _ = handle.process.kill();
                            let _ = handle.process.wait();
                        }
                    }
                }
            }

            log::info!("Stopped extension process: {}", self.manifest.id);
        }
        Ok(())
    }

    /// Check if the child process is running.
    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().expect("process lock poisoned");
        if let Some(handle) = guard.as_mut() {
            match handle.process.try_wait() {
                Ok(None) => true,  // Still running
                _ => {
                    // Process has exited — clean up the handle
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Execute a JSON-RPC call to the child process.
    fn rpc_call(&self, operation: &str, input: Value, caller_plugin_id: Option<&str>) -> Result<OperationResult, ExtensionError> {
        let mut guard = self.child.lock().expect("process lock poisoned");
        let handle = guard.as_mut().ok_or(ExtensionError::ProcessNotRunning)?;

        // Check if process is still alive
        match handle.process.try_wait() {
            Ok(Some(status)) => {
                *guard = None;
                return Err(ExtensionError::Other(format!(
                    "Extension process exited unexpectedly with status: {}",
                    status
                )));
            }
            Err(e) => {
                *guard = None;
                return Err(ExtensionError::Other(format!(
                    "Failed to check process status: {}",
                    e
                )));
            }
            Ok(None) => {} // Still running
        }

        let call_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut params = serde_json::json!({
            "operation": operation,
            "input": input,
        });
        if let Some(caller) = caller_plugin_id {
            params["caller_plugin_id"] = Value::String(caller.to_string());
        }

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "execute".to_string(),
            params,
            id: call_id,
        };

        send_request(&mut handle.stdin, &request)?;
        let response = read_response(&mut handle.stdout)?;

        if let Some(err) = response.error {
            return Err(ExtensionError::ExecutionFailed(err.message));
        }

        let result_value = response.result.unwrap_or(Value::Null);

        // Parse the result as an OperationResult
        let op_result: OperationResult = serde_json::from_value(result_value.clone())
            .unwrap_or(OperationResult {
                success: true,
                data: result_value,
                message: None,
            });

        Ok(op_result)
    }
}

#[async_trait]
impl Extension for ProcessExtension {
    fn id(&self) -> &str {
        &self.id_str
    }

    fn display_name(&self) -> &str {
        &self.display_name_str
    }

    fn description(&self) -> &str {
        &self.description_str
    }

    fn operations(&self) -> Vec<OperationDef> {
        self.manifest.operations.clone()
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.manifest.capabilities.clone()
    }

    async fn execute(&self, operation: &str, input: Value) -> Result<OperationResult, ExtensionError> {
        // The mutex inside rpc_call handles thread safety.
        self.rpc_call(operation, input, None)
    }
}

impl Drop for ProcessExtension {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Write a JSON-RPC request as a single line to the writer.
fn send_request(writer: &mut BufWriter<ChildStdin>, request: &JsonRpcRequest) -> Result<(), ExtensionError> {
    let json = serde_json::to_string(request)
        .map_err(|e| ExtensionError::Protocol(format!("Failed to serialize request: {}", e)))?;

    writer
        .write_all(json.as_bytes())
        .map_err(ExtensionError::Io)?;
    writer
        .write_all(b"\n")
        .map_err(ExtensionError::Io)?;
    writer.flush().map_err(ExtensionError::Io)?;

    Ok(())
}

/// Read a single JSON-RPC response line from the reader.
fn read_response(reader: &mut BufReader<ChildStdout>) -> Result<JsonRpcResponse, ExtensionError> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(ExtensionError::Io)?;

    if line.is_empty() {
        return Err(ExtensionError::Protocol(
            "Extension process closed stdout (no response)".into(),
        ));
    }

    serde_json::from_str(&line)
        .map_err(|e| ExtensionError::Protocol(format!("Invalid JSON-RPC response: {}", e)))
}
