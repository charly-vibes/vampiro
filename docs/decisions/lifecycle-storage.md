# Lifecycle Storage Decision

> Snapshot storage, schema/version, retention, baseline override, migration
> declaration, migration-reader policy, and initial write/resource idiom sets
> for Vampiro's facade evolution history (REQ-T1, REQ-T4).

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.7.1 — Approve lifecycle storage and idiom policy
**Date:** 2026-07-28

---

## 1. Snapshot storage format

### Alternatives considered

| Approach | Dependencies | Cross-session durability | Schema validation | Decision |
|----------|-------------|------------------------|-------------------|----------|
| **Versioned JSON files under `.vampiro/snapshots/`** | `serde_json` (already in tree) | Yes — on disk | Manual version field + serde | ✅ Selected |
| SQLite database | `rusqlite` (new dep) | Yes | DB schema | ❌ Rejected |
| In-memory only (ScanCache pattern) | None | No — lost on exit | N/A | ❌ Rejected |
| Git notes | Git plumbing | Yes | N/A | ❌ Rejected |
| Flatbuffers / protobuf | New build deps, codegen | Yes | Schema compilation | ❌ Rejected |

**Decision:** Versioned JSON files on disk at `.vampiro/snapshots/`.

**Rationale:**
- Vampiro already depends on `serde` + `serde_json` (via genesis-vibes and
  its own serialization); no new dependency required.
- JSON is inspectable, debuggable, and trivially readable by external tools
  and test assertions.
- A version field embedded in each snapshot file provides schema evolution
  without a database migration framework.
- The `.vampiro/` directory is already the convention for project-local
  config (`config.toml`); snapshots live in a sibling subdirectory at the
  same scope.
- SQLite adds a non-trivial dependency and build-time complexity (vcpkg,
  bundled compilation) for a simple key-value lookup. Revisit if the
  snapshot set exceeds ~10,000 entries in practice.
- In-memory-only would lose history between invocations, defeating the
  purpose of cross-version facade comparison.
- Git notes are invisible to most tooling and fragile under GC.
- Flatbuffers / protobuf add build toolchain complexity for no serialization
  performance benefit at Vampiro's snapshot sizes.

### File layout

```
.vampiro/snapshots/
  v0.1.0/
    <commit-sha>.json
    ...
```

The `<commit-sha>` is the full 40-character hex SHA-1 of the analyzed commit.
The `v0.1.0/` subdirectory is the snapshot *schema* version — this permits
multiple schema versions to coexist during migration windows.

---

## 2. Schema version

**Snapshot schema version string:** `"0.1.0"` — follows the same scheme as
`LIFECYCLE_FACT_SCHEMA_VERSION` (`"0.1.0"`).

**Versioning policy:**
- **Patch bump** (`0.1.0` → `0.1.1`): additive-only changes — new optional
  fields, new diagnostic categories. Existing readers MUST tolerate unknown
  fields via `#[serde(deny_unknown_fields)]` is NOT set on the snapshot
  struct; unknown fields are silently ignored on read.
- **Minor bump** (`0.1.x` → `0.2.y`): backward-compatible changes — old
  readers can still read new files by ignoring unrecognized fields.
  Writers MUST write the current version string.
- **Major bump** (`0.x.z` → `1.0.0`): breaking change — old readers MUST
  reject files with an incompatible major version. A migration reader
  (Section 5) converts old snapshots.

**Trigger to bump:** A breaking change in the facade item identity scheme,
  the snapshot key structure, or the serialization wire format that would
  silently misread data.

---

## 3. Retention policy

| Aspect | Decision |
|--------|----------|
| Default | Keep all snapshots indefinitely |
| Re-analysis | If a snapshot already exists for a commit SHA, overwrite it |
| Explicit prune | `vampiro prune-snapshots --older-than <duration>` or `--keep <n>` |
| GC safety | Never delete a snapshot that is the nearest ancestor of another snapshot |

**Rationale:**
- Snapshots are small (one JSON file per analyzed commit, containing L4
  facade items). A typical repo with 10,000 commits averages ~100 MB of
  snapshot data — negligible on modern storage.
- Overwrite-on-re-analysis ensures fresh snapshots reflect the current
  extraction logic without manual cleanup.
- Pruning is an explicit opt-in command, not automatic, to avoid silently
  breaking a future `--baseline` reference.

---

## 4. Baseline override

**CLI flag:** `--baseline <ref>` on the `vampiro check` or
`vampiro history` command.

### Semantics

- `<ref>` is any revision resolvable by `git rev-parse` (SHA, branch name,
  tag, `HEAD~n`, etc.).
- The resolved commit MUST be an ancestor of the analyzed target commit.
- If the resolved commit has no snapshot file, the tool SHALL emit an
  operational error:
  ```
  error: no snapshot found for baseline <resolved-sha> (from "<ref>")
  hint: run `vampiro check` on that revision first, or omit --baseline
        to use the default nearest-ancestor heuristic.
  ```
- If the resolved commit is NOT an ancestor, the tool SHALL emit:
  ```
  error: baseline <resolved-sha> (from "<ref>") is not an ancestor of target <target-sha>
  ```
  Per REQ-T1, this is an operational error, NOT a fallback to a different baseline.

### Default behavior (no `--baseline`)

Walk the first-parent lineage of the target commit. For each ancestor in
order, check for a snapshot file in `.vampiro/snapshots/v0.1.0/<sha>.json`.
Use the first one found. If none found on the entire first-parent chain
(including the root commit), emit no breaking-edge findings for this run
(persist the snapshot as a new baseline).

---

## 5. Migration declaration

**Syntax:** Config file entries under a `[migrations]` key in
`.vampiro/config.toml`.

```toml
[migrations]
# Each key is a stable migration ID; value describes the breaking change
# and the affected facade items. Multiple items per migration are listed
# under the same ID.
breaking-change-v0.2.0 = """
  charge(Money) -> Receipt changed to charge(Int) -> Receipt
  to adopt integer-based pricing throughout the codebase.
"""
```

### Rules

- A migration entry MUST reference specific facade items by their qualified
  identity (e.g. `pricing::Tier::charge`).
- A migration entry is considered an authorized exemption for REQ-T4:
  a breaking edge covered by a declared migration emits no finding.
- Migration IDs SHOULD follow a stable naming convention
  (`breaking-change-<version>` or `<ticket-id>`).
- Multiple migration entries for the same facade item at the same snapshot
  pair are rejected as a configuration error.

**Schema:** Add a `migrations` field of type `HashMap<String, String>` to
the `Config` struct, plus a `validate` check for duplicate item references.

---

## 6. Migration-reader policy

| Rule | Decision |
|------|----------|
| Same major version | Read all snapshot versions with the same major component (`0.x.y`) |
| Higher minor | Tolerate — unknown fields are silently ignored |
| Higher patch | Tolerate — always safe |
| Different major | Reject with a clear error; point to a migration reader |
| Fallback | If the current reader cannot parse a snapshot, try the previous minor reader |

**Implementation in Rust:**

```rust
pub struct SnapshotReader {
    schema_version: String,
}

impl SnapshotReader {
    /// True if this reader can read a snapshot with the given schema version.
    pub fn can_read(&self, version: &str) -> bool {
        let current_major = self.major_version();
        let other_major = Self::parse_major(version);
        current_major == other_major
    }

    fn major_version(&self) -> u32 {
        Self::parse_major(&self.schema_version)
    }

    fn parse_major(v: &str) -> u32 {
        v.split('.').next().and_then(|s| s.parse().ok()).unwrap_or(0)
    }
}
```

---

## 7. Initial write and resource idiom sets

### Write idioms (for retry idempotency classification, REQ-T2)

Idempotency classification is derived from a **write-shape idiom table**,
matching the mechanism of REQ-3's effect idiom table.

**Initial v0.1.0 write-shape idiom table:**

| Pattern | Classification | Notes |
|---------|---------------|-------|
| `INSERT INTO ...` (SQL) | `non-idempotent` | Plain insert — no dedupe |
| `INSERT ... ON CONFLICT ...` | `idempotent` | Upsert — idempotent |
| `UPDATE ... SET ... WHERE pk = ?` | `idempotent` | Idempotent by key |
| `DELETE FROM ... WHERE pk = ?` | `idempotent` | Idempotent by key |
| `PUT` (HTTP) | `idempotent` | Full replacement |
| `PATCH` (HTTP) | `non-idempotent` | Merge — not guaranteed idempotent |
| `fs::write` | `non-idempotent` | Plain file write |
| `fs::OpenOptions::append` | `non-idempotent` | Append — not idempotent |
| `HashMap::insert` / `BTreeMap::insert` | `idempotent` | Key-based overwrite |
| `Vec::push` / list append | `non-idempotent` | Append — not idempotent |
| Unknown / no match | `unknown` | Coverage diagnostic (REQ-T9) |

The table is versioned with a `WRITE_IDIOM_SCHEMA_VERSION = "0.1.0"`
and validated via conformance fixtures (same mechanism as REQ-6).

### Resource idioms (for resource linearity, REQ-T3)

The initial set of recognized resource types and acquisition events is
based on the existing `RESOURCE_TYPES` table in
`crates/vampiro-rust-frontend/src/lifecycle.rs`:

**Resource types** (from `RESOURCE_TYPES`):

| Type | Kind |
|------|------|
| `File` / `fs::File` / `std::fs::File` | `file` |
| `TcpStream` | `socket` |
| `TcpListener` | `socket` |
| `UdpSocket` | `socket` |
| `Mutex` | `lock` |
| `RwLock` | `lock` |
| `MutexGuard` / `RwLockReadGuard` / `RwLockWriteGuard` | `lock-guard` |
| `Barrier` | `barrier` |
| `Condvar` | `condvar` |
| `mpsc::Sender` / `mpsc::Receiver` | `channel` |
| `Arc` / `Rc` | `ref-count` |
| `Box` / `String` / `Vec` / `HashMap` / `BTreeMap` | `heap-alloc` |
| `PathBuf` | `path` |
| `Cursor` | `cursor` |
| `BufReader` / `BufWriter` | `buffered-io` |

**Acquisition events** (from `is_resource_acquisition`):
`open`, `create`, `new`, `connect`, `bind`, `listen`, `accept`, `lock`,
`try_lock`, `write`, `read`.

**Release events** (initial set):
`close`, `drop`, `unlock`, `release`, `shutdown`, `disconnect`.

**Transfer events** (initial set):
`move`, `into`, `as_ref`, `as_mut`, `clone` (if source is consumed
or obligation is explicitly annotated).

The resource idiom table is versioned (`RESOURCE_IDIOM_SCHEMA_VERSION = "0.1.0"`)
and validated via conformance fixtures — same mechanism as REQ-6 for
effect idiom tables.

---

## 8. Supported idioms and exclusions

**Supported lifecycle idioms (v0.1.0):**

| Category | Idioms | Evidence type |
|----------|--------|-------------|
| Snapshot persistence | File-level JSON write/read (serde) | Integration test |
| Baseline resolution | Git first-parent walk | Unit test |
| Write classification | Pattern-based (keyword, method name) | Conformance fixture |
| Resource identity | Allocation-site + handle alias graph | Unit test |
| Exit path enumeration | Scope-exit AST visitor | Conformance fixture |

**Explicit exclusions (not supported in v0.1.0):**

- **Heuristic rename detection** — facade items renamed without a declared
  alias are reported as `identity:ambiguous` (REQ-T8), never matched heuristically.
- **N+1 query detection** — see Addendum T.0 of the EARS spec.
- **Lock-ordering consistency** — see Addendum T.0.
- **Multithreaded resource sharing** — ownership transfer across threads is
  treated conservatively as `identity:unknown` unless an explicit transfer
  annotation is present.
- **Cross-language lifecycle tracking** — lifecycle facts are per-language;
  cross-language resource handoff is `identity:unknown`.

---

## 9. Immutable review reference

This decision was reviewed against:
- EARS specification v1.3.0 (Approved 2026-07-28)
- Addendum T (Facade Evolution, Retry Idempotency, Resource Linearity)
- Design doc: `openspec/changes/add-lifecycle-safety-analysis/design.md`
- Existing code: `crates/vampiro-rust-frontend/src/lifecycle.rs`
- Existing pattern: `crates/vampiro-cli/src/config.rs` (`.vampiro/` convention)
- Existing pattern: `crates/vampiro-cli/src/scan/mod.rs` (cache key versioning)

All decisions above are recorded and can be revisited via a new HITL gate
ticket when a trigger condition is met.