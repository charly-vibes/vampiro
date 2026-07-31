Adopt ALL genesis v0.4.0 modules across vampiro codebase

## Phase 1: Bump deps & adopt missing CLI modules (vampiro-cli)
- Add genesis::cli (generate_completions, maybe_print_version_json)
- Add genesis::feedback (redactor, scratch, gh)
- Add genesis::suggestions (SuggestionEngine for error suggestions)
- Add genesis::scaffold (Scaffold for init)
- Add genesis::discovery (.genesis/tools.toml manifest)
- Remove local reimplementations of the above

## Phase 2: Test infrastructure adoption
- Add genesis::fixture to vampiro-cli (replace tempfile for test envs)
- Route test fixtures in other subcrates through genesis::fixture where applicable

## Phase 3: Cleanup and verification
- Remove dead code / duplicated modules
- Update llms.txt/llm.txt to reflect genesis adoption
- Verify: cargo build, cargo test, cargo clippy, cargo fmt
