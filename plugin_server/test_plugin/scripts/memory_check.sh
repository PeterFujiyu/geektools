#!/bin/bash
# Name: 内存使用检查
# Description: 显示内存使用情况和可用空间
# Author: GeekTools 开发团队
# Version: 1.2.0

echo "=== 内存使用情况检查 ==="
echo ""

# 检查操作系统类型
case "$(uname -s)" in
    Linux*)
        echo "🐧 Linux 内存信息："
        if [ -f /proc/meminfo ]; then
            # 读取内存信息
            total_mem=$(grep '^MemTotal:' /proc/meminfo | awk '{print $2}')
            available_mem=$(grep '^MemAvailable:' /proc/meminfo | awk '{print $2}')
            free_mem=$(grep '^MemFree:' /proc/meminfo | awk '{print $2}')
            
            # 转换为MB
            total_mb=$((total_mem / 1024))
            available_mb=$((available_mem / 1024))
            free_mb=$((free_mem / 1024))
            used_mb=$((total_mb - available_mb))
            
            echo "💾 总内存: ${total_mb} MB"
            echo "✅ 可用内存: ${available_mb} MB"
            echo "📊 已用内存: ${used_mb} MB"
            echo "🆓 空闲内存: ${free_mb} MB"
            
            # 计算使用百分比
            usage_pct=$((used_mb * 100 / total_mb))
            echo "📈 使用率: ${usage_pct}%"
            
            # 警告检查
            if [ $usage_pct -gt 80 ]; then
                echo "⚠️  警告: 内存使用率过高！"
            elif [ $usage_pct -gt 60 ]; then
                echo "⚡ 注意: 内存使用率较高"
            else
                echo "✅ 内存使用率正常"
            fi
        else
            echo "❌ 无法读取内存信息"
        fi
        ;;
    Darwin*)
        echo "🍎 macOS 内存信息："
        if command -v vm_stat >/dev/null 2>&1; then
            # 获取页面大小
            page_size=$(vm_stat | grep "page size" | awk '{print $8}')
            
            # 获取内存统计
            vm_stat | while read line; do
                case "$line" in
                    "Pages free:"*)
                        free_pages=$(echo "$line" | awk '{print $3}' | sed 's/\.//')
                        free_mb=$((free_pages * page_size / 1024 / 1024))
                        echo "🆓 空闲内存: ${free_mb} MB"
                        ;;
                    "Pages active:"*)
                        active_pages=$(echo "$line" | awk '{print $3}' | sed 's/\.//')
                        active_mb=$((active_pages * page_size / 1024 / 1024))
                        echo "🔥 活动内存: ${active_mb} MB"
                        ;;
                    "Pages inactive:"*)
                        inactive_pages=$(echo "$line" | awk '{print $3}' | sed 's/\.//')
                        inactive_mb=$((inactive_pages * page_size / 1024 / 1024))
                        echo "💤 非活动内存: ${inactive_mb} MB"
                        ;;
                    "Pages wired down:"*)
                        wired_pages=$(echo "$line" | awk '{print $4}' | sed 's/\.//')
                        wired_mb=$((wired_pages * page_size / 1024 / 1024))
                        echo "🔒 系统占用: ${wired_mb} MB"
                        ;;
                esac
            done
        else
            echo "❌ 无法获取内存信息"
        fi
        ;;
    *)
        echo "❓ 未知操作系统: $(uname -s)"
        echo "⚠️  此脚本支持 Linux 和 macOS 系统"
        exit 1
        ;;
esac

echo ""
echo "✅ 内存检查完成"