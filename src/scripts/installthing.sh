#!/usr/bin/env bash
set -euo pipefail

install_from_source() {
    local build_dir
    build_dir="$(mktemp -d /tmp/thing_build.XXXXXX)"
    echo "Using source build in $build_dir"
    cd "$build_dir"

    if ! command -v fortune >/dev/null 2>&1; then
        echo "Building fortune from source..."
        curl -L https://github.com/shlomif/fortune-mod/archive/fortune-mod-3.18.0.tar.gz -o fortune.tar.gz
        tar xf fortune.tar.gz
        cd fortune-mod-* || exit 1
        ./autogen.sh >/dev/null 2>&1 || true
        ./configure --prefix=/usr/local
        make
        sudo make install
        cd ..
    fi

    if ! command -v cowsay >/dev/null 2>&1; then
        echo "Building cowsay from source..."
        curl -L https://github.com/tnalpgge/rank-amateur-cowsay/archive/refs/tags/v1.6.0.tar.gz -o cowsay.tar.gz
        tar xf cowsay.tar.gz
        cd rank-amateur-cowsay-* || exit 1
        sudo make install
        cd ..
    fi
}

install_with_pm() {
    case "$1" in
        apt-get)
            sudo apt-get update
            sudo apt-get install -y fortune-mod cowsay
            ;;
        yum)
            sudo yum install -y fortune-mod cowsay
            ;;
        dnf)
            sudo dnf install -y fortune-mod cowsay
            ;;
        pacman)
            sudo pacman -Sy --noconfirm fortune-mod cowsay
            ;;
    esac
}

detect_pm() {
    for pm in apt-get dnf yum pacman; do
        if command -v "$pm" >/dev/null 2>&1; then
            echo "$pm"
            return 0
        fi
    done
    return 1
}

setup_alias() {
    local rc_file=$1
    local alias_line="alias thing='fortune | cowsay'"
    local run_line="thing"

    if [ ! -f "$rc_file" ]; then
        touch "$rc_file"
    fi

    if ! grep -Fq "$alias_line" "$rc_file"; then
        echo "$alias_line" >> "$rc_file"
    fi
    if ! grep -Fq "$run_line" "$rc_file"; then
        echo "$run_line" >> "$rc_file"
    fi
}

main() {
    if pm=$(detect_pm); then
        echo "Installing using $pm"
        install_with_pm "$pm"
    else
        echo "No supported package manager detected, building from source"
        install_from_source
    fi

    setup_alias "$HOME/.bashrc"
    setup_alias "$HOME/.zshrc"
    echo "Installation complete. Please restart your terminal."
}

main "$@"
