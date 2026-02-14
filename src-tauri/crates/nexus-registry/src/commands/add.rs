use anyhow::{bail, Context, Result};
use chrono::Utc;
use dialoguer::{Input, Select};
use regex::Regex;
use serde_json::Value;
use sha2::{Digest, Sha256};
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

/// Returns the value if provided, otherwise prompts interactively.
fn require_or_prompt(
    value: Option<String>,
    prompt: &str,
    default: Option<&str>,
    validator: Option<fn(&String) -> Result<(), String>>,
) -> Result<String> {
    if let Some(v) = value {
        return Ok(v);
    }
    let input = Input::<String>::new().with_prompt(prompt);
    let input = match default {
        Some(d) => input.default(d.to_string()),
        None => input,
    };
    let result = match validator {
        Some(v) => input.validate_with(v).interact_text()?,
        None => input.interact_text()?,
    };
    Ok(result)
}

// ── Arg structs passed from main.rs ──

pub struct PluginArgs {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub author_url: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub image: Option<String>,
    pub manifest_url: Option<String>,
    pub categories: Option<String>,
    pub status: String,
}

pub struct ExtensionArgs {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub author_url: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub manifest_url: Option<String>,
    pub platforms: Option<String>,
    pub categories: Option<String>,
    pub status: String,
}

impl PluginArgs {
    fn is_non_interactive(&self) -> bool {
        self.id.is_some()
            && self.name.is_some()
            && self.description.is_some()
            && self.author.is_some()
            && self.image.is_some()
            && self.manifest_url.is_some()
    }
}

impl ExtensionArgs {
    fn is_non_interactive(&self) -> bool {
        self.id.is_some()
            && self.name.is_some()
            && self.description.is_some()
            && self.author.is_some()
            && self.manifest_url.is_some()
    }
}

pub fn run_plugin(args: PluginArgs) -> Result<()> {
    let non_interactive = args.is_non_interactive();
    if !non_interactive {
        println!("Add a new plugin to the registry\n");
    }

    let id = require_or_prompt(
        args.id,
        "Plugin ID (reverse-domain)",
        None,
        Some(validate_id),
    )?;
    let name = require_or_prompt(args.name, "Display name", None, Some(validate_nonempty))?;
    let version = require_or_prompt(
        args.version,
        "Version",
        Some("0.1.0"),
        Some(validate_version),
    )?;
    let description =
        require_or_prompt(args.description, "Description", None, Some(validate_nonempty))?;
    let author = require_or_prompt(
        args.author.clone(),
        "Author (GitHub username)",
        None,
        Some(validate_nonempty),
    )?;

    let default_author_url = format!("https://github.com/{author}");
    let author_url = require_or_prompt(
        args.author_url,
        "Author URL (optional)",
        Some(&default_author_url),
        None,
    )?;

    let license = require_or_prompt(args.license, "License", Some("MIT"), None)?;
    let homepage = require_or_prompt(args.homepage, "Homepage URL (optional)", Some(""), None)?;
    let image = require_or_prompt(args.image, "Docker image", None, Some(validate_nonempty))?;
    let manifest_url =
        require_or_prompt(args.manifest_url, "Manifest URL", None, Some(validate_nonempty))?;

    // Auto-compute manifest SHA-256
    let manifest_sha256 = fetch_manifest_hash(&manifest_url);

    let categories_input =
        require_or_prompt(args.categories, "Categories (comma-separated, optional)", Some(""), None)?;

    let status = if non_interactive {
        args.status
    } else {
        let status_options = &["active", "deprecated", "unlisted"];
        let status_idx = Select::new()
            .with_prompt("Status")
            .items(status_options)
            .default(0)
            .interact()?;
        status_options[status_idx].to_string()
    };

    // Build the YAML content
    let mut plugin = serde_json::Map::new();
    plugin.insert("id".into(), Value::String(id.clone()));
    plugin.insert("name".into(), Value::String(name));
    plugin.insert("version".into(), Value::String(version));
    plugin.insert("description".into(), Value::String(description));
    plugin.insert("author".into(), Value::String(author));
    if !author_url.is_empty() {
        plugin.insert("author_url".into(), Value::String(author_url));
    }
    plugin.insert(
        "created_at".into(),
        Value::String(Utc::now().to_rfc3339()),
    );
    if !license.is_empty() {
        plugin.insert("license".into(), Value::String(license));
    }
    if !homepage.is_empty() {
        plugin.insert("homepage".into(), Value::String(homepage));
    }
    plugin.insert("image".into(), Value::String(image));
    plugin.insert("manifest_url".into(), Value::String(manifest_url));
    if let Some(hash) = manifest_sha256 {
        plugin.insert("manifest_sha256".into(), Value::String(hash));
    }
    plugin.insert("status".into(), Value::String(status));

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

pub fn run_extension(args: ExtensionArgs) -> Result<()> {
    let non_interactive = args.is_non_interactive();
    if !non_interactive {
        println!("Add a new extension to the registry\n");
    }

    let id = require_or_prompt(
        args.id,
        "Extension ID (reverse-domain)",
        None,
        Some(validate_id),
    )?;
    let name = require_or_prompt(args.name, "Display name", None, Some(validate_nonempty))?;
    let version = require_or_prompt(
        args.version,
        "Version",
        Some("0.1.0"),
        Some(validate_version),
    )?;
    let description =
        require_or_prompt(args.description, "Description", None, Some(validate_nonempty))?;
    let author = require_or_prompt(
        args.author.clone(),
        "Author (GitHub username)",
        None,
        Some(validate_nonempty),
    )?;

    let default_author_url = format!("https://github.com/{author}");
    let author_url = require_or_prompt(
        args.author_url,
        "Author URL (optional)",
        Some(&default_author_url),
        None,
    )?;

    let license = require_or_prompt(args.license, "License", Some("MIT"), None)?;
    let homepage = require_or_prompt(args.homepage, "Homepage URL (optional)", Some(""), None)?;
    let manifest_url =
        require_or_prompt(args.manifest_url, "Manifest URL", None, Some(validate_nonempty))?;

    // Auto-compute manifest SHA-256
    let manifest_sha256 = fetch_manifest_hash(&manifest_url);

    let platforms_input = require_or_prompt(
        args.platforms,
        "Platforms (comma-separated, e.g. macos-aarch64,linux-x86_64)",
        Some(""),
        None,
    )?;

    let categories_input =
        require_or_prompt(args.categories, "Categories (comma-separated, optional)", Some(""), None)?;

    let status = if non_interactive {
        args.status
    } else {
        let status_options = &["active", "deprecated", "unlisted"];
        let status_idx = Select::new()
            .with_prompt("Status")
            .items(status_options)
            .default(0)
            .interact()?;
        status_options[status_idx].to_string()
    };

    // Build the YAML content
    let mut extension = serde_json::Map::new();
    extension.insert("id".into(), Value::String(id.clone()));
    extension.insert("name".into(), Value::String(name));
    extension.insert("version".into(), Value::String(version));
    extension.insert("description".into(), Value::String(description));
    extension.insert("author".into(), Value::String(author));
    if !author_url.is_empty() {
        extension.insert("author_url".into(), Value::String(author_url));
    }
    extension.insert(
        "created_at".into(),
        Value::String(Utc::now().to_rfc3339()),
    );
    if !license.is_empty() {
        extension.insert("license".into(), Value::String(license));
    }
    if !homepage.is_empty() {
        extension.insert("homepage".into(), Value::String(homepage));
    }
    extension.insert("manifest_url".into(), Value::String(manifest_url));
    if let Some(hash) = manifest_sha256 {
        extension.insert("manifest_sha256".into(), Value::String(hash));
    }
    extension.insert("status".into(), Value::String(status));

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

/// Fetch a manifest URL and compute its SHA-256 hash.
/// Returns None if the fetch fails (non-fatal — the hash is optional).
fn fetch_manifest_hash(url: &str) -> Option<String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }
    print!("Fetching manifest for SHA-256... ");
    match reqwest::blocking::get(url) {
        Ok(resp) if resp.status().is_success() => match resp.bytes() {
            Ok(body) => {
                let hash = format!("{:x}", Sha256::digest(&body));
                println!("{hash}");
                Some(hash)
            }
            Err(_) => {
                println!("failed to read body, skipping hash");
                None
            }
        },
        _ => {
            println!("failed to fetch, skipping hash");
            None
        }
    }
}
