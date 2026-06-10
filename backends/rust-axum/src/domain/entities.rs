use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Link {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub short_code: String,
    pub original_url: String,
    pub clicks: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClickEvent {
    pub id: Uuid,
    pub link_id: Uuid,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub isp: Option<String>,
    pub clicked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Pixel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub code: String,
    pub name: Option<String>,
    pub clicks: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PixelClickEvent {
    pub id: Uuid,
    pub pixel_id: Uuid,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub isp: Option<String>,
    pub clicked_at: DateTime<Utc>,
}