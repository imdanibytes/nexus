use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use super::capability::Capability;
use super::ipc::IpcRouter;
use super::manifest::ExtensionManifest;
use super::{Extension, ExtensionError, OperationDef, OperationResult};
use crate::event_bus::cloud_event::CloudEvent;

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

/// JSON-RPC 2.0 notification (no `id` field — fire-and-forget to extension).
#[derive(Serialize)]
struct JsonRpcNotification {
    jsonrpc: &'static str,
    method: &'static str,
    params: Value,
}

/// What we read from the extension's stdout: either a response to our request
/// or a new request from the extension (IPC).
enum StdioMessage {
    Response(JsonRpcResponse),
    Request(IncomingRequest),
}

/// A host extension backed by a child process, communicating via JSON-RPC over stdio.
///
/// Stdin and stdout are split into independent locks so that event delivery tasks
/// can write notifications to stdin without blocking ongoing RPC operations.
pub struct ProcessExtension {
    manifest: ExtensionManifest,
    /// Leaked strings so we can return &str from trait methods.
    id_str: String,
    display_name_str: String,
    description_str: String,
    /// Child process handle — only used for lifecycle (try_wait, kill).
    process: Mutex<Option<Child>>,
    /// Stdin writer — shared between rpc_call() and notification delivery tasks.
    stdin: Arc<Mutex<Option<BufWriter<ChildStdin>>>>,
    /// Stdout reader — exclusively used by rpc_call() (one operation at a time).
    stdout: Mutex<Option<BufReader<ChildStdout>>>,
    binary_path: PathBuf,
    /// Dedicated storage directory for extension data (passed in initialize).
    data_dir: Option<PathBuf>,
    next_id: AtomicU64,
    /// IPC router for calling other extensions. Set after registration.
    ipc_router: Mutex<Option<Arc<dyn IpcRouter>>>,
    /// Event bus dispatch facade. Set after creation via set_dispatch().
    dispatch: Mutex<Option<crate::event_bus::Dispatch>>,
    /// Abort handles for active subscription delivery tasks.
    subscription_tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
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
            process: Mutex::new(None),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Mutex::new(None),
            binary_path,
            data_dir: None,
            next_id: AtomicU64::new(0),
            ipc_router: Mutex::new(None),
            dispatch: Mutex::new(None),
            subscription_tasks: Mutex::new(Vec::new()),
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

        let raw_stdin = child.stdin.take()
            .ok_or_else(|| ExtensionError::Other("Failed to capture stdin".into()))?;
        let raw_stdout = child.stdout.take()
            .ok_or_else(|| ExtensionError::Other("Failed to capture stdout".into()))?;
        let stderr = child.stderr.take();

        let mut stdin_writer = BufWriter::new(raw_stdin);
        let mut stdout_reader = BufReader::new(raw_stdout);

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

        send_request(&mut stdin_writer, &init_request)?;
        log::debug!("Extension '{}': init message sent, waiting for response", self.manifest.id);
        // During init, extensions don't send IPC requests, so use simple read
        let response = match read_response(&mut stdout_reader) {
            Ok(r) => r,
            Err(e) => {
                // Check if the child process already exited
                let exit_status = child.try_wait();
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

        // Store handles in their respective fields
        *self.process.lock().expect("process lock poisoned") = Some(child);
        *self.stdin.lock().expect("stdin lock poisoned") = Some(stdin_writer);
        *self.stdout.lock().expect("stdout lock poisoned") = Some(stdout_reader);

        log::info!("Started extension process: {}", self.manifest.id);
        Ok(())
    }

    /// Send a `shutdown` message and wait for the process to exit.
    pub fn stop(&self) -> Result<(), ExtensionError> {
        // Abort all subscription delivery tasks first
        {
            let mut tasks = self.subscription_tasks.lock().expect("sub_tasks lock poisoned");
            for handle in tasks.drain(..) {
                handle.abort();
            }
        }

        // Take all handles
        let stdin_opt = self.stdin.lock().expect("stdin lock poisoned").take();
        let stdout_opt = self.stdout.lock().expect("stdout lock poisoned").take();
        let process_opt = self.process.lock().expect("process lock poisoned").take();

        if let (Some(mut stdin), Some(mut stdout), Some(mut process)) =
            (stdin_opt, stdout_opt, process_opt)
        {
            let shutdown_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let shutdown_request = JsonRpcRequest {
                jsonrpc: "2.0",
                method: "shutdown".to_string(),
                params: serde_json::json!({}),
                id: shutdown_id,
            };

            // Best-effort: send shutdown and wait briefly
            let _ = send_request(&mut stdin, &shutdown_request);
            let _ = read_response(&mut stdout);

            // Give the process 5 seconds to exit, then kill it
            match process.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    match process.try_wait() {
                        Ok(Some(_)) => {}
                        _ => {
                            log::warn!("Extension '{}' did not exit gracefully, killing", self.manifest.id);
                            let _ = process.kill();
                            let _ = process.wait();
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
        let mut guard = self.process.lock().expect("process lock poisoned");
        if let Some(proc) = guard.as_mut() {
            match proc.try_wait() {
                Ok(None) => true,  // Still running
                _ => {
                    // Process has exited — clean up all handles
                    *guard = None;
                    self.stdin.lock().expect("stdin lock poisoned").take();
                    self.stdout.lock().expect("stdout lock poisoned").take();
                    false
                }
            }
        } else {
            false
        }
    }

    /// Execute a JSON-RPC call to the child process.
    /// Handles bidirectional communication: if the extension sends IPC requests
    /// while processing our execute call, we handle them inline before returning
    /// the final response.
    ///
    /// Stdin and stdout use separate locks so that event delivery tasks can
    /// write notifications concurrently without blocking the operation.
    fn rpc_call(&self, operation: &str, input: Value, caller_plugin_id: Option<&str>) -> Result<OperationResult, ExtensionError> {
        // Check if process is still alive
        {
            let mut proc_guard = self.process.lock().expect("process lock poisoned");
            match proc_guard.as_mut() {
                None => return Err(ExtensionError::ProcessNotRunning),
                Some(proc) => {
                    match proc.try_wait() {
                        Ok(Some(status)) => {
                            proc_guard.take();
                            self.stdin.lock().expect("stdin lock poisoned").take();
                            self.stdout.lock().expect("stdout lock poisoned").take();
                            return Err(ExtensionError::Other(format!(
                                "Extension process exited unexpectedly with status: {}",
                                status
                            )));
                        }
                        Err(e) => {
                            proc_guard.take();
                            self.stdin.lock().expect("stdin lock poisoned").take();
                            self.stdout.lock().expect("stdout lock poisoned").take();
                            return Err(ExtensionError::Other(format!(
                                "Failed to check process status: {}",
                                e
                            )));
                        }
                        Ok(None) => {} // Still running
                    }
                }
            }
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

        // Write request — brief stdin lock
        {
            let mut stdin_guard = self.stdin.lock().expect("stdin lock poisoned");
            let stdin = stdin_guard.as_mut().ok_or(ExtensionError::ProcessNotRunning)?;
            send_request(stdin, &request)?;
        }

        // Snapshot the IPC router (if set) before entering the read loop.
        let router = self.ipc_router.lock().expect("ipc_router lock poisoned").clone();

        // Read loop — hold stdout lock for the duration
        let mut stdout_guard = self.stdout.lock().expect("stdout lock poisoned");
        let stdout = stdout_guard.as_mut().ok_or(ExtensionError::ProcessNotRunning)?;

        loop {
            let msg = read_message(stdout)?;

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
                    // Brief stdin lock for IPC response
                    let mut stdin_guard = self.stdin.lock().expect("stdin lock poisoned");
                    if let Some(stdin) = stdin_guard.as_mut() {
                        send_response(stdin, ipc_response)?;
                    } else {
                        return Err(ExtensionError::ProcessNotRunning);
                    }
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
                let dispatch = self.dispatch.lock().expect("dispatch lock poisoned").clone();
                match dispatch {
                    Some(dispatch) => {
                        let publish_req: Result<crate::event_bus::cloud_event::PublishRequest, _> =
                            serde_json::from_value(req.params.clone());
                        match publish_req {
                            Ok(pr) => {
                                let source = format!("nexus://extension/{}", self.id_str);
                                let event = pr.into_cloud_event(source);
                                let event_id = event.id.clone();
                                let event_clone = event.clone();
                                let actions = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        let mut bus = dispatch.bus.write().await;
                                        bus.publish(event)
                                    })
                                });
                                if !actions.is_empty() {
                                    dispatch.executor.execute_durable(
                                        &dispatch.store,
                                        actions,
                                        &event_clone,
                                    );
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
                let dispatch = self.dispatch.lock().expect("dispatch lock poisoned").clone();
                match dispatch {
                    Some(dispatch) => {
                        let type_pattern = req.params.get("type_pattern")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*");
                        let source_pattern = req.params.get("source_pattern")
                            .and_then(|v| v.as_str());

                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let mut bus = dispatch.bus.write().await;
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
                            Ok((sub_id, rx)) => {
                                // Spawn a delivery task that pushes matching events
                                // to the extension's stdin as JSON-RPC notifications.
                                let stdin = self.stdin.clone();
                                let sub_id_clone = sub_id.clone();
                                let handle = tokio::spawn(async move {
                                    deliver_events(stdin, sub_id_clone, rx).await;
                                });
                                self.subscription_tasks.lock()
                                    .expect("sub_tasks lock poisoned")
                                    .push(handle);

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
            "event.unsubscribe" => {
                let dispatch = self.dispatch.lock().expect("dispatch lock poisoned").clone();
                match dispatch {
                    Some(dispatch) => {
                        let sub_id = req.params.get("subscription_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if sub_id.is_empty() {
                            return JsonRpcResponseOut {
                                jsonrpc: "2.0",
                                result: None,
                                error: Some(JsonRpcErrorOut {
                                    code: -32602,
                                    message: "event.unsubscribe requires 'subscription_id' param".into(),
                                }),
                                id: req.id.clone(),
                            };
                        }

                        // Remove the subscription from the bus. The sender is dropped,
                        // which causes the delivery task's rx.recv() to return None,
                        // gracefully exiting the task.
                        tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                let mut bus = dispatch.bus.write().await;
                                bus.unsubscribe(sub_id);
                            })
                        });

                        JsonRpcResponseOut {
                            jsonrpc: "2.0",
                            result: Some(serde_json::json!({"ok": true})),
                            error: None,
                            id: req.id.clone(),
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
        // The mutexes inside rpc_call handle thread safety.
        self.rpc_call(operation, input, None)
    }

    fn set_ipc_router(&self, router: Arc<dyn IpcRouter>) {
        let mut guard = self.ipc_router.lock().expect("ipc_router lock poisoned");
        *guard = Some(router);
    }

    fn set_dispatch(&self, dispatch: crate::event_bus::Dispatch) {
        let mut guard = self.dispatch.lock().expect("dispatch lock poisoned");
        *guard = Some(dispatch);
    }
}

impl Drop for ProcessExtension {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Background task that receives events from the bus and pushes them
/// to the extension's stdin as JSON-RPC notifications.
async fn deliver_events(
    stdin: Arc<Mutex<Option<BufWriter<ChildStdin>>>>,
    sub_id: String,
    mut rx: mpsc::UnboundedReceiver<CloudEvent>,
) {
    while let Some(event) = rx.recv().await {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0",
            method: "event.received",
            params: serde_json::json!({
                "subscription_id": sub_id,
                "event": event,
            }),
        };
        let mut guard = stdin.lock().expect("stdin lock");
        if let Some(writer) = guard.as_mut() {
            if send_notification(writer, &notification).is_err() {
                log::warn!("Event delivery failed for sub {}, stopping", sub_id);
                break;
            }
        } else {
            // stdin gone — extension stopped
            break;
        }
    }
    log::debug!("Event delivery task for sub {} exiting", sub_id);
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

/// Write a JSON-RPC notification to the extension's stdin (no response expected).
fn send_notification(writer: &mut BufWriter<ChildStdin>, notification: &JsonRpcNotification) -> Result<(), ExtensionError> {
    let json = serde_json::to_string(notification)
        .map_err(|e| ExtensionError::Protocol(format!("Failed to serialize notification: {}", e)))?;

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
