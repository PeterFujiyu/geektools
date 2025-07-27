# GeekTools Plugin Marketplace 实现文档

## 概述

GeekTools 插件市场是一个基于 Rust/Axum 框架开发的现代化插件分发系统，提供完整的插件上传、搜索、评分、管理和下载功能。

## 🏗️ 技术架构

### 系统组件

```
┌─────────────────┐    HTTP/WebSockets   ┌──────────────────┐
│   前端界面      │ ◄─────────────────► │   代理服务器      │
│  (Tailwind CSS) │                      │  (Python HTTP)   │
└─────────────────┘                      └──────────────────┘
                                                  │
                                                  ▼
                    ┌─────────────────────────────────────────┐
                    │          Rust 后端服务                  │
                    │        (Axum + SQLx)                   │
                    └─────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌───────────────┐ ┌──────────────┐ ┌──────────────┐
            │  PostgreSQL   │ │ 本地文件存储  │ │   JWT 认证   │
            │   数据库      │ │ (插件文件)    │ │  (邮箱验证)  │
            └───────────────┘ └──────────────┘ └──────────────┘
```

### 技术栈

- **后端框架**: Rust + Axum (异步Web框架)
- **数据库**: PostgreSQL (生产) / SQLite (开发)
- **数据库ORM**: SQLx (类型安全的SQL查询)
- **认证系统**: JWT + bcrypt (邮箱验证码登录)
- **文件存储**: 本地文件系统 (支持扩展到云存储)
- **前端**: 原生HTML + Tailwind CSS + JavaScript
- **代理服务器**: Python HTTP (解决CORS问题)
- **容器化**: Docker + Docker Compose

## 📊 数据库设计

### 核心表结构

#### 1. 插件表 (plugins)
```sql
CREATE TABLE plugins (
    id VARCHAR(255) PRIMARY KEY,                    -- 插件唯一ID
    name VARCHAR(255) NOT NULL,                     -- 插件名称
    description TEXT,                               -- 插件描述
    author VARCHAR(255) NOT NULL,                   -- 作者
    current_version VARCHAR(50) NOT NULL,           -- 当前版本
    created_at TIMESTAMPTZ DEFAULT NOW(),           -- 创建时间
    updated_at TIMESTAMPTZ DEFAULT NOW(),           -- 更新时间
    downloads INTEGER DEFAULT 0,                    -- 下载次数
    rating NUMERIC(3,2) DEFAULT 0.00,              -- 平均评分
    status plugin_status DEFAULT 'active',          -- 状态枚举
    min_geektools_version VARCHAR(50),              -- 最低版本要求
    homepage_url TEXT,                              -- 主页URL
    repository_url TEXT,                            -- 仓库URL
    license VARCHAR(255)                            -- 许可证
);

CREATE TYPE plugin_status AS ENUM ('active', 'deprecated', 'banned');
```

#### 2. 插件版本表 (plugin_versions)
```sql
CREATE TABLE plugin_versions (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    changelog TEXT DEFAULT '',
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    downloads INTEGER DEFAULT 0,
    is_stable BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(plugin_id, version)
);
```

#### 3. 插件脚本表 (plugin_scripts)
```sql
CREATE TABLE plugin_scripts (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    file_name VARCHAR(255) NOT NULL,
    description TEXT,
    executable BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### 4. 插件标签表 (plugin_tags)
```sql
CREATE TABLE plugin_tags (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(plugin_id, tag)
);
```

#### 5. 插件依赖表 (plugin_dependencies)
```sql
CREATE TABLE plugin_dependencies (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    dependency_id VARCHAR(255) NOT NULL,
    min_version VARCHAR(50),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(plugin_id, dependency_id)
);
```

#### 6. 用户表 (users)
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    role user_role DEFAULT 'user',
    status user_status DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TYPE user_role AS ENUM ('user', 'admin');
CREATE TYPE user_status AS ENUM ('active', 'banned');
```

#### 7. 插件评分表 (plugin_ratings)
```sql
CREATE TABLE plugin_ratings (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(plugin_id, user_id)
);
```

#### 8. 验证码表 (verification_codes)
```sql
CREATE TABLE verification_codes (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    code VARCHAR(6) NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### 9. 登录活动表 (login_activities)
```sql
CREATE TABLE login_activities (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    email VARCHAR(255) NOT NULL,
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

## 🚀 API 接口设计

### 认证接口

#### POST /api/v1/auth/send-code
发送邮箱验证码
```json
// 请求
{
  "email": "user@example.com"
}

// 响应
{
  "success": true,
  "data": {
    "message": "验证码已生成，请查看下方显示的验证码",
    "code": "123456"  // 开发模式下直接返回验证码
  }
}
```

#### POST /api/v1/auth/verify-code
验证码登录
```json
// 请求
{
  "email": "user@example.com",
  "code": "123456"
}

// 响应
{
  "success": true,
  "data": {
    "token": "jwt_token_here",
    "user": {
      "id": 1,
      "email": "user@example.com",
      "display_name": "User Name",
      "role": "user"
    }
  },
  "message": "登录成功"
}
```

### 插件接口

#### GET /api/v1/plugins
获取插件列表
```
查询参数:
- page: 页码 (默认: 1)
- limit: 每页数量 (默认: 20)
- search: 搜索关键词
- category: 分类筛选
- sort: 排序方式 (downloads|rating|name|created_at|updated_at)
- order: 排序顺序 (asc|desc)
```

```json
// 响应
{
  "success": true,
  "data": {
    "plugins": [
      {
        "id": "system_monitor_demo",
        "name": "系统监控演示插件",
        "description": "一个功能完整的系统监控工具集合",
        "author": "GeekTools 开发团队",
        "current_version": "1.2.0",
        "downloads": 0,
        "rating": 0,
        "tags": ["system", "monitoring", "performance", "tools"],
        "created_at": "2025-07-27T09:24:44Z",
        "updated_at": "2025-07-27T09:24:44Z"
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total": 1,
      "pages": 1
    }
  }
}
```

#### GET /api/v1/plugins/{id}
获取插件详情
```json
// 响应
{
  "success": true,
  "data": {
    "id": "system_monitor_demo",
    "name": "系统监控演示插件",
    "description": "一个功能完整的系统监控工具集合",
    "author": "GeekTools 开发团队",
    "current_version": "1.2.0",
    "downloads": 0,
    "rating": 0,
    "tags": ["system", "monitoring", "performance", "tools"],
    "min_geektools_version": "0.5.0",
    "homepage_url": "https://github.com/geektools/system-monitor-demo",
    "repository_url": "https://github.com/geektools/system-monitor-demo.git",
    "license": "MIT",
    "versions": [
      {
        "version": "1.2.0",
        "changelog": "",
        "file_size": 2981,
        "downloads": 0,
        "is_stable": true,
        "created_at": "2025-07-27T09:24:44Z"
      }
    ],
    "scripts": [
      {
        "name": "CPU使用率监控",
        "file": "cpu_monitor.sh",
        "description": "实时显示系统CPU使用率，支持多核心显示",
        "executable": true
      }
    ],
    "dependencies": [],
    "created_at": "2025-07-27T09:24:44Z",
    "updated_at": "2025-07-27T09:24:44Z"
  }
}
```

#### POST /api/v1/plugins/upload
上传插件包 (需要认证)
```
Content-Type: multipart/form-data
Authorization: Bearer <jwt_token>

Form Data:
- plugin_file: 插件包文件 (.tar.gz 格式)
```

```json
// 响应
{
  "success": true,
  "data": {
    "plugin_id": "system_monitor_demo",
    "upload_id": "uuid-here",
    "version": "1.2.0"
  },
  "message": "Plugin uploaded successfully"
}
```

#### GET /api/v1/plugins/{id}/download
下载插件包
```
查询参数:
- version: 指定版本 (可选，默认为最新版本)
```

### 评分接口

#### POST /api/v1/plugins/{id}/ratings
提交评分 (需要认证)
```json
// 请求
{
  "rating": 5,
  "review": "非常好用的插件！"
}

// 响应
{
  "success": true,
  "data": {
    "id": 1,
    "plugin_id": "system_monitor_demo",
    "user_id": 1,
    "username": "用户",
    "rating": 5,
    "review": "非常好用的插件！",
    "created_at": "2025-07-27T09:19:45Z",
    "updated_at": "2025-07-27T09:19:45Z"
  },
  "message": "Rating created successfully"
}
```

#### GET /api/v1/plugins/{id}/ratings
获取插件评分列表
```
查询参数:
- page: 页码 (默认: 1)
- limit: 每页数量 (默认: 20)
```

### 管理员接口

#### GET /api/v1/admin/dashboard
管理员仪表板 (需要管理员权限)
```json
// 响应
{
  "success": true,
  "data": {
    "total_users": 5,
    "total_plugins": 3,
    "total_downloads": 10,
    "weekly_users": 2,
    "weekly_plugins": 1,
    "weekly_downloads": 5
  }
}
```

#### GET /api/v1/admin/users
用户管理
#### PUT /api/v1/admin/users/{id}
更新用户信息
#### DELETE /api/v1/admin/plugins/{id}
删除插件
#### PUT /api/v1/admin/users/{id}/ban
封禁/解封用户

### 健康检查接口

#### GET /api/v1/health
系统健康检查
```json
// 响应
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "1.0.0",
    "timestamp": "2025-07-27T09:15:18Z",
    "services": {
      "database": "healthy",  
      "storage": "healthy"
    }
  }
}
```

#### GET /api/v1/metrics
系统指标
```json
// 响应
{
  "success": true,
  "data": {
    "total_plugins": 3,
    "total_downloads": 10,
    "total_users": 5,
    "weekly_new": 1
  }
}
```

## 🔧 部署方式

### 方式一：Docker Compose (推荐)

1. 克隆项目并进入服务器目录：
```bash
cd plugin_server/
```

2. 使用 Docker Compose 启动：
```bash
docker-compose up -d
```

服务访问地址：
- 前端界面: http://localhost:8080
- 后端API: http://localhost:3000
- 数据库: localhost:5432

### 方式二：本地开发模式

1. 启动数据库：
```bash
docker run -d --name postgres \
  -e POSTGRES_DB=geektools_marketplace \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=password \
  -p 5432:5432 postgres:15
```

2. 启动后端服务：
```bash
cd server/
cargo run
```

3. 启动代理服务器：
```bash
python3 proxy_server.py
```

访问地址：http://localhost:8080

### 方式三：编译部署

1. 编译后端：
```bash
cd server/
cargo build --release
```

2. 部署二进制文件：
```bash
./target/release/server
```

## 🛡️ 安全特性

### 认证与授权
- JWT令牌认证
- 基于角色的访问控制 (RBAC)
- 邮箱验证码登录 (无需密码)
- 请求频率限制

### 数据安全
- SQL注入防护 (SQLx参数化查询)
- XSS防护 (输入验证和输出转义)
- CORS配置
- 文件上传安全检查

### 管理员功能
- 用户管理 (查看、修改、封禁)
- 插件管理 (删除、审核)
- 系统监控 (登录活动、SQL控制台)
- 审计日志

## 📈 性能优化

### 数据库优化
- 索引优化 (id, email, created_at等)
- 分页查询减少内存占用
- 连接池管理
- 查询缓存

### 文件处理
- 流式文件上传
- 文件哈希验证
- 压缩存储
- 清理临时文件

### 前端优化
- 代理服务器解决CORS问题
- 静态资源缓存
- 懒加载和分页
- 响应式设计

## 🔍 监控与日志

### 系统监控
- 健康检查端点
- 数据库连接状态
- 文件系统状态
- 服务指标统计

### 日志记录
- 结构化日志 (tracing)
- 请求/响应日志
- 错误日志追踪
- 性能监控

### 管理界面
- 实时系统状态
- 用户活动监控
- 插件使用统计
- 错误报告

## 🚀 扩展性设计

### 水平扩展
- 无状态服务设计
- 数据库读写分离支持
- 负载均衡就绪
- 缓存层支持

### 功能扩展
- 插件分类系统
- 评论系统
- 收藏功能
- 推荐算法
- 插件市场统计

### 集成能力
- GitHub集成
- CI/CD支持
- Webhook通知
- API扩展接口

---

## 总结

该插件市场实现了完整的插件生态系统，包括：

✅ **完整的插件管理** - 上传、搜索、下载、版本控制  
✅ **用户认证系统** - 邮箱验证码登录、角色管理  
✅ **评分评价系统** - 5星评分、文字评价、统计分析  
✅ **管理员功能** - 用户管理、插件管理、系统监控  
✅ **现代化技术栈** - Rust异步框架、类型安全、高性能  
✅ **完善的API设计** - RESTful接口、标准化响应  
✅ **安全性保障** - 认证授权、输入验证、审计日志  
✅ **部署便利性** - Docker支持、多种部署方式  

该实现为 GeekTools 提供了一个功能完整、安全可靠、易于扩展的插件分发平台。