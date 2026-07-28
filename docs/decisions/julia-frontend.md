# Julia Frontend Decision

> Parser and support boundary for the Julia language frontend.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.8.8 — Julia frontend parser and support boundary decision gate
**Design reference:** `openspec/changes/add-python-clojure-julia-frontends/design.md`
**Date:** 2026-07-28

---

## 1. Parser choice

**Decision:** `tree-sitter-julia` — the official tree-sitter grammar for Julia.

**Rationale:**
- The shared design `design.md` recommends tree-sitter grammars for an initial uniform implementation across all three frontends.
- Tree-sitter provides a consistent CST→CIR mapping layer across Python, Clojure, and Julia, reducing per-language adapter complexity.
- tree-sitter-julia is the official grammar, maintained by tree-sitter, and covers all Julia 1.x syntax.
- It handles Julia's distinctive syntax including multiple dispatch type annotations, parametric types, `where` clauses, macro `@` syntax, and generator/comprehension expressions.
- `JuliaSyntax.jl` (the native Julia parser) was considered but rejected: it requires a Julia runtime, which is incompatible with Rust-based static analysis.
- `JuliaCall.jl` (RPC) is too heavy for a frontend parser.

**Trigger to revisit:** A language-specific gap (e.g., Julia 2.0 syntax changes) that cannot be addressed within the shared tree-sitter framework.

---

## 2. Julia version range

**Decision:** Supported versions are **Julia 1.6 through 1.11**. No version cap — tree-sitter-julia tracks the latest stable.

**Rationale:**
- Julia 1.6 LTS is the minimum long-term support version still in common use.
- Julia 1.9–1.11 are the actively maintained versions during the implementation window.
- tree-sitter-julia's grammar covers all syntax changes across this range.
- Older versions (1.5 and below) are end-of-life.

---

## 3. Module loading boundary

**Decision:** No module resolution or `import`/`using` dereferencing. We parse individual files and treat module references as opaque edges.

**Rationale:**
- Julia's module system is dynamic: `include("file.jl")` executes the file at the point of inclusion, and `using Module: name` may re-export names from transitive dependencies.
- Cross-module analysis would require full package resolution, which is out of scope for the frontend.
- Module references are recorded as CIR edges with `provenance::Boundary::Cross` and `provenance::Precision::Opaque`.
- For type-piracy patterns (extending a foreign function/type from another module), we emit ownership facts per REQ-V5.

---

## 4. Macro and generated-function boundary

**Decision:** The following Julia constructs are treated as explicit `unknown`/`opaque` sentinels:

| Construct | CIR treatment |
|-----------|---------------|
| `@eval` / `eval(...)` | `unknown` — arbitrary code execution at runtime |
| `@generated` functions | `opaque` — body is generated at compile time |
| `include("file.jl")` | `opaque` — executes the file at the call site |
| `@macroexpand` / `macroexpand` | `unknown` — macro expansion is a runtime step |
| `Meta.parse` / `Expr` construction | `unknown` — creates code from strings |
| `invoke(f, T, args...)` | `opaque` — bypasses dispatch with specific type signature |

**Rationale:** Julia's metaprogramming is a core language feature, but the resulting code is not statically analyzable without execution. Explicit unknowns document the limitation.

---

## 5. Initial effect and idiom table

**Decision:** Recognize the following Julia wrapper patterns as idiom entries:

| Pattern | Effect Channel | Resolution |
|---------|---------------|------------|
| `Union{T, Nothing}` / `Union{T, Missing}` | `EffectChannel::Option` | `EffectResolution::Unwrapped` (ordinary) |
| `Union{T, E}` / `try ... catch` | `EffectChannel::Result` | `EffectResolution::Unwrapped` (ordinary) |
| `@async` / `@sync` / `fetch(t)` | `EffectChannel::Async` | `EffectResolution::Propagated` |
| `Channel{T}` / `@task` | `EffectChannel::Stream` | `EffectResolution::Propagated` |
| `open(f, "path") do io; ... end` | `EffectChannel::Resource` | `EffectResolution::Propagated` |
| `@lock` / `ReentrantLock` | `EffectChannel::Resource` | `EffectResolution::Propagated` |

**Rationale:** These patterns cover the idiomatic effect-manipulation constructs in Julia. Additional patterns can be added to the idiom table without changing the extraction contract.

---

## 6. Exclusions

The following Julia constructs are explicitly **unsupported** and will produce an `unknown`/`opaque` sentinel:

| Construct | Reason |
|-----------|--------|
| C/Fortran foreign function calls (`ccall`) | Binary — cannot analyze statically |
| `@inline` / `@noinline` / `@inbounds` | Compiler hints — no semantic effect on CIR |
| `@code_typed` / `@code_llvm` / `@code_native` | Introspection — requires compilation |
| Generated functions (`@generated`) | Body generated at compile time |
| `Base.@invokelatest` | Dynamic dispatch bypass |
| `Core.Compiler` internals | Internal API — unstable across versions |

---

## 7. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| `JuliaSyntax.jl` (native Julia parser) | Requires Julia runtime; incompatible with Rust-based tool |
| `JuliaCall.jl` / RPC approach | Heavy runtime dependency; latency over IPC for static analysis |
| `TreeSitter.jl` → Rust bridge | Circuitous path; better to use tree-sitter directly from Rust |
| Manual recursive descent parser | Would duplicate tree-sitter grammar effort; fragile to syntax evolution |

---

## 8. Scope and compatibility

- **Supported scope:** Julia 1.6–1.11 source files, parsed with tree-sitter-julia, extracted to CIR v0.1.0.
- **Immutability:** This decision is valid until the trigger condition is met. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.