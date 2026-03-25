use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use super::DEFAULT_HTTP_ADDR;

pub(super) async fn handle() -> Response {
    Json(SystemResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        http_addr: DEFAULT_HTTP_ADDR.to_string(),
    })
    .into_response()
}

#[derive(Debug, Serialize)]
struct SystemResponse {
    version: String,
    http_addr: String,
}

