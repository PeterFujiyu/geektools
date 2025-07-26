# GeekTools Plugin Development Guide

## 概述

GeekTools 插件系统允许开发者创建自定义脚本包，扩展 GeekTools 的功能。插件以 `.tar.gz` 格式分发，包含脚本文件和元数据。

## 插件架构

### 插件包结构

一个标准的插件包应该具有以下目录结构：

```
plugin_package.tar.gz
├── info.json          # 插件元数据文件（必需）
└── scripts/           # 脚本目录（必需）
    ├── script1.sh     # 可执行脚本
    ├── script2.sh     # 可执行脚本
    └── ...
```

### 插件元数据 (info.json)

`info.json` 文件包含插件的所有元数据信息，这是插件包中最重要的文件。

#### 基本结构

```json
{
  "id": "unique_plugin_id",
  "name": "插件显示名称",
  "version": "1.0.0",
  "description": "插件功能描述",
  "author": "作者名称",
  "scripts": [
    {
      "name": "脚本显示名称",
      "file": "script_filename.sh",
      "description": "脚本功能描述",
      "executable": true
    }
  ],
  "dependencies": ["dependency_plugin_id"],
  "tags": ["tag1", "tag2"],
  "min_geektools_version": "0.5.0"
}
```

#### 字段说明

| 字段名 | 类型 | 必需 | 描述 |
|--------|------|------|------|
| `id` | String | ✅ | 插件唯一标识符，只能包含字母、数字、下划线和短横线 |
| `name` | String | ✅ | 插件显示名称 |
| `version` | String | ✅ | 语义化版本号 (如: "1.0.0") |
| `description` | String | ✅ | 插件功能描述 |
| `author` | String | ✅ | 插件作者 |
| `scripts` | Array | ✅ | 脚本列表，至少包含一个脚本 |
| `dependencies` | Array | ❌ | 依赖的其他插件ID列表 |
| `tags` | Array | ❌ | 插件标签，用于分类和搜索 |
| `min_geektools_version` | String | ❌ | 最低支持的 GeekTools 版本 |

#### 脚本对象结构

```json
{
  "name": "脚本显示名称",
  "file": "script_file.sh",
  "description": "脚本功能描述",
  "executable": true
}
```

| 字段名 | 类型 | 必需 | 描述 |
|--------|------|------|------|
| `name` | String | ✅ | 脚本显示名称 |
| `file` | String | ✅ | 脚本文件名（相对于 scripts/ 目录） |
| `description` | String | ✅ | 脚本功能描述 |
| `executable` | Boolean | ❌ | 是否需要可执行权限（默认: false） |

## 创建插件

### 第一步：创建插件目录结构

```bash
mkdir my_plugin
cd my_plugin
mkdir scripts
```

### 第二步：编写脚本

在 `scripts/` 目录中创建您的脚本文件。脚本必须是可执行的 shell 脚本。

#### 脚本编写规范

1. **Shebang 行**: 始终以适当的 shebang 开头
   ```bash
   #!/bin/bash
   ```

2. **元数据注释**: 建议在脚本开头添加元数据注释
   ```bash
   #!/bin/bash
   # Name: 脚本名称
   # Description: 脚本功能描述
   # Author: 作者名称
   # Version: 1.0.0
   ```

3. **错误处理**: 添加适当的错误处理
   ```bash
   set -e  # 遇到错误时退出
   
   # 检查命令是否存在
   if ! command -v some_command >/dev/null 2>&1; then
       echo "❌ 未找到 some_command 命令"
       exit 1
   fi
   ```

4. **用户友好输出**: 使用清晰的输出格式
   ```bash
   echo "=== 脚本标题 ==="
   echo "✅ 成功信息"
   echo "❌ 错误信息"
   echo "⚠️  警告信息"
   ```

### 第三步：创建 info.json

创建包含插件元数据的 `info.json` 文件：

```json
{
  "id": "my_custom_plugin",
  "name": "我的自定义插件",
  "version": "1.0.0",
  "description": "这是一个示例插件，展示如何创建自定义工具",
  "author": "Your Name",
  "scripts": [
    {
      "name": "示例脚本",
      "file": "example.sh",
      "description": "一个示例脚本",
      "executable": true
    }
  ],
  "tags": ["example", "demo"],
  "min_geektools_version": "0.5.0"
}
```

### 第四步：设置文件权限

确保脚本文件具有可执行权限：

```bash
chmod +x scripts/*.sh
```

### 第五步：创建插件包

将插件打包为 `.tar.gz` 文件：

```bash
tar -czf my_custom_plugin.tar.gz info.json scripts/
```

### 已经晕了?
尝试使用生成测试插件的脚本
```bash
bash ./docs/geektools_plugin_generator.sh
```

## 最佳实践

### 1. 命名规范

- **插件ID**: 使用小写字母、数字、下划线和短横线，如 `system_tools`, `network-utils`
- **脚本文件**: 使用描述性名称，如 `check_disk_space.sh`, `network_diagnostics.sh`
- **版本号**: 遵循语义化版本规范 (SemVer)

### 2. 脚本设计原则

#### 单一职责
每个脚本应该专注于一个特定功能：

```bash
#!/bin/bash
# Name: 磁盘空间检查
# Description: 检查磁盘空间使用情况并提供警告

# 好的例子：专注于磁盘空间检查
check_disk_space() {
    df -h | awk 'NR>1 && $5+0 > 80 {print "⚠️ " $1 " 使用率过高: " $5}'
}

check_disk_space
```

#### 跨平台兼容性
考虑不同操作系统的兼容性：

```bash
#!/bin/bash
# Name: 系统信息
# Description: 显示系统信息（跨平台）

get_os_info() {
    case "$(uname -s)" in
        Linux*)     echo "操作系统: Linux" ;;
        Darwin*)    echo "操作系统: macOS" ;;
        MINGW*)     echo "操作系统: Windows (Git Bash)" ;;
        *)          echo "操作系统: 未知" ;;
    esac
}

get_os_info
```

#### 优雅的依赖检查
检查外部依赖并提供友好的错误信息：

```bash
#!/bin/bash
# Name: 网络工具
# Description: 网络诊断工具

check_dependencies() {
    local missing_tools=()
    
    for tool in curl ping netstat; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_tools+=("$tool")
        fi
    done
    
    if [ ${#missing_tools[@]} -gt 0 ]; then
        echo "❌ 缺少必需工具: ${missing_tools[*]}"
        echo "请安装这些工具后重新运行"
        exit 1
    fi
}

check_dependencies
# 继续执行主要功能...
```

### 3. 用户体验

#### 交互式输入
提供合理的默认值和输入验证：

```bash
#!/bin/bash
# Name: 文件备份
# Description: 交互式文件备份工具

read -p "请输入要备份的目录 (默认: $HOME): " backup_dir
backup_dir=${backup_dir:-$HOME}

if [ ! -d "$backup_dir" ]; then
    echo "❌ 目录不存在: $backup_dir"
    exit 1
fi

echo "正在备份 $backup_dir ..."
```

#### 进度指示
对于长时间运行的操作，提供进度指示：

```bash
#!/bin/bash
# Name: 批量处理
# Description: 批量处理文件

files=(*.txt)
total=${#files[@]}

for i in "${!files[@]}"; do
    file="${files[i]}"
    progress=$((i + 1))
    
    echo "处理中 ($progress/$total): $file"
    # 处理文件...
    sleep 1  # 模拟处理时间
done

echo "✅ 所有文件处理完成"
```

### 4. 安全性考虑

#### 输入验证
始终验证用户输入：

```bash
#!/bin/bash
# Name: 文件删除工具
# Description: 安全的文件删除工具

validate_path() {
    local path="$1"
    
    # 检查路径是否为空
    if [ -z "$path" ]; then
        echo "❌ 路径不能为空"
        return 1
    fi
    
    # 防止删除重要系统目录
    case "$path" in
        "/" | "/bin" | "/usr" | "/etc" | "/var" | "/home")
            echo "❌ 拒绝操作系统目录: $path"
            return 1
            ;;
    esac
    
    return 0
}
```

#### 权限检查
检查必要的权限：

```bash
#!/bin/bash
# Name: 系统配置修改器
# Description: 修改系统配置（需要管理员权限）

check_permissions() {
    if [ "$(id -u)" -ne 0 ]; then
        echo "❌ 此脚本需要管理员权限"
        echo "请使用 sudo 运行此脚本"
        exit 1
    fi
}

check_permissions
```

## 示例插件

### 完整示例：系统监控插件

#### 目录结构
```
system_monitor/
├── info.json
└── scripts/
    ├── cpu_usage.sh
    ├── memory_usage.sh
    └── disk_usage.sh
```

#### info.json
```json
{
  "id": "system_monitor",
  "name": "系统监控工具",
  "version": "1.2.0",
  "description": "全面的系统资源监控工具集合",
  "author": "System Admin Team",
  "scripts": [
    {
      "name": "CPU使用率监控",
      "file": "cpu_usage.sh",
      "description": "实时显示CPU使用率",
      "executable": true
    },
    {
      "name": "内存使用率监控",
      "file": "memory_usage.sh",
      "description": "显示内存使用情况",
      "executable": true
    },
    {
      "name": "磁盘空间监控",
      "file": "disk_usage.sh",
      "description": "检查磁盘空间使用情况",
      "executable": true
    }
  ],
  "tags": ["system", "monitoring", "performance"],
  "min_geektools_version": "0.5.0"
}
```

#### scripts/cpu_usage.sh
```bash
#!/bin/bash
# Name: CPU使用率监控
# Description: 实时显示CPU使用率
# Author: System Admin Team
# Version: 1.2.0

echo "=== CPU使用率监控 ==="

# 检查平台并使用相应的命令
case "$(uname -s)" in
    Linux*)
        if command -v top >/dev/null 2>&1; then
            echo "按 Ctrl+C 退出监控"
            top -bn1 | grep "Cpu(s)" | sed "s/.*, *\([0-9.]*\)%* id.*/\1/" | awk '{print "CPU使用率: " 100 - $1 "%"}'
        elif [ -f /proc/stat ]; then
            # 使用 /proc/stat 计算CPU使用率
            cpu_usage=$(awk '/^cpu / {usage=($2+$4)*100/($2+$3+$4+$5)} END {printf "%.1f", usage}' /proc/stat)
            echo "当前CPU使用率: ${cpu_usage}%"
        else
            echo "❌ 无法获取CPU使用率信息"
        fi
        ;;
    Darwin*)
        if command -v top >/dev/null 2>&1; then
            cpu_usage=$(top -l 1 -s 0 | grep "CPU usage" | awk '{print $3}' | sed 's/%//')
            echo "当前CPU使用率: ${cpu_usage}%"
        else
            echo "❌ 无法获取CPU使用率信息"
        fi
        ;;
    *)
        echo "❌ 不支持的操作系统"
        exit 1
        ;;
esac

echo "✅ CPU监控完成"
```

## 测试插件

### 本地测试

1. **创建测试环境**:
   ```bash
   mkdir test_environment
   cd test_environment
   ```

2. **安装插件**:
   编译并运行 GeekTools，选择插件市场 → 安装插件

3. **功能测试**:
   - 验证所有脚本都可以正常执行
   - 检查输出格式是否正确
   - 测试错误处理机制

### 调试技巧

#### 启用详细日志
在脚本中添加调试输出：

```bash
#!/bin/bash
# 启用调试模式
set -x  # 显示执行的命令
set -e  # 遇到错误时退出

# 或者使用条件调试
DEBUG=${DEBUG:-false}
debug_log() {
    if [ "$DEBUG" = "true" ]; then
        echo "[DEBUG] $*" >&2
    fi
}

debug_log "开始执行脚本"
```

#### 测试脚本模板
创建一个测试脚本来验证插件功能：

```bash
#!/bin/bash
# test_plugin.sh - 插件测试脚本

PLUGIN_DIR="./my_plugin"
PLUGIN_PACKAGE="my_plugin.tar.gz"

echo "=== 插件测试脚本 ==="

# 1. 验证目录结构
echo "检查插件目录结构..."
if [ ! -f "$PLUGIN_DIR/info.json" ]; then
    echo "❌ 缺少 info.json 文件"
    exit 1
fi

if [ ! -d "$PLUGIN_DIR/scripts" ]; then
    echo "❌ 缺少 scripts 目录"
    exit 1
fi

# 2. 验证 JSON 格式
echo "验证 info.json 格式..."
if ! python -m json.tool "$PLUGIN_DIR/info.json" >/dev/null 2>&1; then
    echo "❌ info.json 格式错误"
    exit 1
fi

# 3. 检查脚本权限
echo "检查脚本可执行权限..."
for script in "$PLUGIN_DIR/scripts"/*.sh; do
    if [ ! -x "$script" ]; then
        echo "⚠️  $script 没有可执行权限"
        chmod +x "$script"
        echo "✅ 已添加可执行权限"
    fi
done

# 4. 创建插件包
echo "创建插件包..."
cd "$PLUGIN_DIR"
tar -czf "../$PLUGIN_PACKAGE" .
cd ..

echo "✅ 插件测试完成"
echo "插件包: $PLUGIN_PACKAGE"
```

## 发布插件

### 版本管理

使用语义化版本规范：
- **主版本号 (Major)**: 不兼容的API修改
- **次版本号 (Minor)**: 向后兼容的功能性新增
- **修订号 (Patch)**: 向后兼容的问题修正

示例：
- `1.0.0` - 初始版本
- `1.1.0` - 添加新功能
- `1.1.1` - 修复bug
- `2.0.0` - 重大更改，不向后兼容

### 文档要求

每个插件应该包含以下文档：

1. **README.md** (可选，但推荐)
2. **CHANGELOG.md** (记录版本变更)
3. **脚本内注释** (详细的功能说明)

## 故障排除

### 常见问题

#### 1. 插件安装失败

**问题**: "Plugin package missing info.json file"
**解决**: 确保 `info.json` 文件位于插件包的根目录

**问题**: "Failed to parse info.json"
**解决**: 验证 JSON 格式的正确性

```bash
# 验证 JSON 格式
python -m json.tool info.json
# 或者使用 jq
jq . info.json
```

#### 2. 脚本执行失败

**问题**: "Permission denied"
**解决**: 确保脚本具有可执行权限

```bash
chmod +x scripts/*.sh
```

**问题**: "command not found"
**解决**: 检查脚本的依赖项是否已安装

#### 3. 插件无法加载

**问题**: "Missing dependency"
**解决**: 确保所有依赖的插件已经安装

### 调试工具

#### 1. JSON 验证器
```bash
# 使用 Python 验证 JSON
python -c "import json; json.load(open('info.json'))"

# 使用 jq 验证和格式化 JSON
jq . info.json > formatted.json
```

#### 2. 脚本语法检查
```bash
# 检查 bash 脚本语法
bash -n script.sh

# 使用 shellcheck (如果可用)
shellcheck script.sh
```

## 高级功能

### 插件依赖管理

插件可以依赖其他插件。在 `info.json` 中指定依赖：

```json
{
  "id": "advanced_tools",
  "dependencies": ["system_monitor", "network_tools"],
  "scripts": ["..."]
}
```

### 配置文件支持

插件可以使用配置文件来存储设置：

```bash
#!/bin/bash
# Name: 配置化工具
# Description: 支持配置文件的工具

CONFIG_FILE="$HOME/.geektools/plugins/my_plugin/config.conf"

# 创建默认配置
create_default_config() {
    mkdir -p "$(dirname "$CONFIG_FILE")"
    cat > "$CONFIG_FILE" << 'EOF'
# My Plugin Configuration
default_timeout=30
enable_logging=true
log_level=info
EOF
}

# 读取配置
read_config() {
    if [ ! -f "$CONFIG_FILE" ]; then
        create_default_config
    fi
    
    # 读取配置值
    timeout=$(grep "^default_timeout=" "$CONFIG_FILE" | cut -d'=' -f2)
    logging=$(grep "^enable_logging=" "$CONFIG_FILE" | cut -d'=' -f2)
    
    echo "超时设置: ${timeout}秒"
    echo "日志启用: $logging"
}

read_config
```

## 社区和支持

### 贡献指南

1. **Fork 项目**
2. **创建功能分支**
3. **提交更改**
4. **创建 Pull Request**

### 报告问题

在 GitHub Issues 中报告问题时，请包含：

1. **插件信息** (ID, 版本)
2. **GeekTools 版本**
3. **操作系统信息**
4. **错误信息和日志**
5. **重现步骤**

### 社区资源

- **GitHub Repository**: https://github.com/your-org/geektools
- **Documentation**: https://docs.geektools.dev
- **Discord Community**: https://discord.gg/geektools

---

## 总结

GeekTools 插件系统为用户提供了强大的扩展能力。通过遵循本指南的最佳实践，您可以创建高质量、用户友好的插件。

记住插件开发的核心原则：
- **简单性**: 保持插件功能专注和易用
- **兼容性**: 确保跨平台兼容性
- **安全性**: 验证输入和处理错误
- **文档化**: 提供清晰的文档和示例

开始创建您的第一个 GeekTools 插件吧！