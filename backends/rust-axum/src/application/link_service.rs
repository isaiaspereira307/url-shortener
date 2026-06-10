use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use redis::aio::MultiplexedConnection;

use crate::domain::entities::{Link, ClickEvent, Pixel, PixelClickEvent};
use crate::domain::errors::AppError;

const ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const PIXEL_PREFIX: &str = "px_";

pub fn generate_short_code(length: usize) -> String {
    nanoid::nanoid!(length, &ALPHABET.chars().collect::<Vec<char>>())
}

pub fn generate_pixel_code(length: usize) -> String {
    format!("{}{}", PIXEL_PREFIX, nanoid::nanoid!(length, &ALPHABET.chars().collect::<Vec<char>>()))
}

pub async fn acquire_shorten_lock(
    redis: &mut MultiplexedConnection,
    url_hash: &str,
    timeout: usize,
) -> Result<bool, AppError> {
    let lock_key = format!("lock:shorten:{}", url_hash);
    let result: bool = redis::cmd("SET")
        .arg(&lock_key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(timeout as u64)
        .query_async(redis)
        .await
        .unwrap_or(false);
    Ok(result)
}

pub async fn release_shorten_lock(
    redis: &mut MultiplexedConnection,
    url_hash: &str,
) -> Result<(), AppError> {
    let lock_key = format!("lock:shorten:{}", url_hash);
    let _: () = redis::cmd("DEL")
        .arg(&lock_key)
        .query_async(redis)
        .await
        .unwrap_or(());
    Ok(())
}

pub async fn create_link(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    original_url: &str,
    short_code_length: usize,
) -> Result<Link, AppError> {
    for _ in 0..3 {
        let short_code = generate_short_code(short_code_length);

        let result = sqlx::query_as::<_, Link>(
            "INSERT INTO links (tenant_id, user_id, short_code, original_url) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(&short_code)
        .bind(original_url)
        .fetch_optional(pool)
        .await?;

        if let Some(link) = result {
            return Ok(link);
        }
    }

    Err(AppError::Internal)
}

pub async fn get_link_by_short_code(pool: &PgPool, short_code: &str) -> Result<Option<Link>, AppError> {
    let link = sqlx::query_as::<_, Link>(
        "SELECT * FROM links WHERE short_code = $1"
    )
    .bind(short_code)
    .fetch_optional(pool)
    .await?;

    Ok(link)
}

pub async fn get_links_by_user(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    page: i64,
    limit: i64,
    sort: &str,
    order: &str,
) -> Result<(Vec<Link>, i64), AppError> {
    let offset = (page - 1) * limit;
    let limit = limit.min(100);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM links WHERE tenant_id = $1 AND user_id = $2"
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let allowed_sort = match sort {
        "created_at" => "created_at",
        "clicks" => "clicks",
        "short_code" => "short_code",
        _ => "created_at",
    };
    let order_dir = if order == "asc" { "ASC" } else { "DESC" };

    let query = format!(
        "SELECT * FROM links WHERE tenant_id = $1 AND user_id = $2 ORDER BY {} {} LIMIT $3 OFFSET $4",
        allowed_sort, order_dir
    );

    let links = sqlx::query_as::<_, Link>(&query)
    .bind(tenant_id)
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((links, total))
}

pub async fn delete_link(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    short_code: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "DELETE FROM links WHERE short_code = $1 AND tenant_id = $2 AND user_id = $3"
    )
    .bind(short_code)
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn record_click(
    pool: &PgPool,
    short_code: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    referer: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO click_events (link_id, ip, user_agent, referer, country, city, latitude, longitude, isp)
         SELECT id, $2, $3, $4, NULL, NULL, NULL, NULL, NULL FROM links WHERE short_code = $1"
    )
    .bind(short_code)
    .bind(ip)
    .bind(user_agent)
    .bind(referer)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE links SET clicks = clicks + 1 WHERE short_code = $1"
    )
    .bind(short_code)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_link_stats(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    short_code: &str,
) -> Result<Option<crate::presentation::types::LinkStatsResponse>, AppError> {
    let link = sqlx::query_as::<_, Link>(
        "SELECT * FROM links WHERE short_code = $1 AND tenant_id = $2 AND user_id = $3"
    )
    .bind(short_code)
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let link = match link {
        Some(l) => l,
        None => return Ok(None),
    };

    let total_clicks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM click_events WHERE link_id = $1"
    )
    .bind(link.id)
    .fetch_one(pool)
    .await?;

    let unique_visitors: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT ip) FROM click_events WHERE link_id = $1"
    )
    .bind(link.id)
    .fetch_one(pool)
    .await?;

    let clicks_by_country_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT COALESCE(country, ''), COUNT(*) as count FROM click_events WHERE link_id = $1 AND country IS NOT NULL GROUP BY country ORDER BY count DESC LIMIT 20"
    )
    .bind(link.id)
    .fetch_all(pool)
    .await?;

    let mut country_map = std::collections::HashMap::new();
    for (country, count) in clicks_by_country_rows {
        if !country.is_empty() {
            country_map.insert(country, count);
        }
    }

    let recent_clicks: Vec<ClickEvent> = sqlx::query_as::<_, ClickEvent>(
        "SELECT * FROM click_events WHERE link_id = $1 ORDER BY clicked_at DESC LIMIT 50"
    )
    .bind(link.id)
    .fetch_all(pool)
    .await?;

    let mut browsers = std::collections::HashMap::new();
    let mut platforms = std::collections::HashMap::new();
    let mut recent = Vec::new();

    for click in recent_clicks {
        if let Some(ref ua) = click.user_agent {
            let b = parse_browser(ua);
            let count = browsers.entry(b).or_insert(0i64);
            *count += 1;
            let p = parse_platform(ua);
            let count = platforms.entry(p).or_insert(0i64);
            *count += 1;
        }
        recent.push(crate::presentation::types::RecentClick {
            ip: click.ip,
            country: click.country,
            city: click.city,
            latitude: click.latitude,
            longitude: click.longitude,
            isp: click.isp,
            user_agent: click.user_agent,
            referer: click.referer,
            clicked_at: Some(click.clicked_at.to_rfc3339()),
        });
    }

    Ok(Some(crate::presentation::types::LinkStatsResponse {
        short_code: link.short_code,
        original_url: link.original_url,
        total_clicks: link.clicks,
        unique_visitors,
        clicks_by_country: country_map,
        clicks_by_day: vec![],
        recent_clicks: recent,
        browsers,
        platforms,
    }))
}

pub async fn create_pixel(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    name: Option<&str>,
) -> Result<Pixel, AppError> {
    let code = generate_pixel_code(8);
    let pixel = sqlx::query_as::<_, Pixel>(
        "INSERT INTO pixels (tenant_id, user_id, code, name) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(&code)
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(pixel)
}

pub async fn get_pixels_by_user(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    page: i64,
    limit: i64,
) -> Result<(Vec<Pixel>, i64), AppError> {
    let offset = (page - 1) * limit;
    let limit = limit.min(100);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pixels WHERE tenant_id = $1 AND user_id = $2"
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let pixels = sqlx::query_as::<_, Pixel>(
        "SELECT * FROM pixels WHERE tenant_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((pixels, total))
}

pub async fn delete_pixel(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    code: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "DELETE FROM pixels WHERE code = $1 AND tenant_id = $2 AND user_id = $3"
    )
    .bind(code)
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn record_pixel_click(
    pool: &PgPool,
    code: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    referer: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO pixel_click_events (pixel_id, ip, user_agent, referer, country, city, latitude, longitude, isp)
         SELECT id, $2, $3, $4, NULL, NULL, NULL, NULL, NULL FROM pixels WHERE code = $1"
    )
    .bind(code)
    .bind(ip)
    .bind(user_agent)
    .bind(referer)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE pixels SET clicks = clicks + 1 WHERE code = $1"
    )
    .bind(code)
    .execute(pool)
    .await?;

    Ok(())
}

fn parse_browser(user_agent: &str) -> String {
    let ua = user_agent.to_lowercase();
    if ua.contains("edg") { return "Edge".to_string(); }
    if ua.contains("chrome") { return "Chrome".to_string(); }
    if ua.contains("firefox") { return "Firefox".to_string(); }
    if ua.contains("safari") { return "Safari".to_string(); }
    if ua.contains("opera") || ua.contains("opr") { return "Opera".to_string(); }
    "Other".to_string()
}

fn parse_platform(user_agent: &str) -> String {
    let ua = user_agent.to_lowercase();
    if ua.contains("windows") { return "Windows".to_string(); }
    if ua.contains("mac") { return "macOS".to_string(); }
    if ua.contains("linux") { return "Linux".to_string(); }
    if ua.contains("android") { return "Android".to_string(); }
    if ua.contains("iphone") || ua.contains("ipad") { return "iOS".to_string(); }
    "Other".to_string()
}