# DecideRS justfile

# Show available commands
default:
    @just --list

# Check if code compiles
[group('build')]
check:
    cargo check --all-targets

# Build debug binary
[group('build')]
build:
    cargo build

# Build release binary
[group('build')]
build-release:
    cargo build --release

# Clean build artifacts
[group('build')]
clean:
    cargo clean

# Run all tests
[group('test')]
test:
    cargo test --all-features

# Format code
[group('lint')]
fmt:
    cargo fmt --all

# Check code formatting without modifying files
[group('lint')]
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints
[group('lint')]
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all lints (fmt-check + clippy)
[group('lint')]
lint: fmt-check clippy

# Run all pre-push checks (lint + test)
[group('workflows')]
check-all: lint test
    @echo "All checks passed!"

# Run security audit on dependencies
[group('optional')]
audit:
    cargo audit

# Generate rustdoc documentation
[group('optional')]
docs:
    cargo doc --no-deps --open
