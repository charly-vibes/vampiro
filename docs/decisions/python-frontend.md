# Python Frontend Decision

> Parser and support boundary for the Python language frontend.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.8.6 — Python frontend parser and support boundary decision gate
**Design reference:** `openspec/changes/add-python-clojure-julia-frontends/design.md`
**Date:** 2026-07-28

---

## 1. Parser choice

**Decision:** `tree-sitter-python` — the official tree-sitter grammar for Python.

**Rationale:**
- The shared design `design.md` recommends tree-sitter grammars for an initial uniform implementation across all three frontends.
- Tree-sitter provides a consistent CST→CIR mapping layer across Python, Clojure, and Julia, reducing per-language adapter complexity.
- tree-sitter-python is the most mature tree-sitter grammar, supporting Python 3.x syntax comprehensively.
- It handles dynamic constructs (decorators, `eval`, `getattr`, `setattr`, `__import__`) at the CST level, making them visible as explicit `unknown`/`opaque` sentinels rather than missed.
- Python's `ast` module was considered but rejected: it operates on *executed* code (requires `compile()` + `eval()`), which is not suitable for static analysis of untrusted or partial source files.
- No native Rust parser for Python exists at comparable maturity.

**Trigger to revisit:** A language-specific gap (e.g., Python 4 syntax not supported by tree-sitter, or a high-performance requirement for a specific Python 3.x feature) that cannot be addressed within the shared tree-sitter framework.

---

## 2. Python version range

**Decision:** Supported versions are **Python 3.8 through 3.13**. No version cap — tree-sitter-python tracks the latest stable Python.

**Rationale:**
- Python 3.8 is the minimum version still in security support (as of 2026-07).
- Python 3.9–3.13 are the actively maintained versions during the implementation window.
- tree-sitter-python's grammar covers all syntax changes across this range (walrus operator `:=` in 3.8, type union `X | Y` in 3.10, match/case in 3.10, exception groups in 3.11, etc.).
- Older versions (3.7 and below) are end-of-life and excluded.

---

## 3. Package/module loading boundary

**Decision:** No module resolution or import dereferencing. We parse individual files and treat imports as opaque edges.

**Rationale:**
- Python's dynamic import mechanism (`importlib`, `__import__`, lazy loading) makes static resolution of imported names infeasible without running the code.
- Cross-module analysis would require a full module graph, which is out of scope for the frontend.
- Import statements are recorded as CIR edges with `provenance::Boundary::Cross` and `provenance::Precision::Opaque`.
- For `__init__.py` facade patterns, we emit facade metadata (re-exported names) as advisory facts per REQ-V5.

---

## 4. Dynamic construct boundary

**Decision:** The following dynamic Python constructs are treated as explicit `unknown`/`opaque` sentinels:

| Construct | CIR treatment |
|-----------|---------------|
| `eval()` / `exec()` | `unknown` — cannot statically determine inputs |
| `getattr(obj, name_str)` | `opaque` — dynamic attribute access |
| `setattr(obj, name_str, val)` | `opaque` — dynamic attribute mutation |
| `__import__` / `importlib.import_module` | `opaque` — dynamic import |
| Decorator chains with dynamic targets | `opaque` — cannot determine decoration order statically |
| `__getattr__` / `__setattr__` on arbitrary classes | `opaque` — class-level dynamic dispatch |
| Metaclass `__new__` / `__init__` | `opaque` — class creation is a runtime effect |

**Rationale:** Explicit unknowns are preferable to silent misses. The design principle from CIR is to surface uncertainty rather than guess.

---

## 5. Initial effect and idiom table

**Decision:** Recognize the following Python wrapper patterns as idiom entries:

| Pattern | Effect Channel | Resolution |
|---------|---------------|------------|
| `Optional[T]` / `T \| None` | `EffectChannel::Option` | `EffectResolution::Unwrapped` (ordinary) |
| `Union[T, E]` / `T \| E` | `EffectChannel::Result` | `EffectResolution::Unwrapped` (ordinary) |
| `async def` / `await` | `EffectChannel::Async` | `EffectResolution::Propagated` |
| `yield` / `yield from` | `EffectChannel::Stream` | `EffectResolution::Propagated` |
| `contextmanager` / `@contextlib.contextmanager` | `EffectChannel::Resource` | `EffectResolution::Propagated` |
| `try: ... except:` | — | `EffectResolution::Unwrapped` (ordinary, total) |

**Rationale:** These patterns cover the idiomatic effect-manipulation constructs in Python. Additional patterns can be added to the idiom table without changing the extraction contract.

---

## 6. Exclusions

The following Python constructs are explicitly **unsupported** and will produce an `unknown`/`opaque` sentinel:

| Construct | Reason |
|-----------|--------|
| C extension modules (`.so`/`.pyd`) | Binary — cannot analyze statically |
| Dynamically generated classes (`type(name, bases, dict)`) | Requires execution |
| `__slots__` introspection | Implementation-defined behavior |
| `@dynamic`-style decorator frameworks | Requires runtime type information |
| `sys.meta_path` / import hooks | Changes the import mechanism at runtime |

---

## 7. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| Python `ast` module (via `compile()`) | Requires execution-safe code; cannot handle partial or untrusted files |
| `rustpython-parser` | Immature, slower than tree-sitter, smaller community |
| `pyo3` + CPython AST | Heavy dependency (CPython embedding); runtime overhead for static analysis |
| Manual regex-based extraction | Fragile, cannot handle nested syntax correctly |

---

## 8. Scope and compatibility

- **Supported scope:** Python 3.8–3.13 source files, parsed with tree-sitter-python, extracted to CIR v0.1.0.
- **Immutability:** This decision is valid until the trigger condition is met. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.