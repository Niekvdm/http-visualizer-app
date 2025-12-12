use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/"]
struct FrontendAssets;

pub async fn serve_static(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Try to serve the exact path first
    if let Some(content) = FrontendAssets::get(path) {
        return response_from_asset(path, &content.data);
    }

    // For non-file paths (no extension or directory), serve index.html (SPA support)
    if !path.contains('.') || path.is_empty() {
        if let Some(content) = FrontendAssets::get("index.html") {
            return response_from_asset("index.html", &content.data);
        }
    }

    // Try with .html extension
    let html_path = format!("{}.html", path);
    if let Some(content) = FrontendAssets::get(&html_path) {
        return response_from_asset(&html_path, &content.data);
    }

    // Try index.html in directory
    let index_path = format!("{}/index.html", path);
    if let Some(content) = FrontendAssets::get(&index_path) {
        return response_from_asset(&index_path, &content.data);
    }

    // Fallback to index.html for SPA routing
    if let Some(content) = FrontendAssets::get("index.html") {
        return response_from_asset("index.html", &content.data);
    }

    // 404 if nothing found
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}

fn response_from_asset(path: &str, data: &[u8]) -> Response<Body> {
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from(data.to_vec()))
        .unwrap()
}
