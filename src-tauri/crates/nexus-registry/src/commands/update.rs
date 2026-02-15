use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub struct UpdateArgs {
    pub id: String,
    pub version: Option<String>,
    pub image: Option<String>,
    pub image_digest: Option<String>,
    pub description: Option<String>,
    pub manifest_url: Option<String>,
    pub status: Option<String>,
}

pub fn run(args: UpdateArgs) -> Result<()> {
    // Find the YAML file — try plugins/ first, then extensions/
    let plugin_path = PathBuf::from("plugins").join(format!("{}.yaml", args.id));
    let extension_path = PathBuf::from("extensions").join(format!("{}.yaml", args.id));

    let path = if plugin_path.exists() {
        plugin_path
    } else if extension_path.exists() {
        extension_path
    } else {
        bail!(
            "No entry found for '{}'. Looked in plugins/ and extensions/",
            args.id
        );
    };

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut entry: serde_json::Map<String, serde_json::Value> =
        serde_yaml::from_str(&content).context("Invalid YAML")?;

    let mut changed = Vec::new();

    if let Some(version) = &args.version {
        entry.insert("version".into(), serde_json::Value::String(version.clone()));
        changed.push(format!("version → {version}"));
    }

    if let Some(image) = &args.image {
        entry.insert("image".into(), serde_json::Value::String(image.clone()));
        changed.push(format!("image → {image}"));
    }

    if let Some(digest) = &args.image_digest {
        entry.insert(
            "image_digest".into(),
            serde_json::Value::String(digest.clone()),
        );
        changed.push(format!("image_digest → {}", &digest[..20]));
    }

    if let Some(desc) = &args.description {
        entry.insert(
            "description".into(),
            serde_json::Value::String(desc.clone()),
        );
        changed.push("description".into());
    }

    if let Some(url) = &args.manifest_url {
        entry.insert(
            "manifest_url".into(),
            serde_json::Value::String(url.clone()),
        );
        changed.push(format!("manifest_url → {url}"));
    }

    if let Some(status) = &args.status {
        entry.insert("status".into(), serde_json::Value::String(status.clone()));
        changed.push(format!("status → {status}"));
    }

    // Re-fetch manifest SHA256 from manifest_url if version or manifest_url changed
    if args.version.is_some() || args.manifest_url.is_some() {
        if let Some(url) = entry.get("manifest_url").and_then(|v| v.as_str()) {
            if let Some(hash) = fetch_manifest_hash(url) {
                entry.insert("manifest_sha256".into(), serde_json::Value::String(hash));
                changed.push("manifest_sha256 (auto)".into());
            }
        }
    }

    if changed.is_empty() {
        println!("Nothing to update.");
        return Ok(());
    }

    let yaml_content = serde_yaml::to_string(&entry)?;
    std::fs::write(&path, &yaml_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    println!("Updated {}:", path.display());
    for c in &changed {
        println!("  {c}");
    }
    Ok(())
}

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
