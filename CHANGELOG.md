# Changelog

## [Unreleased]

### Changed
- Approved the authoritative EARS specification (v1.3.0): added REQ-30
  (tiered gate-mode behavior), a REQ-4 default-severity table, definitions of
  the argument-provenance bound `H` and `intentional branch`, a canonical
  effect-channel combination grammar, and an explicit ordering of the
  `refinement_confirmation` reason vocabulary. Status flipped Draft → Approved;
  unblocks the `add-trust-boundary-analysis` epic (REQ-B1–REQ-B6).

### Added
- Approval-gated `add-trust-boundary-analysis` OpenSpec change and six matching
  Beads tickets covering 17 implementation and verification checklist items.
- Approval-gated `depend-on-genesis` proposal for shared envelope, suggestions,
  managed-block, and AIX infrastructure.

## [0.0.0] — 2026-07-24

### Added
- Initial EARS specification (Draft 1.1.0).
- Eight OpenSpec change proposals for phased implementation.
- Project infrastructure: wai, openspec, beads, mdBook docs, CI/CD.
- Agent instructions with wai, openspec, and beads integration.
