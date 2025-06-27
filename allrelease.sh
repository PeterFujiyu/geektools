#!/usr/bin/env bash
set -eo pipefail

PROJECT_NAME="geektools"
RELEASE_DIR="target/dist"
TARGETDIR="target"
UPX_ARGS="--best --lzma"

# ─── 0. 函数：检测并安装工具 ────────────────────────────────────────────
need_tool () {
  local bin=$1 pkg=$2 msg=$3
  if ! command -v "$bin" >/dev/null 2>&1; then
      echo "❌  $bin not found. $msg"
      if [[ "$OSTYPE" == "darwin"* ]]; then
          echo "➡  Installing via Homebrew: brew install $pkg"
          brew install "$pkg"
      else
          echo "➡  请手动安装：$pkg"
          echo "Tips: 尝试安装 homebrew 以自动安装"
          exit 1
      fi
  fi
}

# musl 交叉编译链：x86_64-linux-musl, aarch64-linux-musl
need_tool x86_64-linux-musl-gcc FiloSottile/musl-cross/musl-cross \
  "编译 x86_64-unknown-linux-musl 目标需要该交叉编译器。"
need_tool aarch64-linux-musl-gcc FiloSottile/musl-cross/musl-cross \
  "编译 aarch64-unknown-linux-musl 目标需要该交叉编译器。"

# MinGW-w64（只要拿到 x86_64-w64-mingw32-gcc 即可）
need_tool x86_64-w64-mingw32-gcc mingw-w64 "编译 Windows 目标需要 MinGW-w64 工具链。"

# UPX（可选）
if command -v upx >/dev/null; then
    USE_UPX=true
    echo "🗜  UPX found, Linux/Windows 可执行文件将被压缩。"
else
    USE_UPX=false
    echo "⚠️  UPX not found, 将跳过二进制压缩。"
fi

# ─── 1. 添加 rustup target ────────────────────────────────────────────
echo "🔍 Checking Rust targets..."
TARGETS=(
  x86_64-apple-darwin aarch64-apple-darwin
  x86_64-unknown-linux-musl aarch64-unknown-linux-musl
  x86_64-pc-windows-gnu
)
for t in "${TARGETS[@]}"; do rustup target add "$t" >/dev/null; done

# ─── 2. 清理 ──────────────────────────────────────────────────────────
echo "🧹 Cleaning old builds..."
rm -rf "$TARGETDIR"
mkdir -p "$RELEASE_DIR"

# ─── 3. 构建函数 ──────────────────────────────────────────────────────
build () {
  local target=$1 out=$2 ext=${3:-}
  echo "⚒  Building $target ..."
  cargo build --release --target "$target"
  cp "target/$target/release/$PROJECT_NAME$ext" "$out"

  # macOS 不压缩；其他系统根据 USE_UPX
  if $USE_UPX && [[ $target != *"apple-darwin"* ]]; then
      upx "$UPX_ARGS" "$out"
  fi
}

# ─── 4. macOS universal ──────────────────────────────────────────────
build x86_64-apple-darwin  "target/tmp-mac-x64"
build aarch64-apple-darwin "target/tmp-mac-arm64"
echo "🦀  Creating macOS universal binary..."
lipo -create -output "$RELEASE_DIR/${PROJECT_NAME}-macos-universal" \
     target/tmp-mac-x64 target/tmp-mac-arm64
rm target/tmp-mac-x64 target/tmp-mac-arm64

# ─── 5. Linux ────────────────────────────────────────────────────────
build x86_64-unknown-linux-musl "$RELEASE_DIR/${PROJECT_NAME}-linux-x64"
build aarch64-unknown-linux-musl "$RELEASE_DIR/${PROJECT_NAME}-linux-arm64"

# ─── 6. Windows ──────────────────────────────────────────────────────
build x86_64-pc-windows-gnu "$RELEASE_DIR/${PROJECT_NAME}-win-x64.exe" ".exe"

echo "✅  All artifacts are in $RELEASE_DIR/"
ls -lh "$RELEASE_DIR"