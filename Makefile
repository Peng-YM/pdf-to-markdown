.PHONY: all build install release clean

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
