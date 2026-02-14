use anyhow::{bail, Context, Result};
use dialoguer::{Input, Select};
use regex::Regex;
use serde_json::Value;
use std::path::Path;

const ID_PATTERN: &str = r"^[a-z][a-z0-9]*\.[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$";
const VERSION_PATTERN: &str = r"^\d+\.\d+\.\d+";

fn validate_id(input: &String) -> Result<(), String> {
    let re = Regex::new(ID_PATTERN).unwrap();
    if re.is_match(input) {
        Ok(())
    } else {
        Err("Must be reverse-domain format (e.g. com.example.my-plugin)".to_string())
    }
}

fn validate_version(input: &String) -> Result<(), String> {
    let re = Regex::new(VERSION_PATTERN).unwrap();
    if re.is_match(input) {
        Ok(())
    } else {
        Err("Must be semver format (e.g. 1.0.0)".to_string())
    }
}

fn validate_nonempty(input: &String) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Cannot be empty".to_string())
    } else {
        Ok(())
    }
}

pub fn run_plugin() -> Result<()> {
    println!("Add a new plugin to the registry\n");

    let id: String = Input::new()
        .with_prompt("Plugin ID (reverse-domain)")
        .validate_with(validate_id)
        .interact_text()?;

    let name: String = Input::new()
        .with_prompt("Display name")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let version: String = Input::new()
        .with_prompt("Version")
        .default("0.1.0".to_string())
        .validate_with(validate_version)
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let author: String = Input::new()
        .with_prompt("Author")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let license: String = Input::new()
        .with_prompt("License")
        .default("MIT".to_string())
        .interact_text()?;

    let homepage: String = Input::new()
        .with_prompt("Homepage URL (optional)")
        .default(String::new())
        .interact_text()?;

    let image: String = Input::new()
        .with_prompt("Docker image")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let manifest_url: String = Input::new()
        .with_prompt("Manifest URL")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let categories_input: String = Input::new()
        .with_prompt("Categories (comma-separated, optional)")
        .default(String::new())
        .interact_text()?;

    let status_options = &["active", "deprecated", "unlisted"];
    let status_idx = Select::new()
        .with_prompt("Status")
        .items(status_options)
        .default(0)
        .interact()?;

    // Build the YAML content
    let mut plugin = serde_json::Map::new();
    plugin.insert("id".into(), Value::String(id.clone()));
    plugin.insert("name".into(), Value::String(name));
    plugin.insert("version".into(), Value::String(version));
    plugin.insert("description".into(), Value::String(description));
    plugin.insert("author".into(), Value::String(author));
    if !license.is_empty() {
        plugin.insert("license".into(), Value::String(license));
    }
    if !homepage.is_empty() {
        plugin.insert("homepage".into(), Value::String(homepage));
    }
    plugin.insert("image".into(), Value::String(image));
    plugin.insert("manifest_url".into(), Value::String(manifest_url));
    plugin.insert(
        "status".into(),
        Value::String(status_options[status_idx].to_string()),
    );

    if !categories_input.is_empty() {
        let categories: Vec<Value> = categories_input
            .split(',')
            .map(|s| Value::String(s.trim().to_string()))
            .collect();
        plugin.insert("categories".into(), Value::Array(categories));
    }

    let yaml_content = serde_yaml::to_string(&plugin)?;
    let output_path = Path::new("plugins").join(format!("{id}.yaml"));

    if output_path.exists() {
        bail!("File already exists: {}", output_path.display());
    }

    std::fs::create_dir_all("plugins")?;
    std::fs::write(&output_path, &yaml_content)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("\nCreated {}", output_path.display());
    Ok(())
}

pub fn run_extension() -> Result<()> {
    println!("Add a new extension to the registry\n");

    let id: String = Input::new()
        .with_prompt("Extension ID (reverse-domain)")
        .validate_with(validate_id)
        .interact_text()?;

    let name: String = Input::new()
        .with_prompt("Display name")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let version: String = Input::new()
        .with_prompt("Version")
        .default("0.1.0".to_string())
        .validate_with(validate_version)
        .interact_text()?;

    let description: String = Input::new()
        .with_prompt("Description")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let author: String = Input::new()
        .with_prompt("Author")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let license: String = Input::new()
        .with_prompt("License")
        .default("MIT".to_string())
        .interact_text()?;

    let homepage: String = Input::new()
        .with_prompt("Homepage URL (optional)")
        .default(String::new())
        .interact_text()?;

    let manifest_url: String = Input::new()
        .with_prompt("Manifest URL")
        .validate_with(validate_nonempty)
        .interact_text()?;

    let platform_options = &[
        "windows-x86_64",
        "macos-x86_64",
        "macos-aarch64",
        "linux-x86_64",
        "linux-aarch64",
    ];
    let platforms_input: String = Input::new()
        .with_prompt(format!(
            "Platforms (comma-separated, options: {})",
            platform_options.join(", ")
        ))
        .default(String::new())
        .interact_text()?;

    let categories_input: String = Input::new()
        .with_prompt("Categories (comma-separated, optional)")
        .default(String::new())
        .interact_text()?;

    let status_options = &["active", "deprecated", "unlisted"];
    let status_idx = Select::new()
        .with_prompt("Status")
        .items(status_options)
        .default(0)
        .interact()?;

    // Build the YAML content
    let mut extension = serde_json::Map::new();
    extension.insert("id".into(), Value::String(id.clone()));
    extension.insert("name".into(), Value::String(name));
    extension.insert("version".into(), Value::String(version));
    extension.insert("description".into(), Value::String(description));
    extension.insert("author".into(), Value::String(author));
    if !license.is_empty() {
        extension.insert("license".into(), Value::String(license));
    }
    if !homepage.is_empty() {
        extension.insert("homepage".into(), Value::String(homepage));
    }
    extension.insert("manifest_url".into(), Value::String(manifest_url));
    extension.insert(
        "status".into(),
        Value::String(status_options[status_idx].to_string()),
    );

    if !platforms_input.is_empty() {
        let platforms: Vec<Value> = platforms_input
            .split(',')
            .map(|s| Value::String(s.trim().to_string()))
            .collect();
        extension.insert("platforms".into(), Value::Array(platforms));
    }

    if !categories_input.is_empty() {
        let categories: Vec<Value> = categories_input
            .split(',')
            .map(|s| Value::String(s.trim().to_string()))
            .collect();
        extension.insert("categories".into(), Value::Array(categories));
    }

    let yaml_content = serde_yaml::to_string(&extension)?;
    let output_path = Path::new("extensions").join(format!("{id}.yaml"));

    if output_path.exists() {
        bail!("File already exists: {}", output_path.display());
    }

    std::fs::create_dir_all("extensions")?;
    std::fs::write(&output_path, &yaml_content)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("\nCreated {}", output_path.display());
    Ok(())
}
