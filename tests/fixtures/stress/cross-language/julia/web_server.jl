# Web framework and async I/O patterns

using HTTP
using JSON

# --- Request/Response types ---

struct Request
    method::String
    path::String
    headers::Dict{String,String}
    body::String
end

struct Response
    status::Int
    headers::Dict{String,String}
    body::String
end

function ok(body::String; content_type="text/plain")
    Response(200, Dict("content-type" => content_type), body)
end

function not_found(msg="Not found")
    Response(404, Dict("content-type" => "text/plain"), msg)
end

# --- Middleware ---

function with_logging(handler)
    return function(req::Request)::Response
        @info "→ $(req.method) $(req.path)"
        resp = handler(req)
        @info "← $(resp.status)"
        resp
    end
end

# --- Router ---

struct Route
    pattern::Regex
    handler::Function
end

function match_route(routes::Vector{Route}, req::Request)::Response
    for route in routes
        m = match(route.pattern, req.path)
        if m !== nothing
            return route.handler(req, m.captures...)
        end
    end
    not_found("No route for $(req.path)")
end

# --- Handlers ---

function home(req::Request, captures...)::Response
    ok("Welcome!")
end

function user_handler(req::Request, user_id::String)::Response
    id = parse(Int, user_id)
    ok("User $id", content_type="application/json")
end

# --- App ---

const routes = Route[
    Route(r"^/$", home),
    Route(r"^/users/(\d+)$", user_handler),
]

function app(req::Request)::Response
    match_route(routes, req)
end

function serve(; host="0.0.0.0", port=8080)
    println("Starting server on $host:$port")
    while true
        sleep(1)
    end
    println("done")
end