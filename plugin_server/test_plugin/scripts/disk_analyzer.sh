#!/bin/bash
# Name: 磁盘空间分析
# Description: 分析磁盘空间使用情况并提供清理建议
# Author: GeekTools 开发团队
# Version: 1.2.0

echo "=== 磁盘空间分析工具 ==="
echo ""

# 显示磁盘使用情况
echo "💽 磁盘使用情况："
if command -v df >/dev/null 2>&1; then
    # 使用df命令显示磁盘使用情况
    df -h | while IFS= read -r line; do
        if echo "$line" | grep -q "^Filesystem"; then
            echo "📋 $line"
        elif echo "$line" | grep -E "^/dev/|^tmpfs|^/System/Volumes" >/dev/null; then
            usage=$(echo "$line" | awk '{print $5}' | sed 's/%//')
            if [ "$usage" -gt 90 ]; then
                echo "🔴 $line  <- 危险：使用率过高！"
            elif [ "$usage" -gt 80 ]; then
                echo "🟡 $line  <- 警告：使用率较高"
            elif [ "$usage" -gt 60 ]; then
                echo "🟢 $line  <- 正常：使用率适中"
            else
                echo "🔵 $line  <- 优秀：使用率较低"
            fi
        fi
    done
else
    echo "❌ df 命令不可用"
fi

echo ""
echo "📊 详细空间分析："

# 检查当前目录的大目录
echo "🔍 当前目录下的大文件夹："
if command -v du >/dev/null 2>&1; then
    du -sh * 2>/dev/null | sort -hr | head -10 | while IFS= read -r line; do
        size=$(echo "$line" | awk '{print $1}')
        path=$(echo "$line" | cut -f2-)
        echo "  📁 $path: $size"
    done
else
    echo "❌ du 命令不可用"
fi

echo ""
echo "🧹 清理建议："

# 检查临时文件
temp_size=0
if [ -d "/tmp" ]; then
    temp_size=$(du -sh /tmp 2>/dev/null | awk '{print $1}' || echo "0")
    echo "🗑️  临时文件目录 (/tmp): $temp_size"
fi

# 检查日志文件 (仅在Linux上)
if [ "$(uname -s)" = "Linux" ] && [ -d "/var/log" ]; then
    log_size=$(du -sh /var/log 2>/dev/null | awk '{print $1}' || echo "0")
    echo "📝 系统日志目录 (/var/log): $log_size"
fi

# 检查缓存目录
if [ -d "$HOME/.cache" ]; then
    cache_size=$(du -sh "$HOME/.cache" 2>/dev/null | awk '{print $1}' || echo "0")
    echo "💾 用户缓存目录 (~/.cache): $cache_size"
fi

# macOS特定检查
if [ "$(uname -s)" = "Darwin" ]; then
    if [ -d "$HOME/Library/Caches" ]; then
        mac_cache_size=$(du -sh "$HOME/Library/Caches" 2>/dev/null | awk '{print $1}' || echo "0")
        echo "🍎 macOS缓存目录 (~/Library/Caches): $mac_cache_size"
    fi
    
    if [ -d "$HOME/.Trash" ]; then
        trash_size=$(du -sh "$HOME/.Trash" 2>/dev/null | awk '{print $1}' || echo "0")
        echo "🗑️  废纸篓 (~/.Trash): $trash_size"
    fi
fi

echo ""
echo "💡 清理提示："
echo "  1. 清空临时文件: sudo rm -rf /tmp/*"
echo "  2. 清理用户缓存: rm -rf ~/.cache/*"
if [ "$(uname -s)" = "Darwin" ]; then
    echo "  3. 清理macOS缓存: rm -rf ~/Library/Caches/*"
    echo "  4. 清空废纸篓: rm -rf ~/.Trash/*"
elif [ "$(uname -s)" = "Linux" ]; then
    echo "  3. 清理系统日志: sudo journalctl --vacuum-time=7d"
    echo "  4. 清理包缓存: sudo apt clean  # 或 sudo yum clean all"
fi

echo ""
echo "⚠️  注意：清理系统文件前请备份重要数据！"
echo "✅ 磁盘分析完成"