use axum::{extract::Query, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct PathQuery {
    pub path: String,
}

#[derive(Serialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
}

#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

#[derive(Deserialize)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

pub async fn read_file(
    Query(query): Query<PathQuery>,
) -> Result<Json<FileContent>, StatusCode> {
    let path = PathBuf::from(&query.path);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    if !path.is_file() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let metadata = std::fs::metadata(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let content =
        std::fs::read_to_string(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(FileContent {
        path: query.path,
        content,
        size: metadata.len(),
    }))
}

pub async fn list_dir(
    Query(query): Query<PathQuery>,
) -> Result<Json<DirListing>, StatusCode> {
    let path = PathBuf::from(&query.path);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    if !path.is_dir() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for entry in read_dir.flatten() {
        let metadata = entry.metadata().unwrap_or_else(|_| {
            std::fs::metadata(entry.path()).expect("failed to read metadata")
        });

        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    Ok(Json(DirListing {
        path: query.path,
        entries,
    }))
}

pub async fn write_file(
    Json(req): Json<WriteRequest>,
) -> Result<StatusCode, StatusCode> {
    let path = PathBuf::from(&req.path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    std::fs::write(&path, &req.content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}
