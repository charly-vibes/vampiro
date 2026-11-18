#!/usr/bin/env python3
"""Ratchet gate over `ah check` for vampiro's legacy spectular corpus.

vampiro's `openspec/specs` tree predates the current contract requirements: it
was authored under espectacular 0.11.0 (`.espectacular/config.toml`
`tool_version`) and its contract stubs carry `authored_with = "0.11.0"` with no
`[tests]` table. Under ah 0.12.3 `ah check` therefore reports a fixed backlog
of 84 structural findings:

- `no-tests-declared` — a contract exists but declares no verifying test
- `no-toml` — a scenario has no contract at all

Silencing them by declaring test filters that match nothing would be worse than
the backlog: those entries fail at execution time (`no-tests-ran`), and it
fabricates verification that does not exist. The backlog is real and has to be
paid down scenario by scenario.

This wrapper keeps the gate hard without hiding drift:

- `no-tests-declared` and `no-toml` findings are allowed ONLY for scenario ids
  listed in scripts/ah-undeclared.txt (each line names its owning tickets).
- Any other finding (orphan-toml, slug-collision, id-mismatch,
  malformed-contract, execution failures, ...) fails the gate.
- A new backlog finding fails: declaring a scenario creates backlog, and
  backlog must be recorded in the same change.
- A stale baseline line fails (the contract/tests are now declared, or the
  scenario is gone): the ratchet only turns one way.

Exit 0 when the observed backlog equals the declared one and everything else
is clean; exit 1 otherwise.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

BASELINE_REL = "scripts/ah-undeclared.txt"
# Backlog kinds: a scenario is declared in the spec, its verification surface
# is not. Every other finding kind is a defect and is never baseline-able.
BACKLOG_KINDS = frozenset({"no-toml", "no-tests-declared"})


def repo_root() -> Path:
    proc = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    )
    return Path(proc.stdout.strip()) if proc.returncode == 0 else Path.cwd()


def load_baseline(path: Path) -> dict[str, str]:
    """Map `spec/scenario-id` -> owning-ticket comment (may be empty)."""
    entries: dict[str, str] = {}
    if not path.exists():
        return entries
    for raw in path.read_text(encoding="utf-8").splitlines():
        body, _, comment = raw.partition("#")
        key = body.strip()
        if key:
            entries[key] = comment.strip()
    return entries


def key_of(finding: dict) -> str:
    return f"{finding['spec']}/{finding['scenario']['id']}"


def load_report(root: Path) -> dict | None:
    """Run `ah check --json` (extra argv passed through) and parse its report."""
    try:
        proc = subprocess.run(
            ["ah", "check", "--json", *sys.argv[1:]],
            cwd=root,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        print(
            "ah-check-ratchet: `ah` not found on PATH — the spec gate cannot run",
            file=sys.stderr,
        )
        return None
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        sys.stdout.write(proc.stdout)
        sys.stderr.write(proc.stderr)
        print(
            f"ah-check-ratchet: `ah check --json` produced no parsable report "
            f"(exit {proc.returncode})",
            file=sys.stderr,
        )
        return None


def classify(findings: list[dict]) -> tuple[dict[str, dict], list[dict]]:
    """Split findings into baseline-keyed backlog entries and everything else."""
    backlog: dict[str, dict] = {}
    other: list[dict] = []
    for finding in findings:
        if finding.get("kind") in BACKLOG_KINDS:
            backlog[key_of(finding)] = finding
        else:
            other.append(finding)
    return backlog, other


def report_failure(
    baseline: dict[str, str], backlog: dict[str, dict], other: list[dict]
) -> int:
    """Print the drift: out-of-baseline findings, new keys, stale keys."""
    new = sorted(set(backlog) - set(baseline))
    stale = sorted(set(baseline) - set(backlog))
    print(f"ah-check-ratchet: FAILED ({BASELINE_REL})")
    if other:
        print(f"\n{len(other)} finding(s) outside the declared backlog:")
        for finding in other:
            print(
                f"  [{finding.get('category', '?')}] {finding.get('kind')}: "
                f"{key_of(finding)}"
            )
            playbook = finding.get("playbook_command")
            if playbook:
                print(f"      fix path: {playbook}")
    if new:
        print(
            f"\n{len(new)} backlog finding(s) not in the baseline. "
            "A new spec-first scenario must be declared with its owning ticket:"
        )
        for key in new:
            print(f"  + {key} [{backlog[key].get('kind')}]")
    if stale:
        print(
            f"\n{len(stale)} stale baseline line(s): the scenario is now declared "
            "or gone. Remove these lines (the ratchet only turns one way):"
        )
        for key in stale:
            owner = baseline[key]
            suffix = f"  # {owner}" if owner else ""
            print(f"  - {key}{suffix}")
    return 1


def main() -> int:
    root = repo_root()
    baseline = load_baseline(root / BASELINE_REL)
    report = load_report(root)
    if report is None:
        return 1

    data = report.get("data") or {}
    summary = data.get("summary") or {}
    if report.get("ok"):
        print(
            "ah-check-ratchet: OK — "
            f"{summary.get('passed', 0)} passed, 0 findings, "
            f"{len(baseline)} declared-backlog scenario(s)"
        )
        return 0

    backlog, other = classify(data.get("findings") or [])
    if other or set(backlog) != set(baseline):
        return report_failure(baseline, backlog, other)
    print(
        "ah-check-ratchet: OK — "
        f"{summary.get('passed', 0)} passed, {len(backlog)} declared-backlog "
        "scenario(s), no other findings"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())