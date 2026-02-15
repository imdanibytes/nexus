use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum NexusError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] crate::runtime::RuntimeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Plugin already exists: {0}")]
    PluginAlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("{0}")]
    Other(String),
}

impl Serialize for NexusError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<NexusError> for axum::http::StatusCode {
    fn from(err: NexusError) -> Self {
        match err {
            NexusError::PluginNotFound(_) => axum::http::StatusCode::NOT_FOUND,
            NexusError::PermissionDenied(_) => axum::http::StatusCode::FORBIDDEN,
            NexusError::InvalidManifest(_) => axum::http::StatusCode::BAD_REQUEST,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type NexusResult<T> = Result<T, NexusError>;
