"""Python source for runner-input extraction testing.

Expected runner-input fields:
- version: "0.1.0"
- source_file: "runner-input.py"
- tagged_fns: functions with type annotations
- serializable_values: parameters with primitive types
- generator_refs: generator functions (yield)
"""

from typing import Optional


def add(a: int, b: int) -> int:
    return a + b


def greet(name: str) -> str:
    return f"Hello, {name}!"


def process(items: list[str]) -> Optional[str]:
    if items:
        return items[0]
    return None


def count_up_to(n: int):
    """Generator function."""
    for i in range(n):
        yield i