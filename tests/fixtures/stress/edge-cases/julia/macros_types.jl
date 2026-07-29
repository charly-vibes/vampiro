@enum Color red green blue

struct Point{T<:Real}
    x::T
    y::T
end

function distance(p1::Point, p2::Point)::Float64
    sqrt((p2.x - p1.x)^2 + (p2.y - p1.y)^2)
end

macro greet(name)
    :(println("Hello, ", $name))
end

function main()
    p = Point(1.0, 2.0)
    @greet("world")
    println(distance(p, Point(4.0, 6.0)))
end