use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;

use super::{Extension, ExtensionError, OperationDef, OperationResult, RiskLevel};

/// ADA credential management extension.
///
/// Provides operations to list, refresh, and manage AWS credentials
/// via the `ada` CLI tool and the associated credential/profile files.
pub struct AdaExtension;

impl AdaExtension {
    pub fn new() -> Self {
        Self
    }

    fn home_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
    }

    fn ada_credentials_path() -> PathBuf {
        Self::home_dir().join(".ada").join("credentials")
    }

    fn aws_credentials_path() -> PathBuf {
        Self::home_dir().join(".aws").join("credentials")
    }

    fn ada_profile_path() -> PathBuf {
        Self::home_dir()
            .join(".config")
            .join("ada")
            .join("profile.json")
    }

    // -----------------------------------------------------------------------
    // INI parser
    // -----------------------------------------------------------------------

    /// Parse an INI-style credentials file into a map of section -> key/value pairs.
    ///
    /// Handles the format used by `~/.ada/credentials` and `~/.aws/credentials`:
    /// ```ini
    /// [section-name]
    /// key = value
    /// ```
    fn parse_ini(content: &str) -> HashMap<String, HashMap<String, String>> {
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current_section: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }

            // Section header: [name]
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let name = trimmed[1..trimmed.len() - 1].to_string();
                sections.entry(name.clone()).or_default();
                current_section = Some(name);
                continue;
            }

            // Key = value pair within a section
            if let Some(ref section) = current_section {
                if let Some(eq_pos) = trimmed.find('=') {
                    let key = trimmed[..eq_pos].trim().to_string();
                    let val = trimmed[eq_pos + 1..].trim().to_string();
                    sections
                        .entry(section.clone())
                        .or_default()
                        .insert(key, val);
                }
            }
        }

        sections
    }

    /// Write INI sections back to a string, preserving key order within each section
    /// is not guaranteed (HashMap), but the output is valid INI.
    fn write_ini(sections: &HashMap<String, HashMap<String, String>>) -> String {
        let mut out = String::new();
        // Sort section names for deterministic output
        let mut names: Vec<&String> = sections.keys().collect();
        names.sort();

        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push('[');
            out.push_str(name);
            out.push_str("]\n");
            if let Some(fields) = sections.get(*name) {
                let mut keys: Vec<&String> = fields.keys().collect();
                keys.sort();
                for key in keys {
                    out.push_str(key);
                    out.push_str(" = ");
                    out.push_str(&fields[key]);
                    out.push('\n');
                }
            }
        }
        out
    }

    // -----------------------------------------------------------------------
    // Credential status helpers
    // -----------------------------------------------------------------------

    /// Compute credential status and human-readable time remaining from a section's fields.
    ///
    /// Returns `(status, time_remaining)` where:
    /// - status: "active" | "warning" | "expired" | "none"
    /// - time_remaining: human string like "2h 15m", "45m", "expired 10m ago", or null
    fn credential_status(fields: &HashMap<String, String>) -> (String, Option<String>) {
        let has_creds = fields.contains_key("aws_access_key_id");
        if !has_creds {
            return ("none".to_string(), Some("No Credentials".to_string()));
        }

        // ADA uses "expiration_time", AWS sometimes uses "expiration"
        let exp_str = fields
            .get("expiration_time")
            .or_else(|| fields.get("expiration"));

        let exp_str = match exp_str {
            Some(s) if !s.is_empty() => s,
            _ => {
                // Has credentials but no expiration — treat as active (e.g. long-lived keys)
                return ("active".to_string(), Some("Active".to_string()));
            }
        };

        let expiration = match DateTime::parse_from_rfc3339(exp_str) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => {
                // Can't parse expiration — treat as active to avoid false alarms
                return ("active".to_string(), None);
            }
        };

        let now = Utc::now();
        let diff = expiration.signed_duration_since(now);
        let total_secs = diff.num_seconds();

        if total_secs <= 0 {
            // Expired
            let ago_mins = (-total_secs) / 60;
            let time_str = if ago_mins >= 60 {
                format!("expired {}h {}m ago", ago_mins / 60, ago_mins % 60)
            } else {
                format!("expired {}m ago", ago_mins)
            };
            ("expired".to_string(), Some(time_str))
        } else if total_secs < 15 * 60 {
            // Warning: less than 15 minutes
            let mins = total_secs / 60;
            ("warning".to_string(), Some(format!("{}m", mins)))
        } else {
            // Active
            let total_mins = total_secs / 60;
            let hrs = total_mins / 60;
            let mins = total_mins % 60;
            let time_str = if hrs > 0 {
                format!("{}h {}m", hrs, mins)
            } else {
                format!("{}m", mins)
            };
            ("active".to_string(), Some(time_str))
        }
    }

    // -----------------------------------------------------------------------
    // File I/O helpers
    // -----------------------------------------------------------------------

    async fn read_file_or_empty(path: &PathBuf) -> String {
        fs::read_to_string(path).await.unwrap_or_default()
    }

    async fn read_profile_json() -> Value {
        let path = Self::ada_profile_path();
        let content = Self::read_file_or_empty(&path).await;
        if content.is_empty() {
            return json!({"Profiles": []});
        }
        serde_json::from_str(&content).unwrap_or_else(|_| json!({"Profiles": []}))
    }

    async fn write_profile_json(data: &Value) -> Result<(), ExtensionError> {
        let path = Self::ada_profile_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(data)
            .map_err(|e| ExtensionError::Other(format!("JSON serialization failed: {}", e)))?;
        fs::write(&path, content).await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Operation implementations
    // -----------------------------------------------------------------------

    async fn list_credentials_impl(&self) -> Result<OperationResult, ExtensionError> {
        let ada_content = Self::read_file_or_empty(&Self::ada_credentials_path()).await;
        let aws_content = Self::read_file_or_empty(&Self::aws_credentials_path()).await;

        let ada_sections = Self::parse_ini(&ada_content);
        let aws_sections = Self::parse_ini(&aws_content);

        let mut results: Vec<Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // ADA credentials first
        for (profile, fields) in &ada_sections {
            seen.insert(profile.clone());
            let (status, time_remaining) = Self::credential_status(fields);
            let fields_json: Value = fields
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<serde_json::Map<String, Value>>()
                .into();

            results.push(json!({
                "profile": profile,
                "source": "ada",
                "status": status,
                "time_remaining": time_remaining,
                "fields": fields_json,
            }));
        }

        // AWS credentials (skip duplicates already in ADA)
        for (profile, fields) in &aws_sections {
            if seen.contains(profile) {
                continue;
            }
            let (status, time_remaining) = Self::credential_status(fields);
            let fields_json: Value = fields
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect::<serde_json::Map<String, Value>>()
                .into();

            results.push(json!({
                "profile": profile,
                "source": "aws",
                "status": status,
                "time_remaining": time_remaining,
                "fields": fields_json,
            }));
        }

        Ok(OperationResult {
            success: true,
            data: json!(results),
            message: Some(format!("Found {} credential profile(s)", results.len())),
        })
    }

    async fn list_profiles_impl(&self) -> Result<OperationResult, ExtensionError> {
        let profile_data = Self::read_profile_json().await;
        let profiles_arr = profile_data["Profiles"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // Load ADA credentials for status enrichment
        let ada_content = Self::read_file_or_empty(&Self::ada_credentials_path()).await;
        let ada_sections = Self::parse_ini(&ada_content);

        let mut results: Vec<Value> = Vec::new();

        for p in &profiles_arr {
            let name = p["Profile"].as_str().unwrap_or("").to_string();
            let account = p["Account"].as_str().unwrap_or("").to_string();
            let role = p["Role"].as_str().unwrap_or("").to_string();
            let provider = p["Provider"].as_str().unwrap_or("").to_string();

            let (credential_status, time_remaining) = if let Some(fields) = ada_sections.get(&name)
            {
                Self::credential_status(fields)
            } else {
                ("none".to_string(), None)
            };

            results.push(json!({
                "name": name,
                "account": account,
                "role": role,
                "provider": provider,
                "credential_status": credential_status,
                "time_remaining": time_remaining,
            }));
        }

        Ok(OperationResult {
            success: true,
            data: json!(results),
            message: Some(format!("Found {} managed profile(s)", results.len())),
        })
    }

    async fn refresh_credentials_impl(
        &self,
        input: Value,
    ) -> Result<OperationResult, ExtensionError> {
        let mut args = vec![
            "credentials".to_string(),
            "update".to_string(),
            "--once".to_string(),
        ];

        if let Some(profile) = input.get("profile").and_then(|v| v.as_str()) {
            if !profile.is_empty() {
                args.push("--profile".to_string());
                args.push(profile.to_string());
            }
        }
        if let Some(account) = input.get("account").and_then(|v| v.as_str()) {
            if !account.is_empty() {
                args.push("--account".to_string());
                args.push(account.to_string());
            }
        }
        if let Some(role) = input.get("role").and_then(|v| v.as_str()) {
            if !role.is_empty() {
                args.push("--role".to_string());
                args.push(role.to_string());
            }
        }
        if let Some(provider) = input.get("provider").and_then(|v| v.as_str()) {
            if !provider.is_empty() {
                args.push("--provider".to_string());
                args.push(provider.to_string());
            }
        }

        let output = Command::new("ada")
            .args(&args)
            .output()
            .await
            .map_err(|e| ExtensionError::ExecutionFailed(format!("Failed to run ada CLI: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            return Err(ExtensionError::CommandFailed {
                exit_code,
                stderr: if stderr.is_empty() {
                    stdout
                } else {
                    stderr
                },
            });
        }

        Ok(OperationResult {
            success: true,
            data: json!({
                "stdout": stdout.trim(),
                "stderr": stderr.trim(),
            }),
            message: Some("Credentials refreshed successfully".to_string()),
        })
    }

    async fn add_profile_impl(&self, input: Value) -> Result<OperationResult, ExtensionError> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExtensionError::InvalidInput("'name' is required".to_string()))?;
        let account = input
            .get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExtensionError::InvalidInput("'account' is required".to_string()))?;
        let role = input
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExtensionError::InvalidInput("'role' is required".to_string()))?;
        let provider = input
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("conduit");

        let mut data = Self::read_profile_json().await;
        let profiles = data["Profiles"]
            .as_array_mut()
            .ok_or_else(|| ExtensionError::Other("Malformed profile.json".to_string()))?;

        // Check for duplicate
        let already_exists = profiles
            .iter()
            .any(|p| p["Profile"].as_str() == Some(name));
        if already_exists {
            return Err(ExtensionError::InvalidInput(format!(
                "Profile \"{}\" already exists",
                name
            )));
        }

        profiles.push(json!({
            "Profile": name,
            "Account": account,
            "Role": role,
            "Provider": provider,
        }));

        Self::write_profile_json(&data).await?;

        Ok(OperationResult {
            success: true,
            data: json!({
                "name": name,
                "account": account,
                "role": role,
                "provider": provider,
            }),
            message: Some(format!("Profile \"{}\" added", name)),
        })
    }

    async fn remove_profile_impl(&self, input: Value) -> Result<OperationResult, ExtensionError> {
        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExtensionError::InvalidInput("'name' is required".to_string()))?;

        let mut data = Self::read_profile_json().await;
        let profiles = data["Profiles"]
            .as_array_mut()
            .ok_or_else(|| ExtensionError::Other("Malformed profile.json".to_string()))?;

        let before_len = profiles.len();
        profiles.retain(|p| p["Profile"].as_str() != Some(name));

        if profiles.len() == before_len {
            return Err(ExtensionError::InvalidInput(format!(
                "Profile \"{}\" not found",
                name
            )));
        }

        Self::write_profile_json(&data).await?;

        Ok(OperationResult {
            success: true,
            data: json!({ "removed": name }),
            message: Some(format!("Profile \"{}\" removed", name)),
        })
    }

    async fn clear_credentials_impl(&self, input: Value) -> Result<OperationResult, ExtensionError> {
        let cred_path = Self::ada_credentials_path();
        let content = Self::read_file_or_empty(&cred_path).await;

        if content.is_empty() {
            return Ok(OperationResult {
                success: true,
                data: json!({ "cleared": 0 }),
                message: Some("No credentials file found".to_string()),
            });
        }

        let profile = input.get("profile").and_then(|v| v.as_str());

        match profile {
            Some(name) if !name.is_empty() => {
                // Remove a specific profile section
                let mut sections = Self::parse_ini(&content);
                if sections.remove(name).is_none() {
                    return Err(ExtensionError::InvalidInput(format!(
                        "Profile \"{}\" not found in credentials",
                        name
                    )));
                }

                let new_content = Self::write_ini(&sections);
                fs::write(&cred_path, new_content).await?;

                Ok(OperationResult {
                    success: true,
                    data: json!({ "cleared": name }),
                    message: Some(format!("Cleared credentials for profile \"{}\"", name)),
                })
            }
            _ => {
                // Clear all non-default sections
                let sections = Self::parse_ini(&content);
                let mut keep: HashMap<String, HashMap<String, String>> = HashMap::new();
                let mut cleared_count = 0usize;

                for (name, fields) in &sections {
                    if name == "default" {
                        keep.insert(name.clone(), fields.clone());
                    } else {
                        cleared_count += 1;
                    }
                }

                let new_content = Self::write_ini(&keep);
                fs::write(&cred_path, new_content).await?;

                Ok(OperationResult {
                    success: true,
                    data: json!({ "cleared": cleared_count }),
                    message: Some(format!(
                        "Cleared {} non-default credential section(s)",
                        cleared_count
                    )),
                })
            }
        }
    }
}

#[async_trait]
impl Extension for AdaExtension {
    fn id(&self) -> &'static str {
        "ada"
    }

    fn display_name(&self) -> &'static str {
        "ADA Credential Manager"
    }

    fn description(&self) -> &'static str {
        "Manage AWS credentials via the ADA CLI — list, refresh, and clear credential profiles"
    }

    fn operations(&self) -> Vec<OperationDef> {
        vec![
            OperationDef {
                name: "list_credentials".to_string(),
                description: "List all credential profiles from ~/.ada/credentials and ~/.aws/credentials with expiration status".to_string(),
                risk_level: RiskLevel::Low,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "list_profiles".to_string(),
                description: "List managed ADA profiles from ~/.config/ada/profile.json with credential status".to_string(),
                risk_level: RiskLevel::Low,
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            OperationDef {
                name: "refresh_credentials".to_string(),
                description: "Refresh credentials by running `ada credentials update --once` with optional profile, account, role, and provider".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string", "description": "Profile name to refresh" },
                        "account": { "type": "string", "description": "AWS account ID" },
                        "role": { "type": "string", "description": "IAM role name" },
                        "provider": { "type": "string", "description": "Credential provider (e.g. conduit)" }
                    }
                }),
            },
            OperationDef {
                name: "add_profile".to_string(),
                description: "Add a new managed ADA profile to ~/.config/ada/profile.json".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Profile name" },
                        "account": { "type": "string", "description": "AWS account ID" },
                        "role": { "type": "string", "description": "IAM role name" },
                        "provider": { "type": "string", "description": "Credential provider (default: conduit)" }
                    },
                    "required": ["name", "account", "role"]
                }),
            },
            OperationDef {
                name: "remove_profile".to_string(),
                description: "Remove a managed ADA profile from ~/.config/ada/profile.json".to_string(),
                risk_level: RiskLevel::Medium,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Profile name to remove" }
                    },
                    "required": ["name"]
                }),
            },
            OperationDef {
                name: "clear_credentials".to_string(),
                description: "Clear cached credentials from ~/.ada/credentials. If a profile is given, clear only that section; otherwise clear all non-default sections.".to_string(),
                risk_level: RiskLevel::High,
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string", "description": "Profile name to clear (omit to clear all non-default)" }
                    }
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
            "list_credentials" => self.list_credentials_impl().await,
            "list_profiles" => self.list_profiles_impl().await,
            "refresh_credentials" => self.refresh_credentials_impl(input).await,
            "add_profile" => self.add_profile_impl(input).await,
            "remove_profile" => self.remove_profile_impl(input).await,
            "clear_credentials" => self.clear_credentials_impl(input).await,
            _ => Err(ExtensionError::UnknownOperation(operation.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini_basic() {
        let input = r#"
[default]
aws_access_key_id = ASIAXXX
aws_secret_access_key = secret123
expiration_time = 2024-01-15T12:00:00Z

[my-profile]
aws_access_key_id = ASIAYYY
aws_secret_access_key = secret456
"#;
        let sections = AdaExtension::parse_ini(input);
        assert_eq!(sections.len(), 2);
        assert_eq!(
            sections["default"]["aws_access_key_id"],
            "ASIAXXX"
        );
        assert_eq!(
            sections["my-profile"]["aws_access_key_id"],
            "ASIAYYY"
        );
        assert_eq!(
            sections["default"]["expiration_time"],
            "2024-01-15T12:00:00Z"
        );
    }

    #[test]
    fn test_parse_ini_empty() {
        let sections = AdaExtension::parse_ini("");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_parse_ini_comments() {
        let input = r#"
# This is a comment
; This is also a comment
[section]
key = value
"#;
        let sections = AdaExtension::parse_ini(input);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections["section"]["key"], "value");
    }

    #[test]
    fn test_write_ini_roundtrip() {
        let mut sections = HashMap::new();
        let mut fields = HashMap::new();
        fields.insert("aws_access_key_id".to_string(), "ASIAXXX".to_string());
        fields.insert("aws_secret_access_key".to_string(), "secret".to_string());
        sections.insert("default".to_string(), fields);

        let output = AdaExtension::write_ini(&sections);
        let parsed = AdaExtension::parse_ini(&output);
        assert_eq!(parsed["default"]["aws_access_key_id"], "ASIAXXX");
        assert_eq!(parsed["default"]["aws_secret_access_key"], "secret");
    }

    #[test]
    fn test_credential_status_no_creds() {
        let fields = HashMap::new();
        let (status, time) = AdaExtension::credential_status(&fields);
        assert_eq!(status, "none");
        assert_eq!(time, Some("No Credentials".to_string()));
    }

    #[test]
    fn test_credential_status_no_expiration() {
        let mut fields = HashMap::new();
        fields.insert("aws_access_key_id".to_string(), "ASIAXXX".to_string());
        let (status, time) = AdaExtension::credential_status(&fields);
        assert_eq!(status, "active");
        assert_eq!(time, Some("Active".to_string()));
    }

    #[test]
    fn test_credential_status_expired() {
        let mut fields = HashMap::new();
        fields.insert("aws_access_key_id".to_string(), "ASIAXXX".to_string());
        fields.insert(
            "expiration_time".to_string(),
            "2020-01-01T00:00:00Z".to_string(),
        );
        let (status, _) = AdaExtension::credential_status(&fields);
        assert_eq!(status, "expired");
    }

    #[test]
    fn test_credential_status_active_future() {
        let mut fields = HashMap::new();
        fields.insert("aws_access_key_id".to_string(), "ASIAXXX".to_string());
        // Far future
        fields.insert(
            "expiration_time".to_string(),
            "2099-01-01T00:00:00Z".to_string(),
        );
        let (status, time) = AdaExtension::credential_status(&fields);
        assert_eq!(status, "active");
        assert!(time.is_some());
        let t = time.unwrap();
        assert!(t.contains('h'), "Expected hours in '{}' for far future", t);
    }

    #[test]
    fn test_extension_metadata() {
        let ext = AdaExtension::new();
        assert_eq!(ext.id(), "ada");
        assert_eq!(ext.display_name(), "ADA Credential Manager");
        assert_eq!(ext.operations().len(), 6);
    }

    #[test]
    fn test_operations_have_correct_risk_levels() {
        let ext = AdaExtension::new();
        let ops = ext.operations();

        let find_op = |name: &str| ops.iter().find(|o| o.name == name).unwrap();

        assert!(matches!(find_op("list_credentials").risk_level, RiskLevel::Low));
        assert!(matches!(find_op("list_profiles").risk_level, RiskLevel::Low));
        assert!(matches!(find_op("refresh_credentials").risk_level, RiskLevel::Medium));
        assert!(matches!(find_op("add_profile").risk_level, RiskLevel::Medium));
        assert!(matches!(find_op("remove_profile").risk_level, RiskLevel::Medium));
        assert!(matches!(find_op("clear_credentials").risk_level, RiskLevel::High));
    }
}
