# Python clean baseline — no composition breaks
# All functions have matching return types at call sites.

from typing import Optional


def source_value() -> int:
    return 42


def aggregate() -> int:
    v = source_value()
    return v