## Context
The CIR already records bounded callee-to-caller argument provenance. Trust
provenance answers a different question: whether a value originates outside a
declared trust domain and has passed through a recognized refinement step.
Conflating the two would make `unknown` behavior and evidence impossible to
interpret.

## Goals / Non-Goals
- Goals: explicit trust metadata, conservative propagation, recognized refined
  shapes, precise boundary findings, and reproducible external evidence import.
- Non-goals: executing analyzed source, inferring semantic equivalence from
  arbitrary syntax, generating boundary-value tests, or implementing the
  companion coverage tool.

## Decisions
- Add trust provenance with exactly `untrusted`, `trusted`, or `unknown` to CIR
  value occurrences (node parameters/outputs, sum arms, and edge argument
  slots); retain argument-provenance chains unchanged.
- Recognize trust-boundary sources and smart constructors only through
  versioned conformance idioms or explicit project declarations.
- Propagate trust provenance through CIR dataflow with the order
  `untrusted > unknown > trusted`: any untrusted contributor makes a derived
  occurrence untrusted; otherwise any unknown contributor makes it unknown;
  otherwise it is trusted. Recognized internal literals are trusted; a smart
  constructor's non-success payload follows the same contributor rule. A path
  over argument-provenance bound `H` or a declaration/idiom conflict is unknown.
  Any unmatched source, propagation, or refinement pattern remains `unknown`
  and emits `trust-provenance:unknown`.
- Keep trust classification independent from refinement confirmation: the
  constructor success arm can be trusted without external coverage evidence,
  while invalid-state reachability remains unconfirmed.
- Establish validation equivalence only through a stable declared validation
  identity or a conformance-tested idiom. Syntactic similarity alone emits no
  duplicate-validation finding. Frontends emit validation observations carrying
  identity, constructor/refined shape, source span, and recognition origin.
- Import boundary-coverage evidence through a versioned schema carrying
  producer identity/version, analyzed revision, constructor stable identity and
  source/shape hash, declared boundary-class IDs, and per-class results.
  Missing, mismatched, stale, incomplete, or failing evidence yields `unknown`.
- Keep boundary-coverage import optional. Without configured evidence, Vampiro
  can still emit boundary-leak and duplicate-validation findings but cannot use
  the constructor as evidence that invalid states are unreachable.
- Normalize imported evidence as `refinement_confirmation.status=confirmed` or
  `unknown`; unknown carries the first applicable reason in the ordered closed
  vocabulary. Empty/duplicate boundary-class declarations and unsupported
  evidence versions are unknown, never confirmed.
- Emit one boundary-leak finding per violating edge at default `HIGH`, and one
  validation-duplication finding per duplicate-check location at default `LOW`;
  both use the existing REQ-24 stable deduplication identity.

## Risks / Trade-offs
- Conservative unknown propagation may reduce coverage; visible uncertainty is
  preferable to a false trusted classification.
- Validation equivalence and boundary-class declarations add configuration;
  explicit identities make results reproducible and avoid unsound semantic
  inference.
- External evidence can become stale; revision and constructor hashes make
  freshness decidable rather than time-based.

## Decision Gate
A HITL decision ticket SHALL approve the initial trust-domain configuration,
Rust source/refinement idiom scope, validation-identity syntax, boundary-class
declaration syntax, value/arm representation, propagation/join and over-`H`
rules, declaration/idiom conflict behavior, default finding severities, and
boundary-coverage evidence schema/version policy before implementation. The
decision SHALL name the companion producer contract but SHALL NOT require
selecting or implementing a specific companion tool.
