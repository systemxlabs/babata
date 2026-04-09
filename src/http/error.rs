use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::BabataError;

#[derive(Debug)]
pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl ApiError {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl From<BabataError> for ApiError {
    fn from(err: BabataError) -> Self {
        match err {
            BabataError::InvalidInput(message, _) => Self {
                status: StatusCode::BAD_REQUEST,
                message,
            },
            BabataError::NotFound(message, _) => Self {
                status: StatusCode::NOT_FOUND,
                message,
            },
            BabataError::Config(message, _)
            | BabataError::Tool(message, _)
            | BabataError::Channel(message, _)
            | BabataError::Internal(message, _)
            | BabataError::Provider(message, _)
            | BabataError::Memory(message, _) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[cfg(test)]
mod tests {
    use super::ApiError;
    use crate::error::BabataError;
    use axum::http::StatusCode;

    #[test]
    fn invalid_input_maps_to_400() {
        let error = ApiError::from(BabataError::invalid_input("bad input"));
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn not_found_maps_to_404() {
        let error = ApiError::from(BabataError::not_found("missing"));
        assert_eq!(error.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn other_errors_map_to_500() {
        let error = ApiError::from(BabataError::tool("tool failure"));
        assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
