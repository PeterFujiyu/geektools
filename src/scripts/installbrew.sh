#!/usr/bin/env bash
set -euo pipefail

# ------------------------------------------------------------
# Homebrew installation helper with optional mirror support
# Homebrew 安装助手，支持选择官方或清华镜像
# ------------------------------------------------------------

if [[ $(id -u) -eq 0 ]]; then
    echo "Please run this script as a regular user, not as root. | 请以普通用户身份运行此脚本，不要使用 root." >&2
    exit 1
fi

if command -v brew >/dev/null 2>&1; then
    echo "Homebrew is already installed. | Homebrew 已安装"
    exit 0
fi

for cmd in curl git; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "Required command '$cmd' not found. Please install it first. | 未找到必需命令 '$cmd'，请先安装。" >&2
        exit 1
    fi
done

OS=$(uname)
if [[ "$OS" == "Darwin" ]]; then
    if ! xcode-select -p >/dev/null 2>&1; then
        echo "Xcode command line tools are required. They will be installed now. | 需要安装 Xcode 命令行工具，正在安装" 
        xcode-select --install || true
        echo "Please re-run this script after the installation finishes. | 安装完成后请重新运行此脚本"
        exit 0
    fi
elif [[ "$OS" != "Linux" ]]; then
    echo "Unsupported OS: $OS | 不支持的系统: $OS" >&2
    exit 1
fi

echo "Choose Homebrew installation source: | 请选择 Homebrew 安装源："
echo "1) Official (GitHub) | 1) 官方源（GitHub）"
echo "2) Tsinghua mirror (mainland China users) | 2) 清华镜像（中国大陆用户）"
read -r -p "Enter choice [1/2] (default 1): | 请输入选择 [1/2]（默认为1）：" choice
choice=${choice:-1}

case "$choice" in
    1)
        echo "Using official installation script... | 使用官方安装脚本..."
        INSTALL() { /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"; }
        ;;
    2)
        echo "Using Tsinghua mirror... | 使用清华镜像..."
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
        echo "Invalid choice. | 无效的选择" >&2
        exit 1
        ;;
esac

echo
echo "About to install Homebrew. | 即将安装 Homebrew"
read -r -p "Proceed? [y/N]: | 是否继续？[y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Installation cancelled. | 取消安装"
    exit 0
fi

INSTALL

# Add Homebrew to PATH
add_brew_to_path() {
    local brew_prefix
    brew_prefix="$(brew --prefix)"
    local init_cmd="eval \"\$(${brew_prefix}/bin/brew shellenv)\""

    echo "Adding Homebrew to PATH... | 正在将 Homebrew 加入 PATH..."

    for profile in ~/.bash_profile ~/.zprofile ~/.profile; do
        if [[ -f $profile ]]; then
            if ! grep -F "${brew_prefix}/bin/brew shellenv" "$profile" >/dev/null 2>&1; then
                echo "$init_cmd" >> "$profile"
            fi
        fi
    done

    eval "$init_cmd"
}

add_brew_to_path

echo "Installation complete. Please restart your shell. | 安装完成，请重新启动终端"
