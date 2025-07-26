# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building
```bash
# Build for current platform (development)
cargo build

# Build optimized release version
cargo build --release

# Cross-platform build for all supported targets (requires brew and musl-cross)
sh ./allrelease.sh
```

### Testing
```bash
# Run the test file for script materialization
cargo test --bin test_scripts

# Manual testing of the application
./target/release/geektools
```

### Installation Testing
```bash
# Test the installation script locally
bash install.sh
```

## Architecture Overview

Geektools is a CLI application written in Rust that serves as a "geek toolbox" for executing shell scripts. It features a hierarchical menu system, bilingual support (English/Chinese), and both built-in scripts and custom script management.

### Core Architecture Components

1. **Main Application (`src/main.rs`)**: Contains the CLI menu system, user interaction logic, and application state management with `AppState` struct
2. **Script Management (`src/scripts/mod.rs`)**: Handles script materialization, dependency resolution, and execution of both built-in and custom scripts
3. **File I/O Layer (`src/fileio.rs`)**: Cross-platform file operations with proper error handling
4. **Internationalization (`src/i18n/mod.rs`)**: Runtime language detection and translation system

### Data Storage Structure
- `~/.geektools/config.json` - User configuration and custom script metadata
- `~/.geektools/custom_scripts/` - Downloaded custom script files (executable)
- `~/.geektools/scripts/` - Materialized built-in scripts
- `~/.geektools/logs/` - Application logs with timestamp

### Script System Design

**Built-in Scripts**: Embedded in the binary using `rust-embed`, materialized to filesystem on demand with dependency resolution via `#@import` syntax.

**Custom Scripts**: Downloaded from URLs and stored locally with metadata. New custom scripts (post-refactor) are saved to `~/.geektools/custom_scripts/` directory for offline execution, while legacy scripts maintain backward compatibility by re-downloading from URLs.

**Script Types**:
- `.sh` files - Direct shell scripts
- `.link` files - Contain URLs to download latest versions from remote sources

### Configuration Management

The application uses a layered configuration approach:
- Static configuration embedded in binary (translations, built-in script metadata)
- User configuration in JSON format with lazy loading
- Runtime state management through `AppState` struct

### Build System

The cross-platform build system (`allrelease.sh`) targets:
- macOS Universal (x86_64 + ARM64)
- Linux x86_64 (musl static linking)
- Linux ARM64 (musl static linking)

Build artifacts include version tagging with kernel information and optional UPX compression for Linux binaries.

## Key Development Notes

### Custom Script Storage Migration
Recent changes moved custom script storage from URL-only (in config) to local file storage in `~/.geektools/custom_scripts/`. The `CustomScript` struct now includes an optional `file_path` field for backward compatibility. New scripts are saved locally, while legacy scripts fall back to URL downloading.

### Language System
The application detects user language via IP geolocation API and supports runtime language switching. All user-facing strings go through the translation system with parameter substitution support.

### Security Model
- Security warnings displayed for custom script execution
- Scripts run in temporary directories for URL-based execution
- File permission management (executable flags, immutable flags)
- User confirmation required for potentially dangerous operations

### Version Management
Built-in OTA update system connects to GitHub Releases API, supporting both stable and pre-release versions with safe binary replacement using temporary files.