#!/usr/bin/env python3
"""Assemble the MkDocs source tree from authoritative project documents."""

from __future__ import annotations

import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BUILD_DOCS = ROOT / ".build" / "docs"

CHANGES = {
    "add-rust-cli-foundation": "cli-foundation",
    "add-cir-plugin-platform": "cir-plugin-platform",
    "add-rust-analysis-frontend": "rust-analysis",
    "add-core-seam-analysis": "seam-analysis",
    "add-scan-gating-reporting": "scan-workflows",
    "add-law-and-proof-verification": "law-verification",
    "add-lifecycle-safety-analysis": "lifecycle-safety",
    "add-python-clojure-julia-frontends": "additional-frontends",
}


def validate_source_inventory() -> None:
    changes_root = ROOT / "openspec" / "changes"
    discovered_changes = {
        path.name
        for path in changes_root.iterdir()
        if path.is_dir() and path.name != "archive"
    }
    declared_changes = set(CHANGES)
    if discovered_changes != declared_changes:
        raise ValueError(
            "documentation change inventory is stale; "
            f"missing: {sorted(discovered_changes - declared_changes)}; "
            f"removed: {sorted(declared_changes - discovered_changes)}"
        )

    for change_id, capability in CHANGES.items():
        source = changes_root / change_id
        discovered_specs = {
            path.relative_to(source)
            for path in (source / "specs").rglob("spec.md")
        }
        declared_specs = {Path("specs") / capability / "spec.md"}
        if discovered_specs != declared_specs:
            raise ValueError(
                f"documentation capability inventory is stale for {change_id}; "
                f"found: {sorted(map(str, discovered_specs))}; "
                f"expected: {sorted(map(str, declared_specs))}"
            )


def copy_with_notice(source: Path, destination: Path, notice: str) -> None:
    if not source.is_file():
        raise FileNotFoundError(f"required documentation source is missing: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(f"{notice}\n\n{source.read_text()}")


def build_roadmap_index() -> str:
    rows = []
    for change_id in CHANGES:
        tasks = (ROOT / "openspec" / "changes" / change_id / "tasks.md").read_text()
        completed = tasks.count("- [x]") + tasks.count("- [X]")
        pending = tasks.count("- [ ]")
        total = completed + pending
        rows.append(
            f"| [`{change_id}`]({change_id}/proposal.md) | Active proposal | "
            f"{completed}/{total} |"
        )
    return "\n".join(
        [
            "# Implementation roadmap",
            "",
            "> **Status:** These are active, unimplemented OpenSpec changes. They become built truth only after implementation, review, and archival.",
            "",
            "| Change | Status | Tasks complete |",
            "|---|---|---:|",
            *rows,
            "",
            "Each change page includes its proposal, capability delta, design, and approval-gated tracer-bullet task plan.",
            "",
        ]
    )


def main() -> None:
    validate_source_inventory()
    if BUILD_DOCS.exists():
        shutil.rmtree(BUILD_DOCS)
    shutil.copytree(ROOT / "docs", BUILD_DOCS)

    copy_with_notice(
        ROOT / "vampiro-ears-spec.md",
        BUILD_DOCS / "specification" / "ears.md",
        '> **Document status:** Draft 1.1.0, not yet approved. The source file `vampiro-ears-spec.md` is authoritative.',
    )
    copy_with_notice(
        ROOT / "openspec" / "project.md",
        BUILD_DOCS / "project-context.md",
        '> **Source:** Generated from `openspec/project.md` during the documentation build.',
    )

    roadmap = BUILD_DOCS / "roadmap"
    roadmap.mkdir(parents=True, exist_ok=True)
    (roadmap / "index.md").write_text(build_roadmap_index())

    notice = (
        "> **Status:** Active OpenSpec proposal; not implemented or deployed. "
        "The source under `openspec/changes/` is authoritative."
    )
    for change_id, capability in CHANGES.items():
        source = ROOT / "openspec" / "changes" / change_id
        destination = roadmap / change_id
        for name in ("proposal", "design", "tasks"):
            copy_with_notice(source / f"{name}.md", destination / f"{name}.md", notice)
        copy_with_notice(
            source / "specs" / capability / "spec.md",
            destination / "spec.md",
            notice,
        )

    print(f"assembled documentation sources in {BUILD_DOCS.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
