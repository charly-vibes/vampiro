## 0. Configuration and Exit Decision Gate (HITL)
- [x] 0.1 Compare configuration filename/format/discovery/precedence and numeric exit-code alternatives against help, precedence, invalid-config, policy-failure, and operational-failure examples; record the choice in `docs/decisions/cli-contract.md`.
- [x] 0.2 Record supported scope, rejected alternatives, compatibility implications, approver, and immutable review reference before implementation begins.

## 1. Executable Workspace Tracer
- [x] 1.1 Add failing help/version snapshots for the reserved `check` and `prove` families without claiming analysis or proof behavior.
- [x] 1.2 Create the stable-toolchain Cargo workspace, thin `vampiro` binary, and library boundaries needed to satisfy those snapshots.
- [x] 1.3 Run the focused CLI snapshot tests, `cargo test --workspace`, `cargo fmt --check`, and Clippy with warnings denied; attach the command output to the ticket.

## 2. Configuration and Exit Tracer
- [x] 2.1 Add failing discovery/precedence/invalid-config and exact success/policy/operational exit-code tests from the approved decision (REQ-4).
- [x] 2.2 Implement only configuration loading, parsed command transport, and neutral exit-code types; do not add analysis or gating behavior.
- [x] 2.3 Run `cargo test --workspace rust_cli_foundation_2` and workspace quality commands; publish `cli-config-exit/v1` at `tests/contracts/cli/config-exit-v1.json` and record evidence in `docs/verification/add-rust-cli-foundation-2.md`.

## 3. Finding Envelope Tracer
- [x] 3.1 Add failing construction/serialization-boundary tests requiring rule, path, exact line range, configured severity, exactly one axis, and optional independent `filtration_distance = sev(e)`.
- [x] 3.2 Implement the shared finding envelope and empty successor adapters without reporting, law, lifecycle, or scan behavior.
- [x] 3.3 Run `cargo test --workspace rust_cli_foundation_3`; publish `finding-envelope/v1` at `tests/contracts/findings/envelope-v1.json`, verify REQ-4 ownership plus REQ-5/REQ-12 references, and record evidence in `docs/verification/add-rust-cli-foundation-3.md`.

## 4. Foundation Acceptance
- [ ] 4.1 Run unit, integration, doc, and command snapshot suites plus workspace formatting and Clippy.
- [ ] 4.2 Run `cargo test --workspace rust_cli_foundation_4` to verify successor compatibility with `cli-config-exit/v1` and `finding-envelope/v1`; record the report in `docs/verification/add-rust-cli-foundation-4.md`.
- [ ] 4.3 Run `openspec validate add-rust-cli-foundation --strict` and confirm no analysis, serialization, proof, CI-generation, or gating behavior is claimed.
