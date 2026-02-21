use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capability::Capability;
use super::ipc::IpcRouter;
use super::manifest::ExtensionManifest;
use super::{Extension, ExtensionError, OperationDef, OperationResult};

/// JSON-RPC 2.0 request (outgoing to extension).
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC 2.0 response (incoming from extension, for our requests).
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

/// An incoming JSON-RPC request FROM the extension (IPC call).
#[derive(Deserialize)]
struct IncomingRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Value,
    id: Value,
}

/// JSON-RPC response we write back to the extension's stdin.
#[derive(Serialize)]
struct JsonRpcResponseOut {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcErrorOut>,
    id: Value,
}

#[derive(Serialize)]
struct JsonRpcErrorOut {
    code: i64,
    message: String,
}

/// What we read from the extension's stdout: either a response to our request
/// or a new request from the extension (IPC).
enum StdioMessage {
    Response(JsonRpcResponse),
    Request(IncomingRequest),
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
    /// Dedicated storage directory for extension data (passed in initialize).
    data_dir: Option<PathBuf>,
    next_id: AtomicU64,
    /// IPC router for calling other extensions. Set after registration.
    ipc_router: Mutex<Option<Arc<dyn IpcRouter>>>,
    /// Event bus for event.publish / event.subscribe IPC. Set after creation.
    event_bus: Mutex<Option<crate::event_bus::SharedEventBus>>,
    /// Route action executor for dispatching actions from event.publish. Set after creation.
    route_executor: Mutex<Option<crate::event_bus::executor::RouteActionExecutor>>,
    /// Durable event store for persisting events and tracking deliveries. Set after creation.
    event_store: Mutex<Option<crate::event_bus::SharedEventStore>>,
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
            data_dir: None,
            next_id: AtomicU64::new(0),
            ipc_router: Mutex::new(None),
            event_bus: Mutex::new(None),
            route_executor: Mutex::new(None),
            event_store: Mutex::new(None),
        }
    }

    /// Set the data directory for this extension (passed in initialize params).
    pub fn set_data_dir(&mut self, dir: PathBuf) {
        self.data_dir = Some(dir);
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
        let stderr = child.stderr.take();

        let mut handle = ProcessHandle {
            process: child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        };

        // Send initialize message
        let init_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut init_params = serde_json::json!({
            "extension_id": self.manifest.id,
            "version": self.manifest.version,
        });
        if let Some(ref dd) = self.data_dir {
            // Create the data directory if it doesn't exist
            let _ = std::fs::create_dir_all(dd);
            init_params["data_dir"] = serde_json::json!(dd.to_string_lossy());
        }
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0",
            method: "initialize".to_string(),
            params: init_params,
            id: init_id,
        };

        send_request(&mut handle.stdin, &init_request)?;
        log::debug!("Extension '{}': init message sent, waiting for response", self.manifest.id);
        // During init, extensions don't send IPC requests, so use simple read
        let response = match read_response(&mut handle.stdout) {
            Ok(r) => r,
            Err(e) => {
                // Check if the child process already exited
                let exit_status = handle.process.try_wait();
                log::error!(
                    "Extension '{}' failed init: {} (process status: {:?}, binary: {})",
                    self.manifest.id,
                    e,
                    exit_status,
                    self.binary_path.display()
                );
                // Capture stderr for diagnostics
                if let Some(mut se) = stderr {
                    use std::io::Read;
                    let mut err_output = String::new();
                    let _ = se.read_to_string(&mut err_output);
                    if !err_output.is_empty() {
                        log::error!(
                            "Extension '{}' stderr: {}",
                            self.manifest.id,
                            err_output.trim()
                        );
                    }
                }
                return Err(e);
            }
        };

        if let Some(err) = response.error {
            return Err(ExtensionError::Other(format!(
                "Extension '{}' initialization failed: {}",
                self.manifest.id, err.message
            )));
        }

        // Spawn a thread to forward extension stderr to Nexus logs.
        // This keeps the pipe alive so eprintln! in the extension doesn't
        // hit a broken pipe and panic.
        if let Some(stderr) = stderr {
            let ext_id = self.manifest.id.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(text) if !text.is_empty() => {
                            log::debug!("[ext:{}] {}", ext_id, text);
                        }
                        Err(_) => break,
                        _ => {}
                    }
                }
            });
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
    /// Now handles bidirectional communication: if the extension sends IPC
    /// requests while processing our execute call, we handle them inline
    /// before returning the final response.
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

        // Resource CRUD operations use dedicated JSON-RPC methods instead of `execute`
        let (method, params) = if let Some(resource_method) = operation.strip_prefix("__resources_") {
            let method = format!("resources.{}", resource_method);
            (method, input)
        } else {
            let mut params = serde_json::json!({
                "operation": operation,
                "input": input,
            });
            if let Some(caller) = caller_plugin_id {
                params["caller_plugin_id"] = Value::String(caller.to_string());
            }
            ("execute".to_string(), params)
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id: call_id,
        };

        send_request(&mut handle.stdin, &request)?;

        // Snapshot the IPC router (if set) before entering the read loop.
        // We clone it once to avoid re-locking the ipc_router mutex on every iteration.
        let router = self.ipc_router.lock().expect("ipc_router lock poisoned").clone();

        // Read loop: handle IPC requests inline until we get the response
        loop {
            let msg = read_message(&mut handle.stdout)?;

            match msg {
                StdioMessage::Response(response) => {
                    // This is the response to our execute request
                    if let Some(err) = response.error {
                        return Err(ExtensionError::ExecutionFailed(err.message));
                    }

                    let result_value = response.result.unwrap_or(Value::Null);

                    let op_result: OperationResult = serde_json::from_value(result_value.clone())
                        .unwrap_or(OperationResult {
                            success: true,
                            data: result_value,
                            message: None,
                        });

                    return Ok(op_result);
                }
                StdioMessage::Request(ipc_req) => {
                    // Extension is making an IPC call — handle it and write the response
                    let ipc_response = self.handle_ipc_request(&ipc_req, &router);
                    send_response(&mut handle.stdin, ipc_response)?;
                }
            }
        }
    }

    /// Handle an incoming IPC request from the extension process.
    fn handle_ipc_request(
        &self,
        req: &IncomingRequest,
        router: &Option<Arc<dyn IpcRouter>>,
    ) -> JsonRpcResponseOut {
        match req.method.as_str() {
            "call_extension" => {
                let router = match router {
                    Some(r) => r,
                    None => {
                        return JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: None,
                            error: Some(JsonRpcErrorOut {
                                code: -32603,
                                message: "IPC not available (router not wired)".into(),
                            }),
                            id: req.id.clone(),
                        };
                    }
                };

                let target_id = req.params.get("extension_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let operation = req.params.get("operation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let input = req.params.get("input")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                if target_id.is_empty() || operation.is_empty() {
                    return JsonRpcResponseOut {
                        jsonrpc: "2.0",
                        result: None,
                        error: Some(JsonRpcErrorOut {
                            code: -32602,
                            message: "call_extension requires 'extension_id' and 'operation' params".into(),
                        }),
                        id: req.id.clone(),
                    };
                }

                match router.call(&self.id_str, target_id, operation, input) {
                    Ok(result) => {
                        let data = serde_json::to_value(&result).unwrap_or(Value::Null);
                        JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: Some(data),
                            error: None,
                            id: req.id.clone(),
                        }
                    }
                    Err(e) => {
                        JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: None,
                            error: Some(JsonRpcErrorOut {
                                code: -32000,
                                message: e.to_string(),
                            }),
                            id: req.id.clone(),
                        }
                    }
                }
            }
            "list_extensions" => {
                match router {
                    Some(r) => {
                        let list = r.list_extensions();
                        let data = serde_json::to_value(&list).unwrap_or(Value::Null);
                        JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: Some(data),
                            error: None,
                            id: req.id.clone(),
                        }
                    }
                    None => {
                        JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: None,
                            error: Some(JsonRpcErrorOut {
                                code: -32603,
                                message: "IPC not available (router not wired)".into(),
                            }),
                            id: req.id.clone(),
                        }
                    }
                }
            }
            "event.publish" => {
                let bus = self.event_bus.lock().expect("event_bus lock poisoned").clone();
                match bus {
                    Some(bus) => {
                        let publish_req: Result<crate::event_bus::cloud_event::PublishRequest, _> =
                            serde_json::from_value(req.params.clone());
                        match publish_req {
                            Ok(pr) => {
                                let source = format!("nexus://extension/{}", self.id_str);
                                let event = pr.into_cloud_event(source);
                                let event_id = event.id.clone();
                                // Publish synchronously using block_in_place since we hold the process mutex
                                let event_clone = event.clone();
                                let actions = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut bus = bus.write().await;
                                        bus.publish(event)
                                    })
                                });
                                // Dispatch route actions via durable delivery (or fire-and-forget fallback)
                                if !actions.is_empty() {
                                    if let Some(executor) = self.route_executor.lock().expect("route_executor lock").as_ref() {
                                        let store = self.event_store.lock().expect("event_store lock").clone();
                                        if let Some(ref store) = store {
                                            executor.execute_durable(store, actions, &event_clone);
                                        } else {
                                            executor.execute(actions, event_clone);
                                        }
                                    }
                                }
                                JsonRpcResponseOut {
                                    jsonrpc: "2.0",
                                    result: Some(serde_json::json!({"event_id": event_id})),
                                    error: None,
                                    id: req.id.clone(),
                                }
                            }
                            Err(e) => JsonRpcResponseOut {
                                jsonrpc: "2.0",
                                result: None,
                                error: Some(JsonRpcErrorOut {
                                    code: -32602,
                                    message: format!("Invalid event.publish params: {}", e),
                                }),
                                id: req.id.clone(),
                            },
                        }
                    }
                    None => JsonRpcResponseOut {
                        jsonrpc: "2.0",
                        result: None,
                        error: Some(JsonRpcErrorOut {
                            code: -32603,
                            message: "Event bus not available".into(),
                        }),
                        id: req.id.clone(),
                    },
                }
            }
            "event.subscribe" => {
                let bus = self.event_bus.lock().expect("event_bus lock poisoned").clone();
                match bus {
                    Some(bus) => {
                        let type_pattern = req.params.get("type_pattern")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*");
                        let source_pattern = req.params.get("source_pattern")
                            .and_then(|v| v.as_str());

                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let mut bus = bus.write().await;
                                bus.subscribe(
                                    type_pattern,
                                    source_pattern,
                                    crate::event_bus::subscription::SubscriberKind::Extension {
                                        ext_id: self.id_str.clone(),
                                    },
                                )
                            })
                        });

                        match result {
                            Ok((sub_id, _rx)) => {
                                // The receiver will be used for event delivery (future: spawn delivery task)
                                JsonRpcResponseOut {
                                    jsonrpc: "2.0",
                                    result: Some(serde_json::json!({"subscription_id": sub_id})),
                                    error: None,
                                    id: req.id.clone(),
                                }
                            }
                            Err(e) => JsonRpcResponseOut {
                                jsonrpc: "2.0",
                                result: None,
                                error: Some(JsonRpcErrorOut {
                                    code: -32602,
                                    message: format!("Invalid subscription pattern: {}", e),
                                }),
                                id: req.id.clone(),
                            },
                        }
                    }
                    None => JsonRpcResponseOut {
                        jsonrpc: "2.0",
                        result: None,
                        error: Some(JsonRpcErrorOut {
                            code: -32603,
                            message: "Event bus not available".into(),
                        }),
                        id: req.id.clone(),
                    },
                }
            }
            _ => {
                JsonRpcResponseOut {
                    jsonrpc: "2.0",
                    result: None,
                    error: Some(JsonRpcErrorOut {
                        code: -32601,
                        message: format!("Unknown IPC method: {}", req.method),
                    }),
                    id: req.id.clone(),
                }
            }
        }
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

    fn set_ipc_router(&self, router: Arc<dyn IpcRouter>) {
        let mut guard = self.ipc_router.lock().expect("ipc_router lock poisoned");
        *guard = Some(router);
    }

    fn set_event_bus(&self, bus: crate::event_bus::SharedEventBus) {
        let mut guard = self.event_bus.lock().expect("event_bus lock poisoned");
        *guard = Some(bus);
    }

    fn set_route_executor(&self, executor: crate::event_bus::executor::RouteActionExecutor) {
        let mut guard = self.route_executor.lock().expect("route_executor lock poisoned");
        *guard = Some(executor);
    }

    fn set_event_store(&self, store: crate::event_bus::SharedEventStore) {
        let mut guard = self.event_store.lock().expect("event_store lock poisoned");
        *guard = Some(store);
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

/// Write a JSON-RPC response back to the extension's stdin.
fn send_response(writer: &mut BufWriter<ChildStdin>, response: JsonRpcResponseOut) -> Result<(), ExtensionError> {
    let json = serde_json::to_string(&response)
        .map_err(|e| ExtensionError::Protocol(format!("Failed to serialize IPC response: {}", e)))?;

    writer
        .write_all(json.as_bytes())
        .map_err(ExtensionError::Io)?;
    writer
        .write_all(b"\n")
        .map_err(ExtensionError::Io)?;
    writer.flush().map_err(ExtensionError::Io)?;

    Ok(())
}

/// Read a single JSON-RPC response line from the reader (used during init/shutdown
/// where no IPC requests are expected).
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

/// Read a line from stdout and classify it as either a response (has "result" or "error"
/// key) or an incoming request (has "method" key).
fn read_message(reader: &mut BufReader<ChildStdout>) -> Result<StdioMessage, ExtensionError> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(ExtensionError::Io)?;

    if line.is_empty() {
        return Err(ExtensionError::Protocol(
            "Extension process closed stdout (no message)".into(),
        ));
    }

    // Parse as generic JSON first to determine message type
    let raw: Value = serde_json::from_str(&line)
        .map_err(|e| ExtensionError::Protocol(format!("Invalid JSON from extension: {}", e)))?;

    if raw.get("method").is_some() {
        // It's a request from the extension
        let req: IncomingRequest = serde_json::from_value(raw)
            .map_err(|e| ExtensionError::Protocol(format!("Invalid IPC request: {}", e)))?;
        Ok(StdioMessage::Request(req))
    } else {
        // It's a response to our request
        let resp: JsonRpcResponse = serde_json::from_value(raw)
            .map_err(|e| ExtensionError::Protocol(format!("Invalid JSON-RPC response: {}", e)))?;
        Ok(StdioMessage::Response(resp))
    }
}
