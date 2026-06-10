from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select
from app.infrastructure.database.models import Tenant, User
from app.infrastructure.auth.password import hash_password
import re


def validate_email(email: str) -> bool:
    pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    return bool(re.match(pattern, email))


def validate_password(password: str) -> tuple[bool, str]:
    if len(password) < 8:
        return False, "Password must be at least 8 characters"
    return True, ""


def generate_slug(name: str) -> str:
    slug = name.lower().strip()
    slug = re.sub(r"[^a-z0-9]+", "-", slug)
    slug = slug.strip("-")
    return slug


async def create_tenant(db: AsyncSession, name: str) -> Tenant:
    slug = generate_slug(name)
    base_slug = slug
    counter = 1
    while True:
        result = await db.execute(select(Tenant).where(Tenant.slug == slug))
        existing = result.scalar_one_or_none()
        if not existing:
            break
        slug = f"{base_slug}-{counter}"
        counter += 1

    tenant = Tenant(name=name, slug=slug)
    db.add(tenant)
    await db.flush()
    return tenant


async def create_user(db: AsyncSession, tenant_id, email: str, password: str) -> User:
    password_hash = hash_password(password)
    user = User(tenant_id=tenant_id, email=email, password_hash=password_hash)
    db.add(user)
    await db.flush()
    return user


async def get_user_by_email(db: AsyncSession, tenant_id, email: str) -> User | None:
    result = await db.execute(
        select(User).where(User.tenant_id == tenant_id, User.email == email)
    )
    return result.scalar_one_or_none()


async def get_user_by_id(db: AsyncSession, user_id) -> User | None:
    result = await db.execute(select(User).where(User.id == user_id))
    return result.scalar_one_or_none()


async def get_tenant_by_slug(db: AsyncSession, slug: str) -> Tenant | None:
    result = await db.execute(select(Tenant).where(Tenant.slug == slug))
    return result.scalar_one_or_none()
