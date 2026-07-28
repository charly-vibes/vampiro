> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## Context
This greenfield change establishes contracts consumed by every later change.

## Goals / Non-Goals
- Goals: reserved `check`/`prove` families, deterministic configuration precedence, shared finding fields including optional `filtration_distance`, documented numeric exits.
- Non-goals: real source analysis, finding serialization, thresholds, CI content, or prover execution.

## Decisions
- Use a Cargo workspace with a thin `vampiro` binary and library-owned contracts so later capabilities do not depend on CLI parsing.
- Before implementation, select/document configuration filename, format, discovery, and precedence, and exact numeric success, policy-failure, and operational/configuration-failure codes; snapshot them before implementation.
- Do not reserve a CI-generation subcommand; `scan-workflows` specifies that integration.

## Risks / Trade-offs
- Premature command details can constrain later work; keep subcommand payloads minimal and version shared envelopes.
- Configuration drift can break automation; snapshot the parsed command/config contract.

## Decision Gate
- A HITL decision ticket SHALL record configuration and numeric-exit choices,
  alternatives, compatibility impact, approver, and immutable review reference
  before implementation tickets become actionable.

## Deferred: tree-sitter front-end and CIR extraction

**Status: deferred — document the trigger, do not extract now.**

vampiro's project.md commits to a plugin architecture: *"Keep checking logic
language-independent and operate on CIR rather than source ASTs. A new
language should add a plugin, not branch the core engine."* crua and livin
are designed to consume vampiro's CIR via `--json` output, not by linking
vampiro's crate (tool-craft §7: *"No tool reads another's internal store;
it reads the other's `--json` output or its public signal files"*).

This means three things for extraction:

1. **CIR as a shared Rust crate is premature.** crua and livin consume CIR
   as JSON; a serialization schema (in `genesis` or a `cir-schema` repo)
   suffices until they need to manipulate CIR as Rust types internally.
2. **Tree-sitter front-ends are the strongest extraction candidate** — pure
   duplicated infra (4 languages × 3 tools = 12x reimplementations) with no
   domain logic. But extract only after vampiro ships ≥2 front-ends and the
   parser interface stabilizes.
3. **Analysis logic (seam detection, cost-pattern catalogue, boundary
   catalogue) is never extracted** — it is each tool's domain. The genesis
   boundary rule protects it.

### Extraction trigger (when to revisit)

Re-evaluate extraction when ALL of these are true:
- vampiro has shipped CIR v1 and ≥2 language front-ends.
- crua or livin is ready to implement and needs to parse the same
  tree-sitter grammars.
- Concrete front-end duplication is visible (the same grammar query files
  or parser wiring appearing in ≥2 tools).

### When triggered, what to extract

- **First:** tree-sitter front-ends (parser wiring + grammar queries) into
  a shared crate (working name `frontier`, or follow suite convention).
  This is pure infrastructure — exactly like genesis's `envelope`/
  `suggestions`/`managed_block`.
- **Maybe:** CIR Rust types (only if crua/livin need typed manipulation, not
  just JSON deserialization).
- **Never:** analysis logic (seams, cost patterns, boundary values).

Until the trigger fires, vampiro owns the front-ends and CIR; crua and livin
consume via `--json`. This is consistent with genesis's discipline: extract
on evidence of real duplication, not on anticipation of it.
