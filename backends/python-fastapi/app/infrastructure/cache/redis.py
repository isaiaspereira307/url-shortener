import redis.asyncio as redis
import os

REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379/0")

redis_client = redis.Redis.from_url(REDIS_URL, decode_responses=True)


async def get_redis():
    return redis_client


async def init_redis():
    await redis_client.ping()


async def close_redis():
    await redis_client.close()
