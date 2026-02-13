use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

use super::{Extension, ExtensionError, OperationDef, OperationResult, RiskLevel};

/// Extension for managing the Brazil package cache daemon and cached packages.
#[derive(Default)]
pub struct BrazilCacheExtension;

impl BrazilCacheExtension {
    pub fn new() -> Self {
        Self
    }

    fn cache_root() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("brazil-pkg-cache")
    }

    fn packages_dir() -> PathBuf {
        Self::cache_root().join("packages")
    }

    fn pid_file() -> PathBuf {
        Self::cache_root().join("daemon-pid")
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Recursively compute the total size of all files under `path`.
    async fn dir_size(path: &Path) -> u64 {
        let mut total: u64 = 0;
        let mut stack = vec![path.to_path_buf()];

        while let Some(current) = stack.pop() {
            let mut entries = match fs::read_dir(&current).await {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if metadata.is_dir() {
                    stack.push(entry.path());
                } else {
                    total += metadata.len();
                }
            }
        }

        total
    }

    /// Read the PID file and return (pid, version) if it exists.
    async fn read_pid_file() -> Option<(u32, String)> {
        let content = fs::read_to_string(Self::pid_file()).await.ok()?;
        let content = content.trim();
        let mut parts = content.splitn(2, ':');
        let pid: u32 = parts.next()?.parse().ok()?;
        let version = parts.next().unwrap_or("unknown").to_string();
        Some((pid, version))
    }

    /// Check whether a process with the given PID is alive.
    async fn is_process_alive(pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Run `brazil-package-cache connection` and parse its JSON output.
    async fn get_connection_info() -> Option<Value> {
        let output = Command::new("brazil-package-cache")
            .arg("connection")
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(stdout.trim()).ok()
    }

    /// Parse `df -k <path>` output to get disk statistics.
    /// Returns (total_bytes, used_bytes, available_bytes).
    async fn get_disk_stats(path: &Path) -> Option<(u64, u64, u64)> {
        let output = Command::new("df")
            .args(["-k", &path.to_string_lossy()])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        if lines.len() < 2 {
            return None;
        }

        // df -k output: Filesystem 1K-blocks Used Available Use% Mounted
        // The data line may be split across two lines on some systems, so we
        // collect all fields from remaining lines.
        let data: String = lines[1..].join(" ");
        let fields: Vec<&str> = data.split_whitespace().collect();
        if fields.len() < 4 {
            return None;
        }

        // fields[1] = total 1K-blocks, fields[2] = used, fields[3] = available
        let total_kb: u64 = fields[1].parse().ok()?;
        let used_kb: u64 = fields[2].parse().ok()?;
        let available_kb: u64 = fields[3].parse().ok()?;

        Some((total_kb * 1024, used_kb * 1024, available_kb * 1024))
    }

    /// Count immediate subdirectories in a path.
    async fn count_subdirs(path: &Path) -> u64 {
        let mut count: u64 = 0;
        let mut entries = match fs::read_dir(path).await {
            Ok(e) => e,
            Err(_) => return 0,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                if meta.is_dir() {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get the most recent modification time of any file within a directory (recursive).
    async fn last_modified_recursive(path: &Path) -> Option<std::time::SystemTime> {
        let mut latest: Option<std::time::SystemTime> = None;
        let mut stack = vec![path.to_path_buf()];

        while let Some(current) = stack.pop() {
            let mut entries = match fs::read_dir(&current).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                if metadata.is_dir() {
                    stack.push(entry.path());
                }

                if let Ok(modified) = metadata.modified() {
                    latest = Some(match latest {
                        Some(prev) if prev >= modified => prev,
                        _ => modified,
                    });
                }
            }
        }

        latest
    }

    // ── Operation implementations ──────────────────────────────────────

    async fn status_impl(&self) -> Result<OperationResult, ExtensionError> {
        let (running, pid, version) = match Self::read_pid_file().await {
            Some((pid, ver)) => {
                let alive = Self::is_process_alive(pid).await;
                (alive, Some(pid), Some(ver))
            }
            None => (false, None, None),
        };

        let connection = Self::get_connection_info().await;

        let packages_dir = Self::packages_dir();
        let package_count = Self::count_subdirs(&packages_dir).await;
        let cache_size_bytes = Self::dir_size(&packages_dir).await;
        let cache_size = Self::format_bytes(cache_size_bytes);

        let cache_root = Self::cache_root();
        let (disk_total, disk_used, disk_available) =
            match Self::get_disk_stats(&cache_root).await {
                Some((total, used, avail)) => (
                    Self::format_bytes(total),
                    Self::format_bytes(used),
                    Self::format_bytes(avail),
                ),
                None => (
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                ),
            };

        Ok(OperationResult {
            success: true,
            data: json!({
                "running": running,
                "pid": pid,
                "version": version,
                "package_count": package_count,
                "cache_size": cache_size,
                "cache_size_bytes": cache_size_bytes,
                "disk_total": disk_total,
                "disk_used": disk_used,
                "disk_available": disk_available,
                "connection": connection,
            }),
            message: None,
        })
    }

    async fn list_packages_impl(&self) -> Result<OperationResult, ExtensionError> {
        let packages_dir = Self::packages_dir();

        let mut entries = match fs::read_dir(&packages_dir).await {
            Ok(e) => e,
            Err(_) => {
                return Ok(OperationResult {
                    success: true,
                    data: json!([]),
                    message: Some("Cache directory does not exist or is empty.".to_string()),
                });
            }
        };

        let mut packages = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !metadata.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            let pkg_path = entry.path();

            let version_count = Self::count_subdirs(&pkg_path).await;
            let size_bytes = Self::dir_size(&pkg_path).await;
            let size = Self::format_bytes(size_bytes);

            let last_modified = Self::last_modified_recursive(&pkg_path)
                .await
                .and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|d| {
                            chrono::DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos())
                                .map(|dt: chrono::DateTime<chrono::Utc>| dt.to_rfc3339())
                                .unwrap_or_default()
                        })
                });

            packages.push(json!({
                "name": name,
                "version_count": version_count,
                "size": size,
                "size_bytes": size_bytes,
                "last_modified": last_modified,
            }));
        }

        // Sort by name for deterministic output.
        packages.sort_by(|a, b| {
            let a_name = a["name"].as_str().unwrap_or("");
            let b_name = b["name"].as_str().unwrap_or("");
            a_name.cmp(b_name)
        });

        Ok(OperationResult {
            success: true,
            data: json!(packages),
            message: None,
        })
    }

    async fn package_detail_impl(&self, input: Value) -> Result<OperationResult, ExtensionError> {
        let package = input["package"]
            .as_str()
            .ok_or_else(|| ExtensionError::InvalidInput("Missing required field: package".into()))?;

        let pkg_dir = Self::packages_dir().join(package);
        if !pkg_dir.exists() {
            return Err(ExtensionError::ExecutionFailed(format!(
                "Package '{}' not found in cache",
                package
            )));
        }

        let mut entries = fs::read_dir(&pkg_dir)
            .await
            .map_err(ExtensionError::Io)?;

        let mut versions = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !metadata.is_dir() {
                continue;
            }

            let version = entry.file_name().to_string_lossy().to_string();
            let version_path = entry.path();

            // Collect platform subdirectories.
            let mut platforms = Vec::new();
            if let Ok(mut platform_entries) = fs::read_dir(&version_path).await {
                while let Ok(Some(pe)) = platform_entries.next_entry().await {
                    if let Ok(pm) = pe.metadata().await {
                        if pm.is_dir() {
                            platforms.push(pe.file_name().to_string_lossy().to_string());
                        }
                    }
                }
            }
            platforms.sort();

            let size_bytes = Self::dir_size(&version_path).await;
            let size = Self::format_bytes(size_bytes);

            versions.push(json!({
                "version": version,
                "platforms": platforms,
                "size": size,
                "size_bytes": size_bytes,
            }));
        }

        // Sort versions alphabetically.
        versions.sort_by(|a, b| {
            let av = a["version"].as_str().unwrap_or("");
            let bv = b["version"].as_str().unwrap_or("");
            av.cmp(bv)
        });

        Ok(OperationResult {
            success: true,
            data: json!({
                "name": package,
                "versions": versions,
            }),
            message: None,
        })
    }

    async fn start_impl(&self) -> Result<OperationResult, ExtensionError> {
        let output = Command::new("brazil-package-cache")
            .arg("start")
            .output()
            .await
            .map_err(ExtensionError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(OperationResult {
                success: true,
                data: json!({ "stdout": stdout.trim() }),
                message: Some("Brazil package cache started.".to_string()),
            })
        } else {
            Err(ExtensionError::CommandFailed {
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    async fn stop_impl(&self) -> Result<OperationResult, ExtensionError> {
        let output = Command::new("brazil-package-cache")
            .arg("stop")
            .output()
            .await
            .map_err(ExtensionError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(OperationResult {
                success: true,
                data: json!({ "stdout": stdout.trim() }),
                message: Some("Brazil package cache stopped.".to_string()),
            })
        } else {
            Err(ExtensionError::CommandFailed {
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    async fn clean_impl(&self, input: Value) -> Result<OperationResult, ExtensionError> {
        let packages = input["packages"]
            .as_array()
            .ok_or_else(|| {
                ExtensionError::InvalidInput("Missing required field: packages (array)".into())
            })?;

        let pkg_list: Vec<&str> = packages
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        if pkg_list.is_empty() {
            return Err(ExtensionError::InvalidInput(
                "packages array must contain at least one package name".into(),
            ));
        }

        let joined = pkg_list.join(",");

        let output = Command::new("brazil-package-cache")
            .args(["clean", "--package", &joined])
            .output()
            .await
            .map_err(ExtensionError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(OperationResult {
                success: true,
                data: json!({
                    "cleaned_packages": pkg_list,
                    "stdout": stdout.trim(),
                }),
                message: Some(format!("Cleaned {} package(s).", pkg_list.len())),
            })
        } else {
            Err(ExtensionError::CommandFailed {
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }

    async fn clean_all_impl(&self) -> Result<OperationResult, ExtensionError> {
        let output = Command::new("brazil-package-cache")
            .arg("clean")
            .output()
            .await
            .map_err(ExtensionError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(OperationResult {
                success: true,
                data: json!({ "stdout": stdout.trim() }),
                message: Some("Cleaned all old cached package versions.".to_string()),
            })
        } else {
            Err(ExtensionError::CommandFailed {
                exit_code: output.status.code().unwrap_or(-1),
                stderr,
            })
        }
    }
}

#[async_trait]
impl Extension for BrazilCacheExtension {
    fn id(&self) -> &'static str {
        "brazil_cache"
    }

    fn display_name(&self) -> &'static str {
        "Brazil Package Cache"
    }

    fn description(&self) -> &'static str {
        "Manage the local Brazil package cache daemon and cached packages"
    }

    fn operations(&self) -> Vec<OperationDef> {
        vec![
            OperationDef {
                name: "status".to_string(),
                description: "Check cache daemon status, package count, and disk usage".to_string(),
                risk_level: RiskLevel::Low,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "list_packages".to_string(),
                description: "List all cached packages with version counts and sizes".to_string(),
                risk_level: RiskLevel::Low,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "package_detail".to_string(),
                description: "Show versions and platforms for a specific cached package".to_string(),
                risk_level: RiskLevel::Low,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "package": {
                            "type": "string",
                            "description": "Name of the package to inspect"
                        }
                    },
                    "required": ["package"]
                }),
            },
            OperationDef {
                name: "start".to_string(),
                description: "Start the Brazil package cache daemon".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "stop".to_string(),
                description: "Stop the Brazil package cache daemon".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "clean".to_string(),
                description: "Remove old cached versions of specific packages".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "packages": {
                            "type": "array",
                            "items": { "type": "string" },
                            "minItems": 1,
                            "description": "Package names to clean old versions of"
                        }
                    },
                    "required": ["packages"]
                }),
            },
            OperationDef {
                name: "clean_all".to_string(),
                description: "Remove all old cached package versions".to_string(),
                risk_level: RiskLevel::High,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    async fn execute(
        &self,
        operation: &str,
        input: Value,
    ) -> Result<OperationResult, ExtensionError> {
        match operation {
            "status" => self.status_impl().await,
            "list_packages" => self.list_packages_impl().await,
            "package_detail" => self.package_detail_impl(input).await,
            "start" => self.start_impl().await,
            "stop" => self.stop_impl().await,
            "clean" => self.clean_impl(input).await,
            "clean_all" => self.clean_all_impl().await,
            _ => Err(ExtensionError::UnknownOperation(operation.to_string())),
        }
    }
}
