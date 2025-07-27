pub mod auth;
pub mod plugins;
pub mod search;
pub mod health;
pub mod admin;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Internal(String),
    ValidationError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string())
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        };

        let body = Json(json!({
            "success": false,
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = err
            .field_errors()
            .into_iter()
            .map(|(field, errors)| {
                let field_errors: Vec<String> = errors
                    .into_iter()
                    .map(|error| {
                        error.message.as_ref()
                            .map(|msg| msg.to_string())
                            .unwrap_or_else(|| format!("Invalid {}", field))
                    })
                    .collect();
                field_errors.join(", ")
            })
            .collect();
        
        AppError::ValidationError(messages.join("; "))
    }
}

pub fn success_response<T: serde::Serialize>(data: T) -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "data": data
    }))
}

pub fn success_response_with_message<T: serde::Serialize>(
    data: T,
    message: &str,
) -> Json<serde_json::Value> {
    Json(json!({
        "success": true,
        "message": message,
        "data": data
    }))
}