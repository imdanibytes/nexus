# Extension Authoring Guide

Build host extensions for Nexus. Extensions are native binaries that run
directly on the host machine, providing operations that plugins can consume.

**Extensions vs Plugins**: Plugins are Docker containers with a web UI.
Extensions are headless native binaries that expose operations via JSON-RPC.
Plugins call extension operations through the Host API.

## Quick Start

```
my-extension/
├── manifest.json        # Extension manifest (required)
├── src/                 # Your source code
│   └── main.rs          # (or main.go, main.py, etc.)
└── .github/
    └── workflows/
        └── release.yml  # Build + publish binaries per platform
```

### 1. Create the Manifest

`manifest.json` declares your extension's identity, operations, capabilities,
and signed binaries for each platform.

```json
{
  "id": "my_extension",
  "display_name": "My Extension",
  "version": "1.0.0",
  "description": "What this extension does.",
  "author": "yourname",
  "license": "MIT",
  "homepage": "https://github.com/yourname/my-extension",
  "author_public_key": "<base64 Ed25519 public key>",
  "operations": [
    {
      "name": "do_something",
      "description": "Performs a useful action.",
      "risk_level": "low",
      "input_schema": {
        "type": "object",
        "properties": {
          "target": {
            "type": "string",
            "description": "What to act on"
          }
        },
        "required": ["target"]
      }
    }
  ],
  "capabilities": [
    { "type": "network_http", "scope": ["api.example.com"] }
  ],
  "binaries": {
    "aarch64-apple-darwin": {
      "url": "https://github.com/yourname/my-extension/releases/download/v1.0.0/my-extension-aarch64-apple-darwin",
      "signature": "<base64 Ed25519 signature>",
      "sha256": "<hex SHA-256 hash>"
    },
    "x86_64-apple-darwin": {
      "url": "https://github.com/yourname/my-extension/releases/download/v1.0.0/my-extension-x86_64-apple-darwin",
      "signature": "<base64 Ed25519 signature>",
      "sha256": "<hex SHA-256 hash>"
    }
  }
}
```

See [Extension Manifest Specification](#manifest-specification) for the full
schema and validation rules.

### 2. Implement the JSON-RPC Server

Your binary communicates with Nexus over **stdin/stdout** using line-delimited
JSON-RPC 2.0. You must handle three methods: `initialize`, `execute`, and
`shutdown`.

Here's a minimal implementation in Rust:

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
struct Request {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Value,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: Value,
}

fn handle_execute(operation: &str, input: &Value) -> Result<Value, String> {
    match operation {
        "do_something" => {
            let target = input["target"].as_str().unwrap_or("");
            Ok(json!({
                "success": true,
                "data": { "result": format!("Did something to {}", target) },
                "message": null
            }))
        }
        _ => Err(format!("Unknown operation: {}", operation)),
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.expect("Failed to read line");
        let req: Request = serde_json::from_str(&line).expect("Invalid JSON-RPC");

        let response = match req.method.as_str() {
            "initialize" => Response {
                jsonrpc: "2.0".into(),
                result: Some(Value::Null),
                error: None,
                id: req.id,
            },
            "execute" => {
                let operation = req.params["operation"].as_str().unwrap_or("");
                let input = &req.params["input"];
                match handle_execute(operation, input) {
                    Ok(result) => Response {
                        jsonrpc: "2.0".into(),
                        result: Some(result),
                        error: None,
                        id: req.id,
                    },
                    Err(msg) => Response {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(json!({"code": -1, "message": msg})),
                        id: req.id,
                    },
                }
            }
            "shutdown" => {
                let resp = Response {
                    jsonrpc: "2.0".into(),
                    result: Some(Value::Null),
                    error: None,
                    id: req.id,
                };
                serde_json::to_writer(&mut out, &resp).unwrap();
                writeln!(out).unwrap();
                out.flush().unwrap();
                std::process::exit(0);
            }
            _ => Response {
                jsonrpc: "2.0".into(),
                result: None,
                error: Some(json!({"code": -32601, "message": "Method not found"})),
                id: req.id,
            },
        };

        serde_json::to_writer(&mut out, &response).unwrap();
        writeln!(out).unwrap();
        out.flush().unwrap();
    }
}
```

### 3. Sign Your Binary

Extensions use Ed25519 signatures for integrity verification.

**Generate a key pair** (once, keep the private key safe):

```bash
# Using openssl
openssl genpkey -algorithm ed25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Extract raw base64 public key for the manifest
openssl pkey -in public.pem -pubout -outform DER | tail -c 32 | base64
```

**Sign a binary**:

```bash
# Compute SHA-256 of the binary
SHA=$(shasum -a 256 my-extension | awk '{print $1}')

# Sign the hash with Ed25519
echo -n "$SHA" | openssl pkeyutl -sign -inkey private.pem | base64

# Put both in the manifest
# sha256: "<hex hash>"
# signature: "<base64 signature>"
```

The `signature` field is the Ed25519 signature over the SHA-256 hash of the
binary (not the binary itself). Nexus verifies this at install time.

**Key management**: Nexus uses a **Trust On First Use (TOFU)** model. The first
time it sees your `author_public_key`, it trusts and stores it. On subsequent
installs or updates, Nexus verifies the key hasn't changed. If it has, the user
sees a security warning.

### 4. Build for Multiple Platforms

Nexus supports these platform triples:

| Triple | OS |
|--------|----|
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-apple-darwin` | macOS Intel |
| `x86_64-unknown-linux-gnu` | Linux x86-64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-pc-windows-msvc` | Windows x86-64 |
| `aarch64-pc-windows-msvc` | Windows ARM64 |

You don't need to support all platforms — only include the ones you build for.
Nexus matches the user's platform at install time and downloads the correct
binary.

**Recommended CI setup** (GitHub Actions):

```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-apple-darwin
            os: macos-13
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - run: cargo build --release --target ${{ matrix.target }}

      - name: Sign binary
        run: |
          BINARY=target/${{ matrix.target }}/release/my-extension
          SHA=$(shasum -a 256 "$BINARY" | awk '{print $1}')
          SIG=$(echo -n "$SHA" | openssl pkeyutl -sign -inkey private.pem | base64)
          echo "sha256=$SHA" >> "$GITHUB_OUTPUT"
          echo "signature=$SIG" >> "$GITHUB_OUTPUT"

      - name: Upload
        uses: actions/upload-artifact@v4
        with:
          name: my-extension-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/my-extension
```

After CI completes, update the `binaries` section of your manifest with the
download URLs, signatures, and hashes.

### 5. Test Locally

During development, install your extension from a local manifest:

1. Build your binary: `cargo build --release`
2. Update `manifest.json` to point at the local binary (use a `file://` URL
   or copy the binary next to the manifest)
3. In Nexus, go to **Settings > Extensions** and use **Install from File**

For rapid iteration, disable/re-enable the extension after rebuilding — Nexus
will restart the subprocess with the new binary.

---

## JSON-RPC Protocol

Extensions communicate with Nexus over **stdin/stdout** using line-delimited
JSON-RPC 2.0. Each message is a single JSON object followed by a newline (`\n`).

### Messages

#### `initialize`

Sent immediately after the subprocess is spawned. Use this to set up any
internal state.

```json
// Request
{"jsonrpc":"2.0","method":"initialize","params":{"extension_id":"my_extension","version":"1.0.0"},"id":0}

// Response (acknowledge)
{"jsonrpc":"2.0","result":null,"id":0}
```

#### `execute`

Called when a plugin invokes one of your operations.

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "execute",
  "params": {
    "operation": "do_something",
    "input": { "target": "example" },
    "caller_plugin_id": "com.example.my-plugin"
  },
  "id": 1
}

// Success response
{
  "jsonrpc": "2.0",
  "result": {
    "success": true,
    "data": { "output": "Done" },
    "message": null
  },
  "id": 1
}

// Error response
{
  "jsonrpc": "2.0",
  "error": { "code": -1, "message": "Something went wrong" },
  "id": 1
}
```

The `caller_plugin_id` tells you which plugin is calling. Use this for
per-plugin state or audit logging if needed.

#### `shutdown`

Sent when Nexus is shutting down or the extension is being disabled. Respond
with an acknowledgment and exit cleanly. Nexus waits 5 seconds — if your
process hasn't exited by then, it gets killed.

```json
// Request
{"jsonrpc":"2.0","method":"shutdown","params":{},"id":2}

// Response (then exit)
{"jsonrpc":"2.0","result":null,"id":2}
```

### Important Protocol Rules

- **Flush stdout after every response.** Nexus reads line-by-line. If you don't
  flush, the host will hang waiting for your response.
- **One JSON object per line.** No pretty-printing. Each response must be a
  single line terminated by `\n`.
- **Don't write to stdout for anything else.** Log messages should go to stderr.
  Anything on stdout is interpreted as a JSON-RPC response.
- **Responses are synchronous.** Nexus sends one request and waits for one
  response before sending the next. No concurrent requests.

---

## Operations

Operations are the functions your extension exposes to plugins.

### Definition

Each operation in the manifest declares:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Lowercase alphanumeric + underscore (e.g., `get_forecast`) |
| `description` | Yes | What this operation does |
| `risk_level` | Yes | `"low"`, `"medium"`, or `"high"` |
| `input_schema` | Yes | JSON Schema for the input object |
| `scope_key` | No | Input field name for scope checking |
| `scope_description` | No | Human-readable label for the scope field |

### Risk Levels

Risk levels control how much user approval is required:

| Level | Behavior |
|-------|----------|
| `low` | Permission check only. No additional approval once granted. |
| `medium` | Permission check only. Same as low (reserved for future use). |
| `high` | **Per-invocation approval.** User sees a dialog every time this operation is called with the full input parameters. |

Use `high` for destructive or sensitive operations (deleting data, sending
messages, modifying system config). Use `low` for read-only or idempotent
operations.

### Scope Checking

Scope checking adds path-level or resource-level granularity. If an operation
declares `scope_key`, Nexus extracts that field from the input and checks it
against the user's approved scopes.

```json
{
  "name": "read_file",
  "description": "Read a file from the filesystem",
  "risk_level": "low",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": { "type": "string" }
    },
    "required": ["path"]
  },
  "scope_key": "path",
  "scope_description": "File Path"
}
```

When a plugin calls `read_file` with `{"path": "/home/user/docs/report.txt"}`:

1. Nexus extracts `input["path"]` → `"/home/user/docs/report.txt"`
2. Checks against approved scopes for this operation
3. If not approved → shows runtime dialog: *"Allow access to File Path: /home/user/docs/report.txt?"*
4. User can Allow (persisted), Allow Once, or Deny

This is similar to the filesystem permission prompts for plugins, but scoped to
your extension's operations.

### Input Validation

Nexus validates operation inputs against the declared `input_schema` before
forwarding the request to your process. Invalid inputs get a 400 error before
your code even sees them. Keep your schemas accurate.

---

## Capabilities

Capabilities declare what system resources your extension uses. They're shown
to the user at install time for informed consent.

**Capabilities are informational only.** They are NOT enforced at runtime.
Your native binary has full host access. Capabilities serve as transparency
and audit trail.

### Types

| Type | Fields | Example |
|------|--------|---------|
| `process_exec` | `scope: string[]` | `["git", "npm", "cargo"]` |
| `file_read` | `scope: string[]` | `["~/.ssh/config", "**/*.json"]` |
| `file_write` | `scope: string[]` | `["/tmp/**", "~/.config/myext/"]` |
| `network_http` | `scope: string[]` | `["api.github.com", "*.example.com"]` |
| `system_info` | (none) | Read hostname, uptime, etc. |
| `native_library` | `scope: string[]` | `["libcurl", "libssl"]` |
| `custom` | `name`, `description` | `{"type":"custom","name":"Bluetooth","description":"Scan for BLE devices"}` |

### Best Practices

- **Be specific.** Don't declare `file_read` with scope `["**/*"]`. List the
  actual paths your extension reads.
- **Be honest.** If your extension makes HTTP requests, declare `network_http`
  with the domains. Users who audit capabilities will notice if you're accessing
  undeclared resources.
- **Use `custom` for unusual access.** Serial ports, Bluetooth, GPU compute —
  if it doesn't fit the standard types, use `custom` with a clear description.

---

## Plugin Integration

Plugins consume extension operations through the Host API.

### Declaring Extension Dependencies

Plugins list the extensions and operations they need in their `plugin.json`:

```json
{
  "id": "com.example.my-plugin",
  "name": "My Plugin",
  "extensions": {
    "my_extension": ["do_something", "do_other_thing"],
    "another_ext": ["query"]
  },
  "permissions": ["system:info"]
}
```

This generates permission strings at install time:
- `ext:my_extension:do_something`
- `ext:my_extension:do_other_thing`
- `ext:another_ext:query`

Users approve these alongside regular permissions during plugin installation.
Permissions can be deferred (approved later on first use).

### Calling Operations from a Plugin

```javascript
// List available extensions and their operations
const res = await fetch(`${apiUrl}/api/v1/extensions`, {
  headers: { Authorization: `Bearer ${token}` }
});
const { extensions } = await res.json();
// [{ id: "my_extension", available: true, operations: [{name: "do_something", permitted: true}] }]

// Call an operation
const result = await fetch(`${apiUrl}/api/v1/extensions/my_extension/do_something`, {
  method: "POST",
  headers: {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json"
  },
  body: JSON.stringify({ input: { target: "example" } })
});
const data = await result.json();
// { success: true, data: { output: "Done" }, message: null }
```

### Error Responses

| Status | Cause |
|--------|-------|
| 400 | Invalid input (failed schema validation) |
| 403 | Permission denied (not declared or revoked) |
| 404 | Extension not installed or operation doesn't exist |
| 500 | Extension process error or crash |

---

## Security Model

### Signature Verification

Every binary must be signed with Ed25519. At install time, Nexus:

1. Downloads the binary
2. Computes `sha256(binary)` and compares against the manifest's `sha256` field
3. Verifies the Ed25519 signature over the hash using `author_public_key`
4. If either check fails, installation is rejected

### Trust On First Use (TOFU)

Nexus maintains a trusted key store (`~/.nexus/trusted_keys.json`):

- **First install by an author**: Key is trusted and stored
- **Subsequent installs**: Key is verified against the stored value
- **Key changed**: Security warning. Updates are rejected unless the user
  explicitly accepts the new key (`force_key`)

This prevents supply chain attacks where an attacker replaces the binary and
signs with a different key.

### Three-Layer Permission Model

When a plugin calls an extension operation, three checks happen in order:

1. **Permission check** — Does the plugin have `ext:{ext_id}:{operation}`?
   - Active → proceed
   - Deferred → show just-in-time approval dialog
   - Revoked → 403

2. **Scope check** — If the operation has `scope_key`, is the scope value
   approved?
   - No scope_key → skip
   - Value approved → proceed
   - Not approved → show scope approval dialog

3. **Risk check** — Is the operation `high` risk?
   - Low/Medium → proceed
   - High → show per-invocation approval dialog with full input details

### No Runtime Sandboxing

Extensions run as native processes with full host access. The security model
relies on:
- Signature verification at install (integrity)
- TOFU key management (supply chain protection)
- User review of capabilities (transparency)
- Operation-level permissions (access control)
- Scope and risk approval (runtime consent)

There is no OS-level sandboxing. Only install extensions you trust.

---

## Manifest Specification

### Required Fields

| Field | Constraint |
|-------|-----------|
| `id` | 1-100 chars, lowercase alphanumeric + `_` + `-` |
| `display_name` | 1-100 chars |
| `version` | 1-50 chars |
| `description` | 1-2000 chars |
| `author` | 1-100 chars |
| `author_public_key` | Base64-encoded Ed25519 public key (32 bytes) |
| `operations` | At least one operation |
| `binaries` | At least one platform entry |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `license` | string | SPDX license identifier |
| `homepage` | string | Project URL |
| `capabilities` | array | Declared system capabilities |

### Operation Schema

| Field | Required | Type |
|-------|----------|------|
| `name` | Yes | Lowercase alphanumeric + `_` |
| `description` | Yes | string |
| `risk_level` | Yes | `"low"` \| `"medium"` \| `"high"` |
| `input_schema` | Yes | JSON Schema object (`"type": "object"`) |
| `scope_key` | No | string (field name in input) |
| `scope_description` | No | string (human label) |

### Binary Entry Schema

| Field | Required | Type |
|-------|----------|------|
| `url` | Yes | HTTP or HTTPS URL |
| `signature` | Yes | Base64 Ed25519 signature over sha256(binary) |
| `sha256` | Yes | Hex-encoded SHA-256 hash |

### Content Restrictions

Unicode bidirectional override characters are rejected in `display_name`,
`description`, and `author` (prevents text spoofing attacks). Same set as the
plugin manifest — see [Manifest Spec](spec/manifest-spec.md#bidirectional-characters-blocked).

---

## Publishing

To publish an extension to the Nexus marketplace:

1. Host your `manifest.json` at a public URL (GitHub raw URL works)
2. Host your signed binaries at public URLs (GitHub Releases works)
3. Add your extension to a registry as a YAML entry:

```yaml
# extensions/my_extension.yaml
id: my_extension
name: My Extension
version: "1.0.0"
description: What it does
author: yourname
author_url: https://github.com/yourname
author_public_key: <base64 Ed25519 public key>
manifest_url: https://raw.githubusercontent.com/yourname/my-extension/main/manifest.json
platforms:
  - aarch64-apple-darwin
  - x86_64-apple-darwin
  - x86_64-unknown-linux-gnu
categories:
  - automation
created_at: "2026-02-14T12:00:00Z"
status: active
```

Users browse extensions in the Nexus marketplace. The install flow is:

1. **Review & Install** — fetches the manifest
2. **Capabilities review** — user sees what the extension accesses
3. **Operations review** — user sees operations with risk levels
4. **Install** — downloads and verifies the binary for their platform

---

## Development Tips

**Language choice**: Use any language that compiles to a native binary. Rust, Go,
C, C++, Zig — anything that produces an executable. Python and Node.js work too
if you bundle them with PyInstaller, pkg, or similar.

**Logging**: Write logs to **stderr**, not stdout. Stdout is reserved for
JSON-RPC communication. Anything on stdout that isn't valid JSON-RPC will cause
protocol errors.

**Testing**: Test your JSON-RPC implementation by piping messages to your binary:

```bash
echo '{"jsonrpc":"2.0","method":"initialize","params":{"extension_id":"test","version":"1.0.0"},"id":0}' | ./my-extension
echo '{"jsonrpc":"2.0","method":"execute","params":{"operation":"do_something","input":{"target":"test"}},"id":1}' | ./my-extension
```

**Debugging**: Enable Nexus debug logging to see the full JSON-RPC message flow
between the host and your extension process.

**Graceful shutdown**: Always handle the `shutdown` method. If your process
ignores it, Nexus will force-kill it after 5 seconds, which may leave state
inconsistent.

**Stdout flushing**: This is the most common bug. If your language buffers
stdout (most do), you must explicitly flush after writing each JSON-RPC response.
In Rust: `stdout.flush()`. In Go: `os.Stdout.Sync()`. In Python:
`sys.stdout.flush()` or run with `-u` flag.
