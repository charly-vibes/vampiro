"""Python source for lifecycle fact extraction testing.

Expected lifecycle facts:
- writes: variable assignments, attribute assignments
- retries: loops mimicking retry patterns
- resources: context manager usage
- exit paths: return, raise, normal
- aliases: variable aliasing
"""

import os


def read_file(path: str) -> str:
    """Read a file with a context manager."""
    with open(path, "r") as f:
        content = f.read()
    return content


def write_file(path: str, content: str) -> None:
    """Write to a file."""
    with open(path, "w") as f:
        f.write(content)


def retry_operation(url: str, max_retries: int = 3) -> bool:
    """Retry pattern with for loop."""
    last_error = None
    for attempt in range(max_retries):
        try:
            result = perform_request(url)
            return result
        except ConnectionError as e:
            last_error = e
            continue
    return False


class ResourceManager:
    def __init__(self) -> None:
        self.conn = None

    def connect(self, url: str) -> None:
        self.conn = create_connection(url)

    def disconnect(self) -> None:
        if self.conn:
            self.conn.close()
        self.conn = None


def perform_request(url: str) -> bool:
    return True


def create_connection(url: str) -> str:
    return url