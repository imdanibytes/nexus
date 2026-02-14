use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

use crate::schema;

pub fn run(path: &Path) -> Result<()> {
    println!("Building index.json from: {}", path.display());

    // Read registry metadata
    let registry_yaml = std::fs::read_to_string(path.join("registry.yaml"))
        .context("Failed to read registry.yaml")?;
    let registry_meta: Value =
        serde_yaml::from_str(&registry_yaml).context("Invalid registry.yaml")?;

    // Validate and collect plugins
    let plugin_schema = schema::plugin_schema();
    let plugin_validator = jsonschema::Validator::new(&plugin_schema)
        .context("Failed to compile plugin schema")?;
    let plugins = collect_entries(&path.join("plugins"), &plugin_validator)?;

    // Validate and collect extensions
    let extension_schema = schema::extension_schema();
    let extension_validator = jsonschema::Validator::new(&extension_schema)
        .context("Failed to compile extension schema")?;
    let extensions = collect_entries(&path.join("extensions"), &extension_validator)?;

    // Build registry object
    let mut registry_obj = serde_json::json!({
        "name": registry_meta.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "description": registry_meta.get("description").and_then(|v| v.as_str()).unwrap_or("")
    });
    if let Some(homepage) = registry_meta.get("homepage") {
        registry_obj["homepage"] = homepage.clone();
    }
    if let Some(maintainer) = registry_meta.get("maintainer") {
        registry_obj["maintainer"] = maintainer.clone();
    }

    let index = serde_json::json!({
        "version": 2,
        "registry": registry_obj,
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "plugins": plugins,
        "extensions": extensions
    });

    let output_path = path.join("index.json");
    std::fs::write(&output_path, serde_json::to_string_pretty(&index)?)
        .context("Failed to write index.json")?;

    println!(
        "Built index.json: {} plugins, {} extensions",
        plugins.len(),
        extensions.len()
    );

    Ok(())
}

fn collect_entries(dir: &Path, validator: &jsonschema::Validator) -> Result<Vec<Value>> {
    let mut entries = Vec::new();

    if !dir.exists() {
        return Ok(entries);
    }

    let pattern = dir.join("*.yaml").to_string_lossy().to_string();

    for entry in glob::glob(&pattern)? {
        let file_path = entry?;
        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        let value: Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Invalid YAML in {}", file_path.display()))?;

        // Validate against schema
        let schema_errors: Vec<_> = validator.iter_errors(&value).collect();
        if !schema_errors.is_empty() {
            let errs: Vec<String> = schema_errors.iter().map(|e| e.to_string()).collect();
            anyhow::bail!(
                "Validation failed for {}: {}",
                file_path.display(),
                errs.join("; ")
            );
        }

        // Skip unlisted entries
        if value.get("status").and_then(|v| v.as_str()) == Some("unlisted") {
            continue;
        }

        entries.push(value);
    }

    // Sort by id for deterministic output
    entries.sort_by(|a, b| {
        let id_a = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let id_b = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
        id_a.cmp(id_b)
    });

    Ok(entries)
}
