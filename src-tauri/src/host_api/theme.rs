use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

const THEME_CSS: &str = include_str!("nexus-theme.css");

const GEIST_REGULAR: &[u8] = include_bytes!("../../../public/fonts/Geist-Regular.woff2");
const GEIST_MEDIUM: &[u8] = include_bytes!("../../../public/fonts/Geist-Medium.woff2");
const GEIST_SEMIBOLD: &[u8] = include_bytes!("../../../public/fonts/Geist-SemiBold.woff2");
const GEIST_BOLD: &[u8] = include_bytes!("../../../public/fonts/Geist-Bold.woff2");
const GEIST_MONO_REGULAR: &[u8] = include_bytes!("../../../public/fonts/GeistMono-Regular.woff2");
const GEIST_MONO_MEDIUM: &[u8] = include_bytes!("../../../public/fonts/GeistMono-Medium.woff2");

pub async fn theme_css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        THEME_CSS,
    )
        .into_response()
}

pub async fn theme_font(Path(filename): Path<String>) -> Result<Response, StatusCode> {
    let bytes: &[u8] = match filename.as_str() {
        "Geist-Regular.woff2" => GEIST_REGULAR,
        "Geist-Medium.woff2" => GEIST_MEDIUM,
        "Geist-SemiBold.woff2" => GEIST_SEMIBOLD,
        "Geist-Bold.woff2" => GEIST_BOLD,
        "GeistMono-Regular.woff2" => GEIST_MONO_REGULAR,
        "GeistMono-Medium.woff2" => GEIST_MONO_MEDIUM,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok((
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}
