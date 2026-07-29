# Dogfooding Run 3: Post-fix regression check

**Date:** 2026-07-29  
**Pipeline:** `vampiro check --path <dir>/src --full --mode guidance --json`  
**Scope:** Single repo (dont) — focused on verifying the vampiro-03s fix eliminated the 2 REQ-V7 facade-leak FPs

## Summary

| Metric | Dogfood-2 | Dogfood-3 | Delta |
|--------|----------:|----------:|------:|
| Total findings | 97 | 95 | -2 |
| Facade-leak (REQ-V7) | 2 | **0** | ✅ -2 |
| Composition (REQ-7) | 40 | 40 | 0 |
| Swallowed-effect (REQ-9) | 38 | 38 | 0 |
| Redundancy (REQ-11) | 17 | 17 | 0 |
| Over-exposure (REQ-V4) | 0 | 0 | 0 |

**Confirmed fix:** vampiro-03s — `mod foo_tests { use super::*; }` no longer produces facade-leak findings.

## Bugs fixed in this session

| Ticket | Change | Verification |
|--------|--------|-------------|
| vampiro-03s | Filter test module facades in `visibility_adapter.rs` | ✅ 2 REQ-V7 FPs eliminated |
| vampiro-y4y | Python `__init__` node IDs include class name | Python fixtures pass (cross-language tests) |
| vampiro-fhg | Python edges to builtins filtered | Python fixtures pass |
| vampiro-276 | Julia anonymous function IDs disambiguated | Julia fixtures pass |
| vampiro-3hk | Clojure edge targets checked against graph | Clojure fixtures pass |

## Conclusion

The 2 facade-leak false positives from dogfood-2 are confirmed fixed. All 5 frontend bugs from the stress-testing epic are resolved. No regression in other finding classes.