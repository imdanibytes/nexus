//! Minimal signing tool for Nexus host extensions.
//!
//! Usage: sign-tool <binary-path>
//!
//! Generates an Ed25519 keypair (or loads from keypair.json),
//! signs the binary, and writes manifest.json next to the binary.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: sign-tool <binary-path>");
        std::process::exit(1);
    }

    let binary_path = PathBuf::from(&args[1]);
    let binary_data = std::fs::read(&binary_path).expect("Failed to read binary");

    // Generate or load keypair
    let keypair_path = PathBuf::from("keypair.json");
    let signing_key = if keypair_path.exists() {
        let data = std::fs::read_to_string(&keypair_path).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&data).unwrap();
        let secret_b64 = stored["secret_key"].as_str().unwrap();
        let secret_bytes = base64::engine::general_purpose::STANDARD
            .decode(secret_b64)
            .unwrap();
        let key_bytes: [u8; 32] = secret_bytes.try_into().unwrap();
        SigningKey::from_bytes(&key_bytes)
    } else {
        let mut rng = rand::rngs::OsRng;
        let key = SigningKey::generate(&mut rng);

        let secret_b64 = base64::engine::general_purpose::STANDARD.encode(key.to_bytes());
        let public_b64 = base64::engine::general_purpose::STANDARD
            .encode(key.verifying_key().to_bytes());

        let stored = serde_json::json!({
            "secret_key": secret_b64,
            "public_key": public_b64,
        });
        std::fs::write(&keypair_path, serde_json::to_string_pretty(&stored).unwrap()).unwrap();
        eprintln!("Generated new keypair → keypair.json");
        key
    };

    let public_key_b64 = base64::engine::general_purpose::STANDARD
        .encode(signing_key.verifying_key().to_bytes());

    // SHA-256 of binary
    let sha256_hash = hex_encode(&Sha256::digest(&binary_data));

    // Sign sha256(binary) with Ed25519
    let hash_bytes = Sha256::digest(&binary_data);
    let signature = signing_key.sign(&hash_bytes);
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    // Determine platform
    let platform = if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else {
        "unknown"
    };

    // Build manifest
    let manifest = serde_json::json!({
        "id": "system-info",
        "display_name": "System Info",
        "version": "0.1.0",
        "description": "Test extension that provides system information, environment variable reading, and a fake high-risk operation",
        "author": "nexus-test",
        "license": "MIT",
        "operations": [
            {
                "name": "get_info",
                "description": "Get host system information (OS, CPU, memory, uptime)",
                "risk_level": "low",
                "input_schema": {
                    "type": "object",
                    "properties": {},
                }
            },
            {
                "name": "read_env",
                "description": "Read an environment variable from the host",
                "risk_level": "low",
                "scope_key": "var_name",
                "scope_description": "Environment variable names this operation can read",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "var_name": {
                            "type": "string",
                            "description": "Name of the environment variable to read"
                        }
                    },
                    "required": ["var_name"]
                }
            },
            {
                "name": "shutdown_host",
                "description": "Pretend to shut down the host (test high-risk approval)",
                "risk_level": "high",
                "input_schema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ],
        "capabilities": [
            { "type": "system_info" },
            { "type": "custom", "name": "env_read", "description": "Reads environment variables" }
        ],
        "author_public_key": public_key_b64,
        "binaries": {
            platform: {
                "url": format!("file://{}", binary_path.canonicalize().unwrap().display()),
                "signature": signature_b64,
                "sha256": sha256_hash,
            }
        }
    });

    let manifest_path = binary_path.parent().unwrap().join("manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    eprintln!("Signed binary: {}", binary_path.display());
    eprintln!("SHA-256:       {}", sha256_hash);
    eprintln!("Public key:    {}", public_key_b64);
    eprintln!("Platform:      {}", platform);
    eprintln!("Manifest:      {}", manifest_path.display());
}
