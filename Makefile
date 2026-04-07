.PHONY: all build install release clean fmt clippy check test

# Default target
all: build

# Build the project in debug mode
build:
	@cd pdf-to-markdown && cargo build

# Build and install the binary
install:
	@cd pdf-to-markdown && cargo install --path .

# Build release version
release:
	@cd pdf-to-markdown && cargo build --release

# Clean build artifacts
clean:
	@cd pdf-to-markdown && cargo clean

# Format code with rustfmt
fmt:
	@cd pdf-to-markdown && cargo fmt

# Check formatting
fmt-check:
	@cd pdf-to-markdown && cargo fmt --check

# Run clippy linter
clippy:
	@cd pdf-to-markdown && cargo clippy -- -D warnings

# Run cargo check
check:
	@cd pdf-to-markdown && cargo check

# Run tests (if any)
test:
	@cd pdf-to-markdown && cargo test
