from fastapi import APIRouter, Depends
from sqlalchemy.ext.asyncio import AsyncSession
from app.infrastructure.database.session import get_db
from app.infrastructure.cache.redis import redis_client
from app.presentation.schemas import HealthResponse
from datetime import datetime, timezone

router = APIRouter(tags=["health"])


@router.get("/health", response_model=HealthResponse)
async def health_check(db: AsyncSession = Depends(get_db)):
    db_status = "ok"
    redis_status = "ok"

    try:
        await db.execute("SELECT 1")
    except Exception:
        db_status = "error"

    try:
        await redis_client.ping()
    except Exception:
        redis_status = "error"

    overall = "healthy" if db_status == "ok" and redis_status == "ok" else "degraded"

    return HealthResponse(
        status=overall,
        service="url-shortener-python",
        database=db_status,
        redis=redis_status,
        timestamp=datetime.now(timezone.utc),
    )
