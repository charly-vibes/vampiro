# Verification: Policy and CI generation (Section 3)

**Date:** 2026-07-28
**Tickets:** vampiro-0vb.5.4
**Contracts:**
- ScanPolicy — mode-based exit code evaluation (REQ-13, REQ-14)
- FiltrationMapRule — filtration_distance to severity mapping (REQ-C2)
- CI generation — GitHub Actions workflow template (REQ-20)

## Test results

```bash
$ cargo test --test scan_gating_reporting_3
# 18 tests, 18 passed

$ cargo test --lib -p vampiro -- policy::tests
# 21 tests, 21 passed

$ cargo test --workspace
# all passed

$ cargo fmt --check
# clean

$ cargo clippy --workspace --all-targets -- -D warnings
# clean

$ openspec validate add-scan-gating-reporting --strict
# valid
```

## Evidence

### Scenario coverage (3.1)

| Scenario | Test | Status |
|----------|------|--------|
| guidance mode passes with findings | `test_3_1_guidance_passes_with_findings` | ✅ |
| tiered mode passes with findings | `test_3_1_tiered_passes_with_findings` | ✅ |
| gate below threshold passes | `test_3_1_gate_below_threshold_passes` | ✅ |
| gate equal threshold fails | `test_3_1_gate_equal_threshold_fails` | ✅ |
| gate above threshold fails | `test_3_1_gate_above_threshold_fails` | ✅ |
| empty findings in gate passes | `test_3_1_empty_findings_in_gate_passes` | ✅ |
| multiple findings, gate fails on highest | `test_3_1_multiple_findings_gate_fails_on_highest` | ✅ |
| multiple findings, gate passes below | `test_3_1_multiple_findings_gate_passes_below` | ✅ |
| valid filtration map | `test_3_1_valid_filtration_map_passes` | ✅ |
| invalid filtration map (duplicates) | `test_3_1_invalid_filtration_map_duplicate_conditions` | ✅ |
| filtration maps severity down | `test_3_1_filtration_maps_severity_down` | ✅ |
| filtration maps severity up | `test_3_1_filtration_maps_severity_up` | ✅ |
| CI golden — head/base variables | `test_3_1_ci_golden_github_actions_includes_head_base` | ✅ |
| CI golden — install step | `test_3_1_ci_golden_includes_install_step` | ✅ |
| CI golden — fetch fallback for non-PR | `test_3_1_ci_golden_failed_fetch_handled` | ✅ |
| CI golden — valid YAML structure | `test_3_1_ci_golden_valid_yaml_syntax` | ✅ |
| CI golden — respects threshold config | `test_3_1_ci_golden_respects_severity_threshold` | ✅ |
| CI golden — structural validity | `test_3_1_ci_golden_structural_validity` | ✅ |

### Edge cases covered

- `condition_gte_matches` — `>=` operators at/above/below threshold
- `condition_lte_matches` — `<=` operators
- `condition_eq_matches` — exact match
- `condition_neq_matches` — not-equal
- `condition_gt_matches` — strict greater-than
- `condition_lt_matches` — strict less-than
- Empty filtration map → invalid
- No filtration mapping → base severity used
- No matching rule → base severity used

### CI golden path

Generated workflow (GitHub Actions):
```yaml
name: Vampiro Scan
on: pull_request / push branches: [main]
steps:
  - actions/checkout@v4 with fetch-depth: 0
  - cargo install vampiro
  - vampiro check --target ${{ github.event.pull_request.head.sha || github.sha }}
                  --base ${{ github.event.pull_request.base.sha || github.event.before }}
                  --mode gate --severity-threshold <config>
```

The dual-variable expression handles both PR and push events. The `--severity-threshold` value is sourced from the ScanPolicy configuration.