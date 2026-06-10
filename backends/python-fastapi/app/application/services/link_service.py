from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy import select, text, func, cast, String
from app.infrastructure.database.models import Link, ClickEvent, Pixel, PixelClickEvent
from app.infrastructure.cache.redis import redis_client
from nanoid import generate as generate_nanoid
import os
import uuid
import ipaddress

SHORT_CODE_LENGTH = int(os.getenv("SHORT_CODE_LENGTH", "7"))
ALPHABET = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
PIXEL_CODE_PREFIX = "px_"
PIXEL_CODE_LENGTH = int(os.getenv("PIXEL_CODE_LENGTH", "8"))


def generate_short_code() -> str:
    return generate_nanoid(ALPHABET, SHORT_CODE_LENGTH)


def generate_pixel_code() -> str:
    return PIXEL_CODE_PREFIX + generate_nanoid(ALPHABET, PIXEL_CODE_LENGTH)


async def acquire_shorten_lock(url_hash: str, timeout: int = 5) -> bool:
    lock_key = f"lock:shorten:{url_hash}"
    acquired = await redis_client.set(lock_key, "1", nx=True, ex=timeout)
    return bool(acquired)


async def release_shorten_lock(url_hash: str) -> None:
    lock_key = f"lock:shorten:{url_hash}"
    await redis_client.delete(lock_key)


async def create_link(db: AsyncSession, tenant_id, user_id, original_url: str) -> Link:
    for _ in range(3):
        short_code = generate_short_code()
        result = await db.execute(select(Link).where(Link.short_code == short_code))
        if not result.scalar_one_or_none():
            break

    link = Link(
        tenant_id=tenant_id,
        user_id=user_id,
        short_code=short_code,
        original_url=original_url,
    )
    db.add(link)
    await db.flush()
    return link


async def get_link_by_short_code(db: AsyncSession, short_code: str) -> Link | None:
    result = await db.execute(select(Link).where(Link.short_code == short_code))
    return result.scalar_one_or_none()


async def get_links_by_user(
    db: AsyncSession, tenant_id, user_id, page: int = 1, limit: int = 20, sort: str = "created_at", order: str = "desc"
) -> tuple[list[Link], int]:
    offset = (page - 1) * limit
    order_col = getattr(Link, sort, Link.created_at)
    if order == "desc":
        order_col = order_col.desc()
    else:
        order_col = order_col.asc()

    count_result = await db.execute(
        select(Link).where(Link.tenant_id == tenant_id, Link.user_id == user_id)
    )
    total = len(count_result.scalars().all())

    result = await db.execute(
        select(Link)
        .where(Link.tenant_id == tenant_id, Link.user_id == user_id)
        .order_by(order_col)
        .offset(offset)
        .limit(limit)
    )
    links = result.scalars().all()
    return links, total


async def delete_link(db: AsyncSession, tenant_id, user_id, short_code: str) -> bool:
    result = await db.execute(
        select(Link).where(
            Link.short_code == short_code,
            Link.tenant_id == tenant_id,
            Link.user_id == user_id,
        )
    )
    link = result.scalar_one_or_none()
    if link:
        await db.delete(link)
        return True
    return False


async def increment_clicks(db: AsyncSession, short_code: str) -> None:
    result = await db.execute(select(Link).where(Link.short_code == short_code))
    link = result.scalar_one_or_none()
    if link:
        link.clicks += 1


async def record_click(db: AsyncSession, short_code: str, ip: str | None = None, user_agent: str | None = None, referer: str | None = None, geo_data: dict | None = None) -> None:
    result = await db.execute(select(Link).where(Link.short_code == short_code))
    link = result.scalar_one_or_none()
    if not link:
        return

    link.clicks += 1

    click_event = ClickEvent(
        link_id=link.id,
        ip=ip,
        user_agent=user_agent,
        referer=referer,
        country=geo_data.get("country") if geo_data else None,
        city=geo_data.get("city") if geo_data else None,
        latitude=geo_data.get("latitude") if geo_data else None,
        longitude=geo_data.get("longitude") if geo_data else None,
        isp=geo_data.get("isp") if geo_data else None,
    )
    db.add(click_event)
    await db.flush()


async def get_link_stats(db: AsyncSession, tenant_id, user_id, short_code: str) -> dict | None:
    result = await db.execute(
        select(Link).where(
            Link.short_code == short_code,
            Link.tenant_id == tenant_id,
            Link.user_id == user_id,
        )
    )
    link = result.scalar_one_or_none()
    if not link:
        return None

    total_clicks_result = await db.execute(
        select(func.count()).where(ClickEvent.link_id == link.id)
    )
    total_clicks = total_clicks_result.scalar() or 0

    unique_visitors_result = await db.execute(
        select(func.count(func.distinct(ClickEvent.ip))).where(ClickEvent.link_id == link.id)
    )
    unique_visitors = unique_visitors_result.scalar() or 0

    clicks_by_country = await db.execute(
        select(ClickEvent.country, func.count())
        .where(ClickEvent.link_id == link.id, ClickEvent.country.isnot(None))
        .group_by(ClickEvent.country)
        .order_by(func.count().desc())
        .limit(20)
    )
    country_data = {row[0]: row[1] for row in clicks_by_country.all()}

    clicks_by_day = await db.execute(
        select(func.date_trunc("day", ClickEvent.clicked_at).label("day"), func.count())
        .where(ClickEvent.link_id == link.id)
        .group_by("day")
        .order_by(text("day DESC"))
        .limit(30)
    )
    day_data = [{"date": str(row[0].date()), "count": row[1]} for row in clicks_by_day.all()]

    recent_clicks = await db.execute(
        select(ClickEvent)
        .where(ClickEvent.link_id == link.id)
        .order_by(ClickEvent.clicked_at.desc())
        .limit(50)
    )
    recent = [
        {
            "ip": str(row.ip) if row.ip else None,
            "country": row.country,
            "city": row.city,
            "latitude": row.latitude,
            "longitude": row.longitude,
            "isp": row.isp,
            "user_agent": row.user_agent,
            "referer": row.referer,
            "clicked_at": row.clicked_at.isoformat() if row.clicked_at else None,
        }
        for row in recent_clicks.scalars().all()
    ]

    browsers = {}
    platforms = {}
    for click in recent:
        ua = click.get("user_agent", "") if isinstance(click, dict) else ""
        browser = parse_browser(ua)
        platform = parse_platform(ua)
        browsers[browser] = browsers.get(browser, 0) + 1
        platforms[platform] = platforms.get(platform, 0) + 1

    return {
        "short_code": link.short_code,
        "original_url": link.original_url,
        "total_clicks": link.clicks,
        "unique_visitors": unique_visitors,
        "clicks_by_country": country_data,
        "clicks_by_day": day_data,
        "recent_clicks": recent,
        "browsers": browsers,
        "platforms": platforms,
    }


def parse_browser(user_agent: str) -> str:
    if not user_agent:
        return "Unknown"
    ua = user_agent.lower()
    if "edg" in ua:
        return "Edge"
    if "chrome" in ua and "edg" not in ua:
        return "Chrome"
    if "firefox" in ua:
        return "Firefox"
    if "safari" in ua and "chrome" not in ua:
        return "Safari"
    if "opera" in ua or "opr" in ua:
        return "Opera"
    return "Other"


def parse_platform(user_agent: str) -> str:
    if not user_agent:
        return "Unknown"
    ua = user_agent.lower()
    if "windows" in ua:
        return "Windows"
    if "mac" in ua:
        return "macOS"
    if "linux" in ua:
        return "Linux"
    if "android" in ua:
        return "Android"
    if "iphone" in ua or "ipad" in ua:
        return "iOS"
    return "Other"


def parse_geo_from_ip(ip_str: str) -> dict:
    return {
        "country": None,
        "city": None,
        "latitude": None,
        "longitude": None,
        "isp": None,
    }


async def create_pixel(db: AsyncSession, tenant_id, user_id, name: str | None = None) -> Pixel:
    code = generate_pixel_code()
    pixel = Pixel(
        tenant_id=tenant_id,
        user_id=user_id,
        code=code,
        name=name,
    )
    db.add(pixel)
    await db.flush()
    return pixel


async def get_pixels_by_user(db: AsyncSession, tenant_id, user_id, page: int = 1, limit: int = 20) -> tuple[list[Pixel], int]:
    offset = (page - 1) * limit
    count_result = await db.execute(
        select(Pixel).where(Pixel.tenant_id == tenant_id, Pixel.user_id == user_id)
    )
    total = len(count_result.scalars().all())

    result = await db.execute(
        select(Pixel)
        .where(Pixel.tenant_id == tenant_id, Pixel.user_id == user_id)
        .order_by(Pixel.created_at.desc())
        .offset(offset)
        .limit(limit)
    )
    pixels = result.scalars().all()
    return pixels, total


async def delete_pixel(db: AsyncSession, tenant_id, user_id, code: str) -> bool:
    result = await db.execute(
        select(Pixel).where(
            Pixel.code == code,
            Pixel.tenant_id == tenant_id,
            Pixel.user_id == user_id,
        )
    )
    pixel = result.scalar_one_or_none()
    if pixel:
        await db.delete(pixel)
        return True
    return False


async def record_pixel_click(db: AsyncSession, code: str, ip: str | None = None, user_agent: str | None = None, referer: str | None = None, geo_data: dict | None = None) -> None:
    result = await db.execute(select(Pixel).where(Pixel.code == code))
    pixel = result.scalar_one_or_none()
    if not pixel:
        return

    pixel.clicks += 1

    click_event = PixelClickEvent(
        pixel_id=pixel.id,
        ip=ip,
        user_agent=user_agent,
        referer=referer,
        country=geo_data.get("country") if geo_data else None,
        city=geo_data.get("city") if geo_data else None,
        latitude=geo_data.get("latitude") if geo_data else None,
        longitude=geo_data.get("longitude") if geo_data else None,
        isp=geo_data.get("isp") if geo_data else None,
    )
    db.add(click_event)
    await db.flush()