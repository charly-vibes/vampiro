# Julia clean baseline — no composition breaks
# All functions have matching return types at call sites.

function source_value()::Int
    return 42
end

function aggregate()::Int
    v = source_value()
    return v
end