from pydantic import BaseModel
from datetime import datetime
from uuid import UUID
from typing import Any


class TenantBase(BaseModel):
    name: str
    slug: str


class TenantCreate(BaseModel):
    name: str


class TenantResponse(TenantBase):
    id: UUID
    created_at: datetime


class UserBase(BaseModel):
    email: str


class UserResponse(UserBase):
    id: UUID
    tenant_id: UUID
    totp_enabled: bool
    created_at: datetime


class LinkBase(BaseModel):
    original_url: str


class LinkCreate(BaseModel):
    url: str


class LinkResponse(BaseModel):
    id: UUID
    short_url: str
    original_url: str
    short_code: str
    clicks: int
    created_at: datetime


class LinkListResponse(BaseModel):
    links: list[LinkResponse]
    total: int
    page: int
    limit: int


class LinkStatsResponse(BaseModel):
    short_code: str
    original_url: str
    total_clicks: int
    unique_visitors: int
    clicks_by_country: dict[str, int]
    clicks_by_day: list[dict[str, Any]]
    recent_clicks: list[dict[str, Any]]
    browsers: dict[str, int]
    platforms: dict[str, int]


class ClickEventResponse(BaseModel):
    id: UUID
    link_id: UUID
    ip: str | None
    user_agent: str | None
    referer: str | None
    clicked_at: datetime


class RegisterRequest(BaseModel):
    email: str
    password: str
    tenant_name: str


class LoginRequest(BaseModel):
    email: str
    password: str


class TokenResponse(BaseModel):
    access_token: str
    refresh_token: str
    token_type: str = "bearer"
    totp_required: bool = False


class TOTPSetupResponse(BaseModel):
    secret: str
    qr_code_uri: str
    backup_codes: list[str]


class TOTPVerifyRequest(BaseModel):
    code: str


class HealthResponse(BaseModel):
    status: str
    service: str
    database: str
    redis: str
    timestamp: datetime


class ErrorResponse(BaseModel):
    error: str
    detail: str | None = None


class MyIPResponse(BaseModel):
    ip: str
    country: str | None = None
    city: str | None = None
    latitude: float | None = None
    longitude: float | None = None
    isp: str | None = None


class URLCheckRequest(BaseModel):
    url: str


class RedirectStep(BaseModel):
    url: str
    status: int | None = None


class URLCheckResponse(BaseModel):
    original_url: str
    final_url: str | None = None
    redirect_chain: list[RedirectStep]
    total_redirects: int
    is_safe: bool
    warnings: list[str]
    server_ip: str | None = None
    server_headers: dict[str, str] | None = None


class PixelCreateRequest(BaseModel):
    name: str | None = None


class PixelResponse(BaseModel):
    id: UUID
    code: str
    name: str | None
    pixel_url: str
    clicks: int
    created_at: datetime


class PixelListResponse(BaseModel):
    pixels: list[PixelResponse]
    total: int
    page: int
    limit: int


class UTMBuildRequest(BaseModel):
    url: str
    utm_source: str | None = None
    utm_medium: str | None = None
    utm_campaign: str | None = None
    utm_term: str | None = None
    utm_content: str | None = None


class UTMResponse(BaseModel):
    original_url: str
    utm_url: str
    params: dict[str, str]