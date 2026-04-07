#!/bin/bash

set -e

# Default values
REPO="Peng-YM/pdf-to-markdown"
REPO_BRANCH="master"
BINARY_NAME="pdf-to-markdown"
INSTALL_DIR="${HOME}/.local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() {
    echo -e "${BLUE}INFO:${NC} $1" >&2
}

print_success() {
    echo -e "${GREEN}SUCCESS:${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1" >&2
}

print_error() {
    echo -e "${RED}ERROR:${NC} $1" >&2
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        MINGW*|MSYS*|CYGWIN*) echo "windows";;
        *)          print_error "Unsupported OS: $(uname -s)"; exit 1;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo "x86_64";;
        arm64|aarch64) echo "aarch64";;
        *) print_error "Unsupported architecture: $(uname -m)"; exit 1;;
    esac
}

# Get latest release tag
get_latest_release() {
    print_info "Fetching latest release from GitHub..."
    local tag=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep -o '"tag_name": "[^"]*"' | cut -d'"' -f4)
    if [ -z "$tag" ]; then
        print_error "Failed to get latest release tag"
        exit 1
    fi
    echo "$tag"
}

# Download binary
download_binary() {
    local os=$1
    local arch=$2
    local tag=$3
    
    local extension=""
    if [ "$os" = "windows" ]; then
        extension=".exe"
    fi
    
    local filename="${BINARY_NAME}-${os}-${arch}${extension}"
    local url="https://github.com/${REPO}/releases/download/${tag}/${filename}"
    
    print_info "Downloading ${filename}..."
    print_info "URL: ${url}"
    
    if command -v curl &> /dev/null; then
        curl -L -o "${BINARY_NAME}${extension}" "${url}"
    elif command -v wget &> /dev/null; then
        wget -O "${BINARY_NAME}${extension}" "${url}"
    else
        print_error "Neither curl nor wget is installed"
        exit 1
    fi
    
    if [ ! -f "${BINARY_NAME}${extension}" ]; then
        print_error "Failed to download binary"
        exit 1
    fi
    
    echo "${BINARY_NAME}${extension}"
}

# Install binary
install_binary() {
    local binary_path=$1
    
    mkdir -p "${INSTALL_DIR}"
    
    print_info "Installing to ${INSTALL_DIR}..."
    mv "${binary_path}" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    
    if [ ! -f "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        print_error "Installation failed"
        exit 1
    fi
}

# Check if in PATH
check_path() {
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        print_warning "${INSTALL_DIR} is not in your PATH"
        print_info "Add the following to your shell configuration file (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        print_info "Then restart your shell or run:"
        echo ""
        echo "  source ~/.bashrc  # or ~/.zshrc"
        echo ""
    fi
}

# Main function
main() {
    print_info "PDF to Markdown Installer"
    print_info "=========================="
    echo ""
    
    local os=$(detect_os)
    local arch=$(detect_arch)
    local tag=${1:-$(get_latest_release)}
    
    print_info "Detected OS: ${os}"
    print_info "Detected architecture: ${arch}"
    print_info "Release tag: ${tag}"
    echo ""
    
    # Create temp directory
    local temp_dir=$(mktemp -d)
    cd "${temp_dir}"
    
    # Download
    local binary=$(download_binary "${os}" "${arch}" "${tag}")
    
    # Install
    install_binary "${binary}"
    
    # Cleanup
    cd /
    rm -rf "${temp_dir}"
    
    echo ""
    print_success "${BINARY_NAME} installed successfully!"
    print_success "Binary location: ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    
    check_path
    
    print_success "You can now run '${BINARY_NAME} --help' to get started"
}

# Run main
main "$@"
