//! API key lifecycle management — generation, validation, revocation, and persistence.
//!
//! # Security model
//!
//! - **Key format**: `nxk_` prefix + 40 Base62 characters (44 chars total).
//!   Base62 = `[0-9A-Za-z]` — 62 symbols per position, so 40 random chars yield
//!   `log2(62^40) ≈ 238` bits of entropy. This exceeds the 128-bit minimum
//!   recommended by NIST SP 800-63B §5.1.1 for memorized secrets (our keys are
//!   not memorized, but the bar is a useful baseline).
//!
//! - **Prefix routing**: The `nxk_` prefix lets the gateway auth middleware (in
//!   [`crate::host_api::mcp`]) distinguish API keys from OAuth access tokens in O(1)
//!   without hitting the OAuth store. Tokens starting with `nxk_` are validated here;
//!   all others fall through to OAuth Bearer validation per RFC 6750.
//!
//! - **Hash storage**: Raw keys are hashed with SHA-256 before persistence (OWASP
//!   credential storage guidelines). The only exception is the auto-generated default
//!   key, whose raw value is stored in a separate `mcp_default_key` file so the
//!   Client Setup UI can display it for one-time copy.
//!
//! - **Constant-time validation**: Hash comparison uses XOR-accumulation to prevent
//!   timing side-channel attacks. See [`constant_time_eq`].
//!
//! - **Atomic persistence**: Writes use temp-file + rename to prevent partial writes
//!   on crash (same pattern as [`crate::oauth::OAuthStore`]).

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};

use super::types::ApiKey;

/// Fixed prefix for all Nexus API keys. Enables O(1) routing in the auth middleware
/// (see module docs).
const KEY_PREFIX: &str = "nxk_";

/// Number of random Base62 characters after the prefix.
/// Entropy: log2(62^40) ≈ 238 bits (exceeds NIST SP 800-63B §5.1.1 minimums).
const KEY_RANDOM_LEN: usize = 40;

/// Base62 alphabet: digits + uppercase + lowercase. URL-safe, no special characters,
/// avoids ambiguity issues of Base64 (`+`, `/`, `=`).
const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Generate a raw API key: `nxk_` + 40 random Base62 characters.
/// Uses `rand::rng()` which is backed by a CSPRNG (ChaCha12 on all platforms).
fn generate_raw_key() -> String {
    let mut rng = rand::rng();
    let random_part: String = (0..KEY_RANDOM_LEN)
        .map(|_| {
            let idx = rng.random_range(0..BASE62_CHARS.len());
            BASE62_CHARS[idx] as char
        })
        .collect();
    format!("{}{}", KEY_PREFIX, random_part)
}

/// Compute the SHA-256 hex digest of a raw key.
/// Used for storage (raw key is never persisted) and validation.
fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Constant-time string comparison to prevent timing side-channel attacks.
///
/// Standard `==` on strings short-circuits on the first differing byte, leaking
/// information about how many leading bytes match. This function XORs all byte
/// pairs and accumulates the result, ensuring comparison time depends only on
/// string length — not on where (or whether) strings differ.
///
/// The length check does leak length information, but both inputs are SHA-256 hex
/// digests (always 64 chars), so no information is exposed in practice.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes()
        .iter()
        .zip(b.as_bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Persistent API key store. Thread-safe via interior RwLock.
#[derive(Debug, Clone)]
pub struct ApiKeyStore {
    keys: Arc<RwLock<Vec<ApiKey>>>,
    keys_path: PathBuf,
    default_key_path: PathBuf,
}

impl ApiKeyStore {
    /// Load from disk, creating a default key on first launch.
    pub fn load(data_dir: &Path) -> Self {
        let keys_path = data_dir.join("mcp_api_keys.json");
        let default_key_path = data_dir.join("mcp_default_key");

        let keys: Vec<ApiKey> = if keys_path.exists() {
            std::fs::read_to_string(&keys_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let store = Self {
            keys: Arc::new(RwLock::new(keys)),
            keys_path,
            default_key_path,
        };

        // Create default key on first launch
        if store.keys.read().unwrap().is_empty() {
            let (key, raw) = store.generate_inner("Default");
            store.keys.write().unwrap().push(key);
            store.save();
            // Persist the raw default key so the UI can display it
            let _ = std::fs::write(&store.default_key_path, &raw);
            log::info!("Generated default MCP API key");
        }

        store
    }

    /// Generate a new API key. Returns the stored key + raw key string (shown once).
    pub fn generate(&self, name: &str) -> (ApiKey, String) {
        let (key, raw) = self.generate_inner(name);
        self.keys.write().unwrap().push(key.clone());
        self.save();
        (key, raw)
    }

    fn generate_inner(&self, name: &str) -> (ApiKey, String) {
        let raw = generate_raw_key();
        let key = ApiKey {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            key_hash: hash_key(&raw),
            prefix: raw[..8].to_string(),
            created_at: Utc::now(),
            last_used_at: None,
        };
        (key, raw)
    }

    /// Validate a raw key against stored hashes. Returns the matching [`ApiKey`] if valid.
    ///
    /// Uses constant-time comparison on SHA-256 digests to prevent timing side-channels.
    /// Updates `last_used_at` on success (for audit/display in the UI).
    pub fn validate(&self, raw_key: &str) -> Option<ApiKey> {
        let hash = hash_key(raw_key);
        let mut keys = self.keys.write().unwrap();
        let key = keys.iter_mut().find(|k| constant_time_eq(&k.key_hash, &hash))?;
        key.last_used_at = Some(Utc::now());
        let result = key.clone();
        drop(keys);
        self.save();
        Some(result)
    }

    /// Revoke (remove) an API key by ID.
    pub fn revoke(&self, id: &str) -> bool {
        let mut keys = self.keys.write().unwrap();
        let before = keys.len();
        keys.retain(|k| k.id != id);
        let removed = keys.len() < before;
        drop(keys);
        if removed {
            self.save();
        }
        removed
    }

    /// List all keys (no raw values).
    pub fn list(&self) -> Vec<ApiKey> {
        self.keys.read().unwrap().clone()
    }

    /// Get the raw default key for UI display. Returns None if deleted.
    pub fn get_default_raw(&self) -> Option<String> {
        std::fs::read_to_string(&self.default_key_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Replace the default key: revoke old, generate new, persist raw.
    pub fn regenerate_default(&self) -> (ApiKey, String) {
        // Find and remove existing default
        {
            let mut keys = self.keys.write().unwrap();
            keys.retain(|k| k.name != "Default");
        }

        let (key, raw) = self.generate_inner("Default");
        self.keys.write().unwrap().push(key.clone());
        self.save();
        let _ = std::fs::write(&self.default_key_path, &raw);
        (key, raw)
    }

    fn save(&self) {
        if let Ok(keys) = self.keys.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*keys) {
                let _ = atomic_write(&self.keys_path, json.as_bytes());
            }
        }
    }
}

/// Write atomically via temp-file + rename.
///
/// Guarantees that readers see either the old or new content, never a partial write.
/// `rename(2)` is atomic on POSIX when src and dst are on the same filesystem
/// (which they are — both are in the Tauri data directory).
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_validate() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let (key, raw) = store.generate("Test Key");
        assert!(raw.starts_with("nxk_"));
        assert_eq!(raw.len(), 44); // "nxk_" + 40 chars
        assert_eq!(key.prefix, &raw[..8]);

        let validated = store.validate(&raw);
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().id, key.id);
    }

    #[test]
    fn validate_wrong_key() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let result = store.validate("nxk_this_is_definitely_not_a_real_key_1234");
        assert!(result.is_none());
    }

    #[test]
    fn revoke_key() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let (key, raw) = store.generate("Revoke Me");
        assert!(store.validate(&raw).is_some());

        assert!(store.revoke(&key.id));
        assert!(store.validate(&raw).is_none());
    }

    #[test]
    fn persistence_survives_reload() {
        let tmp = tempfile::tempdir().unwrap();

        let raw;
        {
            let store = ApiKeyStore::load(tmp.path());
            let (_, r) = store.generate("Persist Test");
            raw = r;
        }

        // Reload from disk
        let store2 = ApiKeyStore::load(tmp.path());
        let validated = store2.validate(&raw);
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().name, "Persist Test");
    }

    #[test]
    fn default_key_created_on_first_load() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let keys = store.list();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "Default");

        let raw = store.get_default_raw();
        assert!(raw.is_some());
        assert!(raw.unwrap().starts_with("nxk_"));
    }

    #[test]
    fn last_used_at_updated() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let (key, raw) = store.generate("Usage Test");
        assert!(key.last_used_at.is_none());

        let validated = store.validate(&raw).unwrap();
        assert!(validated.last_used_at.is_some());
    }

    #[test]
    fn regenerate_default_replaces_key() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ApiKeyStore::load(tmp.path());

        let old_raw = store.get_default_raw().unwrap();
        let (new_key, new_raw) = store.regenerate_default();

        assert_ne!(old_raw, new_raw);
        assert_eq!(new_key.name, "Default");

        // Old key should no longer validate
        assert!(store.validate(&old_raw).is_none());
        // New key should validate
        assert!(store.validate(&new_raw).is_some());

        // Only one Default key should exist
        let defaults: Vec<_> = store.list().into_iter().filter(|k| k.name == "Default").collect();
        assert_eq!(defaults.len(), 1);
    }
}
