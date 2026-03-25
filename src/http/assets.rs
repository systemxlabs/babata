use axum::{
    body::Body,
    extract::Path,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/web/dist"]
struct DashboardAssets;

pub(super) async fn spa_shell(headers: HeaderMap) -> Response {
    if !accepts_html(&headers) {
        return (StatusCode::NOT_ACCEPTABLE, "expected Accept: text/html").into_response();
    }

    shell_response()
}

pub(super) async fn static_asset(Path(path): Path<String>) -> Response {
    serve_embedded(&path)
}

pub(super) fn accepts_html(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .any(|part| part.starts_with("text/html"))
        })
        .unwrap_or(false)
}

pub(super) fn shell_response() -> Response {
    serve_embedded("index.html")
}

fn serve_embedded(path: &str) -> Response {
    let Some(asset) = DashboardAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = from_path(path).first_or_octet_stream();
    let content_type = HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    (
        StatusCode::OK,
        [(CONTENT_TYPE, content_type)],
        Body::from(asset.data),
    )
        .into_response()
}
