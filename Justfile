default:
  just --list

# Build the project
build:
  cargo build --workspace

# Clean build artifacts
clean:
  cargo clean

# Format all source files
fmt:
  cargo fmt
  deno fmt

# Run Rust tests only; pass nextest filters as args (e.g. just test-rust -E 'package(ndg-core)')
test-rs *args:
  cargo nextest run --workspace {{args}}

# Run JS/Deno tests only; pass a file glob or path to narrow scope (e.g. just test-js search.test.js)
test-js *args:
  deno test {{args}}

# Run all tests for the workspace, including JS tests
test: test-rs test-js

# Check formatting
fmt-check:
  cargo fmt --check
  deno fmt --check
