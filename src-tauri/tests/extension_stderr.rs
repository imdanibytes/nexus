//! Integration test for GitHub issue #5:
//! Extension stderr pipe dropped after init causes eprintln! panics.
//!
//! Compiles a minimal mock extension that calls `eprintln!` during `execute`,
//! then verifies the extension doesn't crash with a broken pipe panic.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use nexus_lib::extensions::manifest::{BinaryEntry, ExtensionManifest};
use nexus_lib::extensions::process::ProcessExtension;
use nexus_lib::extensions::{Extension, OperationDef, RiskLevel};
use serde_json::json;

/// Compile the fixture binary with rustc. Returns the path to the compiled binary.
fn compile_fixture() -> PathBuf {
    let fixture_src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/stderr_extension.rs");
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let out_bin = out_dir.join("stderr_extension");

    let status = Command::new("rustc")
        .arg(&fixture_src)
        .arg("-o")
        .arg(&out_bin)
        .status()
        .expect("failed to invoke rustc");

    assert!(status.success(), "rustc failed to compile fixture");
    assert!(out_bin.exists(), "compiled fixture binary not found");
    out_bin
}

/// Build a minimal ExtensionManifest for testing.
fn test_manifest() -> ExtensionManifest {
    ExtensionManifest {
        id: "test-stderr".into(),
        display_name: "Stderr Test".into(),
        version: "0.0.1".into(),
        description: "Test extension for stderr handling".into(),
        author: "test".into(),
        license: None,
        homepage: None,
        operations: vec![OperationDef {
            name: "test_op".into(),
            description: "A test operation that logs to stderr".into(),
            risk_level: RiskLevel::Low,
            input_schema: json!({"type": "object", "properties": {}}),
            scope_key: None,
            scope_description: None,
        }],
        capabilities: vec![],
        author_public_key: "dGVzdA==".into(), // dummy base64
        binaries: HashMap::from([(
            "test".into(),
            BinaryEntry {
                url: "file:///dev/null".into(),
                signature: "dGVzdA==".into(),
                sha256: "0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
            },
        )]),
        extension_dependencies: vec![],
    }
}

#[tokio::test]
async fn extension_stderr_does_not_crash() {
    let binary = compile_fixture();
    let manifest = test_manifest();
    let ext = ProcessExtension::new(manifest, binary);

    // Start the extension (sends initialize, reads response)
    ext.start().expect("extension should start successfully");

    // Execute an operation — the fixture calls eprintln! here.
    // Before the fix, this would fail because stderr was dropped after init,
    // causing the extension to panic on a broken pipe.
    let result = ext
        .execute("test_op", json!({}))
        .await
        .expect("execute should succeed even though extension uses eprintln!");

    assert!(result.success, "operation should report success");

    // Clean shutdown
    ext.stop().expect("extension should stop cleanly");
}

#[tokio::test]
async fn extension_stderr_multiple_calls() {
    let binary = compile_fixture();
    let manifest = test_manifest();
    let ext = ProcessExtension::new(manifest, binary);

    ext.start().expect("start failed");

    // Multiple calls — each one triggers eprintln! in the fixture
    for i in 0..5 {
        let result = ext
            .execute("test_op", json!({}))
            .await
            .unwrap_or_else(|e| panic!("execute #{i} failed: {e}"));
        assert!(result.success);
    }

    ext.stop().expect("stop failed");
}
