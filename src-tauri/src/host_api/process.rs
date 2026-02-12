use axum::Json;
use serde::Serialize;
use sysinfo::System;

#[derive(Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

pub async fn list_processes() -> Json<Vec<ProcessInfo>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, proc_)| ProcessInfo {
            pid: pid.as_u32(),
            name: proc_.name().to_string_lossy().to_string(),
            cpu_usage: proc_.cpu_usage(),
            memory: proc_.memory(),
        })
        .collect();

    Json(processes)
}
