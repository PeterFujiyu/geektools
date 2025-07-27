# Quick Start Guide

## 启动后端服务器

### 方法 1: 本地开发 (推荐用于测试)

1. **安装 PostgreSQL**
   ```bash
   # macOS
   brew install postgresql
   brew services start postgresql
   
   # Ubuntu/Debian
   sudo apt-get install postgresql postgresql-contrib
   sudo systemctl start postgresql
   ```

2. **创建数据库**
   ```bash
   createdb marketplace
   ```

3. **配置环境变量**
   ```bash
   cp .env.example .env
   # 编辑 .env 文件，设置 DATABASE_URL
   ```

4. **运行数据库迁移**
   ```bash
   # 安装 sqlx-cli
   cargo install sqlx-cli --no-default-features --features postgres
   
   # 运行迁移
   sqlx migrate run --database-url "postgres://postgres@localhost/marketplace"
   ```

5. **启动服务器**
   ```bash
   cargo run
   ```

### 方法 2: Docker (生产环境)

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f app
```

## 启动前端

在另一个终端中：

```bash
# 返回前端目录
cd ..

# 启动前端服务器 (选择其中一个)
python -m http.server 8080
# 或
npx serve . -p 8080
```

## 访问应用

- **前端**: http://localhost:8080
- **后端 API**: http://localhost:3000/api/v1
- **健康检查**: http://localhost:3000/api/v1/health

## 快速测试

### 1. 健康检查
```bash
curl http://localhost:3000/api/v1/health
```

### 2. 注册用户
```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "password123",
    "display_name": "Test User"
  }'
```

### 3. 获取插件列表
```bash
curl http://localhost:3000/api/v1/plugins
```

## 常见问题

### 端口冲突
如果端口被占用，可以修改配置：
```bash
# 修改后端端口
export SERVER_PORT=3001

# 修改前端端口
python -m http.server 8081
```

### 数据库连接失败
检查 PostgreSQL 是否正常运行：
```bash
pg_isready -h localhost -p 5432
```

### CORS 错误
确保前端端口在后端 CORS 配置中：
- 检查 `src/main.rs` 中的 CORS 设置
- 默认允许 localhost:8080 和 localhost:3000

## 开发提示

1. **代码热重载**: 使用 `cargo watch -x run` 实现代码自动重载
2. **数据库重置**: 使用 `sqlx migrate revert` 回滚迁移
3. **日志级别**: 设置 `RUST_LOG=debug` 查看详细日志
4. **API 文档**: 参考 `README.md` 中的完整 API 文档

现在你可以访问 http://localhost:8080 开始使用插件市场了！