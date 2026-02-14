use super::discovery::DiscoveredTool;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

/// A tool with inferred permission classifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub permissions: Vec<String>,
    pub requires_approval: bool,
    pub high_risk: bool,
}

struct PermissionRule {
    patterns: Vec<Regex>,
    permission: &'static str,
}

fn permission_rules() -> &'static Vec<PermissionRule> {
    static RULES: OnceLock<Vec<PermissionRule>> = OnceLock::new();
    RULES.get_or_init(|| {
        vec![
            PermissionRule {
                patterns: vec![
                    Regex::new(r"(?i)\bfile\b").unwrap(),
                    Regex::new(r"(?i)\bread\b").unwrap(),
                    Regex::new(r"(?i)\bpath\b").unwrap(),
                    Regex::new(r"(?i)\bdirectory\b").unwrap(),
                    Regex::new(r"(?i)\bfolder\b").unwrap(),
                    Regex::new(r"(?i)\bls\b").unwrap(),
                    Regex::new(r"(?i)list_dir").unwrap(),
                    Regex::new(r"(?i)get_file").unwrap(),
                ],
                permission: "filesystem:read",
            },
            PermissionRule {
                patterns: vec![
                    Regex::new(r"(?i)\bwrite\b").unwrap(),
                    Regex::new(r"(?i)\bsave\b").unwrap(),
                    Regex::new(r"(?i)\bcreate\b").unwrap(),
                    Regex::new(r"(?i)\bdelete\b").unwrap(),
                    Regex::new(r"(?i)\bremove\b").unwrap(),
                    Regex::new(r"(?i)\bmkdir\b").unwrap(),
                    Regex::new(r"(?i)\brename\b").unwrap(),
                    Regex::new(r"(?i)\bmove\b").unwrap(),
                ],
                permission: "filesystem:write",
            },
            PermissionRule {
                patterns: vec![
                    Regex::new(r"(?i)\bfetch\b").unwrap(),
                    Regex::new(r"(?i)\brequest\b").unwrap(),
                    Regex::new(r"(?i)\bhttp\b").unwrap(),
                    Regex::new(r"(?i)\burl\b").unwrap(),
                    Regex::new(r"(?i)\bdownload\b").unwrap(),
                    Regex::new(r"(?i)\bapi\b").unwrap(),
                    Regex::new(r"(?i)\bwebhook\b").unwrap(),
                ],
                permission: "network:internet",
            },
        ]
    })
}

fn high_risk_patterns() -> &'static Vec<Regex> {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\bexec\b").unwrap(),
            Regex::new(r"(?i)\brun\b").unwrap(),
            Regex::new(r"(?i)\bshell\b").unwrap(),
            Regex::new(r"(?i)\bcommand\b").unwrap(),
            Regex::new(r"(?i)\beval\b").unwrap(),
            Regex::new(r"(?i)\bspawn\b").unwrap(),
            Regex::new(r"(?i)\bsystem\b").unwrap(),
            Regex::new(r"(?i)\bbash\b").unwrap(),
        ]
    })
}

/// Classify a discovered tool by inferring permissions from its name and description.
pub fn classify_tool(tool: &DiscoveredTool) -> ClassifiedTool {
    let text = format!("{} {}", tool.name, tool.description);
    let mut permissions: HashSet<&str> = HashSet::new();

    for rule in permission_rules() {
        if rule.patterns.iter().any(|p: &Regex| p.is_match(&text)) {
            permissions.insert(rule.permission);
        }
    }

    let high_risk = high_risk_patterns().iter().any(|p: &Regex| p.is_match(&text));

    ClassifiedTool {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.input_schema.clone(),
        permissions: permissions.into_iter().map(String::from).collect(),
        requires_approval: true,
        high_risk,
    }
}

/// Classify all discovered tools.
pub fn classify_tools(tools: &[DiscoveredTool]) -> Vec<ClassifiedTool> {
    tools.iter().map(classify_tool).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str, description: &str) -> DiscoveredTool {
        DiscoveredTool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn test_filesystem_read_inference() {
        let tool = make_tool("read_file", "Read a file from disk");
        let classified = classify_tool(&tool);
        assert!(classified.permissions.contains(&"filesystem:read".to_string()));
    }

    #[test]
    fn test_filesystem_write_inference() {
        let tool = make_tool("create_file", "Create a new file on disk");
        let classified = classify_tool(&tool);
        assert!(classified.permissions.contains(&"filesystem:write".to_string()));
    }

    #[test]
    fn test_network_inference() {
        let tool = make_tool("fetch_data", "Fetch data from an HTTP API");
        let classified = classify_tool(&tool);
        assert!(classified.permissions.contains(&"network:internet".to_string()));
    }

    #[test]
    fn test_high_risk_detection() {
        let tool = make_tool("exec_command", "Execute a shell command");
        let classified = classify_tool(&tool);
        assert!(classified.high_risk);
    }

    #[test]
    fn test_safe_tool() {
        let tool = make_tool("get_weather", "Get current weather for a city");
        let classified = classify_tool(&tool);
        assert!(!classified.high_risk);
    }

    #[test]
    fn test_all_tools_require_approval() {
        let tool = make_tool("anything", "Does something");
        let classified = classify_tool(&tool);
        assert!(classified.requires_approval);
    }
}
