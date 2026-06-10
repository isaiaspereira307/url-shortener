import jwt
import os
from datetime import datetime, timedelta, timezone
from uuid import UUID

JWT_SECRET = os.getenv("JWT_SECRET", "super-secret-key-change-this-in-production-32chars")
JWT_ALGORITHM = os.getenv("JWT_ALGORITHM", "HS256")
JWT_ACCESS_EXPIRE_MINUTES = int(os.getenv("JWT_ACCESS_EXPIRE_MINUTES", "15"))
JWT_REFRESH_EXPIRE_DAYS = int(os.getenv("JWT_REFRESH_EXPIRE_DAYS", "7"))


def create_access_token(user_id: UUID, tenant_id: UUID, email: str) -> str:
    now = datetime.now(timezone.utc)
    payload = {
        "sub": str(user_id),
        "tenant_id": str(tenant_id),
        "email": email,
        "type": "access",
        "iat": now,
        "exp": now + timedelta(minutes=JWT_ACCESS_EXPIRE_MINUTES),
    }
    return jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)


def create_refresh_token(user_id: UUID, tenant_id: UUID) -> str:
    now = datetime.now(timezone.utc)
    payload = {
        "sub": str(user_id),
        "tenant_id": str(tenant_id),
        "type": "refresh",
        "iat": now,
        "exp": now + timedelta(days=JWT_REFRESH_EXPIRE_DAYS),
    }
    return jwt.encode(payload, JWT_SECRET, algorithm=JWT_ALGORITHM)


def decode_token(token: str) -> dict:
    return jwt.decode(token, JWT_SECRET, algorithms=[JWT_ALGORITHM])


def verify_token(token: str, token_type: str = "access") -> dict:
    payload = decode_token(token)
    if payload.get("type") != token_type:
        raise ValueError("Invalid token type")
    return payload
