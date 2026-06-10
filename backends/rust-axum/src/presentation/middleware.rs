use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::domain::errors::AppError;
use crate::infrastructure::{auth, config::Settings};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
}

pub async fn auth_middleware(
    settings: axum::extract::State<Settings>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    if !auth_str.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Invalid authorization header".to_string()));
    }

    let token = &auth_str[7..];
    let claims = auth::verify_token(token, &settings.jwt)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    if claims.token_type != "access" {
        return Err(AppError::Unauthorized("Invalid token type".to_string()));
    }

    let user = AuthUser {
        user_id: Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID".to_string()))?,
        tenant_id: Uuid::parse_str(&claims.tenant_id)
            .map_err(|_| AppError::Unauthorized("Invalid tenant ID".to_string()))?,
        email: claims.email,
    };

    let mut request = request;
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
