use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::schema;

pub fn run(path: &Path) -> Result<()> {
    println!("Scaffolding new registry at: {}", path.display());

    fs::create_dir_all(path).context("Failed to create registry directory")?;

    // Create subdirectories
    for dir in &["plugins", "extensions", "schema", "scripts", ".github/workflows"] {
        fs::create_dir_all(path.join(dir))
            .with_context(|| format!("Failed to create {dir}"))?;
    }

    // .gitkeep files
    fs::write(path.join("plugins/.gitkeep"), "")?;
    fs::write(path.join("extensions/.gitkeep"), "")?;

    // registry.yaml
    fs::write(
        path.join("registry.yaml"),
        r#"name: "My Nexus Registry"
description: "A community plugin and extension registry for Nexus"
# homepage: "https://example.com"
# maintainer: "Your Name <you@example.com>"
"#,
    )?;

    // Schema files
    fs::write(
        path.join("schema/plugin.schema.json"),
        schema::plugin_schema_pretty(),
    )?;
    fs::write(
        path.join("schema/extension.schema.json"),
        schema::extension_schema_pretty(),
    )?;
    fs::write(
        path.join("schema/registry.schema.json"),
        schema::registry_schema_pretty(),
    )?;

    // Build script
    fs::write(path.join("scripts/build-index.py"), BUILD_INDEX_SCRIPT)?;

    // GitHub Actions workflows
    fs::write(
        path.join(".github/workflows/validate.yml"),
        VALIDATE_WORKFLOW,
    )?;
    fs::write(
        path.join(".github/workflows/build-index.yml"),
        BUILD_INDEX_WORKFLOW,
    )?;

    // README
    fs::write(path.join("README.md"), README_TEMPLATE)?;

    // Empty index.json
    let empty_index = serde_json::json!({
        "version": 2,
        "registry": {
            "name": "My Nexus Registry",
            "description": "A community plugin and extension registry for Nexus"
        },
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "plugins": [],
        "extensions": []
    });
    fs::write(
        path.join("index.json"),
        serde_json::to_string_pretty(&empty_index)?,
    )?;

    println!("Registry scaffolded successfully!");
    println!();
    println!("Next steps:");
    println!("  1. Edit registry.yaml with your registry details");
    println!("  2. Add plugins with: nexus-registry add plugin");
    println!("  3. Add extensions with: nexus-registry add extension");
    println!("  4. Build the index: nexus-registry build {}", path.display());

    Ok(())
}

const BUILD_INDEX_SCRIPT: &str = r#"#!/usr/bin/env python3
"""Build index.json from plugin and extension YAML files."""

import json
import glob
import sys
from datetime import datetime, timezone

try:
    import yaml
except ImportError:
    print("Error: PyYAML is required. Install with: pip install pyyaml")
    sys.exit(1)


def load_yaml_files(directory: str) -> list:
    entries = []
    for path in sorted(glob.glob(f"{directory}/*.yaml")):
        with open(path) as f:
            data = yaml.safe_load(f)
            if data and data.get("status") != "unlisted":
                entries.append(data)
    return entries


def main():
    # Load registry metadata
    with open("registry.yaml") as f:
        registry = yaml.safe_load(f)

    plugins = load_yaml_files("plugins")
    extensions = load_yaml_files("extensions")

    index = {
        "version": 2,
        "registry": {
            "name": registry.get("name", ""),
            "description": registry.get("description", ""),
        },
        "updated_at": datetime.now(timezone.utc).isoformat(),
        "plugins": plugins,
        "extensions": extensions,
    }

    if registry.get("homepage"):
        index["registry"]["homepage"] = registry["homepage"]
    if registry.get("maintainer"):
        index["registry"]["maintainer"] = registry["maintainer"]

    with open("index.json", "w") as f:
        json.dump(index, f, indent=2)

    print(f"Built index.json: {len(plugins)} plugins, {len(extensions)} extensions")


if __name__ == "__main__":
    main()
"#;

const VALIDATE_WORKFLOW: &str = r#"name: Validate Registry
on:
  pull_request:
    paths:
      - "plugins/**"
      - "extensions/**"
      - "registry.yaml"

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install nexus-registry CLI
        run: cargo install nexus-registry
      - name: Validate
        run: nexus-registry validate .
"#;

const BUILD_INDEX_WORKFLOW: &str = r#"name: Build Index
on:
  push:
    branches: [main]
    paths:
      - "plugins/**"
      - "extensions/**"
      - "registry.yaml"

jobs:
  build:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      - name: Install nexus-registry CLI
        run: cargo install nexus-registry
      - name: Build index
        run: nexus-registry build .
      - name: Commit index.json
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add index.json
          git diff --cached --quiet || git commit -m "chore: rebuild index.json"
          git push
"#;

const README_TEMPLATE: &str = r#"# Nexus Registry

A plugin and extension registry for [Nexus](https://github.com/imdanibytes/nexus).

## Structure

```
plugins/          — Plugin YAML definitions
extensions/       — Extension YAML definitions
schema/           — JSON Schema files for validation
index.json        — Compiled registry index (auto-generated)
registry.yaml     — Registry metadata
```

## Adding a Package

### Using the CLI

```bash
# Install the CLI
cargo install nexus-registry

# Add a plugin interactively
nexus-registry add plugin

# Add an extension interactively
nexus-registry add extension

# Validate all files
nexus-registry validate .

# Build the index
nexus-registry build .
```

### Manually

1. Create a YAML file in `plugins/` or `extensions/`
2. Follow the schema in `schema/`
3. Submit a pull request

## Publishing

```bash
nexus-registry publish --registry <git-url> --package plugins/my-plugin.yaml
```
"#;
