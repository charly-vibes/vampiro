import asyncio
from functools import lru_cache

@lru_cache(maxsize=None)
def cached_add(a: int, b: int) -> int:
    return a + b

async def fetch_data(url: str) -> dict:
    await asyncio.sleep(0)
    return {"ok": True}

class Meta(type):
    pass

class MyClass(metaclass=Meta):
    pass

async def main():
    result = await fetch_data("https://example.com")
    print(result)