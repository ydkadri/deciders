default:
    @just --list

# Install the toolchain and fetch dependencies
[group('development')]
install:
    rustup show active-toolchain || rustup toolchain install
    cargo fetch --locked

# Build the release binary
[group('development')]
build:
    cargo build --workspace --release --locked

# Remove build artefacts
[group('development')]
clean:
    cargo clean

# Run unit tests and doctests
[group('testing')]
test: test-unit test-doc

# Run unit tests
[group('testing')]
test-unit:
    cargo test --workspace --lib --bins

# Run doctests (cargo llvm-cov does not run them)
[group('testing')]
test-doc:
    cargo test --workspace --doc

# Run integration tests
[group('testing')]
test-integration:
    cargo test --workspace --test '*'

# Run tests with coverage, failing under 80% line coverage
[group('testing')]
test-coverage:
    cargo llvm-cov --workspace --fail-under-lines 80

# Run format check, clippy, rustdoc
[group('quality')]
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Auto-format code
[group('quality')]
format:
    cargo fmt --all

# Compile-check all targets
[group('quality')]
typecheck:
    cargo check --workspace --all-targets --locked

# Scan dependencies for known vulnerabilities
[group('security')]
vulnerability:
    cargo audit --deny warnings

# Run all quality checks
[group('quality')]
check: lint typecheck test-coverage test-doc
    @echo "All checks passed!"

# Run format, lint, typecheck and unit tests
[group('git')]
git-pre-commit: format lint typecheck test

# Run every check including integration tests and vulnerability scan
[group('git')]
git-pre-push: check test-integration vulnerability
