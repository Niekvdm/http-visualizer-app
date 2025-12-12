use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::RequestFailed(e) => {
                let code = if e.is_timeout() {
                    "TIMEOUT"
                } else if e.is_connect() {
                    "CONNECTION_FAILED"
                } else if e.is_request() {
                    "REQUEST_ERROR"
                } else {
                    "REQUEST_FAILED"
                };
                (StatusCode::BAD_GATEWAY, code, self.to_string())
            }
            AppError::InvalidUrl(_) => {
                (StatusCode::BAD_REQUEST, "INVALID_URL", self.to_string())
            }
            AppError::Timeout(_) => {
                (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT", self.to_string())
            }
            AppError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", self.to_string())
            }
        };

        let body = Json(json!({
            "success": false,
            "error": {
                "message": message,
                "code": error_code,
            }
        }));

        (status, body).into_response()
    }
}
