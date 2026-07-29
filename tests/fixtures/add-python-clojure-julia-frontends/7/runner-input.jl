# Julia source for runner-input extraction testing.
#
# Expected runner-input fields:
# - tagged_fns: function declarations with params
# - generator_refs: channels/tasks (Julia's async generators)

function add(a, b)
    return a + b
end

function greet(name)
    return "Hello, $name!"
end

function process(items)
    if !isempty(items)
        return first(items)
    end
    return nothing
end

function count_up(n)
    # Generator-like pattern using Channel
    Channel() do c
        for i in 1:n
            put!(c, i)
        end
    end
end