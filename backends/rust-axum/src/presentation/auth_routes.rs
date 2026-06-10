use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use validator::Validate;

use crate::application::{auth_service, totp_service};
use crate::domain::errors::AppError;
use crate::infrastructure::{auth, config::Settings, password};
use crate::presentation::{
    middleware::AuthUser,
    types::{
        ErrorResponse, LoginRequest, RefreshRequest, RegisterRequest, TokenResponse,
        TotpSetupResponse, TotpVerifyRequest,
    },
};
use crate::AppState;

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = TokenResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if !auth_service::validate_email(&body.email) {
        return Err(AppError::Validation("Invalid email format".to_string()));
    }

    auth_service::validate_password(&body.password)
        .map_err(AppError::Validation)?;

    let tenant = auth_service::create_tenant(&state.db, &body.tenant_name).await?;
    let user = auth_service::create_user(&state.db, tenant.id, &body.email, &body.password).await?;

    let access_token = auth::create_access_token(&state.settings.jwt, user.id, tenant.id, &user.email);
    let refresh_token = auth::create_refresh_token(&state.settings.jwt, user.id, tenant.id);

    Ok((
        StatusCode::CREATED,
        Json(TokenResponse {
            access_token,
            refresh_token,
            token_type: "bearer".to_string(),
            totp_required: false,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let user = auth_service::get_user_by_email(&state.db, &body.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    if !password::verify_password(&body.password, &user.password_hash) {
        return Err(AppError::Unauthorized("Invalid email or password".to_string()));
    }

    if user.totp_enabled {
        return Ok(Json(TokenResponse {
            access_token: String::new(),
            refresh_token: String::new(),
            token_type: "bearer".to_string(),
            totp_required: true,
        }));
    }

    let access_token = auth::create_access_token(&state.settings.jwt, user.id, user.tenant_id, &user.email);
    let refresh_token = auth::create_refresh_token(&state.settings.jwt, user.id, user.tenant_id);

    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "bearer".to_string(),
        totp_required: false,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login/2fa",
    request_body = TotpVerifyRequest,
    responses(
        (status = 200, description = "2FA verification successful", body = TokenResponse),
        (status = 401, description = "Invalid 2FA code", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn login_2fa(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Json(body): Json<TotpVerifyRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let user = auth_service::get_user_by_id(&state.db, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    let secret = user.totp_secret.ok_or_else(|| {
        AppError::Validation("2FA not enabled".to_string())
    })?;

    if !totp_service::verify_totp(&secret, &body.code) {
        return Err(AppError::Unauthorized("Invalid 2FA code".to_string()));
    }

    let access_token = auth::create_access_token(&state.settings.jwt, user.id, user.tenant_id, &user.email);
    let refresh_token = auth::create_refresh_token(&state.settings.jwt, user.id, user.tenant_id);

    Ok(Json(TokenResponse {
        access_token,
        refresh_token,
        token_type: "bearer".to_string(),
        totp_required: false,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = TokenResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let claims = auth::verify_token(&body.refresh_token, &state.settings.jwt)
        .map_err(|_| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    if claims.token_type != "refresh" {
        return Err(AppError::Unauthorized("Invalid token type".to_string()));
    }

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID".to_string()))?;

    let user = auth_service::get_user_by_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    let access_token = auth::create_access_token(&state.settings.jwt, user.id, user.tenant_id, &user.email);
    let new_refresh_token = auth::create_refresh_token(&state.settings.jwt, user.id, user.tenant_id);

    Ok(Json(TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        token_type: "bearer".to_string(),
        totp_required: false,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/2fa/setup",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "2FA setup data", body = TotpSetupResponse),
        (status = 400, description = "2FA already enabled", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Auth"
)]
pub async fn setup_2fa(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
) -> Result<Json<TotpSetupResponse>, AppError> {
    let user = auth_service::get_user_by_id(&state.db, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::Validation("User not found".to_string()))?;

    if user.totp_enabled {
        return Err(AppError::Validation("2FA already enabled".to_string()));
    }

    let result = totp_service::setup_totp(&user.email);

    sqlx::query("UPDATE users SET totp_secret = $1 WHERE id = $2")
        .bind(&result.secret)
        .bind(auth_user.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(TotpSetupResponse {
        secret: result.secret,
        qr_code_uri: result.qr_code_uri,
        backup_codes: result.backup_codes,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/2fa/verify",
    security(("bearerAuth" = [])),
    request_body = TotpVerifyRequest,
    responses(
        (status = 200, description = "2FA enabled successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Auth"
)]
pub async fn verify_2fa(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Json(body): Json<TotpVerifyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = auth_service::get_user_by_id(&state.db, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::Validation("User not found".to_string()))?;

    let secret = user.totp_secret.ok_or_else(|| {
        AppError::Validation("2FA not set up".to_string())
    })?;

    if !totp_service::verify_totp(&secret, &body.code) {
        return Err(AppError::Unauthorized("Invalid 2FA code".to_string()));
    }

    sqlx::query("UPDATE users SET totp_enabled = true WHERE id = $1")
        .bind(auth_user.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "message": "2FA enabled successfully" })))
}

#[utoipa::path(
    post,
    path = "/api/auth/2fa/disable",
    security(("bearerAuth" = [])),
    request_body = TotpVerifyRequest,
    responses(
        (status = 200, description = "2FA disabled successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Auth"
)]
pub async fn disable_2fa(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Json(body): Json<TotpVerifyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = auth_service::get_user_by_id(&state.db, auth_user.user_id)
        .await?
        .ok_or_else(|| AppError::Validation("User not found".to_string()))?;

    if !user.totp_enabled {
        return Err(AppError::Validation("2FA not enabled".to_string()));
    }

    let secret = user.totp_secret.ok_or_else(|| {
        AppError::Validation("2FA secret not found".to_string())
    })?;

    if !totp_service::verify_totp(&secret, &body.code) {
        return Err(AppError::Unauthorized("Invalid 2FA code".to_string()));
    }

    sqlx::query("UPDATE users SET totp_enabled = false, totp_secret = NULL WHERE id = $1")
        .bind(auth_user.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "message": "2FA disabled successfully" })))
}
