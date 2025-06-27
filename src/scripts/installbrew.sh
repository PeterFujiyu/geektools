#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# Homebrew installation helper with optional mirror support
# ------------------------------------------------------------

if [[ $(id -u) -eq 0 ]]; then
    echo "Please run this script as a regular user, not as root." >&2
    exit 1
fi

if command -v brew >/dev/null 2>&1; then
    echo "Homebrew is already installed."
    exit 0
fi

for cmd in curl git; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Required command '$cmd' not found. Please install it first." >&2
        exit 1
    fi
done

OS=$(uname)
if [[ "$OS" == "Darwin" ]]; then
    if ! xcode-select -p >/dev/null 2>&1; then
        echo "Xcode command line tools are required. They will be installed now."
        xcode-select --install || true
        echo "Please re-run this script after the installation finishes."
        exit 0
    fi
elif [[ "$OS" != "Linux" ]]; then
    echo "Unsupported OS: $OS" >&2
    exit 1
fi

echo "Choose Homebrew installation source:"
echo "1) Official (GitHub)"
echo "2) Tsinghua mirror (mainland China users)"
read -r -p "Enter choice [1/2] (default 1): " choice
choice=${choice:-1}

case "$choice" in
    1)
        echo "Using official installation script..."
        INSTALL() { /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"; }
        ;;
    2)
        echo "Using Tsinghua mirror..."
        INSTALL() {
            export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
            export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
            export HOMEBREW_INSTALL_FROM_API=1
            git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git brew-install
            /bin/bash brew-install/install.sh
            rm -rf brew-install
        }
        ;;
    *)
        echo "Invalid choice." >&2
        exit 1
        ;;
esac

echo
echo "About to install Homebrew."
read -r -p "Proceed? [y/N]: " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Installation cancelled."
    exit 0
fi

INSTALL
