use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub jwt: JwtSettings,
    pub server: ServerSettings,
    pub app: AppSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisSettings {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtSettings {
    pub secret: String,
    pub algorithm: String,
    pub access_expire_minutes: i64,
    pub refresh_expire_days: i64,
}

impl Default for JwtSettings {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            algorithm: std::env::var("JWT_ALGORITHM").unwrap_or_else(|_| "HS256".to_string()),
            access_expire_minutes: std::env::var("JWT_ACCESS_EXPIRE_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            refresh_expire_days: std::env::var("JWT_REFRESH_EXPIRE_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub short_code_length: usize,
    pub env: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let builder = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(Environment::with_prefix("APP").separator("__"));

        let cfg = builder.build()?;
        cfg.try_deserialize()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            database: DatabaseSettings {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://url_shortener:url_shortener_secret@localhost:5432/url_shortener".to_string()),
            },
            redis: RedisSettings {
                url: std::env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379/0".to_string()),
            },
            jwt: JwtSettings {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "super-secret-key-change-this-in-production-32chars".to_string()),
                algorithm: std::env::var("JWT_ALGORITHM")
                    .unwrap_or_else(|_| "HS256".to_string()),
                access_expire_minutes: std::env::var("JWT_ACCESS_EXPIRE_MINUTES")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(15),
                refresh_expire_days: std::env::var("JWT_REFRESH_EXPIRE_DAYS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(7),
            },
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8001,
            },
            app: AppSettings {
                short_code_length: std::env::var("SHORT_CODE_LENGTH")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(7),
                env: std::env::var("APP_ENV")
                    .unwrap_or_else(|_| "development".to_string()),
            },
        }
    }
}
