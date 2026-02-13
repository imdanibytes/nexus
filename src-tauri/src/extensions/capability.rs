use serde::{Deserialize, Serialize};

/// Declared capabilities — shown to users at install time for informed consent.
///
/// NOT technically enforced (native process has full host access).
/// Serves: user review, audit trail, UI display, future OS-level sandboxing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Capability {
    /// Runs these CLI commands (e.g. ["git", "npm"])
    ProcessExec { scope: Vec<String> },
    /// Reads files matching these patterns (e.g. ["**/.git/**"])
    FileRead { scope: Vec<String> },
    /// Writes files matching these patterns
    FileWrite { scope: Vec<String> },
    /// Makes HTTP requests to these domains
    NetworkHttp { scope: Vec<String> },
    /// Reads OS metadata (hostname, uptime, etc.)
    SystemInfo,
    /// Links native libraries (e.g. ["libopencv", "libcuda"])
    NativeLibrary { scope: Vec<String> },
    /// Arbitrary capability not covered above (e.g. "bluetooth", "serial_port")
    Custom { name: String, description: String },
}
