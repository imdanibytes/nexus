# Plugin Metadata API & Credential Vending

The Plugin Metadata API provides self-introspection endpoints for plugins and a
credential vending mechanism backed by host extensions. Plugins can query their
own identity, monitor container resource usage, and request credentials from
extension-backed credential providers — all through the standard Host API with
OAuth Bearer authentication.

**Design references:**
- [AWS IMDSv2](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/configuring-instance-metadata-service.html) — REST metadata path hierarchy, short-lived credentials
- [SPIFFE Workload API](https://github.com/spiffe/spiffe/blob/main/standards/SPIFFE_Workload_API.md) — workload identity self-introspection
- [RFC 8693](https://datatracker.ietf.org/doc/html/rfc8693) — token exchange for scoped credentials

---

## Table of Contents

1. [Self-Introspection](#self-introspection)
   - [GET /api/v1/meta/self](#get-apiv1metaself)
   - [GET /api/v1/meta/stats](#get-apiv1metastats)
2. [Credential Vending](#credential-vending)
   - [GET /api/v1/meta/credentials](#get-apiv1metacredentials)
   - [POST /api/v1/meta/credentials/{ext_id}](#post-apiv1metacredentialsext_id)
3. [Permission Model](#permission-model)
4. [Credential Provider Extension Spec](#credential-provider-extension-spec)
   - [Manifest](#manifest)
   - [Required Operations](#required-operations)
   - [JSON-RPC Protocol](#json-rpc-protocol)
   - [Implementation Checklist](#implementation-checklist)
5. [SDK Usage](#sdk-usage)
6. [Error Responses](#error-responses)

---

## Self-Introspection

These endpoints require only a valid OAuth Bearer token. No additional
permissions are needed — every authenticated plugin can introspect itself.

### GET /api/v1/meta/self

Returns the calling plugin's identity, version, current status, and all
permissions with their states and approved scopes.

**Request:**
```http
GET /api/v1/meta/self HTTP/1.1
Authorization: Bearer <access_token>
```

**Response:**
```json
{
  "plugin_id": "com.example.my-plugin",
  "name": "My Plugin",
  "version": "1.2.0",
  "status": "running",
  "permissions": [
    {
      "permission": "filesystem:read",
      "state": "active",
      "approved_scopes": ["/Users/dani/projects", "/tmp"]
    },
    {
      "permission": "credential:aws-credentials",
      "state": "deferred",
      "approved_scopes": null
    }
  ]
}
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `plugin_id` | string | The plugin's unique identifier |
| `name` | string | Human-readable display name |
| `version` | string | Semver version from the manifest |
| `status` | string | Current container status: `"running"`, `"stopped"`, `"error"` |
| `permissions` | array | All permissions declared by this plugin |
| `permissions[].permission` | string | Permission identifier (e.g., `"filesystem:read"`, `"credential:aws-credentials"`) |
| `permissions[].state` | string | `"active"`, `"deferred"`, or `"revoked"` |
| `permissions[].approved_scopes` | string[] \| null | Approved scopes/paths. `null` = unrestricted (or not applicable). `[]` = restricted but nothing approved yet. |

---

### GET /api/v1/meta/stats

Returns real-time container resource usage for the calling plugin.

**Request:**
```http
GET /api/v1/meta/stats HTTP/1.1
Authorization: Bearer <access_token>
```

**Response:**
```json
{
  "container_id": "a1b2c3d4e5f6",
  "cpu_percent": 2.34,
  "memory_usage_bytes": 52428800,
  "memory_limit_bytes": 268435456,
  "network_rx_bytes": 1048576,
  "network_tx_bytes": 524288
}
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `container_id` | string | Docker container ID (short form) |
| `cpu_percent` | number | Current CPU usage as a percentage |
| `memory_usage_bytes` | integer | Current memory usage in bytes |
| `memory_limit_bytes` | integer | Memory limit assigned to the container |
| `network_rx_bytes` | integer | Total bytes received since container start |
| `network_tx_bytes` | integer | Total bytes transmitted since container start |

Returns `503 Service Unavailable` if the container is not running or stats are
temporarily unavailable.

---

## Credential Vending

Credential vending allows plugins to request host-provisioned credentials (AWS
keys, GitHub tokens, Vault secrets, etc.) from extension-backed credential
providers. This eliminates the need to mount host credential files into
containers or pass secrets as environment variables.

### How It Works

1. A **credential provider** is a standard Nexus host extension that declares
   the `credential_provider` capability and exposes `list_scopes` and `resolve`
   operations.
2. A plugin declares `"credential:{ext_id}"` in its manifest permissions to
   indicate it needs credentials from that provider.
3. At install time, the user approves or defers the credential permission.
4. At runtime, when the plugin requests a specific scope (e.g., AWS profile
   "prod"), Nexus checks the permission state and may trigger a **scope approval
   dialog** — the same UX pattern as filesystem path approval.
5. Once approved, Nexus calls the extension's `resolve` operation and returns
   the credentials to the plugin.

### GET /api/v1/meta/credentials

Lists all credential providers available to the calling plugin.

**Requires:** At least one `credential:{ext_id}` permission (any state,
including deferred).

**Request:**
```http
GET /api/v1/meta/credentials HTTP/1.1
Authorization: Bearer <access_token>
```

**Response:**
```json
{
  "providers": [
    {
      "id": "aws-credentials",
      "name": "AWS Credentials",
      "scopes": [
        { "id": "default", "label": "Default profile", "description": "~/.aws/credentials [default]" },
        { "id": "prod", "label": "Production", "description": "SSO session for account 123456789" }
      ]
    },
    {
      "id": "github-credentials",
      "name": "GitHub Credentials",
      "scopes": [
        { "id": "personal", "label": "Personal", "description": "GitHub PAT for @yourname" }
      ]
    }
  ]
}
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `providers` | array | Credential providers the plugin has permission for |
| `providers[].id` | string | Extension ID of the credential provider |
| `providers[].name` | string | Human-readable provider name |
| `providers[].scopes` | array | Available credential scopes from the provider's `list_scopes` operation |
| `providers[].scopes[].id` | string | Machine identifier, passed to the resolve endpoint |
| `providers[].scopes[].label` | string | Human-readable scope name (shown in approval dialogs) |
| `providers[].scopes[].description` | string \| null | Additional context about the scope |

---

### POST /api/v1/meta/credentials/{ext_id}

Resolve credentials from a specific provider for a given scope.

**Requires:** `credential:{ext_id}` permission in Active or Deferred state.

**Request:**
```http
POST /api/v1/meta/credentials/aws-credentials HTTP/1.1
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "scope": "default"
}
```

**Request body:**

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `scope` | string | no | `"default"` | The credential scope to resolve |

**Response (success):**
```json
{
  "provider": "aws-credentials",
  "scope": "default",
  "data": {
    "credentials": {
      "access_key_id": "AKIAIOSFODNN7EXAMPLE",
      "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
      "session_token": "FwoGZX..."
    },
    "expires_at": "2026-02-20T15:30:00Z",
    "metadata": {
      "source": "sso",
      "account_id": "123456789012",
      "region": "us-east-1"
    }
  }
}
```

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `provider` | string | Extension ID that provided the credentials |
| `scope` | string | The scope that was resolved |
| `data.credentials` | object | Provider-specific credential payload (opaque to Nexus) |
| `data.expires_at` | string \| null | ISO 8601 expiration timestamp. Plugins should re-request before expiry. |
| `data.metadata` | object \| null | Non-secret context the plugin may find useful |

**Runtime behavior:**

| Permission state | Scope status | What happens |
|-----------------|-------------|--------------|
| Active | Scope previously approved | Instant response, no dialog |
| Active | Scope not yet approved | Scope approval dialog shown to user |
| Deferred | — | Full JIT approval dialog (permission + scope) |
| Revoked | — | `403 Forbidden` |
| Not declared | — | `403 Forbidden` |

---

## Permission Model

### Declaring Credential Permissions

Plugins declare credential needs in their `plugin.json` manifest:

```json
{
  "permissions": [
    "credential:aws-credentials",
    "credential:github-credentials"
  ]
}
```

The format is `credential:{extension_id}` where `{extension_id}` is the ID of
a credential provider extension installed on the host.

### Permission Properties

| | |
|-|-|
| **Risk** | High |
| **Grants access to** | `GET /api/v1/meta/credentials`, `POST /api/v1/meta/credentials/{ext_id}` |
| **What it does** | Allows the plugin to request credentials from the named extension. Each specific scope (e.g., an AWS profile) requires separate user approval on first use. |
| **What it does NOT do** | Does not grant blanket access to all scopes. The user approves each scope individually. |
| **Typical use case** | Plugins that interact with cloud services, external APIs, or other systems requiring authentication. |

### Scope Approval

When a plugin requests credentials for a scope that hasn't been approved yet,
Nexus shows a runtime approval dialog to the user:

> **Allow "My Plugin" to access credentials?**
> Provider: AWS Credentials
> Scope: prod (Production — SSO session for account 123456789)
> [Allow] [Allow Always] [Deny]

"Allow Always" adds the scope to `approved_scopes` so future requests for the
same scope proceed without a dialog. This mirrors the filesystem path approval
UX.

### RFC 9396 Authorization Details

Credential permissions are encoded in OAuth tokens as RFC 9396 authorization
details:

```json
{
  "type": "nexus:credential",
  "actions": ["resolve"],
  "identifier": "aws-credentials",
  "locations": ["default", "prod"]
}
```

The `locations` field contains approved scopes. The middleware fast-path checks
the token's authorization details before falling back to the PermissionStore.

---

## Credential Provider Extension Spec

Credential providers are standard Nexus host extensions. Any developer can build
one by following this contract.

### Manifest

```json
{
  "id": "aws-credentials",
  "display_name": "AWS Credentials",
  "version": "1.0.0",
  "description": "Resolves AWS credentials from ~/.aws/credentials and SSO sessions",
  "author": "yourname",
  "license": "MIT",
  "homepage": "https://github.com/yourname/nexus-ext-aws-credentials",
  "author_public_key": "<base64 Ed25519 public key>",
  "capabilities": [
    { "type": "credential_provider" },
    { "type": "file_read", "scope": ["~/.aws/credentials", "~/.aws/config"] }
  ],
  "operations": [
    {
      "name": "list_scopes",
      "description": "List available credential scopes (e.g. AWS profiles)",
      "risk_level": "low",
      "input_schema": { "type": "object", "properties": {} }
    },
    {
      "name": "resolve",
      "description": "Resolve credentials for a given scope",
      "risk_level": "high",
      "input_schema": {
        "type": "object",
        "properties": {
          "scope": { "type": "string", "description": "Credential scope to resolve" }
        },
        "required": ["scope"]
      },
      "scope_key": "scope",
      "scope_description": "credential scope"
    }
  ],
  "binaries": {
    "aarch64-apple-darwin": {
      "url": "https://github.com/yourname/nexus-ext-aws-credentials/releases/download/v1.0.0/aws-credentials-aarch64-apple-darwin",
      "signature": "<base64 Ed25519 signature>",
      "sha256": "<hex SHA-256>"
    },
    "x86_64-apple-darwin": {
      "url": "https://github.com/yourname/nexus-ext-aws-credentials/releases/download/v1.0.0/aws-credentials-x86_64-apple-darwin",
      "signature": "<base64 Ed25519 signature>",
      "sha256": "<hex SHA-256>"
    }
  }
}
```

**Key manifest requirements:**
- Declare `{ "type": "credential_provider" }` in `capabilities`. This is how
  Nexus discovers the extension as a credential provider.
- Capability types and risk levels must be **lowercase snake_case** (Nexus uses
  `#[serde(rename_all = "snake_case")]` — PascalCase will fail deserialization).
- `list_scopes` must be `risk_level: "low"` — it returns only scope identifiers,
  no secrets.
- `resolve` must be `risk_level: "high"` and set `scope_key: "scope"` so Nexus
  enforces per-scope approval.

### Required Operations

#### `list_scopes`

Returns available credential scopes. Called by the metadata API when a plugin
lists providers, and in approval dialogs. Must be cheap — no secrets, no
side effects.

**Input:** `{}` (empty object)

**Output:**
```json
{
  "success": true,
  "data": {
    "scopes": [
      {
        "id": "default",
        "label": "Default profile",
        "description": "~/.aws/credentials [default]"
      },
      {
        "id": "prod",
        "label": "Production",
        "description": "SSO session for account 123456789"
      }
    ]
  }
}
```

**Scope fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Machine identifier, passed to `resolve` as the `scope` value |
| `label` | string | yes | Human-readable name shown in approval dialogs |
| `description` | string | no | Additional context displayed in settings and audit views |

#### `resolve`

Resolves actual credentials for a scope. This is the sensitive operation — Nexus
only calls it after permission and scope approval.

**Input:** `{ "scope": "<scope_id>" }`

**Output (success):**
```json
{
  "success": true,
  "data": {
    "credentials": {
      "access_key_id": "AKIAIOSFODNN7EXAMPLE",
      "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
      "session_token": "FwoGZX..."
    },
    "expires_at": "2026-02-20T15:30:00Z",
    "metadata": {
      "source": "sso",
      "account_id": "123456789012",
      "region": "us-east-1"
    }
  }
}
```

**Output fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `credentials` | object | yes | Provider-specific credential payload. Opaque to Nexus — passed through to the plugin as-is. The provider and plugin agree on the shape. |
| `expires_at` | string (ISO 8601) | no | When these credentials expire. Nexus includes this in the response envelope. Plugins should re-request before expiry. |
| `metadata` | object | no | Non-secret context the plugin may find useful (e.g., region, account ID, source type) |

**Output (failure):**
```json
{
  "success": false,
  "data": null,
  "message": "SSO session expired. Run `aws sso login --profile prod` on the host."
}
```

Failure messages are returned to the plugin as error text. Write messages that
help the user take action — the plugin may display these in its UI.

### JSON-RPC Protocol

Extensions are native binaries communicating over stdin/stdout using
line-delimited JSON-RPC 2.0. The host sends requests, the extension responds.

**Lifecycle:**
1. Host sends `initialize` → extension responds with `null`
2. Host sends `execute` for each operation call → extension responds with
   `OperationResult`
3. Host sends `shutdown` → extension responds and exits

**Example — `list_scopes`:**
```
→ {"jsonrpc":"2.0","method":"execute","params":{"operation":"list_scopes","input":{}},"id":1}
← {"jsonrpc":"2.0","result":{"success":true,"data":{"scopes":[{"id":"default","label":"Default profile"}]}},"id":1}
```

**Example — `resolve`:**
```
→ {"jsonrpc":"2.0","method":"execute","params":{"operation":"resolve","input":{"scope":"prod"},"caller_plugin_id":"com.example.my-plugin"},"id":2}
← {"jsonrpc":"2.0","result":{"success":true,"data":{"credentials":{"access_key_id":"AKIA...","secret_access_key":"...","session_token":"..."},"expires_at":"2026-02-20T15:30:00Z"}},"id":2}
```

**Critical implementation details:**
- Write exactly **one JSON line per response**, terminated by `\n`
- **Flush stdout** after every write — buffered I/O will deadlock the host
- All logging goes to **stderr**, never stdout
- `caller_plugin_id` tells you which plugin is requesting credentials (for
  audit/policy decisions)

### Implementation Checklist

1. Declare `{ "type": "credential_provider" }` in `capabilities`
2. Expose `list_scopes` operation (`risk_level: "low"`, empty input schema)
3. Expose `resolve` operation (`risk_level: "high"`, `scope_key: "scope"`)
4. Return `OperationResult` format from both operations
5. Handle unknown scopes gracefully (`success: false` with a helpful message)
6. Include `expires_at` when credentials are time-limited
7. Never log credential values to stderr — log scope requests and outcomes only
8. Sign the binary with your Ed25519 key and fill in `binaries` with per-platform entries

### Example Providers

| Provider | Scopes | Credential payload |
|----------|--------|--------------------|
| `aws-credentials` | AWS profile names (`default`, `prod`) | `{ access_key_id, secret_access_key, session_token }` |
| `github-credentials` | Account or org names | `{ token, token_type, expires_at }` |
| `vault-credentials` | Vault secret paths (`secret/data/myapp`) | `{ data: { ... } }` (arbitrary KV) |
| `gcp-credentials` | GCP project IDs | `{ access_token, token_type, expires_in }` |

The `credentials` object is **opaque to Nexus** — it's passed through directly
to the requesting plugin. The provider and plugin agree on the shape; Nexus only
mediates access control.

---

## SDK Usage

### TypeScript (Browser — NexusPlugin)

```typescript
import { NexusPlugin } from "@imdanibytes/nexus-sdk";

const nexus = new NexusPlugin();

// Self-introspection
const me = await nexus.meta.self();
console.log(`I am ${me.name} v${me.version}, status: ${me.status}`);
console.log(`Permissions: ${me.permissions.length}`);

// Container stats
const stats = await nexus.meta.stats();
console.log(`Memory: ${stats.memory_usage_bytes} / ${stats.memory_limit_bytes}`);

// List available credential providers
const providers = await nexus.meta.credentials();
for (const p of providers.providers) {
  console.log(`Provider ${p.id}: ${p.scopes.length} scopes`);
}

// Resolve credentials
const aws = await nexus.credentials("aws-credentials", { scope: "default" });
console.log(`AWS key: ${aws.data.credentials.access_key_id}`);
if (aws.data.expires_at) {
  console.log(`Expires: ${aws.data.expires_at}`);
}
```

### TypeScript (Server — NexusServer)

```typescript
import { NexusServer } from "@imdanibytes/nexus-sdk/server";

const server = new NexusServer();

// Same API surface
const me = await server.meta.self();
const aws = await server.credentials("aws-credentials", { scope: "prod" });
```

---

## Error Responses

All error responses use a consistent envelope:

```json
{
  "error": "human-readable error message"
}
```

| Status | Condition |
|--------|-----------|
| `400 Bad Request` | Malformed request body or invalid scope |
| `401 Unauthorized` | Missing or invalid Bearer token |
| `403 Forbidden` | Permission not declared, revoked, or user denied the approval dialog |
| `404 Not Found` | Extension not found or not a credential provider |
| `500 Internal Server Error` | Extension call failed (timeout, crash, etc.) |
| `503 Service Unavailable` | Container not running (for `/meta/stats`) or extension not enabled |
