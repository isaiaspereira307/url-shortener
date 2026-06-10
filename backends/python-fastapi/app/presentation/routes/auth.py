from fastapi import APIRouter, Depends, HTTPException, status
from fastapi.security import HTTPAuthorizationCredentials
from sqlalchemy.ext.asyncio import AsyncSession
from app.infrastructure.database.session import get_db
from app.application.services.auth_service import (
    create_tenant,
    create_user,
    get_user_by_email,
    validate_email,
    validate_password,
)
from app.application.services.totp_service import setup_totp, verify_totp
from app.infrastructure.auth.jwt import create_access_token, create_refresh_token, verify_token
from app.infrastructure.auth.password import verify_password
from app.infrastructure.auth.totp import verify_backup_code
from app.presentation.schemas import (
    RegisterRequest,
    LoginRequest,
    TokenResponse,
    TOTPSetupResponse,
    TOTPVerifyRequest,
)
from app.presentation.middleware.auth import get_current_user, security
from app.infrastructure.database.models import User
from sqlalchemy import select
import json

router = APIRouter(prefix="/api/auth", tags=["auth"])


@router.post("/register", response_model=TokenResponse, status_code=status.HTTP_201_CREATED)
async def register(body: RegisterRequest, db: AsyncSession = Depends(get_db)):
    if not validate_email(body.email):
        raise HTTPException(status_code=400, detail="Invalid email format")

    valid, msg = validate_password(body.password)
    if not valid:
        raise HTTPException(status_code=400, detail=msg)

    tenant = await create_tenant(db, body.tenant_name)
    user = await create_user(db, tenant.id, body.email, body.password)

    access_token = create_access_token(user.id, tenant.id, user.email)
    refresh_token = create_refresh_token(user.id, tenant.id)

    return TokenResponse(access_token=access_token, refresh_token=refresh_token)


@router.post("/login", response_model=TokenResponse)
async def login(body: LoginRequest, db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(User).where(User.email == body.email))
    user = result.scalar_one_or_none()

    if not user or not verify_password(body.password, user.password_hash):
        raise HTTPException(status_code=401, detail="Invalid email or password")

    if user.totp_enabled:
        return TokenResponse(
            access_token="",
            refresh_token="",
            totp_required=True,
        )

    access_token = create_access_token(user.id, user.tenant_id, user.email)
    refresh_token = create_refresh_token(user.id, user.tenant_id)

    return TokenResponse(access_token=access_token, refresh_token=refresh_token)


@router.post("/login/2fa", response_model=TokenResponse)
async def login_2fa(
    body: TOTPVerifyRequest,
    credentials: HTTPAuthorizationCredentials = Depends(security),
    db: AsyncSession = Depends(get_db),
):
    try:
        payload = verify_token(credentials.credentials, token_type="access")
    except Exception:
        raise HTTPException(status_code=401, detail="Invalid token")

    user_id = payload["sub"]
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()

    if not user or not user.totp_secret:
        raise HTTPException(status_code=400, detail="2FA not enabled")

    if not verify_totp(user.totp_secret, body.code):
        raise HTTPException(status_code=401, detail="Invalid 2FA code")

    access_token = create_access_token(user.id, user.tenant_id, user.email)
    refresh_token = create_refresh_token(user.id, user.tenant_id)

    return TokenResponse(access_token=access_token, refresh_token=refresh_token)


@router.post("/refresh", response_model=TokenResponse)
async def refresh(body: dict, db: AsyncSession = Depends(get_db)):
    refresh_token = body.get("refresh_token")
    if not refresh_token:
        raise HTTPException(status_code=400, detail="Missing refresh token")

    try:
        payload = verify_token(refresh_token, token_type="refresh")
    except Exception:
        raise HTTPException(status_code=401, detail="Invalid refresh token")

    user_id = payload["sub"]
    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()

    if not user:
        raise HTTPException(status_code=401, detail="User not found")

    access_token = create_access_token(user.id, user.tenant_id, user.email)
    new_refresh_token = create_refresh_token(user.id, user.tenant_id)

    return TokenResponse(access_token=access_token, refresh_token=new_refresh_token)


@router.post("/2fa/setup", response_model=TOTPSetupResponse)
async def setup_2fa(current_user: dict = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    user_id = current_user["sub"]
    email = current_user["email"]

    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()

    if not user:
        raise HTTPException(status_code=404, detail="User not found")

    if user.totp_enabled:
        raise HTTPException(status_code=400, detail="2FA already enabled")

    totp_data = setup_totp(email)

    user.totp_secret = totp_data["secret"]
    await db.flush()

    return TOTPSetupResponse(
        secret=totp_data["secret"],
        qr_code_uri=totp_data["qr_code_uri"],
        backup_codes=totp_data["backup_codes"],
    )


@router.post("/2fa/verify", status_code=status.HTTP_200_OK)
async def verify_2fa(body: TOTPVerifyRequest, current_user: dict = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    user_id = current_user["sub"]

    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()

    if not user or not user.totp_secret:
        raise HTTPException(status_code=400, detail="2FA not set up")

    if not verify_totp(user.totp_secret, body.code):
        raise HTTPException(status_code=401, detail="Invalid 2FA code")

    user.totp_enabled = True
    await db.flush()

    return {"message": "2FA enabled successfully"}


@router.post("/2fa/disable", status_code=status.HTTP_200_OK)
async def disable_2fa(body: TOTPVerifyRequest, current_user: dict = Depends(get_current_user), db: AsyncSession = Depends(get_db)):
    user_id = current_user["sub"]

    result = await db.execute(select(User).where(User.id == user_id))
    user = result.scalar_one_or_none()

    if not user:
        raise HTTPException(status_code=404, detail="User not found")

    if not user.totp_enabled:
        raise HTTPException(status_code=400, detail="2FA not enabled")

    if not verify_totp(user.totp_secret, body.code):
        raise HTTPException(status_code=401, detail="Invalid 2FA code")

    user.totp_enabled = False
    user.totp_secret = None
    await db.flush()

    return {"message": "2FA disabled successfully"}
