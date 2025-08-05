use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Missing field in multipart form: {0}")]
    MissingMultipartField(String),

    #[error("Failed to read image data")]
    ImageReadError(#[from] image::ImageError),

    #[error("AI model inference failed: {0}")]
    InferenceError(#[from] ort::Error),

    #[error("Failed to parse shape: {0}")]
    ShapeError(#[from] ndarray::ShapeError),

    #[error("Database query failed: {0}")]
    DatabaseError(#[from] surrealdb::Error),

    #[error("An internal error occurred: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Invalid request: {0}")]
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::MissingMultipartField(field) => (StatusCode::BAD_REQUEST, format!("Missing field: {}", field)),
            e => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}