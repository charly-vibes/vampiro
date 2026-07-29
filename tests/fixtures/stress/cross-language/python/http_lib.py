"""Real-world-ish Python: HTTP client library with session, retries, and error handling."""

import json
import logging
from typing import Optional, Any
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError

logger = logging.getLogger(__name__)


class HTTPErrorV(Exception):
    """Custom HTTP error with status code."""

    def __init__(self, status: int, message: str) -> None:
        self.status = status
        self.message = message
        super().__init__(f"HTTP {status}: {message}")


class Session:
    """Reusable HTTP session with base URL and headers."""

    def __init__(self, base_url: str = "", headers: Optional[dict] = None) -> None:
        self.base_url = base_url.rstrip("/")
        self.headers = headers or {}

    def request(
        self, method: str, path: str, data: Optional[dict] = None
    ) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        body = json.dumps(data).encode() if data else None
        req = Request(url, data=body, method=method)
        for k, v in self.headers.items():
            req.add_header(k, v)

        try:
            with urlopen(req) as resp:
                raw = resp.read().decode()
                return json.loads(raw)
        except HTTPError as e:
            logger.error("HTTP %s on %s %s", e.code, method, url)
            raise HTTPErrorV(e.code, str(e.reason)) from e
        except URLError as e:
            logger.error("Connection failed: %s", e.reason)
            raise

    def get(self, path: str) -> dict[str, Any]:
        return self.request("GET", path)

    def post(self, path: str, data: dict) -> dict[str, Any]:
        return self.request("POST", path, data)


def retry(max_attempts: int = 3) -> callable:
    """Decorator: retry a function up to max_attempts times."""

    def decorator(fn: callable) -> callable:
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            last_exc = None
            for attempt in range(max_attempts):
                try:
                    return fn(*args, **kwargs)
                except HTTPErrorV as e:
                    last_exc = e
                    if e.status < 500:
                        raise
                    logger.warning("Retry %d/%d after %s", attempt + 1, max_attempts, e)
            raise last_exc  # type: ignore

        return wrapper

    return decorator


session = Session("https://api.example.com", {"Authorization": "Bearer token123"})


@retry(max_attempts=3)
def fetch_user(user_id: int) -> dict[str, Any]:
    return session.get(f"/users/{user_id}")