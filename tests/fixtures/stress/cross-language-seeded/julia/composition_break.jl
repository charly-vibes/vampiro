# Julia data-flow edge exercise
# Tests that per-slot edges are emitted for call arguments with known shapes.
# The call add(42, 3) should produce 2 slot edges (slots 0 and 1)
# with expression nodes for the integer literals.

function add(x, y)
    return x + y
end

function main()
    add(42, 3)
end