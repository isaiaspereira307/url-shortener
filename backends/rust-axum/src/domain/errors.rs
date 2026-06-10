use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Too many requests")]
    TooManyRequests,

    #[error("Internal error")]
    Internal,
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::Validation(msg) => (axum::http::StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized(msg) => (axum::http::StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Conflict(msg) => (axum::http::StatusCode::CONFLICT, msg.clone()),
            AppError::TooManyRequests => (axum::http::StatusCode::TOO_MANY_REQUESTS, "Too many concurrent requests. Please try again.".to_string()),
            AppError::Database(_) | AppError::Redis(_) | AppError::Jwt(_) => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string())
            }
            AppError::Internal => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string()),
        };

        let body = serde_json::json!({
            "error": message,
        });

        (status, axum::Json(body)).into_response()
    }
}
