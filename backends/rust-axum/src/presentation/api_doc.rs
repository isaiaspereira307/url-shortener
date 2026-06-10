use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

use crate::presentation::{auth_routes, health_route, link_routes, types};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "URL Shortener API",
        description = "URL Shortener API with authentication, 2FA, link management, tracking pixels and UTM builder",
        version = "1.0.0",
    ),
    servers(
        (url = "http://localhost:8001", description = "Local development")
    ),
    paths(
        health_route::health,
        auth_routes::register,
        auth_routes::login,
        auth_routes::login_2fa,
        auth_routes::refresh,
        auth_routes::setup_2fa,
        auth_routes::verify_2fa,
        auth_routes::disable_2fa,
        link_routes::shorten,
        link_routes::redirect,
        link_routes::list_links,
        link_routes::link_stats,
        link_routes::delete_link,
        link_routes::my_ip,
        link_routes::check_url,
        link_routes::create_pixel,
        link_routes::list_pixels,
        link_routes::delete_pixel,
        link_routes::serve_pixel,
        link_routes::build_utm,
    ),
    components(
        schemas(
            types::RegisterRequest,
            types::LoginRequest,
            types::RefreshRequest,
            types::TotpVerifyRequest,
            types::ShortenRequest,
            types::LinkResponse,
            types::LinkListResponse,
            types::LinkStatsResponse,
            types::ClicksByDay,
            types::RecentClick,
            types::HealthResponse,
            types::ErrorResponse,
            types::PaginationParams,
            types::TokenResponse,
            types::TotpSetupResponse,
            types::URLCheckRequest,
            types::URLCheckResponse,
            types::RedirectStep,
            types::MyIPResponse,
            types::PixelCreateRequest,
            types::PixelResponse,
            types::PixelListResponse,
            types::UTMBuildRequest,
            types::UTMResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication & 2FA endpoints"),
        (name = "Links", description = "URL shortening & management"),
        (name = "Redirect", description = "URL redirect"),
        (name = "Pixels", description = "Tracking pixels"),
        (name = "Tools", description = "Utility endpoints (IP, URL check, UTM)"),
        (name = "Health", description = "Health check"),
    )
)]
pub struct ApiDoc;
