#!/usr/bin/env bash

# 若被 /bin/sh 等非-bash 解释器调用，自动切换到 bash
[ -n "${BASH_VERSION:-}" ] || { exec bash "$0" "$@"; }
# Geektools 一键安装脚本
set -euo pipefail

REPO="PeterFujiyu/geektools"

# ───── 获取最新 tag ─────────────────────────────────────────────
get_tag_from_api() {
  # 先尝试 latest（正式），失败再退回到第一页（可能是 Pre-release）
  local api_latest="https://api.github.com/repos/${REPO}/releases/latest"
  local api_all="https://api.github.com/repos/${REPO}/releases?per_page=1"

  if [[ -n "${GH_TOKEN:-}" ]]; then
    curl -fsSL -H "Authorization: Bearer ${GH_TOKEN}" "$api_latest" \
      || curl -fsSL -H "Authorization: Bearer ${GH_TOKEN}" "$api_all"
  else
    { curl -fsSL -s "$api_latest" 2>/dev/null || curl -fsSL -s "$api_all" 2>/dev/null; } \
      || echo "ERROR: Failed to fetch from API"
  fi | grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

get_tag_from_redirect() {
  local url
  url=$(curl -fsIL -o /dev/null -w '%{url_effective}' \
        "https://github.com/${REPO}/releases/latest") || return 1
  sed -E 's#.*/releases/tag/([^/]+).*#\1#' <<<"$url"
}

TAG="${TAG_OVERRIDE:-}"

[[ -z $TAG ]] && TAG=$(get_tag_from_api     || true)
[[ -z $TAG ]] && TAG=$(get_tag_from_redirect|| true)

if [[ -z $TAG ]]; then
  read -rp "⚠️  无法自动获取版本号，请手动输入（如 v1.2.3）: " TAG
fi
[[ -z $TAG ]] && { echo "❌ 无法确定版本号，安装终止"; exit 1; }

echo "➡️  最新版本: $TAG"

# ───── 选择产物文件名 ───────────────────────────────────────────
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$OS" in
  linux)
    case "$ARCH" in
      x86_64|amd64) FILE="geektools-linux-x64" ;;
      aarch64|arm64) FILE="geektools-linux-arm64" ;;
      *) echo "❌ 不支持的 Linux 架构: $ARCH"; exit 1 ;;
    esac ;;
  darwin) FILE="geektools-macos-universal" ;;
  *) echo "❌ 不支持的操作系统: $OS"; exit 1 ;;
esac

# 0) 准备 curl 头部数组（即使没有令牌也要初始化，避免 “header[@] 未定义”）
header=()
if [[ -n "${GH_TOKEN:-}" ]]; then
  header=(-H "Authorization: Bearer ${GH_TOKEN}")
fi

# 1) 获取最新 tag（API → 跳转双保险）
get_latest_tag() {
  # 尝试 GitHub API
  curl -fsSL "${header[@]}" \
       "https://api.github.com/repos/${REPO}/releases/latest" |
    grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' && return 0

  # 若失败再用 302 跳转解析
  curl -fsIL "${header[@]}" \
       "https://github.com/${REPO}/releases/latest" |
    grep -im1 '^location:' | sed -E 's#.*/tag/([^[:space:]]+).*#\1#'
}

# 复用前面已拿到的 $TAG（避免重复获取失败）
VERSION="$TAG"
# 2) 拼出正确的下载地址
URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILE}"
echo "⬇️  正在下载: $URL"
curl -fL -o "$FILE" "$URL" || { echo "❌ 下载失败"; exit 1; }
chmod +x "$FILE"

# ───── 安装 ────────────────────────────────────────────────────
mkdir -p "${HOME}/.local/bin/"
mv "$FILE" "${HOME}/.local/bin/"
ln -s "${HOME}/.local/bin/${FILE}" "${HOME}/.local/bin/gt"
echo "export PATH=$PATH:${HOME}/.local/bin" >> ~/.bashrc
echo "export PATH=$PATH:${HOME}/.local/bin" >> ~/.zshrc
. ~/.bashrc
. ~/.zshrc

echo "🎉 完成！现在可以直接运行 geektools（或 gt）"