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
