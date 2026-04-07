.PHONY: all build install release clean fmt clippy check test

# Default target
all: build

# Build the project in debug mode
build:
	@cargo build

# Build and install the binary
install:
	@cargo install --path .

# Build release version
release:
	@cargo build --release

# Clean build artifacts
clean:
	@cargo clean

# Format code with rustfmt
fmt:
	@cargo fmt

# Check formatting
fmt-check:
	@cargo fmt --check

# Run clippy linter
clippy:
	@cargo clippy -- -D warnings

# Run cargo check
check:
	@cargo check

# Run tests (if any)
test:
	@cargo test
