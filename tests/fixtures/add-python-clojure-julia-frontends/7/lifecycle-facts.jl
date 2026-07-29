# Julia source for lifecycle fact extraction testing.
#
# Expected lifecycle facts:
# - writes: local variable assignments
# - retries: while/for loops with try/catch
# - resources: open/close patterns using do blocks

function read_file(path)
    # Resource acquisition via do-block
    open(path) do io
        return read(io, String)
    end
end

function retry_operation(url, max_retries=3)
    last_error = nothing
    for attempt in 1:max_retries
        try
            result = perform_request(url)
            return result
        catch e
            last_error = e
            continue
        end
    end
    return false
end

function perform_request(url)
    return true
end