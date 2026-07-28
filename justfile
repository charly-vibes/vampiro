# vampiro justfile - unified local/CI workflow
#
# Same commands run locally and in CI for consistent diagnostics.
# Run `just` for default (build + test), `just ci` for full pipeline.

set shell := ["bash", "-uc"]

# Default: list commands
default:
    @just --list

# === Build Commands ===

# Build debug binary
build:
    cargo build

# Build release binary (optimized)
build-release:
    cargo build --release

# Run with arguments
run *args:
    cargo run -- {{args}}

# Install locally to ~/.cargo/bin
install:
    cargo install --path .

# === Test Commands ===

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run a specific test
test-one name:
    cargo test {{name}} -- --nocapture

# === Lint Commands ===

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Format all Rust files
fmt:
    cargo fmt

# Check formatting (no changes)
fmt-check:
    cargo fmt -- --check

# === CI Commands ===

# Full CI pipeline
ci: fmt-check lint test build-release
    @echo "✅ CI pipeline passed"

# Pre-push checks (fast gate)
pre-push: fmt-check lint test
    @echo "✅ Pre-push checks passed"

# === Setup Commands ===

# Setup development environment
setup:
    @echo "Checking Rust installation..."
    rustc --version
    cargo --version
    @echo ""
    @echo "Installing dev tools..."
    rustup component add clippy rustfmt
    @echo ""
    @echo "Installing lefthook..."
    @command -v lefthook >/dev/null 2>&1 || cargo install lefthook
    lefthook install
    @echo ""
    @echo "✅ Development environment ready"
    @echo "Run 'just test' to verify setup"

# === Workflow Commands ===

# Check wai workspace health
doctor:
    wai doctor

# Sync wai agent configs
sync:
    wai sync --yes

# Orient at session start
prime:
    wai prime

# Find available work
ready:
    bd ready

# === Docs Commands ===

# Build docs locally (requires mdbook)
docs:
    python scripts/build_docs.py && mdbook build

# Live preview docs
docs-serve:
    mdbook serve docs/src

# === Planning Commands ===

# Validate OpenSpec
validate:
    openspec validate --all --strict --no-interactive

# Validate planning graph
check-planning:
    python scripts/check_planning.py

# Full planning check
check: validate check-planning

# === Utility Commands ===

# Clean build artifacts
clean:
    cargo clean

# Check without building (faster feedback)
cargo-check:
    cargo check

# Dogfood: run vampiro check on own source workspace
dogfood:
    cargo run -- check --path crates/

# Update dependencies
update:
    cargo update

# Run wai reflect
reflect:
    wai reflect