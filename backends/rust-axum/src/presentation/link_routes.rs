use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use md5::Md5;
use redis::aio::MultiplexedConnection;
use serde::Deserialize;
use url::Url;
use validator::Validate;
use digest::Digest;

use crate::application::link_service;
use crate::domain::errors::AppError;
use crate::infrastructure::config::Settings;
use crate::presentation::{
    middleware::AuthUser,
    types::{
        ErrorResponse, LinkListResponse, LinkResponse, ShortenRequest, PaginationParams,
        LinkStatsResponse, MyIPResponse, URLCheckRequest, URLCheckResponse, RedirectStep,
        PixelCreateRequest, PixelResponse, PixelListResponse, UTMBuildRequest, UTMResponse,
    },
};
use crate::AppState;

const PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02,
    0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[utoipa::path(
    post,
    path = "/api/shorten",
    security(("bearerAuth" = [])),
    request_body = ShortenRequest,
    responses(
        (status = 201, description = "Short URL created", body = LinkResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Links"
)]
pub async fn shorten(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    uri: Uri,
    Json(body): Json<ShortenRequest>,
) -> Result<(StatusCode, Json<LinkResponse>), AppError> {
    if let Err(e) = body.validate() {
        return Err(AppError::Validation(e.to_string()));
    }

    let parsed = Url::parse(&body.url).map_err(|_| AppError::Validation("Invalid URL".to_string()))?;
    if !["http", "https"].contains(&parsed.scheme()) {
        return Err(AppError::Validation("Invalid URL. Must start with http:// or https://".to_string()));
    }

    let url_hash = format!("{:x}", Md5::new_with_prefix(body.url.as_bytes()).finalize())[..12].to_string();

    let mut redis_conn = state.redis.clone();
    let acquired = link_service::acquire_shorten_lock(&mut redis_conn, &url_hash, 5).await?;
    if !acquired {
        return Err(AppError::TooManyRequests);
    }

    let link = link_service::create_link(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        &body.url,
        state.settings.app.short_code_length,
    )
    .await;

    let _ = link_service::release_shorten_lock(&mut redis_conn, &url_hash).await;

    let link = link?;

    let scheme = uri.scheme_str().unwrap_or("http");
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16();
    let base_url = match port {
        Some(p) => format!("{}://{}:{}", scheme, host, p),
        None => format!("{}://{}", scheme, host),
    };

    let short_url = format!("{}/{}", base_url, link.short_code);

    let mut conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(format!("url:{}", link.short_code))
        .arg(&link.original_url)
        .arg("EX")
        .arg(86400u64)
        .query_async(&mut conn)
        .await
        .unwrap_or(());

    Ok((
        StatusCode::CREATED,
        Json(LinkResponse {
            id: link.id,
            short_url,
            original_url: link.original_url,
            short_code: link.short_code,
            clicks: link.clicks,
            created_at: link.created_at,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/{short_code}",
    params(
        ("short_code" = String, Path, description = "Short URL code")
    ),
    responses(
        (status = 302, description = "Redirect to original URL"),
        (status = 404, description = "Link not found", body = ErrorResponse),
    ),
    tag = "Redirect"
)]
pub async fn redirect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    if short_code.starts_with("px_") {
        return Ok(serve_pixel_inner(&state, &short_code, &headers));
    }

    let mut conn = state.redis.clone();
    let cached: Option<String> = redis::cmd("GET")
        .arg(format!("url:{}", short_code))
        .query_async(&mut conn)
        .await
        .unwrap_or(None);

    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()));
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let referer = headers.get("referer").and_then(|v| v.to_str().ok());

    if let Some(original_url) = cached {
        let parsed = Url::parse(&original_url).map_err(|_| AppError::Internal)?;
        if !["http", "https"].contains(&parsed.scheme()) {
            return Err(AppError::Validation("Invalid redirect URL".to_string()));
        }
        let _ = link_service::record_click(&state.db, &short_code, ip, user_agent, referer).await;
        return Ok(Redirect::to(&original_url).into_response());
    }

    let link = link_service::get_link_by_short_code(&state.db, &short_code)
        .await?
        .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

    let parsed = Url::parse(&link.original_url).map_err(|_| AppError::Internal)?;
    if !["http", "https"].contains(&parsed.scheme()) {
        return Err(AppError::Validation("Invalid redirect URL".to_string()));
    }

    let mut redis_conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(format!("url:{}", short_code))
        .arg(&link.original_url)
        .arg("EX")
        .arg(86400u64)
        .query_async(&mut redis_conn)
        .await
        .unwrap_or(());

    let _ = link_service::record_click(&state.db, &short_code, ip, user_agent, referer).await;

    Ok(Redirect::to(&link.original_url).into_response())
}

#[utoipa::path(
    get,
    path = "/api/links",
    security(("bearerAuth" = [])),
    params(PaginationParams),
    responses(
        (status = 200, description = "List of links", body = LinkListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Links"
)]
pub async fn list_links(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Query(params): Query<PaginationParams>,
    uri: Uri,
) -> Result<Json<LinkListResponse>, AppError> {
    let (links, total) = link_service::get_links_by_user(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        params.page,
        params.limit,
        &params.sort,
        &params.order,
    )
    .await?;

    let scheme = uri.scheme_str().unwrap_or("http");
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16();
    let base_url = match port {
        Some(p) => format!("{}://{}:{}", scheme, host, p),
        None => format!("{}://{}", scheme, host),
    };

    let link_responses: Vec<LinkResponse> = links
        .into_iter()
        .map(|link| LinkResponse {
            id: link.id,
            short_url: format!("{}/{}", base_url, link.short_code),
            original_url: link.original_url,
            short_code: link.short_code,
            clicks: link.clicks,
            created_at: link.created_at,
        })
        .collect();

    Ok(Json(LinkListResponse {
        links: link_responses,
        total,
        page: params.page,
        limit: params.limit,
    }))
}

#[utoipa::path(
    get,
    path = "/api/links/{short_code}/stats",
    security(("bearerAuth" = [])),
    params(
        ("short_code" = String, Path, description = "Short URL code")
    ),
    responses(
        (status = 200, description = "Link statistics", body = LinkStatsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Link not found", body = ErrorResponse),
    ),
    tag = "Links"
)]
pub async fn link_stats(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Path(short_code): Path<String>,
) -> Result<Json<LinkStatsResponse>, AppError> {
    let stats = link_service::get_link_stats(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        &short_code,
    )
    .await?;

    match stats {
        Some(s) => Ok(Json(s)),
        None => Err(AppError::NotFound("Link not found".to_string())),
    }
}

#[utoipa::path(
    delete,
    path = "/api/links/{short_code}",
    security(("bearerAuth" = [])),
    params(
        ("short_code" = String, Path, description = "Short URL code")
    ),
    responses(
        (status = 200, description = "Link deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Link not found", body = ErrorResponse),
    ),
    tag = "Links"
)]
pub async fn delete_link(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Path(short_code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = link_service::delete_link(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        &short_code,
    )
    .await?;

    if !deleted {
        return Err(AppError::NotFound("Link not found".to_string()));
    }

    let mut conn = state.redis.clone();
    let _: () = redis::cmd("DEL")
        .arg(format!("url:{}", short_code))
        .query_async(&mut conn)
        .await
        .unwrap_or(());

    Ok(Json(serde_json::json!({ "message": "Link deleted successfully" })))
}

#[utoipa::path(
    get,
    path = "/api/myip",
    responses(
        (status = 200, description = "Client IP information", body = MyIPResponse),
    ),
    tag = "Tools"
)]
pub async fn my_ip(headers: HeaderMap) -> Json<MyIPResponse> {
    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();

    Json(MyIPResponse {
        ip,
        country: None,
        city: None,
        latitude: None,
        longitude: None,
        isp: None,
    })
}

#[utoipa::path(
    post,
    path = "/api/check-url",
    request_body = URLCheckRequest,
    responses(
        (status = 200, description = "URL check result", body = URLCheckResponse),
        (status = 400, description = "Invalid URL", body = ErrorResponse),
    ),
    tag = "Tools"
)]
pub async fn check_url(
    Json(body): Json<URLCheckRequest>,
) -> Result<Json<URLCheckResponse>, AppError> {
    let parsed = Url::parse(&body.url).map_err(|_| AppError::Validation("Invalid URL".to_string()))?;
    if !["http", "https"].contains(&parsed.scheme()) {
        return Err(AppError::Validation("Invalid URL. Must start with http:// or https://".to_string()));
    }

    if let Some(host) = parsed.host_str() {
        if is_internal_host(host) {
            return Err(AppError::Validation("URL resolves to an internal address".to_string()));
        }
    }

    Ok(Json(URLCheckResponse {
        original_url: body.url,
        final_url: Some(parsed.to_string()),
        redirect_chain: vec![RedirectStep { url: parsed.to_string(), status: Some(200) }],
        total_redirects: 0,
        is_safe: true,
        warnings: vec![],
        server_ip: None,
        server_headers: None,
    }))
}

fn is_internal_host(host: &str) -> bool {
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    if host.starts_with("10.") || host.starts_with("192.168.") {
        return true;
    }
    if host.starts_with("172.") {
        if let Some(second) = host.split('.').nth(1).and_then(|s| s.parse::<u8>().ok()) {
            if (16..=31).contains(&second) {
                return true;
            }
        }
    }
    if host.starts_with("169.254.") {
        return true;
    }
    if host == "0.0.0.0" {
        return true;
    }
    false
}

#[utoipa::path(
    post,
    path = "/api/pixel",
    security(("bearerAuth" = [])),
    request_body = PixelCreateRequest,
    responses(
        (status = 201, description = "Tracking pixel created", body = PixelResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Pixels"
)]
pub async fn create_pixel(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    uri: Uri,
    Json(body): Json<PixelCreateRequest>,
) -> Result<(StatusCode, Json<PixelResponse>), AppError> {
    let pixel = link_service::create_pixel(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        body.name.as_deref(),
    )
    .await?;

    let scheme = uri.scheme_str().unwrap_or("http");
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16();
    let base_url = match port {
        Some(p) => format!("{}://{}:{}", scheme, host, p),
        None => format!("{}://{}", scheme, host),
    };

    Ok((
        StatusCode::CREATED,
        Json(PixelResponse {
            id: pixel.id,
            code: pixel.code.clone(),
            name: pixel.name.clone(),
            pixel_url: format!("{}{}.png", base_url, pixel.code),
            clicks: pixel.clicks,
            created_at: pixel.created_at,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/pixels",
    security(("bearerAuth" = [])),
    params(PaginationParams),
    responses(
        (status = 200, description = "List of tracking pixels", body = PixelListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Pixels"
)]
pub async fn list_pixels(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    uri: Uri,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PixelListResponse>, AppError> {
    let (pixels, total) = link_service::get_pixels_by_user(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        params.page,
        params.limit,
    )
    .await?;

    let scheme = uri.scheme_str().unwrap_or("http");
    let host = uri.host().unwrap_or("localhost");
    let port = uri.port_u16();
    let base_url = match port {
        Some(p) => format!("{}://{}:{}", scheme, host, p),
        None => format!("{}://{}", scheme, host),
    };

    let pixel_responses: Vec<PixelResponse> = pixels
        .into_iter()
        .map(|p| PixelResponse {
            id: p.id,
            code: p.code.clone(),
            name: p.name.clone(),
            pixel_url: format!("{}{}.png", base_url, p.code),
            clicks: p.clicks,
            created_at: p.created_at,
        })
        .collect();

    Ok(Json(PixelListResponse {
        pixels: pixel_responses,
        total,
        page: params.page,
        limit: params.limit,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/pixel/{code}",
    security(("bearerAuth" = [])),
    params(
        ("code" = String, Path, description = "Pixel code")
    ),
    responses(
        (status = 200, description = "Pixel deleted successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Pixel not found", body = ErrorResponse),
    ),
    tag = "Pixels"
)]
pub async fn delete_pixel(
    State(state): State<AppState>,
    auth_user: axum::extract::Extension<AuthUser>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = link_service::delete_pixel(
        &state.db,
        auth_user.tenant_id,
        auth_user.user_id,
        &code,
    )
    .await?;

    if !deleted {
        return Err(AppError::NotFound("Pixel not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "message": "Pixel deleted successfully" })))
}

fn serve_pixel_inner(state: &AppState, code: &str, headers: &HeaderMap) -> Response {
    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim())
        .and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let referer = headers.get("referer").and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    let pixel_code = if code.ends_with(".png") { &code[..code.len()-4] } else { code };
    let pixel_code_owned = pixel_code.to_string();

    let db = state.db.clone();
    tokio::spawn(async move {
        let _ = link_service::record_pixel_click(
            &db,
            &pixel_code_owned,
            ip.as_deref(),
            user_agent.as_deref(),
            referer.as_deref(),
        ).await;
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "image/png")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .body(Body::from(PIXEL_PNG.to_vec()))
        .unwrap()
}

#[utoipa::path(
    get,
    path = "/pixel/{code}",
    params(
        ("code" = String, Path, description = "Pixel code")
    ),
    responses(
        (status = 200, description = "Pixel image (1x1 transparent PNG)", content_type = "image/png"),
    ),
    tag = "Pixels"
)]
pub async fn serve_pixel(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
) -> Response {
    serve_pixel_inner(&state, &code, &headers)
}

#[utoipa::path(
    post,
    path = "/api/utm-builder",
    request_body = UTMBuildRequest,
    responses(
        (status = 200, description = "UTM URL generated", body = UTMResponse),
        (status = 400, description = "Invalid URL", body = ErrorResponse),
    ),
    tag = "Tools"
)]
pub async fn build_utm(
    Json(body): Json<UTMBuildRequest>,
) -> Result<Json<UTMResponse>, AppError> {
    let mut parsed = Url::parse(&body.url).map_err(|_| AppError::Validation("Invalid URL".to_string()))?;
    if !["http", "https"].contains(&parsed.scheme()) {
        return Err(AppError::Validation("Invalid URL. Must start with http:// or https://".to_string()));
    }

    let mut params = std::collections::HashMap::new();
    if let Some(ref v) = body.utm_source { params.insert("utm_source".to_string(), v.clone()); }
    if let Some(ref v) = body.utm_medium { params.insert("utm_medium".to_string(), v.clone()); }
    if let Some(ref v) = body.utm_campaign { params.insert("utm_campaign".to_string(), v.clone()); }
    if let Some(ref v) = body.utm_term { params.insert("utm_term".to_string(), v.clone()); }
    if let Some(ref v) = body.utm_content { params.insert("utm_content".to_string(), v.clone()); }

    let mut query_pairs: Vec<(String, String)> = parsed.query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    for (k, v) in &params {
        query_pairs.push((k.clone(), v.clone()));
    }

    let mut new_query = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in &query_pairs {
        new_query.append_pair(k, v);
    }
    parsed.set_query(Some(&new_query.finish()));

    Ok(Json(UTMResponse {
        original_url: body.url,
        utm_url: parsed.to_string(),
        params,
    }))
}