use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Serialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub async fn proxy_request(
    Json(req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, StatusCode> {
    let client = reqwest::Client::new();

    let method = req
        .method
        .parse::<reqwest::Method>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut builder = client.request(method, &req.url);

    for (key, value) in &req.headers {
        builder = builder.header(key, value);
    }

    if let Some(body) = req.body {
        builder = builder.body(body);
    }

    let response = builder
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response
        .text()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProxyResponse {
        status,
        headers,
        body,
    }))
}
