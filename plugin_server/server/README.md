# GeekTools Plugin Marketplace Server

A high-performance Rust-based backend server for the GeekTools plugin marketplace, providing RESTful APIs for plugin management, user authentication, and file storage.

## Features

- 🚀 **High Performance**: Built with Rust and Axum for maximum speed and safety
- 🔐 **Secure Authentication**: JWT-based authentication with role-based access control
- 📦 **Plugin Management**: Upload, download, search, and manage plugins
- 🗄️ **Database Support**: PostgreSQL with automatic migrations
- 📊 **Monitoring**: Built-in health checks and metrics endpoints
- 🐳 **Container Ready**: Docker and Docker Compose support
- 🔧 **Configurable**: Environment-based configuration with YAML support
- 📈 **Scalable**: Designed for horizontal scaling and microservices

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 13+
- Docker & Docker Compose (optional)

### Option 1: Docker Compose (Recommended)

```bash
# Clone and navigate to server directory
cd plugin_server/server

# Copy environment configuration
cp .env.example .env

# Start all services
docker-compose up -d

# Check service health
curl http://localhost:3000/api/v1/health
```

### Option 2: Local Development

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install pkg-config libssl-dev libpq-dev

# Install dependencies (macOS)
brew install pkg-config openssl postgresql

# Set up database
createdb marketplace

# Copy and configure environment
cp .env.example .env
# Edit .env with your database URL

# Run migrations
cargo install sqlx-cli
sqlx migrate run

# Build and run
cargo run
```

### Option 3: Pre-built Binary

```bash
# Download from releases
wget https://github.com/geektools/marketplace/releases/latest/download/geektools-marketplace-server-linux-x86_64.tar.gz

# Extract and run
tar -xzf geektools-marketplace-server-linux-x86_64.tar.gz
cd geektools-marketplace-server-*
./server
```

## Architecture

### Project Structure

```
server/
├── src/
│   ├── handlers/          # HTTP request handlers
│   ├── models/           # Data models and DTOs
│   ├── services/         # Business logic layer
│   ├── middleware/       # Authentication, rate limiting
│   ├── utils/           # Utilities and configuration
│   └── main.rs          # Application entry point
├── migrations/          # Database migrations
├── config/             # Configuration files
├── scripts/            # Build and deployment scripts
└── docker-compose.yml  # Container orchestration
```

### Technology Stack

- **Framework**: Axum (async web framework)
- **Database**: PostgreSQL with SQLx
- **Authentication**: JWT with bcrypt
- **Serialization**: Serde (JSON/YAML)
- **Logging**: Tracing
- **Validation**: Validator
- **File Handling**: Tokio + Tar + Flate2
- **Containerization**: Docker

## API Documentation

### Base URL
- Development: `http://localhost:3000/api/v1`
- Production: `https://api.geektools.dev/v1`

### Authentication Endpoints

```http
POST /auth/register
POST /auth/login
POST /auth/refresh
```

### Plugin Endpoints

```http
GET    /plugins                 # List plugins with search/filter
GET    /plugins/{id}            # Get plugin details
POST   /plugins                 # Upload plugin (requires auth)
GET    /plugins/{id}/download   # Download plugin
GET    /plugins/{id}/stats      # Get plugin statistics
GET    /plugins/{id}/ratings    # Get plugin ratings
POST   /plugins/{id}/ratings    # Create/update rating (requires auth)
```

### Search Endpoints

```http
POST   /search                  # Advanced search
GET    /search/suggestions      # Search suggestions
```

### System Endpoints

```http
GET    /health                  # Health check
GET    /metrics                 # System metrics
```

### Example Requests

#### Register User
```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "developer",
    "email": "dev@example.com",
    "password": "secure_password",
    "display_name": "Developer Name"
  }'
```

#### Upload Plugin
```bash
curl -X POST http://localhost:3000/api/v1/plugins \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -F "plugin_file=@my-plugin.tar.gz"
```

#### Search Plugins
```bash
curl "http://localhost:3000/api/v1/plugins?search=system&tag=tools&sort=downloads&page=1&limit=20"
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Server bind address | `0.0.0.0` |
| `SERVER_PORT` | Server port | `3000` |
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `JWT_SECRET` | JWT signing secret | Required |
| `STORAGE_UPLOAD_PATH` | File upload directory | `./uploads` |
| `RUST_LOG` | Log level | `info` |

### Configuration File

The server supports YAML configuration files. See `config/config.yaml` for the full example.

```yaml
server:
  host: "0.0.0.0"
  port: 3000

database:
  url: "postgres://user:pass@localhost/marketplace"
  max_connections: 10

jwt:
  secret: "your-secret-key"
  access_token_expires_in: 3600

storage:
  upload_path: "./uploads"
  max_file_size: 104857600
```

## Development

### Building

```bash
# Development build
./scripts/build.sh dev

# Release build
./scripts/build.sh release

# Cross-platform build
./scripts/build.sh cross

# Build everything
./scripts/build.sh all
```

### Testing

```bash
# Run tests
cargo test

# Run with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check all
./scripts/build.sh check
```

### Database Migrations

```bash
# Install sqlx-cli
cargo install sqlx-cli

# Create migration
sqlx migrate add create_users_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Deployment

### Docker Deployment

```bash
# Deploy with monitoring stack
./scripts/deploy.sh

# Deploy specific services
docker-compose up -d app db

# View logs
./scripts/deploy.sh logs app

# Health check
./scripts/deploy.sh health
```

### Production Deployment

1. **Prepare Environment**
   ```bash
   # Set production environment variables
   export DATABASE_URL="postgres://..."
   export JWT_SECRET="..."
   ```

2. **Build Release**
   ```bash
   ./scripts/build.sh release
   ```

3. **Deploy**
   ```bash
   ./scripts/deploy.sh
   ```

4. **Monitor**
   - Application: http://localhost:3000/api/v1/health
   - Prometheus: http://localhost:9090
   - Grafana: http://localhost:3001

### Scaling Considerations

- **Database**: Use connection pooling and read replicas
- **Storage**: Implement CDN for file downloads
- **Cache**: Add Redis for session management
- **Load Balancing**: Use multiple app instances behind Nginx
- **Monitoring**: Set up alerting for critical metrics

## Security

### Best Practices Implemented

- **Input Validation**: All inputs validated using Validator crate
- **SQL Injection Prevention**: Parameterized queries with SQLx
- **Authentication**: JWT with configurable expiration
- **File Upload Security**: Type validation and size limits
- **CORS**: Configurable origins and methods
- **Rate Limiting**: Built-in rate limiting middleware
- **Security Headers**: Comprehensive security headers in Nginx

### Security Checklist for Production

- [ ] Change default JWT secret
- [ ] Use strong database passwords
- [ ] Enable SSL/TLS (HTTPS)
- [ ] Configure firewall rules
- [ ] Set up log monitoring
- [ ] Enable backup encryption
- [ ] Implement audit logging
- [ ] Regular security updates

## Monitoring and Logging

### Metrics

The server exposes metrics at `/api/v1/metrics`:

- Request counts and duration
- Database connection pool status
- Custom business metrics
- System resource usage

### Logging

Structured logging with configurable levels:

```bash
# Set log level
export RUST_LOG=debug

# JSON logging for production
export RUST_LOG=info,tower_http=debug
```

### Health Checks

- **Application**: `/api/v1/health`
- **Database**: Connection pool status
- **Storage**: File system availability

## Troubleshooting

### Common Issues

1. **Database Connection Failed**
   ```bash
   # Check database is running
   docker-compose ps db
   
   # Check connection
   psql $DATABASE_URL -c "SELECT 1;"
   ```

2. **Permission Denied for Uploads**
   ```bash
   # Fix upload directory permissions
   chmod 755 uploads/
   chown -R $(whoami) uploads/
   ```

3. **Port Already in Use**
   ```bash
   # Find process using port
   lsof -i :3000
   
   # Change port in configuration
   export SERVER_PORT=3001
   ```

### Performance Issues

1. **Slow Database Queries**
   - Enable query logging: `RUST_LOG=sqlx=debug`
   - Check database indexes
   - Monitor connection pool usage

2. **High Memory Usage**
   - Reduce max_connections in database config
   - Implement streaming for large file uploads
   - Add memory limits in Docker

3. **High CPU Usage**
   - Check for CPU-intensive operations in logs
   - Optimize database queries
   - Consider horizontal scaling

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run quality checks: `./scripts/build.sh check`
6. Submit a pull request

### Development Guidelines

- Follow Rust naming conventions
- Write tests for new features
- Update documentation
- Use semantic versioning
- Add migration scripts for database changes

## License

MIT License - see LICENSE file for details

## Support

- 📧 Email: support@geektools.dev
- 🐛 Issues: GitHub Issues
- 📚 Documentation: https://docs.geektools.dev
- 💬 Community: Discord server