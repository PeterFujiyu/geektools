#!/usr/bin/env bash
set -eo pipefail

PROJECT_NAME="geektools"
RELEASE_DIR="target/dist"
UPX_ARGS=(--best --lzma)         # 数组！避免 "--best --lzma" 整串被当成一个参数

# ───── 0. 函数：工具检测 / 自动安装 ────────────────────────────────
need() {
  command -v "$1" >/dev/null 2>&1 && return
  echo "❌  $1 not found — $3"
  if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "➡  brew install $2"
    brew install "$2"
  else
    echo "➡  请手动安装：$2"; echo "Tips:尝试安装Homebrew以自动安装"; exit 1
  fi
}

# musl 交叉链
need x86_64-linux-musl-gcc FiloSottile/musl-cross/musl-cross \
     "编译 x86_64-unknown-linux-musl 需要 musl-gcc"
need aarch64-linux-musl-gcc FiloSottile/musl-cross/musl-cross \
     "编译 aarch64-unknown-linux-musl 需要 musl-gcc"

# UPX（可选）
if command -v upx >/dev/null; then
  USE_UPX=true;  echo "🗜  UPX 可用，将压缩 Linux 产物"
else
  USE_UPX=false; echo "⚠️  未找到 UPX，跳过压缩"
fi

# ───── 1. rustup target 确保齐全 ─────────────────────────────────
rustup target add \
  x86_64-apple-darwin aarch64-apple-darwin \
  x86_64-unknown-linux-musl aarch64-unknown-linux-musl >/dev/null

# ───── 2. 保留编译缓存，仅清理 dist ───────────────────────────────
echo "🧹 Cleaning old dist..."
rm -rf "$RELEASE_DIR"; mkdir -p "$RELEASE_DIR"

# ───── 2.5 生成 buildtag ─────────────────────────────────────────
generate_buildtag() {
  local kernel_version ts raw tag
  kernel_version=$(uname -r)               # 编译设备内核版本
  kernel=$(uname)                          # 编译设备内核
  ts=$(date -u +"%Y%m%d%H%M%S")            # 编译时间戳（UTC）
  raw="${kernel_version}${ts}${kernel}"

  if command -v sha256sum > /dev/null 2>&1; then
    tag=$(printf '%s' "$raw" | sha256sum  | awk '{print substr($1,length($1)-15)}')
  else
    tag=$(printf '%s' "$raw" | shasum -a 256 | awk '{print substr($1,length($1)-15)}')
  fi
  echo "$tag"
}

# 生成并写入 ./src/buildtag.env（覆盖写入）
kernel_version=$(uname -r)                       # 编译设备内核版本
kernel=$(uname)                          # 编译设备内核
ts=$(date -u +"%Y%m%d%H%M%S")            # 编译时间戳（UTC）
BUILD_TAG=$(generate_buildtag)
echo "📝  Build tag: $BUILD_TAG"
touch ./src/buildtag.env
echo "$BUILD_TAG" > ./src/buildtag.env
echo "- $BUILD_TAG = ${kernel} ${kernel_version} Time:${ts}" >> ./Buildtag.md

# ───── 3. 构建帮助函数 ───────────────────────────────────────────
build() {
  local target=$1 out=$2 ext=${3:-}
  echo "⚒  $target"
  cargo build --release --target "$target"
  cp "target/$target/release/$PROJECT_NAME$ext" "$out"
  if $USE_UPX && [[ $target == *"-linux-"* ]]; then
    upx "${UPX_ARGS[@]}" "$out"
  fi
}

# ───── 4. macOS Universal ────────────────────────────────────────
build x86_64-apple-darwin  "target/tmp-mac-x64"
build aarch64-apple-darwin "target/tmp-mac-arm64"
echo "🦀  Lipo macOS universal"
lipo -create \
  -output "$RELEASE_DIR/${PROJECT_NAME}-macos-universal" \
  target/tmp-mac-x64 target/tmp-mac-arm64
rm target/tmp-mac-*

# ───── 5. Linux (musl 静态) ──────────────────────────────────────
build x86_64-unknown-linux-musl "$RELEASE_DIR/${PROJECT_NAME}-linux-x64"
build aarch64-unknown-linux-musl "$RELEASE_DIR/${PROJECT_NAME}-linux-arm64"

echo "✅  Artifacts in $RELEASE_DIR"
ls -lh "$RELEASE_DIR"
