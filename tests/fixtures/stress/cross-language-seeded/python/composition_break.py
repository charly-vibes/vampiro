# Python data-flow edge exercise
# Tests that per-slot edges are emitted for call arguments with known shapes.
# The call `add(42, 3)` should produce 2 slot edges (slots 0 and 1)
# with expression nodes for the integer literals.

def add(a: int, b: int) -> int:
    return a + b

def main() -> int:
    return add(42, 3)