import pytest
import pytest_asyncio
from httpx import AsyncClient, ASGITransport
from fastapi import FastAPI
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine, async_sessionmaker
from app.main import app as original_app
from app.infrastructure.database.session import get_db, init_db, close_db
from app.infrastructure.cache.redis import redis_client, init_redis, close_redis
from app.application.services.link_service import create_link, get_link_by_short_code, delete_link, record_click
from app.infrastructure.auth.jwt import create_access_token
from app.infrastructure.database.models import Base

TEST_DATABASE_URL = "postgresql+asyncpg://url_shortener:url_shortener_secret@localhost:5432/url_shortener_test"


@pytest_asyncio.fixture
async def db_session():
    engine = create_async_engine(TEST_DATABASE_URL, echo=False)
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
        await conn.run_sync(Base.metadata.create_all)

    session_factory = async_sessionmaker(engine, expire_on_commit=False)
    async with session_factory() as session:
        yield session

    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
def valid_token():
    return create_access_token("test-user-id", "test-tenant-id", "test@example.com")


@pytest.fixture
def expired_token():
    import jwt
    import time
    from app.infrastructure.auth.jwt import JWT_SECRET, JWT_ALGORITHM
    payload = {
        "sub": "test-user-id",
        "tenant_id": "test-tenant-id",
        "email": "test@example.com",
        "type": "access",
        "exp": int(time.time()) - 100,
    }
    return jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)


class TestRegisterEndpoint:
    def test_register_invalid_email(self, client):
        response = client.post("/api/auth/register", json={
            "email": "invalid-email",
            "password": "securepass123",
            "tenant_name": "Test",
        })
        assert response.status_code == 400

    def test_register_short_password(self, client):
        response = client.post("/api/auth/register", json={
            "email": "user@example.com",
            "password": "short",
            "tenant_name": "Test",
        })
        assert response.status_code == 400

    def test_register_missing_fields(self, client):
        response = client.post("/api/auth/register", json={})
        assert response.status_code == 422


class TestLoginEndpoint:
    def test_login_missing_fields(self, client):
        response = client.post("/api/auth/login", json={})
        assert response.status_code == 422

    def test_login_invalid_credentials(self, client):
        response = client.post("/api/auth/login", json={
            "email": "nonexistent@example.com",
            "password": "wrongpass",
        })
        assert response.status_code == 401


class TestShortenEndpoint:
    def test_shorten_url_no_auth(self, client):
        response = client.post("/api/shorten", json={"url": "https://example.com"})
        assert response.status_code == 401

    def test_shorten_missing_auth_header(self, client):
        response = client.post("/api/shorten", json={"url": "https://example.com"})
        data = response.json()
        assert response.status_code == 401
        assert "authorization" in data["detail"].lower()

    def test_shorten_invalid_bearer(self, client):
        response = client.post(
            "/api/shorten",
            json={"url": "https://example.com"},
            headers={"Authorization": "InvalidFormat"},
        )
        assert response.status_code == 401

    def test_shorten_invalid_url(self, client, valid_token):
        response = client.post(
            "/api/shorten",
            json={"url": "not-a-valid-url"},
            headers={"Authorization": f"Bearer {valid_token}"},
        )
        assert response.status_code == 400

    def test_shorten_javascript_url(self, client, valid_token):
        response = client.post(
            "/api/shorten",
            json={"url": "javascript:alert(1)"},
            headers={"Authorization": f"Bearer {valid_token}"},
        )
        assert response.status_code == 400


class TestListLinksEndpoint:
    def test_list_links_no_auth(self, client):
        response = client.get("/api/links")
        assert response.status_code == 401


class TestDeleteLinkEndpoint:
    def test_delete_link_no_auth(self, client):
        response = client.delete("/api/links/abc1234")
        assert response.status_code == 401


class TestAuthMiddleware:
    def test_missing_authorization_header(self, client):
        response = client.post("/api/shorten", json={"url": "https://example.com"})
        assert response.status_code == 401

    def test_invalid_authorization_format(self, client):
        response = client.post(
            "/api/shorten",
            json={"url": "https://example.com"},
            headers={"Authorization": "Basic abc123"},
        )
        assert response.status_code == 401

    def test_expired_token(self, client, expired_token):
        response = client.post(
            "/api/shorten",
            json={"url": "https://example.com"},
            headers={"Authorization": f"Bearer {expired_token}"},
        )
        assert response.status_code == 401

    def test_malformed_token(self, client):
        response = client.post(
            "/api/shorten",
            json={"url": "https://example.com"},
            headers={"Authorization": "Bearer not-a-jwt-token"},
        )
        assert response.status_code == 401


class TestHealthEndpoint:
    def test_health_check(self, client):
        response = client.get("/health")
        assert response.status_code == 200
        data = response.json()
        assert "status" in data


class TestURLValidation:
    def test_valid_http_url(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("http://example.com") is True

    def test_valid_https_url(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("https://example.com/path?query=1") is True

    def test_invalid_javascript_url(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("javascript:alert(1)") is False

    def test_invalid_data_url(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("data:text/html,<script>alert(1)</script>") is False

    def test_invalid_empty_url(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("") is False

    def test_invalid_no_scheme(self):
        from app.presentation.routes.links import validate_url
        assert validate_url("example.com") is False
