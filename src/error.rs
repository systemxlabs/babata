use std::panic::Location;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub enum BabataError {
    InvalidInput(String, &'static Location<'static>),
    NotFound(String, &'static Location<'static>),
    Config(String, &'static Location<'static>),
    Provider(String, &'static Location<'static>),
    Memory(String, &'static Location<'static>),
    Tool(String, &'static Location<'static>),
    Channel(String, &'static Location<'static>),
    Internal(String, &'static Location<'static>),
}

impl BabataError {
    #[track_caller]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        BabataError::InvalidInput(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn not_found(message: impl Into<String>) -> Self {
        BabataError::NotFound(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn config(message: impl Into<String>) -> Self {
        BabataError::Config(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn provider(message: impl Into<String>) -> Self {
        BabataError::Provider(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn memory(message: impl Into<String>) -> Self {
        BabataError::Memory(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn tool(message: impl Into<String>) -> Self {
        BabataError::Tool(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn channel(message: impl Into<String>) -> Self {
        BabataError::Channel(message.into(), Location::caller())
    }

    #[track_caller]
    pub fn internal(message: impl Into<String>) -> Self {
        BabataError::Internal(message.into(), Location::caller())
    }
}

impl std::fmt::Display for BabataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (category, msg, loc) = match self {
            BabataError::InvalidInput(msg, loc) => ("Invalid input", msg, loc),
            BabataError::NotFound(msg, loc) => ("Not found", msg, loc),
            BabataError::Config(msg, loc) => ("Config", msg, loc),
            BabataError::Provider(msg, loc) => ("Provider", msg, loc),
            BabataError::Memory(msg, loc) => ("Memory", msg, loc),
            BabataError::Tool(msg, loc) => ("Tool", msg, loc),
            BabataError::Channel(msg, loc) => ("Channel", msg, loc),
            BabataError::Internal(msg, loc) => ("Internal", msg, loc),
        };
        write!(f, "{} error at {}: {}", category, loc, msg)
    }
}

impl std::error::Error for BabataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl IntoResponse for BabataError {
    fn into_response(self) -> Response {
        let status = match self {
            BabataError::InvalidInput(_, _) => StatusCode::BAD_REQUEST,
            BabataError::NotFound(_, _) => StatusCode::NOT_FOUND,
            BabataError::Config(_, _)
            | BabataError::Provider(_, _)
            | BabataError::Memory(_, _)
            | BabataError::Tool(_, _)
            | BabataError::Channel(_, _)
            | BabataError::Internal(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}

impl From<serde_json::Error> for BabataError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        BabataError::Provider(err.to_string(), Location::caller())
    }
}

impl From<std::io::Error> for BabataError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        BabataError::Internal(err.to_string(), Location::caller())
    }
}

#[cfg(test)]
mod tests {
    use super::BabataError;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

    #[test]
    fn invalid_input_maps_to_400() {
        let response = BabataError::invalid_input("bad input").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn not_found_maps_to_404() {
        let response = BabataError::not_found("missing").into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn other_errors_map_to_500() {
        let response = BabataError::tool("tool failure").into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn error_response_body_is_plain_text_with_location() {
        let response = BabataError::invalid_input("bad input").into_response();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        let text = String::from_utf8(body.to_vec()).expect("utf8 body");

        assert!(text.contains("Invalid input error at"));
        assert!(text.contains("bad input"));
    }
}
