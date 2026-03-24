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

    pub(crate) fn from_babata_error(err: BabataError) -> Self {
        match err {
            BabataError::Config(message, _)
            | BabataError::Tool(message, _)
            | BabataError::Channel(message, _) => Self {
                status: StatusCode::BAD_REQUEST,
                message,
            },
            BabataError::Internal(message, _) if message.contains("not found") => Self {
                status: StatusCode::NOT_FOUND,
                message,
            },
            BabataError::Internal(message, _)
            | BabataError::Provider(message, _)
            | BabataError::Memory(message, _) => Self {
                status: StatusCode::BAD_REQUEST,
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
