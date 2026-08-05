# CLI Contract

> Approved decisions for the `vampiro` CLI: configuration, exit codes, and command families.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.1.1 — CLI contract review
**Date:** 2026-07-27

---

## 1. Configuration

### Discovery and precedence

| Order | Source | Example |
|-------|--------|---------|
| 1 (highest) | CLI flags | `--config path`, `--verbose` |
| 2 | Project-local config | `./.vampiro/config.toml` (relative to CWD) |
| 3 | XDG config | `~/.config/vampiro/config.toml` |
| 4 (fallback) | Built-in defaults | Hard-coded sensible defaults |

### Filename and format

- **File:** `config.toml`
- **Format:** TOML
- **Case:** Lowercase, kebab-case keys

### Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| `vampiro.toml` in CWD | Pollutes project root; `.vampiro/` is consistent with other tools (`.beads/`, `.wai/`, `.git/`) |
| YAML / JSON | Inconsistent with every tool on this system — all use TOML |
| Environment-variable-only | Limits discoverability and composability |

---

## 2. Exit codes

| Code | Scenario | Notes |
|------|----------|-------|
| `0` | Success | All checks passed, or help/version displayed |
| `1` | Invalid config | Config file not found, unreadable, or malformed |
| `2` | Usage error | Unknown flag, unknown command, missing required argument |
| `3` | Policy failure | `check` found violations above the accept threshold |
| `4` | Internal error | I/O failure, panic, unexpected runtime condition |

Consistent with the predominant pattern in the toolchain (`wai`, `jj`, `just` use `2` for usage errors; `bd` and `chezmoi` use `1`).

### Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| `1` for all errors | Loses diagnostic granularity |
| sysexits(3) (`EX_USAGE=64`, `EX_CONFIG=78`) | Non-standard; most CLI tools don't follow this |
| Exit code 0 for help/version | Consistent with `bd`, `wai`, `jj`, `just`, `chezmoi` — all return 0 for `--help` |

---

## 3. Command families

| Family | Status | Notes |
|--------|--------|-------|
| `vampiro check` | Reserved | Not yet implemented. Shows help stub. |
| `vampiro prove` | Reserved | Not yet implemented. Shows help stub. |

Both families are reserved by the CLI foundation. The `scan` and `gating` proposal owns CI generation and its spelling.

---

## 4. Scope and compatibility

- **Supported scope:** All scenarios listed above.
- **Compatibility:** The exit-code contract is stable. Adding a new exit code requires a minor version bump. Changing existing exit codes requires a major version bump.
- **Immutability:** The contract is reviewed and approved before any implementation begins. Once approved, changes require a new proposal.