use anyhow::{bail, Context, Result};
use jsonschema::Validator;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

use crate::schema;

pub fn run(path: &Path) -> Result<()> {
    println!("Validating registry at: {}", path.display());

    let mut errors: Vec<String> = Vec::new();
    let mut plugin_ids: HashSet<String> = HashSet::new();
    let mut extension_ids: HashSet<String> = HashSet::new();

    // Validate plugins
    let plugin_schema = schema::plugin_schema();
    let plugin_validator = Validator::new(&plugin_schema)
        .context("Failed to compile plugin schema")?;

    let plugin_dir = path.join("plugins");
    if plugin_dir.exists() {
        errors.extend(validate_directory(
            &plugin_dir,
            &plugin_validator,
            "plugin",
            &mut plugin_ids,
        )?);
    }

    // Validate extensions
    let extension_schema = schema::extension_schema();
    let extension_validator = Validator::new(&extension_schema)
        .context("Failed to compile extension schema")?;

    let extension_dir = path.join("extensions");
    if extension_dir.exists() {
        errors.extend(validate_directory(
            &extension_dir,
            &extension_validator,
            "extension",
            &mut extension_ids,
        )?);
    }

    // Validate registry.yaml exists
    let registry_path = path.join("registry.yaml");
    if !registry_path.exists() {
        errors.push("registry.yaml not found".to_string());
    }

    if errors.is_empty() {
        println!(
            "All files valid ({} plugins, {} extensions)",
            plugin_ids.len(),
            extension_ids.len()
        );
        Ok(())
    } else {
        for err in &errors {
            eprintln!("  ERROR: {err}");
        }
        bail!("{} validation error(s) found", errors.len());
    }
}

fn validate_directory(
    dir: &Path,
    validator: &Validator,
    kind: &str,
    seen_ids: &mut HashSet<String>,
) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    let pattern = dir.join("*.yaml").to_string_lossy().to_string();

    for entry in glob::glob(&pattern)? {
        let file_path = entry?;
        let file_name = file_path.display().to_string();

        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read {file_name}"))?;

        let yaml_value: Value = match serde_yaml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("{file_name}: invalid YAML: {e}"));
                continue;
            }
        };

        // Schema validation
        let schema_errors: Vec<_> = validator.iter_errors(&yaml_value).collect();
        if !schema_errors.is_empty() {
            for error in &schema_errors {
                errors.push(format!(
                    "{file_name}: {}: {}",
                    error.instance_path, error
                ));
            }
            continue;
        }

        // Duplicate ID check
        if let Some(id) = yaml_value.get("id").and_then(|v| v.as_str()) {
            if !seen_ids.insert(id.to_string()) {
                errors.push(format!(
                    "{file_name}: duplicate {kind} id '{id}'"
                ));
            }
        }
    }

    Ok(errors)
}
