use std::collections::HashMap;
use serde::Serialize;
use super::{Extension, OperationDef};

/// Stores all registered extensions, built at startup.
pub struct ExtensionRegistry {
    extensions: HashMap<String, Box<dyn Extension>>,
}

/// Serializable summary of an extension for the list API.
#[derive(Debug, Serialize)]
pub struct ExtensionInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub operations: Vec<OperationDef>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    /// Register an extension. Panics if an extension with the same ID is already registered.
    pub fn register(&mut self, ext: Box<dyn Extension>) {
        let id = ext.id().to_string();
        if self.extensions.contains_key(&id) {
            panic!("Duplicate extension ID: {}", id);
        }
        log::info!("Registered extension: {} ({})", ext.display_name(), id);
        self.extensions.insert(id, ext);
    }

    /// Look up an extension by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Extension> {
        self.extensions.get(id).map(|b| b.as_ref())
    }

    /// List all registered extensions with their operations.
    pub fn list(&self) -> Vec<ExtensionInfo> {
        let mut result: Vec<_> = self.extensions.values().map(|ext| ExtensionInfo {
            id: ext.id().to_string(),
            display_name: ext.display_name().to_string(),
            description: ext.description().to_string(),
            operations: ext.operations(),
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
            let id = ext.id();
            ext.operations().into_iter().map(move |op| {
                Self::permission_string(id, &op.name)
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
