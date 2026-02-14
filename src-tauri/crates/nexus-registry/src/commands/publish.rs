use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn run(registry_url: &str, package_path: &Path) -> Result<()> {
    // Verify the package file exists
    if !package_path.exists() {
        bail!("Package file not found: {}", package_path.display());
    }

    // Determine if it's a plugin or extension from the path
    let parent = package_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let kind = match parent {
        "plugins" => "plugins",
        "extensions" => "extensions",
        _ => {
            bail!(
                "Package must be in a plugins/ or extensions/ directory. Got: {}",
                package_path.display()
            );
        }
    };

    // Read package metadata for branch naming
    let content = std::fs::read_to_string(package_path)
        .context("Failed to read package file")?;
    let meta: serde_json::Value =
        serde_yaml::from_str(&content).context("Invalid package YAML")?;
    let package_id = meta
        .get("id")
        .and_then(|v| v.as_str())
        .context("Package missing 'id' field")?;
    let package_version = meta
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0");

    let file_name = package_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid package file name")?;

    // Create temp directory and clone
    let temp_dir = tempdir()?;
    let clone_path = temp_dir.join("registry");

    println!("Cloning registry from {registry_url}...");
    run_git(&["clone", registry_url, &clone_path.to_string_lossy()])?;

    // Copy package into the clone
    let dest_dir = clone_path.join(kind);
    std::fs::create_dir_all(&dest_dir)?;
    let dest_file = dest_dir.join(file_name);
    std::fs::copy(package_path, &dest_file)
        .context("Failed to copy package file")?;

    // Validate the registry
    println!("Validating...");
    crate::commands::validate::run(&clone_path)?;

    // Build the index
    println!("Building index...");
    crate::commands::build::run(&clone_path)?;

    // Create branch
    let branch_name = format!("add/{package_id}-{package_version}");
    println!("Creating branch: {branch_name}");
    run_git_in(
        &clone_path,
        &["checkout", "-b", &branch_name],
    )?;

    // Stage and commit
    run_git_in(&clone_path, &["add", &format!("{kind}/{file_name}"), "index.json"])?;
    run_git_in(
        &clone_path,
        &[
            "commit",
            "-m",
            &format!("Add {kind}/{package_id} {package_version}"),
        ],
    )?;

    // Push
    println!("Pushing branch...");
    run_git_in(&clone_path, &["push", "-u", "origin", &branch_name])?;

    // Try to create a PR via gh CLI
    if gh_available() {
        println!("Creating pull request...");
        let pr_result = Command::new("gh")
            .args([
                "pr",
                "create",
                "--title",
                &format!("Add {package_id} {package_version}"),
                "--body",
                &format!(
                    "Adds `{kind}/{file_name}` to the registry.\n\n\
                     - **ID**: {package_id}\n\
                     - **Version**: {package_version}\n\
                     - **Type**: {kind}"
                ),
            ])
            .current_dir(&clone_path)
            .output();

        match pr_result {
            Ok(output) if output.status.success() => {
                let url = String::from_utf8_lossy(&output.stdout);
                println!("Pull request created: {}", url.trim());
            }
            _ => {
                println!("Could not create PR automatically.");
                println!(
                    "Push succeeded. Open a PR manually from branch '{branch_name}'."
                );
            }
        }
    } else {
        println!();
        println!("Branch '{branch_name}' pushed successfully.");
        println!("Install the GitHub CLI (gh) for automatic PR creation, or open a PR manually.");
    }

    Ok(())
}

fn tempdir() -> Result<std::path::PathBuf> {
    let dir = std::env::temp_dir().join(format!(
        "nexus-registry-publish-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn run_git(args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to run git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

fn run_git_in(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .context("Failed to run git")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(())
}

fn gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
