use nexus_lib::host_api::ApiDoc;
use utoipa::OpenApi;

#[test]
fn export_openapi_spec() {
    let spec = ApiDoc::openapi().to_pretty_json().unwrap();
    let out_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("packages/nexus-sdk/openapi.json");

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    std::fs::write(&out_path, &spec).unwrap();
    println!("Wrote OpenAPI spec to {}", out_path.display());

    // Sanity check
    let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
    assert_eq!(parsed["info"]["title"], "Nexus Host API");
    assert!(parsed["paths"].as_object().unwrap().len() >= 9);
}
