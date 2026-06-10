use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::infrastructure::config::JwtSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant_id: String,
    pub email: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_access_token(settings: &JwtSettings, user_id: Uuid, tenant_id: Uuid, email: &str) -> String {
    let now = Utc::now();
    let exp = now + Duration::minutes(settings.access_expire_minutes as i64);

    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        email: email.to_string(),
        token_type: "access".to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secret.as_bytes()),
    )
    .expect("Failed to encode access token")
}

pub fn create_refresh_token(settings: &JwtSettings, user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let exp = now + Duration::days(settings.refresh_expire_days as i64);

    let claims = Claims {
        sub: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        email: String::new(),
        token_type: "refresh".to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secret.as_bytes()),
    )
    .expect("Failed to encode refresh token")
}

pub fn verify_token(token: &str, settings: &JwtSettings) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(settings.secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}
