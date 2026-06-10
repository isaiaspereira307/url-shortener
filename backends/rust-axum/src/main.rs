use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use redis::aio::MultiplexedConnection;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::infrastructure::config::Settings;
use crate::presentation::{api_doc::ApiDoc, auth_routes, health_route, link_routes, middleware as auth_mw};

mod application;
mod domain;
mod infrastructure;
mod presentation;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: MultiplexedConnection,
    pub settings: Settings,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "url_shortener_rust=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::default();

    let db = infrastructure::database::create_pool(&settings.database.url).await;
    let redis = infrastructure::cache::create_client(&settings.redis.url).await;

    let state = AppState {
        db,
        redis,
        settings: settings.clone(),
    };

    let protected_routes = Router::new()
        .route("/api/shorten", post(link_routes::shorten))
        .route("/api/links", get(link_routes::list_links))
        .route("/api/links/{short_code}", delete(link_routes::delete_link))
        .route("/api/links/{short_code}/stats", get(link_routes::link_stats))
        .route("/api/auth/2fa/setup", post(auth_routes::setup_2fa))
        .route("/api/auth/2fa/verify", post(auth_routes::verify_2fa))
        .route("/api/auth/2fa/disable", post(auth_routes::disable_2fa))
        .route("/api/pixel", post(link_routes::create_pixel))
        .route("/api/pixels", get(link_routes::list_pixels))
        .route("/api/pixel/{code}", delete(link_routes::delete_pixel))
        .layer(middleware::from_fn_with_state(
            settings.clone(),
            auth_mw::auth_middleware,
        ));

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/api/auth/register", post(auth_routes::register))
        .route("/api/auth/login", post(auth_routes::login))
        .route("/api/auth/login/2fa", post(auth_routes::login_2fa))
        .route("/api/auth/refresh", post(auth_routes::refresh))
        .route("/{short_code}", get(link_routes::redirect))
        .route("/health", get(health_route::health))
        .route("/api/myip", get(link_routes::my_ip))
        .route("/api/check-url", post(link_routes::check_url))
        .route("/pixel/{code}", get(link_routes::serve_pixel))
        .route("/api/utm-builder", post(link_routes::build_utm))
        .merge(protected_routes)
        .layer(CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION])
            .max_age(std::time::Duration::from_secs(3600)))
        .with_state(state);

    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}