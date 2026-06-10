from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from contextlib import asynccontextmanager
from app.infrastructure.database.session import init_db, close_db
from app.infrastructure.cache.redis import init_redis, close_redis
from app.presentation.routes.auth import router as auth_router
from app.presentation.routes.links import router as links_router, tools_router
from app.presentation.routes.health import router as health_router
import os


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    await init_redis()
    yield
    await close_db()
    await close_redis()


app = FastAPI(
    title="URL Shortener - Python Backend",
    description="Python/FastAPI backend for the URL Shortener portfolio project",
    version="0.2.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["GET", "POST", "DELETE", "OPTIONS"],
    allow_headers=["Content-Type", "Authorization"],
)

app.include_router(health_router)
app.include_router(auth_router)
app.include_router(links_router)
app.include_router(tools_router)