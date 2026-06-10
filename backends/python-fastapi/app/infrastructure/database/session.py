from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession
from app.infrastructure.database.models import Base
import os

DATABASE_URL = os.getenv("DATABASE_URL", "postgresql+asyncpg://url_shortener:url_shortener_secret@localhost:5432/url_shortener")

engine = create_async_engine(DATABASE_URL, echo=False, pool_size=10, max_overflow=20)

async_session = async_sessionmaker(engine, class_=AsyncSession, expire_on_commit=False)


async def get_db() -> AsyncSession:
    async with async_session() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise


async def init_db():
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)


async def close_db():
    await engine.dispose()
