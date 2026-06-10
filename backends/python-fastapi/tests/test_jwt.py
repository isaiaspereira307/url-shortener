import pytest
from app.infrastructure.auth.jwt import create_access_token, create_refresh_token, verify_token
from uuid import uuid4


class TestJWT:
    def test_create_and_verify_access_token(self):
        user_id = uuid4()
        tenant_id = uuid4()
        token = create_access_token(user_id, tenant_id, "user@example.com")
        payload = verify_token(token, token_type="access")

        assert payload["sub"] == str(user_id)
        assert payload["tenant_id"] == str(tenant_id)
        assert payload["email"] == "user@example.com"
        assert payload["type"] == "access"

    def test_create_and_verify_refresh_token(self):
        user_id = uuid4()
        tenant_id = uuid4()
        token = create_refresh_token(user_id, tenant_id)
        payload = verify_token(token, token_type="refresh")

        assert payload["sub"] == str(user_id)
        assert payload["tenant_id"] == str(tenant_id)
        assert payload["type"] == "refresh"

    def test_invalid_token_type(self):
        user_id = uuid4()
        tenant_id = uuid4()
        access_token = create_access_token(user_id, tenant_id, "user@example.com")

        with pytest.raises(ValueError, match="Invalid token type"):
            verify_token(access_token, token_type="refresh")

    def test_expired_token(self):
        from datetime import datetime, timedelta, timezone
        import jwt
        from app.infrastructure.auth.jwt import JWT_SECRET, JWT_ALGORITHM

        now = datetime.now(timezone.utc)
        payload = {
            "sub": str(uuid4()),
            "tenant_id": str(uuid4()),
            "email": "user@example.com",
            "type": "access",
            "iat": now - timedelta(hours=1),
            "exp": now - timedelta(minutes=30),
        }
        expired_token = jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)

        with pytest.raises(Exception):
            verify_token(expired_token)
