#!/usr/bin/env python3
"""Validate the exported Beads graph against the active OpenSpec task plans."""

from __future__ import annotations

import json
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKLIST_RE = re.compile(r"- \[[ xX]\] (\d+\.\d+ .+)")
SECTION_RE = re.compile(r"## (\d+)\.")


def load_issues() -> list[dict]:
    path = ROOT / ".beads" / "issues.jsonl"
    issues = [json.loads(line) for line in path.read_text().splitlines() if line]
    ids = [issue["id"] for issue in issues]
    if len(ids) != len(set(ids)):
        raise ValueError("issues.jsonl contains duplicate issue IDs")
    return issues


def blocking_dependencies(issue: dict) -> set[str]:
    return {
        dependency["depends_on_id"]
        for dependency in issue.get("dependencies", [])
        if dependency.get("type") == "blocks"
    }


def checklist_sections() -> dict[str, Counter[str]]:
    sections: dict[str, Counter[str]] = {}
    for path in (ROOT / "openspec" / "changes").glob("*/tasks.md"):
        section_id: str | None = None
        for line in path.read_text().splitlines():
            if match := SECTION_RE.match(line):
                section_id = f"{path.relative_to(ROOT)}#{match.group(1)}"
                sections[section_id] = Counter()
            elif match := CHECKLIST_RE.match(line):
                if section_id is None:
                    raise ValueError(f"checklist item precedes a section in {path}")
                sections[section_id][match.group(1)] += 1
    return sections


def main() -> None:
    issues = load_issues()
    by_id = {issue["id"]: issue for issue in issues}

    for issue in issues:
        dependencies = issue.get("dependencies", [])
        malformed = [
            dependency
            for dependency in dependencies
            if dependency.get("issue_id") != issue["id"]
            or not dependency.get("depends_on_id")
            or not dependency.get("type")
        ]
        if malformed:
            raise ValueError(f"{issue['id']} has malformed dependencies: {malformed}")
        unknown = {
            dependency["depends_on_id"] for dependency in dependencies
        } - by_id.keys()
        if unknown:
            raise ValueError(f"{issue['id']} has unknown blockers: {sorted(unknown)}")

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(issue_id: str) -> None:
        if issue_id in visiting:
            raise ValueError(f"dependency cycle includes {issue_id}")
        if issue_id in visited:
            return
        visiting.add(issue_id)
        for dependency in by_id[issue_id].get("dependencies", []):
            visit(dependency["depends_on_id"])
        visiting.remove(issue_id)
        visited.add(issue_id)

    for issue_id in by_id:
        visit(issue_id)

    expected_sections = checklist_sections()
    exported_sections: dict[str, Counter[str]] = {}
    for issue in issues:
        entries = [
            match.group(1)
            for line in issue.get("description", "").splitlines()
            if (match := CHECKLIST_RE.match(line))
        ]
        if not entries:
            continue
        spec_id = issue.get("spec_id")
        if spec_id not in expected_sections:
            raise ValueError(
                f"{issue['id']} has checklist entries with unknown spec_id: {spec_id!r}"
            )
        exported_sections.setdefault(spec_id, Counter()).update(entries)

    for spec_id in expected_sections.keys() | exported_sections.keys():
        expected = expected_sections.get(spec_id, Counter())
        exported = exported_sections.get(spec_id, Counter())
        if expected != exported:
            missing = list((expected - exported).elements())
            stale = list((exported - expected).elements())
            raise ValueError(
                f"exported checklist entries for {spec_id} do not match OpenSpec; "
                f"missing: {missing[:5]!r}; stale or duplicated: {stale[:5]!r}"
            )

    checklist_count = sum(sum(items.values()) for items in expected_sections.values())

    approval_gates = [
        issue for issue in issues if "approval-gate" in issue.get("labels", [])
    ]
    if len(approval_gates) != 1:
        raise ValueError("the roadmap must have exactly one approval gate")
    approval_id = approval_gates[0]["id"]

    def ancestors(issue_id: str, result: set[str] | None = None) -> set[str]:
        result = set() if result is None else result
        for dependency in blocking_dependencies(by_id[issue_id]):
            if dependency not in result:
                result.add(dependency)
                ancestors(dependency, result)
        return result

    implementation = [
        issue
        for issue in issues
        if issue["id"] != approval_id
        and issue.get("issue_type") in {"task", "decision"}
    ]
    unguarded = [
        issue["id"] for issue in implementation if approval_id not in ancestors(issue["id"])
    ]
    if unguarded:
        raise ValueError(f"implementation issues bypass approval: {unguarded}")

    print(
        f"validated {len(issues)} issues, {checklist_count} checklist items, "
        f"one acyclic approval-gated graph"
    )


if __name__ == "__main__":
    main()
