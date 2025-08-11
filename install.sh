#!/usr/bin/env bash


set -euo pipefail

REPO="PeterFujiyu/geektools"

# ---------- 工具函数 ----------
has_cmd() { command -v "$1" >/dev/null 2>&1; }

# curl 统一封装（自动带上 GH_TOKEN）
curl_gh() {
  if [[ -n "${GH_TOKEN:-}" ]]; then
    curl -fsSL -H "Authorization: Bearer ${GH_TOKEN}" "$@"
  else
    curl -fsSL "$@"
  fi
}

# ---------- 获取 tag（API → 跳转双保险） ----------
get_tag_from_api() {
  # 先 latest（正式），失败退回 releases 列表第一页
  curl_gh "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || \
  curl_gh "https://api.github.com/repos/${REPO}/releases?per_page=1" \
    | grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

get_tag_from_redirect() {
  # 用 302 跳转定位真实 tag
  local url
  url=$(curl_gh -I "https://github.com/${REPO}/releases/latest" \
        | grep -im1 '^location:' | awk '{print $2}' | tr -d '\r') || return 1
  sed -E 's#.*/releases/tag/([^/[:space:]]+).*#\1#' <<<"$url"
}

TAG="${TAG_OVERRIDE:-}"
[[ -z "${TAG}" ]] && TAG="$(get_tag_from_api || true)"
[[ -z "${TAG}" ]] && TAG="$(get_tag_from_redirect || true)"

if [[ -z "${TAG}" ]]; then
  read -rp "⚠️  无法自动获取版本号，请手动输入（如 v1.2.3）: " TAG
fi
[[ -z "${TAG}" ]] && { echo "❌ 无法确定版本号，安装终止"; exit 1; }

echo "➡️  版本: $TAG"

# ---------- 选择产物 ----------
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    case "$ARCH" in
      x86_64|amd64)   FILE="geektools-linux-x64" ;;
      aarch64|arm64)  FILE="geektools-linux-arm64" ;;
      armv7l|armv6l)  FILE="geektools-linux-armhf" ;;  # 若无此产物会在下载时报错
      *) echo "❌ 不支持的 Linux 架构: $ARCH"; exit 1 ;;
    esac
    ;;
  darwin)
    FILE="geektools-macos-universal"
    ;;
  *)
    echo "❌ 不支持的操作系统: $OS"
    exit 1
    ;;
esac

VERSION="$TAG"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILE}"

# ---------- 下载 ----------
echo "⬇️  下载: $URL"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
dst="$tmpdir/$FILE"

# 重试以适配偶发网络波动
curl_gh --retry 3 --retry-delay 2 -o "$dst" "$URL" \
  || { echo "❌ 下载失败（可能是该架构无对应产物或 tag 不存在）"; exit 1; }
chmod +x "$dst"

# ---------- 安装 ----------
bindir="${HOME}/.local/bin"
mkdir -p "$bindir"

# 统一命名为 geektools，并提供 gt 的别名
install_path="${bindir}/geektools"
mv -f "$dst" "$install_path"
ln -sfn "$install_path" "${bindir}/gt"

# ---------- 配置 PATH（幂等） ----------
add_path_line='export PATH="$HOME/.local/bin:$PATH"'
ensure_path_in() {
  local rc="$1"
  [[ -f "$rc" ]] || return 0
  if ! grep -Fq '.local/bin' "$rc"; then
    printf '\n# added by geektools installer\n%s\n' "$add_path_line" >> "$rc"
  fi
}

# 按当前用户常见 rc 文件写入，但不 source（避免 bash 去跑 zsh 语法）
shell_name="$(ps -p $$ -o comm= 2>/dev/null || echo bash)"

case "$shell_name" in
  zsh)
    ensure_path_in "${HOME}/.zshrc"
    ;;
  bash)
    # 兼容 Debian/Ubuntu（~/.bashrc）与 macOS（~/.bash_profile）
    ensure_path_in "${HOME}/.bashrc"
    ensure_path_in "${HOME}/.bash_profile"
    ;;
  *)
    # 兜底都写一份
    ensure_path_in "${HOME}/.profile"
    ensure_path_in "${HOME}/.bashrc"
    ensure_path_in "${HOME}/.zshrc"
    ;;
esac

echo
echo "✅ 安装完成：${install_path}"
echo "👉 别名：${bindir}/gt"
echo "ℹ️  若新开终端仍找不到命令，请手动执行："
echo "   ${add_path_line}"
echo
echo "🚀 现在可以运行：geektools  或  gt"