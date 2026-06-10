use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub tenant_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TotpVerifyRequest {
    pub code: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ShortenRequest {
    #[validate(url)]
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkResponse {
    pub id: uuid::Uuid,
    pub short_url: String,
    pub original_url: String,
    pub short_code: String,
    pub clicks: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkListResponse {
    pub links: Vec<LinkResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkStatsResponse {
    pub short_code: String,
    pub original_url: String,
    pub total_clicks: i64,
    pub unique_visitors: i64,
    pub clicks_by_country: std::collections::HashMap<String, i64>,
    pub clicks_by_day: Vec<ClicksByDay>,
    pub recent_clicks: Vec<RecentClick>,
    pub browsers: std::collections::HashMap<String, i64>,
    pub platforms: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClicksByDay {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecentClick {
    pub ip: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub isp: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub clicked_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub database: String,
    pub redis: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 20 }
fn default_sort() -> String { "created_at".to_string() }
fn default_order() -> String { "desc".to_string() }

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(rename = "token_type")]
    pub token_type: String,
    #[serde(rename = "totp_required")]
    pub totp_required: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub qr_code_uri: String,
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct URLCheckRequest {
    pub url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct URLCheckResponse {
    pub original_url: String,
    pub final_url: Option<String>,
    pub redirect_chain: Vec<RedirectStep>,
    pub total_redirects: i64,
    pub is_safe: bool,
    pub warnings: Vec<String>,
    pub server_ip: Option<String>,
    pub server_headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RedirectStep {
    pub url: String,
    pub status: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MyIPResponse {
    pub ip: String,
    pub country: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub isp: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PixelCreateRequest {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PixelResponse {
    pub id: uuid::Uuid,
    pub code: String,
    pub name: Option<String>,
    pub pixel_url: String,
    pub clicks: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PixelListResponse {
    pub pixels: Vec<PixelResponse>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UTMBuildRequest {
    pub url: String,
    pub utm_source: Option<String>,
    pub utm_medium: Option<String>,
    pub utm_campaign: Option<String>,
    pub utm_term: Option<String>,
    pub utm_content: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UTMResponse {
    pub original_url: String,
    pub utm_url: String,
    pub params: std::collections::HashMap<String, String>,
}