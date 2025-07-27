#!/bin/bash

# GeekTools Plugin Marketplace Build Script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="geektools-marketplace-server"
TARGET_DIR="target"
RELEASE_DIR="releases"

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

# Check Rust installation
check_rust() {
    log "Checking Rust installation..."
    
    if ! command -v cargo &> /dev/null; then
        error "Rust/Cargo is not installed. Please install from https://rustup.rs/"
    fi
    
    if ! command -v rustc &> /dev/null; then
        error "Rust compiler not found"
    fi
    
    log "Rust $(rustc --version) detected"
}

# Install required system dependencies
install_dependencies() {
    log "Installing system dependencies..."
    
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y pkg-config libssl-dev libpq-dev
        elif command -v yum &> /dev/null; then
            sudo yum install -y pkg-config openssl-devel postgresql-devel
        elif command -v pacman &> /dev/null; then
            sudo pacman -S --noconfirm pkg-config openssl postgresql-libs
        else
            warn "Unknown package manager. Please install: pkg-config, openssl-dev, postgresql-dev"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &> /dev/null; then
            brew install pkg-config openssl postgresql
        else
            warn "Homebrew not found. Please install dependencies manually"
        fi
    else
        warn "Unknown OS. Please install dependencies manually"
    fi
    
    log "Dependencies installation completed"
}

# Clean previous builds
clean_build() {
    log "Cleaning previous builds..."
    cargo clean
    log "Clean completed"
}

# Run tests
run_tests() {
    log "Running tests..."
    cargo test --verbose
    log "Tests completed successfully"
}

# Check code formatting and linting
check_code_quality() {
    log "Checking code formatting..."
    
    if ! cargo fmt -- --check; then
        warn "Code formatting issues found. Run 'cargo fmt' to fix."
    fi
    
    log "Running clippy for linting..."
    cargo clippy -- -D warnings
    
    log "Code quality checks completed"
}

# Build for development
build_dev() {
    log "Building for development..."
    cargo build
    log "Development build completed"
}

# Build for release
build_release() {
    log "Building optimized release..."
    cargo build --release
    log "Release build completed"
}

# Build for multiple targets
build_cross_platform() {
    log "Building for multiple targets..."
    
    # Install cross-compilation targets
    rustup target add x86_64-unknown-linux-gnu
    rustup target add x86_64-unknown-linux-musl
    rustup target add aarch64-unknown-linux-gnu
    rustup target add x86_64-pc-windows-gnu
    
    # Create releases directory
    mkdir -p "$RELEASE_DIR"
    
    # Build for Linux x86_64
    log "Building for Linux x86_64..."
    cargo build --release --target x86_64-unknown-linux-gnu
    cp target/x86_64-unknown-linux-gnu/release/server "$RELEASE_DIR/${PROJECT_NAME}-linux-x86_64"
    
    # Build for Linux x86_64 (musl - static)
    log "Building for Linux x86_64 (static)..."
    if command -v musl-gcc &> /dev/null; then
        cargo build --release --target x86_64-unknown-linux-musl
        cp target/x86_64-unknown-linux-musl/release/server "$RELEASE_DIR/${PROJECT_NAME}-linux-x86_64-static"
    else
        warn "musl-gcc not found, skipping static build"
    fi
    
    # Build for Linux ARM64
    log "Building for Linux ARM64..."
    if command -v aarch64-linux-gnu-gcc &> /dev/null; then
        export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
        cargo build --release --target aarch64-unknown-linux-gnu
        cp target/aarch64-unknown-linux-gnu/release/server "$RELEASE_DIR/${PROJECT_NAME}-linux-arm64"
    else
        warn "ARM64 cross-compiler not found, skipping ARM64 build"
    fi
    
    log "Cross-platform builds completed"
}

# Package release
package_release() {
    log "Packaging release..."
    
    VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "geektools-marketplace-server") | .version')
    PACKAGE_NAME="${PROJECT_NAME}-${VERSION}"
    
    mkdir -p "$RELEASE_DIR/$PACKAGE_NAME"
    
    # Copy binary
    cp target/release/server "$RELEASE_DIR/$PACKAGE_NAME/"
    
    # Copy configuration
    cp -r config "$RELEASE_DIR/$PACKAGE_NAME/"
    cp -r migrations "$RELEASE_DIR/$PACKAGE_NAME/"
    
    # Copy documentation
    cp README.md "$RELEASE_DIR/$PACKAGE_NAME/" 2>/dev/null || true
    cp LICENSE "$RELEASE_DIR/$PACKAGE_NAME/" 2>/dev/null || true
    
    # Create archive
    cd "$RELEASE_DIR"
    tar -czf "${PACKAGE_NAME}.tar.gz" "$PACKAGE_NAME"
    zip -r "${PACKAGE_NAME}.zip" "$PACKAGE_NAME" > /dev/null
    cd ..
    
    log "Release packaged: $RELEASE_DIR/${PACKAGE_NAME}.tar.gz"
    log "Release packaged: $RELEASE_DIR/${PACKAGE_NAME}.zip"
}

# Build Docker image
build_docker() {
    log "Building Docker image..."
    
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed"
    fi
    
    VERSION=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "geektools-marketplace-server") | .version')
    IMAGE_NAME="geektools-marketplace:${VERSION}"
    
    docker build -t "$IMAGE_NAME" .
    docker tag "$IMAGE_NAME" "geektools-marketplace:latest"
    
    log "Docker image built: $IMAGE_NAME"
}

# Show build info
show_info() {
    log "Build Information:"
    echo "  Rust version: $(rustc --version)"
    echo "  Cargo version: $(cargo --version)"
    echo "  Target directory: $TARGET_DIR"
    echo "  Release directory: $RELEASE_DIR"
    
    if [ -f "target/release/server" ]; then
        echo "  Binary size: $(du -h target/release/server | cut -f1)"
    fi
}

# Main build process
main() {
    case "${1:-release}" in
        "dev" | "debug")
            check_rust
            build_dev
            show_info
            ;;
        "release")
            check_rust
            install_dependencies
            check_code_quality
            run_tests
            build_release
            show_info
            ;;
        "cross")
            check_rust
            install_dependencies
            check_code_quality
            run_tests
            build_cross_platform
            show_info
            ;;
        "package")
            check_rust
            install_dependencies
            check_code_quality
            run_tests
            build_release
            package_release
            show_info
            ;;
        "docker")
            build_docker
            ;;
        "all")
            check_rust
            install_dependencies
            check_code_quality
            run_tests
            build_release
            build_cross_platform
            package_release
            build_docker
            show_info
            ;;
        "clean")
            clean_build
            ;;
        "test")
            check_rust
            run_tests
            ;;
        "check")
            check_rust
            check_code_quality
            ;;
        *)
            echo "Usage: $0 {dev|release|cross|package|docker|all|clean|test|check}"
            echo ""
            echo "Commands:"
            echo "  dev      - Build for development"
            echo "  release  - Build optimized release (default)"
            echo "  cross    - Cross-compile for multiple platforms"
            echo "  package  - Package release with documentation"
            echo "  docker   - Build Docker image"
            echo "  all      - Run all build steps"
            echo "  clean    - Clean build artifacts"
            echo "  test     - Run tests only"
            echo "  check    - Run code quality checks only"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"