#!/bin/bash
# Name: CPU使用率监控
# Description: 实时显示系统CPU使用率，支持多核心显示
# Author: GeekTools 开发团队
# Version: 1.2.0

echo "=== CPU使用率监控 ==="
echo "按 Ctrl+C 退出监控"
echo ""

# 检查操作系统类型
case "$(uname -s)" in
    Linux*)
        echo "🐧 Linux 系统检测"
        if command -v htop >/dev/null 2>&1; then
            echo "✅ 使用 htop 显示详细信息"
            htop
        elif [ -f /proc/stat ]; then
            echo "📊 CPU使用率统计："
            # 读取 /proc/stat 计算CPU使用率
            grep 'cpu ' /proc/stat | awk '{usage=($2+$4)*100/($2+$3+$4+$5)} END {printf "总体CPU使用率: %.1f%%\n", usage}'
            echo ""
            echo "🔥 各核心使用情况："
            grep '^cpu[0-9]' /proc/stat | awk '{printf "CPU%d: %.1f%%\n", NR-1, ($2+$4)*100/($2+$3+$4+$5)}'
        else
            echo "❌ 无法获取CPU信息"
        fi
        ;;
    Darwin*)
        echo "🍎 macOS 系统检测"
        if command -v top >/dev/null 2>&1; then
            cpu_usage=$(top -l 1 -s 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')
            echo "📊 当前CPU使用率: ${cpu_usage}%"
            echo ""
            echo "🔍 详细进程信息："
            top -l 1 -o cpu | head -15
        else
            echo "❌ 无法获取CPU使用率信息"
        fi
        ;;
    *)
        echo "❓ 未知操作系统: $(uname -s)"
        echo "⚠️  此脚本支持 Linux 和 macOS 系统"
        exit 1
        ;;
esac

echo ""
echo "✅ CPU监控完成"