# Scan Policy Decision

> CI provider set and tiered-policy configuration for Vampiro's scan/gating/reporting workflows.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.5.1 — CI Provider and Tier Policy Decision Gate
**Date:** 2026-07-28

---

## 1. Provider choice

### Alternatives considered

| Provider | Supports head/base refs | Public API | Native Rust support | Decision |
|----------|------------------------|------------|-------------------|----------|
| **GitHub Actions** | `github.event.pull_request.head.sha`, `github.event.pull_request.base.sha` | REST + GraphQL | cargo install, action bindings | ✅ Selected |
| GitLab CI | `CI_MERGE_REQUEST_DIFF_BASE_SHA`, `CI_COMMIT_SHA` | REST API | cargo install works | ❌ Rejected |
| CircleCI | `CIRCLE_PULL_REQUEST`, needs API for base | REST API | cargo install works | ❌ Rejected |
| Woodpecker CI | `CI_COMMIT_SOURCE`, `CI_COMMIT_TARGET` | Limited | cargo install works | ❌ Rejected |

**Decision:** GitHub Actions.

**Rationale:**
- Vampiro is already published as `charly-vibes/vampiro` on GitHub — zero provider onboarding cost.
- Existing `.github/workflows/ci.yml` and `pages.yml` already establish the GitHub Actions pattern.
- All other providers require non-trivial CI migration, credential setup, and cross-provider testing for no current benefit.
- GitHub Actions exposes `github.event.pull_request.head.sha` and `github.event.pull_request.base.sha` natively for PR-head/base resolution without API calls.
- `github.ref_name` and `github.sha` cover push/merge-queue events.

**Trigger to revisit:** A downstream consumer requests support for an additional provider (GitLab CI, Woodpecker) at P1 priority or higher.

---

## 2. Event and ref variables

For the supported provider (GitHub Actions), the following variables drive scan scope resolution:

| Context | Target (`--target`) | Base (`--base`) | Notes |
|---------|--------------------|----------------|-------|
| PR (pull_request) | `github.event.pull_request.head.sha` | `github.event.pull_request.base.sha` | Both are immutable commit SHAs |
| PR (pull_request_target) | `github.event.pull_request.head.sha` | `github.event.pull_request.base.sha` | Head may have restricted token. Same variables |
| Push to branch | `github.sha` | `github.event.before` | `before` is the previous commit, `sha` is the new one |
| Merge queue | `github.sha` | `github.event.merge_group.base_sha` | GitHub provides the merge group refs |
| Manual (workflow_dispatch) | `github.sha` | — | No base; falls back to empty-tree diff |

---

## 3. Tier semantics

Three modes as specified in the scan-workflows spec:

| Mode | Behavior | Exit code |
|------|----------|-----------|
| `guidance` | Reports all findings. Never fails due to findings. | 0 |
| `tiered` | Classifies findings into configured reporting tiers. | 0 (findings reported by tier) |
| `gate` | Exits non-zero only when a seam-scoped finding ≥ configured severity threshold. Ignores `filtration_distance` unless a validated mapping is configured. | 0 if all findings < threshold; non-zero if any ≥ threshold |

**Policy configuration shape** (TOML, in `.vampiro/config.toml`):

```toml
[scan]
mode = "gate"
severity-threshold = "warning"

# Optional filtration_distance mapping
[filtration]
[scan.filtration-map]
"< = 2" = "warning"
"= 0" = "error"
```

**Why TOML?** Consistent with genesis::config format already adopted. No new config format.

---

## 4. Exclusions

- Self-hosted / on-premise runners are **not** supported in the initial provider set. The generated workflow assumes `ubuntu-latest` (GitHub-hosted).
- Non-GitHub CI providers (GitLab, Woodpecker, Jenkins, etc.) are **excluded** from the initial implementation scope.
- GitHub Enterprise Server is **excluded** — API variable differences require per-version mapping.
- Monorepo partial-checkout strategies (sparse checkout, subdirectory-only) are **not** handled in v1 — the generated workflow assumes full checkout.

---

## 5. Alternative configuration formats

| Format | Rationale | Decision |
|--------|-----------|----------|
| **TOML** (in config.toml) | Already adopted via genesis::config; single config file | ✅ Selected |
| YAML (dedicated file) | Over-engineered for 3-mode policy; another file to discover | ❌ Rejected |
| CLI flags only | No persistence; every invocation would need `--mode=gate` | ❌ Rejected |
| Environment variables | No persistence; invisible to CI-generated workflows | ❌ Rejected |

---

## 6. PR-head/base resolution failure scenarios

| Scenario | Behavior |
|----------|----------|
| PR head commit is force-pushed away | The `head.sha` resolves to the current (correct) head. No issue. |
| Base branch is deleted | `base.sha` resolves to the commit that was the base at PR creation; `git merge-base` fails. → Operational error with guidance to use `--full`. |
| Shallow clone missing merge base | `git merge-base` returns error. → Operational error; no silent full fallback. |
| Workflow runs from fork without fetch | `actions/checkout@v4` with `fetch-depth: 0` fetches everything; if base is unavailable, → Operational error. |
| Non-PR event | Base defaults to first parent or empty tree; no resolution required. |

---

## 7. Valid and invalid `filtration_distance` mapping examples

| Mapping | Valid? | Reason |
|---------|--------|--------|
| `sev(e) >= 2 → "warning"` | ✅ Total, deterministic, schema-valid | Key is a comparand against `sev(e)` |
| `sev(e) >= 0 → "error"` | ✅ | Bounds-checked, covers all inputs |
| `sev(e) < 2 → "warning"`, `sev(e) >= 2 → "error"` | ✅ | Covers all inputs with non-overlapping cases |
| `sev(e) < 2 → "warning"` | ❌ Partial — missing `sev(e) >= 2` case | Gate would silently default without full coverage |
| `sev(e) ≈ 2 → "warning"` | ❌ Nondeterministic | Approximation is not a stable comparand |
| `sev(e) >= 2 → "info"` | ✅ But likely a configuration error | Schema-valid, user's choice |

---

## 8. Scope and compatibility

- **Supported provider:** GitHub Actions (ubuntu-latest, `actions/checkout@v4+`).
- **Events covered:** `pull_request`, `push`, `merge_group`, `workflow_dispatch`.
- **Config format:** TOML via `.vampiro/config.toml` (single `[scan]` section, plus optional `[scan.filtration-map]`).
- **Immutability:** This decision is valid until a P1 provider request triggers re-evaluation. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.