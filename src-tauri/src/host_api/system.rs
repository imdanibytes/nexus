use axum::Json;
use serde::Serialize;
use sysinfo::System;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub hostname: String,
    pub uptime: u64,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub nexus_version: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/system/info",
    tag = "system",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Host system information", body = SystemInfo),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn system_info() -> Json<SystemInfo> {
    let sys = System::new_all();

    Json(SystemInfo {
        os: System::name().unwrap_or_else(|| "unknown".into()),
        os_version: System::os_version().unwrap_or_else(|| "unknown".into()),
        hostname: System::host_name().unwrap_or_else(|| "unknown".into()),
        uptime: System::uptime(),
        cpu_count: sys.cpus().len(),
        total_memory: sys.total_memory(),
        nexus_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
