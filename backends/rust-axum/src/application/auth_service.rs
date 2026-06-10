use regex::Regex;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entities::{Tenant, User};
use crate::domain::errors::AppError;
use crate::infrastructure::password;

pub fn validate_email(email: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    re.is_match(email)
}

pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    Ok(())
}

pub fn generate_slug(name: &str) -> String {
    let slug = name.to_lowercase().trim().to_string();
    let slug = slug
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let slug = slug.trim_matches('-').to_string();
    slug
}

pub async fn create_tenant(pool: &PgPool, name: &str) -> Result<Tenant, AppError> {
    let slug = generate_slug(name);
    let mut final_slug = slug.clone();
    let mut counter = 1;

    loop {
        let exists: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM tenants WHERE slug = $1")
                .bind(&final_slug)
                .fetch_optional(pool)
                .await?;

        if exists.is_none() {
            break;
        }
        final_slug = format!("{}-{}", slug, counter);
        counter += 1;
    }

    let tenant = sqlx::query_as::<_, Tenant>(
        "INSERT INTO tenants (name, slug) VALUES ($1, $2) RETURNING *"
    )
    .bind(name)
    .bind(&final_slug)
    .fetch_one(pool)
    .await?;

    Ok(tenant)
}

pub async fn create_user(
    pool: &PgPool,
    tenant_id: Uuid,
    email: &str,
    password: &str,
) -> Result<User, AppError> {
    let password_hash = password::hash_password(password);

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (tenant_id, email, password_hash) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(tenant_id)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1"
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}
