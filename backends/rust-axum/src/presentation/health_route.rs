use axum::{extract::State, Json};
use chrono::Utc;
use redis::AsyncCommands;

use crate::presentation::types::HealthResponse;
use crate::AppState;

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check response", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").fetch_optional(&state.db).await {
        Ok(_) => "ok".to_string(),
        Err(_) => "error".to_string(),
    };

    let redis_status = {
        let mut conn = state.redis.clone();
        match conn.ping::<()>().await {
            Ok(_) => "ok".to_string(),
            Err(_) => "error".to_string(),
        }
    };

    let status = if db_status == "ok" && redis_status == "ok" {
        "healthy"
    } else {
        "degraded"
    }
    .to_string();

    Json(HealthResponse {
        status,
        service: "url-shortener-rust".to_string(),
        database: db_status,
        redis: redis_status,
        timestamp: Utc::now(),
    })
}
