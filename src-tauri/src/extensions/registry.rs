use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use super::ipc::IpcRouter;
use super::{Capability, Extension, OperationDef};

/// Stores all registered extensions, built at startup.
pub struct ExtensionRegistry {
    extensions: HashMap<String, Arc<dyn Extension>>,
    /// Stored so newly registered extensions get the router automatically.
    ipc_router: Option<Arc<dyn IpcRouter>>,
}

/// Serializable summary of an extension for the list API.
#[derive(Debug, Serialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub operations: Vec<OperationDef>,
    pub capabilities: Vec<Capability>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            ipc_router: None,
        }
    }

    /// Register an extension. Panics if an extension with the same ID is already registered.
    pub fn register(&mut self, ext: Arc<dyn Extension>) {
        let id = ext.id().to_string();
        if self.extensions.contains_key(&id) {
            panic!("Duplicate extension ID: {}", id);
        }
        // Inject IPC router if we have one
        if let Some(ref router) = self.ipc_router {
            ext.set_ipc_router(router.clone());
        }
        log::info!("Registered extension: {} ({})", ext.display_name(), id);
        self.extensions.insert(id, ext);
    }

    /// Unregister an extension by ID. Returns true if it was removed.
    pub fn unregister(&mut self, id: &str) -> bool {
        self.extensions.remove(id).is_some()
    }

    /// Look up an extension by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Extension> {
        self.extensions.get(id).map(|a| a.as_ref())
    }

    /// Get a cloneable Arc reference to an extension (for lock-free execution).
    pub fn get_arc(&self, id: &str) -> Option<Arc<dyn Extension>> {
        self.extensions.get(id).cloned()
    }

    /// Store the IPC router and propagate it to all registered extensions.
    pub fn set_ipc_router(&mut self, router: Arc<dyn IpcRouter>) {
        for ext in self.extensions.values() {
            ext.set_ipc_router(router.clone());
        }
        self.ipc_router = Some(router);
    }

    /// List all registered extensions with their operations.
    pub fn list(&self) -> Vec<ExtensionInfo> {
        let mut result: Vec<_> = self.extensions.values().map(|ext| ExtensionInfo {
            id: ext.id().to_string(),
            display_name: ext.display_name().to_string(),
            description: ext.description().to_string(),
            operations: ext.operations(),
            capabilities: ext.capabilities(),
        }).collect();
        result.sort_by(|a, b| a.id.cmp(&b.id));
        result
    }

    /// Generate the permission string for an extension operation.
    /// Format: "ext:{extension_id}:{operation_name}"
    pub fn permission_string(ext_id: &str, operation: &str) -> String {
        format!("ext:{}:{}", ext_id, operation)
    }

    /// Get all permission strings for all operations across all extensions.
    pub fn all_permission_strings(&self) -> Vec<String> {
        self.extensions.values().flat_map(|ext| {
            let id = ext.id().to_string();
            ext.operations().into_iter().map(move |op| {
                Self::permission_string(&id, &op.name)
            })
        }).collect()
    }

    /// Check if an extension+operation pair is valid.
    pub fn has_operation(&self, ext_id: &str, operation: &str) -> bool {
        self.extensions.get(ext_id).is_some_and(|ext| {
            ext.operations().iter().any(|op| op.name == operation)
        })
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
