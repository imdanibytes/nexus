use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ExtensionError;

/// Trusted author keys, persisted to ~/.nexus/trusted_keys.json.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustedKeyStore {
    /// author_id → base64-encoded public key
    keys: HashMap<String, String>,
    #[serde(skip)]
    path: PathBuf,
}

impl TrustedKeyStore {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("trusted_keys.json");
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(mut store) = serde_json::from_str::<TrustedKeyStore>(&data) {
                    store.path = path;
                    return store;
                }
            }
        }
        TrustedKeyStore {
            keys: HashMap::new(),
            path,
        }
    }

    pub fn save(&self) -> Result<(), ExtensionError> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| ExtensionError::Other(format!("Failed to serialize trusted keys: {}", e)))?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    /// Get the trusted public key for an author, if any.
    pub fn get(&self, author_id: &str) -> Option<&str> {
        self.keys.get(author_id).map(|s| s.as_str())
    }

    /// Trust a new author's public key (TOFU).
    pub fn trust(&mut self, author_id: &str, public_key_b64: &str) -> Result<(), ExtensionError> {
        self.keys.insert(author_id.to_string(), public_key_b64.to_string());
        self.save()
    }

    /// Check if the author's key has changed (possible supply chain attack).
    pub fn check_key_consistency(&self, author_id: &str, public_key_b64: &str) -> KeyConsistency {
        match self.keys.get(author_id) {
            None => KeyConsistency::NewAuthor,
            Some(stored) if stored == public_key_b64 => KeyConsistency::Matches,
            Some(_) => KeyConsistency::Changed,
        }
    }
}

/// Result of checking an author's key against the trusted store.
#[derive(Debug, PartialEq, Eq)]
pub enum KeyConsistency {
    /// Author not seen before (TOFU — trust on first use)
    NewAuthor,
    /// Key matches the stored value
    Matches,
    /// Key CHANGED from the stored value (warning!)
    Changed,
}

/// Compute SHA-256 hash of a byte slice, returning hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hex::encode(hash)
}

/// Verify an Ed25519 signature over the SHA-256 hash of binary data.
///
/// - `public_key_b64`: base64-encoded 32-byte Ed25519 public key
/// - `binary_data`: the raw binary bytes
/// - `signature_b64`: base64-encoded 64-byte Ed25519 signature
/// - `expected_sha256`: hex-encoded expected SHA-256 hash (integrity check)
pub fn verify_binary(
    public_key_b64: &str,
    binary_data: &[u8],
    signature_b64: &str,
    expected_sha256: &str,
) -> Result<(), ExtensionError> {
    // 1. Integrity check: SHA-256 hash matches
    let actual_sha256 = sha256_hex(binary_data);
    if actual_sha256 != expected_sha256 {
        return Err(ExtensionError::SignatureError(format!(
            "SHA-256 mismatch: expected {}, got {}",
            expected_sha256, actual_sha256
        )));
    }

    // 2. Decode public key
    let pk_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, public_key_b64)
        .map_err(|e| ExtensionError::SignatureError(format!("Invalid public key base64: {}", e)))?;

    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| {
        ExtensionError::SignatureError("Public key must be exactly 32 bytes".into())
    })?;

    let verifying_key = VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| ExtensionError::SignatureError(format!("Invalid Ed25519 public key: {}", e)))?;

    // 3. Decode signature
    let sig_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
        .map_err(|e| ExtensionError::SignatureError(format!("Invalid signature base64: {}", e)))?;

    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| {
        ExtensionError::SignatureError("Signature must be exactly 64 bytes".into())
    })?;

    let signature = Signature::from_bytes(&sig_array);

    // 4. Verify: signature was over sha256(binary)
    let hash_bytes = Sha256::digest(binary_data);
    verifying_key
        .verify(&hash_bytes, &signature)
        .map_err(|_| ExtensionError::SignatureError("Ed25519 signature verification failed".into()))?;

    Ok(())
}

/// Compute the fingerprint of a public key (first 16 hex chars of SHA-256).
pub fn key_fingerprint(public_key_b64: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key_b64.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..8])
}

// We need the `hex` module for encoding. Since we already have sha2,
// let's provide a minimal hex encoding utility rather than adding another dep.
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}
