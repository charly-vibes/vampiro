# Data analysis with DataFrames-like operations, multiple dispatch, parametric types

using Statistics

# --- Types ---

abstract type Dataset end

struct DataFrame{T<:Real, N} <: Dataset
    names::Vector{String}
    columns::Vector{Vector{T}}
    function DataFrame{T,N}(names::Vector{String}, columns::Vector{Vector{T}}) where {T<:Real, N}
        @assert length(names) == length(columns)
        new(names, columns)
    end
end

struct Series{T}
    name::String
    data::Vector{T}
end

# --- Constructors ---

DataFrame(names::Vector{String}, columns::Vector{Vector{T}}) where {T<:Real} =
    DataFrame{T,length(columns)}(names, columns)

function read_csv(path::String)::DataFrame{Float64}
    lines = readlines(path)
    header = split(lines[1], ',')
    ncols = length(header)
    cols = [Float64[] for _ in 1:ncols]
    for line in lines[2:end]
        isempty(strip(line)) && continue
        vals = split(line, ',')
        for i in 1:min(length(vals), ncols)
            push!(cols[i], parse(Float64, strip(vals[i])))
        end
    end
    DataFrame(header, cols)
end

# --- Operations ---

function filter_by(df::DataFrame, col::String, op::Function)
    idx = findfirst(==(col), df.names)
    idx === nothing && error("Column $col not found")
    mask = op.(df.columns[idx])
    filtered = [col[mask] for col in df.columns]
    DataFrame(df.names, filtered)
end

function groupby_sum(df::DataFrame, key::String, val::String)
    ki = findfirst(==(key), df.names)
    vi = findfirst(==(val), df.names)
    (ki === nothing || vi === nothing) && error("Column not found")

    groups = Dict{eltype(df.columns[ki]),eltype(df.columns[vi])}()
    for i in eachindex(df.columns[ki])
        k = df.columns[ki][i]
        v = df.columns[vi][i]
        groups[k] = get(groups, k, zero(v)) + v
    end
    groups
end

# --- Statistics ---

describe(df::DataFrame) = begin
    for (name, col) in zip(df.names, df.columns)
        println("$name: mean=$(mean(col)), std=$(std(col)), length=$(length(col))")
    end
end

# --- Macros ---

macro assert_df(df)
    quote
        @assert isa($df, DataFrame) "Expected DataFrame, got $(typeof($df))"
        $df
    end
end

# --- Entry point ---

function main()
    df = DataFrame(["a", "b", "c"], [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
    @assert_df df

    filtered = filter_by(df, "a", x -> x > 1.0)
    describe(filtered)

    grouped = groupby_sum(df, "a", "c")
    for (k, v) in grouped
        println("a=$k => sum_c=$v")
    end
end