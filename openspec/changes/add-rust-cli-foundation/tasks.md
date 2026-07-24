## 1. Contract Tests
- [ ] 1.1 Select/document configuration filename, format, discovery, precedence, and exact numeric exit codes (REQ-4).
- [ ] 1.2 Add failing snapshots freezing those decisions and reserved `check`/`prove` parsing; add precedence/error and finding-envelope/exit tests.

## 2. Foundation
- [ ] 2.1 Create the stable-toolchain Cargo workspace, library boundaries, and `vampiro` binary.
- [ ] 2.2 Implement command parsing and configuration loading only.
- [ ] 2.3 Implement the shared finding envelope and neutral exit-code types without later analysis/gating behavior.

## 3. Verification
- [ ] 3.1 Run Cargo unit/integration/doc tests and command snapshots.
- [ ] 3.2 Run rustfmt and Clippy under the established lint policy.
- [ ] 3.3 Verify REQ-4 ownership, REQ-5/REQ-12 conformance references, separate `filtration_distance = sev(e)` transport, and empty successor adapters without claiming behavior.
