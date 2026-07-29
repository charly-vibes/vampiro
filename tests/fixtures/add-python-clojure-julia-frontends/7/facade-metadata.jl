# Julia module facade metadata fixture.
#
# Expected facade declarations:
# - exported names from the module

module MyLibrary

export run, configure, Helper, format_result

function run(x)
    return x
end

function configure(x)
    return x
end

end