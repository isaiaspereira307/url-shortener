from fastapi import APIRouter, Depends, HTTPException, status, Request, Response
from sqlalchemy.ext.asyncio import AsyncSession
from app.infrastructure.database.session import get_db
from app.application.services.link_service import (
    create_link,
    get_link_by_short_code,
    get_links_by_user,
    get_link_stats,
    delete_link,
    increment_clicks,
    acquire_shorten_lock,
    release_shorten_lock,
    record_click,
    parse_geo_from_ip,
    create_pixel,
    get_pixels_by_user,
    delete_pixel,
    record_pixel_click,
)
from app.presentation.schemas import (
    LinkCreate,
    LinkResponse,
    LinkListResponse,
    LinkStatsResponse,
    PixelCreateRequest,
    PixelResponse,
    PixelListResponse,
    URLCheckRequest,
    URLCheckResponse,
    MyIPResponse,
    UTMBuildRequest,
    UTMResponse,
    RedirectStep,
)
from app.presentation.middleware.auth import get_current_user
from app.infrastructure.database.models import Link, ClickEvent
from app.infrastructure.cache.redis import redis_client
from urllib.parse import urlparse, urlencode, parse_qs, urlunparse
import hashlib
import os
import httpx

SHORT_CODE_LENGTH = int(os.getenv("SHORT_CODE_LENGTH", "7"))
PIXEL_PNG = bytes([
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x62, 0x00, 0x00, 0x00, 0x02,
    0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
])

router = APIRouter(tags=["links"])
tools_router = APIRouter(tags=["tools"])


def validate_url(url: str) -> bool:
    try:
        parsed = urlparse(url)
        return parsed.scheme in ("http", "https") and bool(parsed.netloc)
    except Exception:
        return False


@router.post("/api/shorten", response_model=LinkResponse, status_code=status.HTTP_201_CREATED)
async def shorten_url(
    body: LinkCreate,
    request: Request,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    if not validate_url(body.url):
        raise HTTPException(status_code=400, detail="Invalid URL. Must start with http:// or https://")

    url_hash = hashlib.md5(body.url.encode()).hexdigest()[:12]

    acquired = await acquire_shorten_lock(url_hash)
    if not acquired:
        raise HTTPException(status_code=429, detail="Too many concurrent requests. Please try again.")

    try:
        link = await create_link(db, current_user["tenant_id"], current_user["sub"], body.url)
        await db.commit()
    except Exception:
        await db.rollback()
        raise
    finally:
        await release_shorten_lock(url_hash)

    base_url = request.base_url
    short_url = f"{base_url}{link.short_code}"

    return LinkResponse(
        id=link.id,
        short_url=short_url,
        original_url=link.original_url,
        short_code=link.short_code,
        clicks=link.clicks,
        created_at=link.created_at,
    )


@router.get("/{short_code}")
async def redirect_url(short_code: str, request: Request, db: AsyncSession = Depends(get_db)):
    if short_code.startswith("px_"):
        return await serve_pixel(short_code, request, db)

    cache_key = f"url:{short_code}"
    cached = await redis_client.get(cache_key)

    ip = request.client.host if request.client else None
    user_agent = request.headers.get("user-agent")
    referer = request.headers.get("referer")
    geo_data = parse_geo_from_ip(ip) if ip else None

    if cached:
        await record_click(db, short_code, ip, user_agent, referer, geo_data)
        await db.commit()
        return Response(status_code=302, headers={"Location": cached})

    link = await get_link_by_short_code(db, short_code)
    if not link:
        raise HTTPException(status_code=404, detail="Link not found")

    await redis_client.setex(cache_key, 86400, link.original_url)

    await record_click(db, short_code, ip, user_agent, referer, geo_data)
    await db.commit()

    return Response(status_code=302, headers={"Location": link.original_url})


@router.get("/api/links", response_model=LinkListResponse)
async def list_links(
    request: Request,
    page: int = 1,
    limit: int = 20,
    sort: str = "created_at",
    order: str = "desc",
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    limit = min(limit, 100)
    links, total = await get_links_by_user(
        db, current_user["tenant_id"], current_user["sub"], page, limit, sort, order
    )

    base_url = request.base_url
    link_responses = [
        LinkResponse(
            id=link.id,
            short_url=f"{base_url}{link.short_code}",
            original_url=link.original_url,
            short_code=link.short_code,
            clicks=link.clicks,
            created_at=link.created_at,
        )
        for link in links
    ]

    return LinkListResponse(links=link_responses, total=total, page=page, limit=limit)


@router.get("/api/links/{short_code}/stats", response_model=LinkStatsResponse)
async def link_stats(
    short_code: str,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    stats = await get_link_stats(db, current_user["tenant_id"], current_user["sub"], short_code)
    if not stats:
        raise HTTPException(status_code=404, detail="Link not found")
    return stats


@router.delete("/api/links/{short_code}", status_code=status.HTTP_200_OK)
async def delete_short_link(
    short_code: str,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    deleted = await delete_link(db, current_user["tenant_id"], current_user["sub"], short_code)
    if not deleted:
        raise HTTPException(status_code=404, detail="Link not found")

    await redis_client.delete(f"url:{short_code}")

    return {"message": "Link deleted successfully"}


# === Tools Endpoints ===

@tools_router.get("/api/myip", response_model=MyIPResponse)
async def my_ip(request: Request):
    ip = request.client.host if request.client else None
    forwarded = request.headers.get("x-forwarded-for")
    if forwarded:
        ip = forwarded.split(",")[0].strip()
    elif request.headers.get("x-real-ip"):
        ip = request.headers.get("x-real-ip")

    geo = parse_geo_from_ip(ip) if ip else {}
    return MyIPResponse(
        ip=ip or "unknown",
        country=geo.get("country"),
        city=geo.get("city"),
        latitude=geo.get("latitude"),
        longitude=geo.get("longitude"),
        isp=geo.get("isp"),
    )


@tools_router.post("/api/check-url", response_model=URLCheckResponse)
async def check_url(body: URLCheckRequest):
    if not validate_url(body.url):
        raise HTTPException(status_code=400, detail="Invalid URL. Must start with http:// or https://")

    chain = []
    current_url = body.url
    final_url = current_url
    visited = set()
    warnings = []
    is_safe = True
    server_ip = None
    server_headers = None
    parsed_original = urlparse(body.url)

    async with httpx.AsyncClient(timeout=10.0, follow_redirects=False) as client:
        for hop in range(11):
            if current_url in visited:
                warnings.append("Redirect loop detected")
                is_safe = False
                break
            visited.add(current_url)

            try:
                resp = await client.get(current_url)
                current_parsed = urlparse(current_url)

                if hop == 0:
                    server_headers = dict(resp.headers)

                chain.append(RedirectStep(url=current_url, status=resp.status_code))

                if resp.status_code in (301, 302, 303, 307, 308):
                    next_url = resp.headers.get("location", "")
                    next_parsed = urlparse(next_url)

                    if not next_url.startswith("http"):
                        next_url = urlunparse((
                            current_parsed.scheme,
                            current_parsed.netloc,
                            next_url,
                            "", "", ""
                        ))

                    next_domain = urlparse(next_url).netloc
                    orig_domain = parsed_original.netloc
                    if next_domain != orig_domain:
                        warnings.append(f"Redirects to different domain: {next_domain}")

                    final_url = next_url
                    current_url = next_url
                else:
                    final_url = current_url
                    break
            except httpx.RequestError as e:
                chain.append(RedirectStep(url=current_url, status=None))
                warnings.append(f"Failed to fetch URL: {str(e)}")
                is_safe = False
                break

    if len(chain) > 3:
        warnings.append("Multiple redirects detected")
    if len(warnings) > 0:
        is_safe = False

    return URLCheckResponse(
        original_url=body.url,
        final_url=final_url,
        redirect_chain=chain,
        total_redirects=len(chain) - 1 if len(chain) > 1 else 0,
        is_safe=is_safe,
        warnings=warnings,
        server_ip=server_ip,
        server_headers=server_headers,
    )


@tools_router.post("/api/pixel", response_model=PixelResponse, status_code=status.HTTP_201_CREATED)
async def create_pixel_endpoint(
    body: PixelCreateRequest,
    request: Request,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    pixel = await create_pixel(db, current_user["tenant_id"], current_user["sub"], body.name)
    await db.commit()

    base_url = request.base_url
    pixel_url = f"{base_url}{pixel.code}.png"

    return PixelResponse(
        id=pixel.id,
        code=pixel.code,
        name=pixel.name,
        pixel_url=pixel_url,
        clicks=pixel.clicks,
        created_at=pixel.created_at,
    )


@tools_router.get("/api/pixels", response_model=PixelListResponse)
async def list_pixels(
    request: Request,
    page: int = 1,
    limit: int = 20,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    limit = min(limit, 100)
    pixels, total = await get_pixels_by_user(db, current_user["tenant_id"], current_user["sub"], page, limit)

    base_url = request.base_url
    pixel_responses = [
        PixelResponse(
            id=p.id,
            code=p.code,
            name=p.name,
            pixel_url=f"{base_url}{p.code}.png",
            clicks=p.clicks,
            created_at=p.created_at,
        )
        for p in pixels
    ]

    return PixelListResponse(pixels=pixel_responses, total=total, page=page, limit=limit)


@tools_router.delete("/api/pixel/{code}", status_code=status.HTTP_200_OK)
async def delete_pixel_endpoint(
    code: str,
    current_user: dict = Depends(get_current_user),
    db: AsyncSession = Depends(get_db),
):
    deleted = await delete_pixel(db, current_user["tenant_id"], current_user["sub"], code)
    if not deleted:
        raise HTTPException(status_code=404, detail="Pixel not found")
    return {"message": "Pixel deleted successfully"}


@tools_router.get("/pixel/{code}.png")
async def serve_pixel(code: str, request: Request, db: AsyncSession = Depends(get_db)):
    pixel_code = code
    if pixel_code.endswith(".png"):
        pixel_code = pixel_code[:-4]

    ip = request.client.host if request.client else None
    forwarded = request.headers.get("x-forwarded-for")
    if forwarded:
        ip = forwarded.split(",")[0].strip()
    user_agent = request.headers.get("user-agent")
    referer = request.headers.get("referer")
    geo_data = parse_geo_from_ip(ip) if ip else None

    await record_pixel_click(db, pixel_code, ip, user_agent, referer, geo_data)
    await db.commit()

    return Response(
        content=PIXEL_PNG,
        media_type="image/png",
        headers={
            "Cache-Control": "no-store, no-cache, must-revalidate",
            "Pragma": "no-cache",
            "Expires": "0",
        },
    )


@tools_router.post("/api/utm-builder", response_model=UTMResponse)
async def build_utm(body: UTMBuildRequest):
    if not validate_url(body.url):
        raise HTTPException(status_code=400, detail="Invalid URL. Must start with http:// or https://")

    parsed = urlparse(body.url)
    params = {}

    utm_fields = {
        "utm_source": body.utm_source,
        "utm_medium": body.utm_medium,
        "utm_campaign": body.utm_campaign,
        "utm_term": body.utm_term,
        "utm_content": body.utm_content,
    }

    for key, value in utm_fields.items():
        if value:
            params[key] = value

    existing_params = parse_qs(parsed.query, keep_blank_values=True)
    existing_params.update(params)

    new_query = urlencode(existing_params, doseq=True)
    utm_url = urlunparse((
        parsed.scheme,
        parsed.netloc,
        parsed.path,
        parsed.params,
        new_query,
        parsed.fragment,
    ))

    return UTMResponse(
        original_url=body.url,
        utm_url=utm_url,
        params={k: v for k, v in utm_fields.items() if v},
    )