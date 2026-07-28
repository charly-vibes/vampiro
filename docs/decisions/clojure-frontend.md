# Clojure Frontend Decision

> Parser and support boundary for the Clojure language frontend.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.8.7 — Clojure frontend parser and support boundary decision gate
**Design reference:** `openspec/changes/add-python-clojure-julia-frontends/design.md`
**Date:** 2026-07-28

---

## 1. Parser choice

**Decision:** `tree-sitter-clojure` — the community tree-sitter grammar for Clojure.

**Rationale:**
- The shared design `design.md` recommends tree-sitter grammars for an initial uniform implementation across all three frontends.
- Tree-sitter provides a consistent CST→CIR mapping layer across Python, Clojure, and Julia, reducing per-language adapter complexity.
- Clojure's homoiconic syntax (S-expressions) maps naturally to a CST; tree-sitter-clojure handles all reader macro forms including `#()`, `@`, `'`, `` ` ``, `~`, `~@`, tagged literals, and metadata readers.
- No native Rust parser for Clojure exists at comparable maturity. The `edn-rs` crate parses EDN data notation only, not full Clojure source.
- The tree-sitter approach allows the frontend to treat reader macros as explicit `opaque`/`unknown` sentinels rather than failing to parse.

**Trigger to revisit:** A language-specific gap (e.g., Clojure 1.13+ reader macro syntax not supported) that cannot be addressed within the shared tree-sitter framework.

---

## 2. Clojure version range

**Decision:** Supported versions are **Clojure 1.10 through 1.12**. No version cap — tree-sitter-clojure tracks the latest stable.

**Rationale:**
- Clojure 1.10 is the minimum version still in common use; 1.11 introduced keyword argument and namespace map syntax improvements; 1.12 is the latest stable.
- tree-sitter-clojure's grammar covers all syntax across this range.
- Older versions (1.9 and below) are effectively end-of-life in the ecosystem.

---

## 3. Namespace loading boundary

**Decision:** No namespace resolution or `require`/`use` dereferencing. We parse individual files and treat namespace references as opaque edges.

**Rationale:**
- Clojure's namespace loading is dynamic (`require`, `use`, `import`, `refer`) and may trigger execution of loaded namespaces' side effects.
- `:require` with `:refer :all` makes it impossible to statically determine which symbols are introduced without resolving the target namespace.
- Cross-namespace references are recorded as CIR edges with `provenance::Boundary::Cross` and `provenance::Precision::Opaque`.
- For private-var reach-through patterns (`(.field ns/private-var)`), we emit transparency facts per REQ-V6.

---

## 4. Reader macro boundary

**Decision:** The following Clojure reader-macro constructs are treated as explicit `unknown`/`opaque` sentinels:

| Construct | CIR treatment |
|-----------|---------------|
| `#=(reader-cond)` | `unknown` — reader conditionals are evaluated at read time |
| `#?(:clj ... :cljs ...)` | `opaque` — platform-dependent; only the matching branch is readable |
| `#""` (regex literal) | `opaque` — regex patterns are not statically analyzable |
| `#=(read-eval)` | `unknown` — arbitrary code execution at read time |
| `#inst "..."` / `#uuid "..."` | `opaque` — tagged literals with external resolution |
| Custom `*data-readers*` | `unknown` — user-defined function at read time |

**Rationale:** Reader macros are evaluated at read time, not compile time, making static analysis inherently limited. Explicit unknowns document the limitation.

---

## 5. Initial effect and idiom table

**Decision:** Recognize the following Clojure wrapper patterns as idiom entries:

| Pattern | Effect Channel | Resolution |
|---------|---------------|------------|
| `(or x nil)` / `(some? x)` | `EffectChannel::Option` | `EffectResolution::Unwrapped` (ordinary) |
| `(try ... (catch ...))` | `EffectChannel::Result` | `EffectResolution::Unwrapped` (ordinary) |
| `(future ...)` | `EffectChannel::Async` | `EffectResolution::Propagated` |
| `(lazy-seq ...)` | `EffectChannel::Stream` | `EffectResolution::Propagated` |
| `(with-open ...)` | `EffectChannel::Resource` | `EffectResolution::Propagated` |
| `(binding [*var* val] ...)` | `EffectChannel::Resource` | `EffectResolution::Propagated` |

**Rationale:** These patterns cover the idiomatic effect-manipulation constructs in Clojure. Additional patterns can be added to the idiom table without changing the extraction contract.

---

## 6. Exclusions

The following Clojure constructs are explicitly **unsupported** and will produce an `unknown`/`opaque` sentinel:

| Construct | Reason |
|-----------|--------|
| Java interop (`(.method obj)` / `(new Class)`) | Requires JVM classpath resolution |
| Macros (compile-time code generation) | Cannot determine expansion without execution |
| `eval` / `load` / `load-file` | Dynamic code execution |
| `require` / `use` / `import` side effects | May trigger execution at load time |
| `definterface` / `deftype` / `defrecord` (Java interop parts) | Generates JVM classes at compile time |
| `gen-class` / `gen-interface` | AOT compilation — requires JVM |

---

## 7. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| `edn-rs` Rust crate | Parses EDN data notation only, not full Clojure source |
| Clojure compiler API (via JVM) | Requires JVM embedding; heavyweight for a static analysis tool |
| Manual S-expression parser | Duplicates tree-sitter's grammar work; fragile to Clojure syntax changes |
| `clojure.tools.analyzer` (via JVM) | Requires JVM runtime; produces AST that is not CIR-compatible |

---

## 8. Scope and compatibility

- **Supported scope:** Clojure 1.10–1.12 source files, parsed with tree-sitter-clojure, extracted to CIR v0.1.0.
- **Immutability:** This decision is valid until the trigger condition is met. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.