#!/bin/bash
# GeekTools 插件包自动生成器
# 用于快速创建测试插件包
# Author: Assistant
# Version: 1.0.0

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的信息
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 显示脚本标题
show_banner() {
    echo -e "${BLUE}"
    echo "======================================"
    echo "   GeekTools 插件包生成器"
    echo "======================================"
    echo -e "${NC}"
}

# 获取用户输入
get_user_input() {
    echo
    print_info "请提供以下插件信息："
    
    # 插件ID
    while true; do
        read -p "插件ID (只能包含字母、数字、下划线和短横线): " plugin_id
        if [[ "$plugin_id" =~ ^[a-zA-Z0-9_-]+$ ]]; then
            break
        else
            print_error "插件ID格式不正确，请重新输入"
        fi
    done
    
    # 插件名称
    read -p "插件显示名称: " plugin_name
    plugin_name=${plugin_name:-"测试插件"}
    
    # 插件版本
    read -p "插件版本 (默认: 1.0.0): " plugin_version
    plugin_version=${plugin_version:-"1.0.0"}
    
    # 插件描述
    read -p "插件描述: " plugin_description
    plugin_description=${plugin_description:-"这是一个自动生成的测试插件"}
    
    # 作者名称
    read -p "作者名称 (默认: Test Author): " plugin_author
    plugin_author=${plugin_author:-"Test Author"}
    
    # 插件标签
    read -p "插件标签 (用逗号分隔，默认: test,demo): " plugin_tags
    plugin_tags=${plugin_tags:-"test,demo"}
    
    # 最低GeekTools版本
    read -p "最低GeekTools版本 (默认: 0.5.0): " min_version
    min_version=${min_version:-"0.5.0"}
}

# 创建插件目录结构
create_plugin_structure() {
    local plugin_dir="$1"
    
    print_info "创建插件目录结构..."
    
    # 清理已存在的目录
    if [ -d "$plugin_dir" ]; then
        print_warning "目录 $plugin_dir 已存在，正在删除..."
        rm -rf "$plugin_dir"
    fi
    
    # 创建目录
    mkdir -p "$plugin_dir/scripts"
    
    print_success "插件目录结构创建完成"
}

# 生成示例脚本文件
create_sample_scripts() {
    local plugin_dir="$1"
    local scripts_dir="$plugin_dir/scripts"
    
    print_info "生成示例脚本文件..."
    
    # 脚本1：系统信息
    cat > "$scripts_dir/system_info.sh" << 'EOF'
#!/bin/bash
# Name: 系统信息查看器
# Description: 显示详细的系统信息
# Author: Test Author
# Version: 1.0.0

echo "=== 系统信息查看器 ==="

# 获取操作系统信息
get_os_info() {
    case "$(uname -s)" in
        Linux*)     echo "操作系统: Linux ($(lsb_release -d 2>/dev/null | cut -f2 || echo "Unknown"))" ;;
        Darwin*)    echo "操作系统: macOS ($(sw_vers -productVersion 2>/dev/null || echo "Unknown"))" ;;
        MINGW*)     echo "操作系统: Windows (Git Bash)" ;;
        *)          echo "操作系统: $(uname -s)" ;;
    esac
}

# 获取硬件信息
get_hardware_info() {
    echo "主机名: $(hostname)"
    echo "架构: $(uname -m)"
    
    # CPU信息
    if [ -f /proc/cpuinfo ]; then
        cpu_model=$(grep "model name" /proc/cpuinfo | head -1 | cut -d':' -f2 | sed 's/^ *//')
        cpu_cores=$(grep -c "processor" /proc/cpuinfo)
        echo "CPU: $cpu_model ($cpu_cores 核心)"
    elif command -v sysctl >/dev/null 2>&1; then
        cpu_model=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Unknown")
        cpu_cores=$(sysctl -n hw.ncpu 2>/dev/null || echo "Unknown")
        echo "CPU: $cpu_model ($cpu_cores 核心)"
    fi
    
    # 内存信息
    if [ -f /proc/meminfo ]; then
        total_mem=$(grep "MemTotal" /proc/meminfo | awk '{printf "%.1f GB", $2/1024/1024}')
        echo "内存: $total_mem"
    elif command -v sysctl >/dev/null 2>&1; then
        total_mem=$(sysctl -n hw.memsize 2>/dev/null | awk '{printf "%.1f GB", $1/1024/1024/1024}')
        echo "内存: $total_mem"
    fi
}

# 获取运行时间
get_uptime() {
    if command -v uptime >/dev/null 2>&1; then
        echo "运行时间: $(uptime -p 2>/dev/null || uptime)"
    fi
}

# 主函数
main() {
    get_os_info
    get_hardware_info
    get_uptime
    
    echo
    echo "✅ 系统信息获取完成"
}

main
EOF

    # 脚本2：磁盘使用情况
    cat > "$scripts_dir/disk_usage.sh" << 'EOF'
#!/bin/bash
# Name: 磁盘使用情况检查器
# Description: 检查磁盘空间使用情况并发出警告
# Author: Test Author
# Version: 1.0.0

echo "=== 磁盘使用情况检查器 ==="

# 检查磁盘使用情况
check_disk_usage() {
    local warning_threshold=80
    local critical_threshold=90
    
    echo "磁盘使用情况 (警告阈值: ${warning_threshold}%, 严重阈值: ${critical_threshold}%):"
    echo
    
    # 获取磁盘使用情况
    if command -v df >/dev/null 2>&1; then
        # 使用 df 命令
        df -h | awk 'NR==1 {print $0}' # 打印标题行
        
        df -h | awk -v warn="$warning_threshold" -v crit="$critical_threshold" '
        NR>1 && $5!="" {
            gsub(/%/, "", $5)
            usage = $5 + 0
            
            if (usage >= crit) {
                status = "🔴 严重"
            } else if (usage >= warn) {
                status = "🟡 警告"
            } else {
                status = "🟢 正常"
            }
            
            printf "%-20s %-8s %-8s %-8s %-6s %s\n", $1, $2, $3, $4, $5"%", status
        }'
    else
        echo "❌ 无法获取磁盘使用信息 (df 命令不可用)"
        return 1
    fi
    
    echo
}

# 查找大文件
find_large_files() {
    echo "查找大文件 (>100MB)..."
    
    # 定义搜索路径
    local search_paths=("/home" "/var" "/tmp" "/usr")
    
    for path in "${search_paths[@]}"; do
        if [ -d "$path" ]; then
            echo "搜索路径: $path"
            if command -v find >/dev/null 2>&1; then
                find "$path" -type f -size +100M -exec ls -lh {} \; 2>/dev/null | head -5 | while read -r line; do
                    echo "  📁 $line"
                done
            fi
        fi
    done
    
    echo
}

# 清理建议
cleanup_suggestions() {
    echo "💡 清理建议："
    echo "  • 清空回收站"
    echo "  • 删除临时文件 (/tmp)"
    echo "  • 清理系统日志"
    echo "  • 卸载不需要的软件"
    echo "  • 移动大文件到外部存储"
}

# 主函数
main() {
    check_disk_usage
    
    echo "---"
    find_large_files
    
    echo "---"
    cleanup_suggestions
    
    echo
    echo "✅ 磁盘检查完成"
}

main
EOF

    # 脚本3：网络工具
    cat > "$scripts_dir/network_tools.sh" << 'EOF'
#!/bin/bash
# Name: 网络诊断工具
# Description: 基本的网络连接和诊断工具
# Author: Test Author
# Version: 1.0.0

echo "=== 网络诊断工具 ==="

# 检查必需的工具
check_dependencies() {
    local missing_tools=()
    
    for tool in ping curl; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_tools+=("$tool")
        fi
    done
    
    if [ ${#missing_tools[@]} -gt 0 ]; then
        echo "❌ 缺少必需工具: ${missing_tools[*]}"
        echo "请安装这些工具后重新运行"
        return 1
    fi
    
    return 0
}

# 网络连接测试
test_connectivity() {
    echo "🌐 网络连接测试..."
    
    local test_hosts=("8.8.8.8" "114.114.114.114" "google.com" "baidu.com")
    
    for host in "${test_hosts[@]}"; do
        if ping -c 1 -W 3 "$host" >/dev/null 2>&1; then
            echo "  ✅ $host - 连接正常"
        else
            echo "  ❌ $host - 连接失败"
        fi
    done
    
    echo
}

# 获取本机IP信息
get_ip_info() {
    echo "🏠 本机IP信息..."
    
    # 本地IP地址
    if command -v ip >/dev/null 2>&1; then
        local_ip=$(ip route get 8.8.8.8 | awk '/src/ {print $7}' | head -1)
    elif command -v ifconfig >/dev/null 2>&1; then
        local_ip=$(ifconfig | grep "inet " | grep -v "127.0.0.1" | awk '{print $2}' | head -1)
    fi
    
    if [ -n "$local_ip" ]; then
        echo "  本地IP: $local_ip"
    else
        echo "  本地IP: 无法获取"
    fi
    
    # 公网IP地址
    if command -v curl >/dev/null 2>&1; then
        public_ip=$(curl -s --max-time 5 ifconfig.me 2>/dev/null || curl -s --max-time 5 ipinfo.io/ip 2>/dev/null)
        if [ -n "$public_ip" ]; then
            echo "  公网IP: $public_ip"
        else
            echo "  公网IP: 无法获取"
        fi
    fi
    
    echo
}

# 端口扫描 (简单版)
simple_port_scan() {
    echo "🔍 常用端口检查 (本地)..."
    
    local common_ports=(22 80 443 3389 5432 3306 6379 27017)
    
    for port in "${common_ports[@]}"; do
        if command -v netstat >/dev/null 2>&1; then
            if netstat -tuln 2>/dev/null | grep ":$port " >/dev/null; then
                echo "  ✅ 端口 $port - 开放"
            else
                echo "  ❌ 端口 $port - 关闭"
            fi
        elif command -v ss >/dev/null 2>&1; then
            if ss -tuln 2>/dev/null | grep ":$port " >/dev/null; then
                echo "  ✅ 端口 $port - 开放"
            else
                echo "  ❌ 端口 $port - 关闭"
            fi
        else
            echo "  ⚠️  无法检查端口状态 (缺少 netstat 或 ss 命令)"
            break
        fi
    done
    
    echo
}

# DNS解析测试
test_dns() {
    echo "🔍 DNS解析测试..."
    
    local test_domains=("google.com" "github.com" "baidu.com")
    
    for domain in "${test_domains[@]}"; do
        if command -v nslookup >/dev/null 2>&1; then
            if nslookup "$domain" >/dev/null 2>&1; then
                echo "  ✅ $domain - 解析成功"
            else
                echo "  ❌ $domain - 解析失败"
            fi
        elif command -v dig >/dev/null 2>&1; then
            if dig "$domain" >/dev/null 2>&1; then
                echo "  ✅ $domain - 解析成功"
            else
                echo "  ❌ $domain - 解析失败"
            fi
        else
            echo "  ⚠️  无法测试DNS解析 (缺少 nslookup 或 dig 命令)"
            break
        fi
    done
    
    echo
}

# 主函数
main() {
    if ! check_dependencies; then
        exit 1
    fi
    
    test_connectivity
    get_ip_info
    simple_port_scan
    test_dns
    
    echo "✅ 网络诊断完成"
}

main
EOF

    # 设置脚本执行权限
    chmod +x "$scripts_dir"/*.sh
    
    print_success "示例脚本文件创建完成"
}

# 生成 info.json 文件
create_info_json() {
    local plugin_dir="$1"
    
    print_info "生成 info.json 文件..."
    
    # 处理标签数组
    local tags_array=""
    IFS=',' read -ra TAGS <<< "$plugin_tags"
    for i in "${!TAGS[@]}"; do
        if [ $i -eq 0 ]; then
            tags_array="\"${TAGS[i]}\""
        else
            tags_array="$tags_array, \"${TAGS[i]}\""
        fi
    done
    
    cat > "$plugin_dir/info.json" << EOF
{
  "id": "$plugin_id",
  "name": "$plugin_name",
  "version": "$plugin_version",
  "description": "$plugin_description",
  "author": "$plugin_author",
  "scripts": [
    {
      "name": "系统信息查看器",
      "file": "system_info.sh",
      "description": "显示详细的系统信息，包括操作系统、硬件和运行时间",
      "executable": true
    },
    {
      "name": "磁盘使用情况检查器",
      "file": "disk_usage.sh",
      "description": "检查磁盘空间使用情况并提供清理建议",
      "executable": true
    },
    {
      "name": "网络诊断工具",
      "file": "network_tools.sh",
      "description": "基本的网络连接测试和诊断工具",
      "executable": true
    }
  ],
  "dependencies": [],
  "tags": [$tags_array],
  "min_geektools_version": "$min_version",
  "homepage_url": null,
  "repository_url": null,
  "license": null
}
EOF

    # 验证 JSON 格式
    if command -v python3 >/dev/null 2>&1; then
        if python3 -m json.tool "$plugin_dir/info.json" >/dev/null 2>&1; then
            print_success "info.json 格式验证通过"
        else
            print_error "info.json 格式错误"
            exit 1
        fi
    elif command -v python >/dev/null 2>&1; then
        if python -m json.tool "$plugin_dir/info.json" >/dev/null 2>&1; then
            print_success "info.json 格式验证通过"
        else
            print_error "info.json 格式错误"
            exit 1
        fi
    else
        print_warning "无法验证 JSON 格式 (缺少 python 命令)"
    fi
}

# 创建插件包
create_plugin_package() {
    local plugin_dir="$1"
    local package_name="${plugin_id}.tar.gz"
    
    print_info "创建插件包..."
    
    # 进入插件目录
    cd "$plugin_dir"
    
    # 创建 tar.gz 包
    if tar -czf "../$package_name" .; then
        cd ..
        print_success "插件包创建成功: $package_name"
        
        # 显示包内容
        echo
        print_info "插件包内容："
        tar -tzf "$package_name" | while read -r file; do
            echo "  📄 $file"
        done
        
        # 显示包大小
        local package_size=$(ls -lh "$package_name" | awk '{print $5}')
        echo
        print_info "插件包大小: $package_size"
        
        return 0
    else
        cd ..
        print_error "插件包创建失败"
        return 1
    fi
}

# 运行测试
run_tests() {
    local plugin_dir="$1"
    
    print_info "运行插件测试..."
    
    echo
    echo "🧪 测试脚本执行..."
    
    for script in "$plugin_dir/scripts"/*.sh; do
        script_name=$(basename "$script")
        echo
        echo "--- 测试: $script_name ---"
        
        if [ -x "$script" ]; then
            echo "执行 $script_name (前5行输出):"
            timeout 10s bash "$script" 2>&1 | head -5 || {
                echo "⚠️  脚本执行超时或出错"
            }
        else
            print_error "$script_name 没有执行权限"
        fi
    done
    
    echo
    print_success "插件测试完成"
}

# 生成 README 文件
create_readme() {
    local plugin_dir="$1"
    
    print_info "生成 README.md 文件..."
    
    cat > "$plugin_dir/README.md" << EOF
# $plugin_name

## 描述

$plugin_description

## 版本

- **当前版本**: $plugin_version
- **作者**: $plugin_author
- **最低GeekTools版本**: $min_version

## 功能

这个插件包含以下脚本：

### 1. 系统信息查看器 (system_info.sh)
- 显示操作系统信息
- 显示硬件信息（CPU、内存）
- 显示系统运行时间

### 2. 磁盘使用情况检查器 (disk_usage.sh)
- 检查磁盘空间使用情况
- 发出使用率警告
- 查找大文件
- 提供清理建议

### 3. 网络诊断工具 (network_tools.sh)
- 网络连接测试
- 获取本机IP信息
- 常用端口检查
- DNS解析测试

## 安装

1. 下载插件包 \`${plugin_id}.tar.gz\`
2. 在 GeekTools 中选择"插件市场" → "安装插件"
3. 选择下载的插件包文件
4. 完成安装

## 使用

安装完成后，在 GeekTools 主界面中可以看到插件提供的脚本。点击相应的脚本即可执行。

## 标签

$(echo "$plugin_tags" | sed 's/,/, /g')

## 更新日志

### v$plugin_version
- 初始版本
- 包含系统信息、磁盘检查和网络诊断功能

## 支持

如果遇到问题，请检查：
1. GeekTools 版本是否满足最低要求 ($min_version)
2. 系统是否安装了必要的工具 (ping, curl, df 等)
3. 脚本是否具有执行权限

## 许可证

此插件仅供测试和学习使用。
EOF

    print_success "README.md 文件创建完成"
}

# 显示完成信息
show_completion_info() {
    local plugin_dir="$1"
    local package_name="${plugin_id}.tar.gz"
    
    echo
    echo -e "${GREEN}🎉 插件包生成完成！${NC}"
    echo
    echo "📦 插件信息："
    echo "  • 插件ID: $plugin_id"
    echo "  • 插件名称: $plugin_name"
    echo "  • 版本: $plugin_version"
    echo "  • 作者: $plugin_author"
    echo "  • 标签: $plugin_tags"
    echo
    echo "📁 生成的文件："
    echo "  • 插件目录: $plugin_dir/"
    echo "  • 插件包: $package_name"
    echo "  • 配置文件: $plugin_dir/info.json"
    echo "  • 说明文档: $plugin_dir/README.md"
    echo "  • 脚本文件: $plugin_dir/scripts/*.sh"
    echo
    echo "🚀 下一步："
    echo "  1. 测试插件包: tar -tzf $package_name"
    echo "  2. 安装到GeekTools中进行测试"
    echo "  3. 根据需要修改脚本和配置"
    echo "  4. 重新打包: cd $plugin_dir && tar -czf ../${plugin_id}_v2.tar.gz ."
    echo
}

# 主函数
main() {
    show_banner
    
    # 检查必要工具
    if ! command -v tar >/dev/null 2>&1; then
        print_error "缺少必要工具: tar"
        exit 1
    fi
    
    # 获取用户输入
    get_user_input
    
    local plugin_dir="$plugin_id"
    
    # 创建插件
    create_plugin_structure "$plugin_dir"
    create_sample_scripts "$plugin_dir"
    create_info_json "$plugin_dir"
    create_readme "$plugin_dir"
    
    # 创建插件包
    if create_plugin_package "$plugin_dir"; then
        # 运行测试
        run_tests "$plugin_dir"
        
        # 显示完成信息
        show_completion_info "$plugin_dir"
    else
        print_error "插件包创建失败"
        exit 1
    fi
}

# 检查是否直接运行脚本
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi