# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

GeekTools Plugin Marketplace is a full-stack web application for managing and distributing plugins. It consists of:
- **Frontend**: HTML/JS/CSS with Tailwind CSS for the UI
- **Backend**: Rust-based API server using Axum framework
- **Database**: PostgreSQL with SQLx migrations
- **Proxy**: Python-based development proxy server for CORS handling

## Development Commands

### Backend (Rust)
```bash
# Navigate to server directory
cd server/

# Build the application
cargo build

# Run in development mode
cargo run

# Build for production
cargo build --release

# Run production binary
./target/release/server

# Install database migration tool
cargo install sqlx-cli --no-default-features --features postgres

# Run database migrations
sqlx migrate run

# Create new migration
sqlx migrate add <migration_name>

# Reset database (development)
sqlx migrate revert
```

### Frontend Development
```bash
# Start proxy server (handles CORS and serves static files)
python3 proxy_server.py

# Alternative: Simple HTTP server
python3 -m http.server 8080
```

### Docker Deployment
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Database Operations
```bash
# Connect to PostgreSQL
psql -U postgres

# Create database
CREATE DATABASE marketplace;

# Reset database schema
DROP SCHEMA public CASCADE; 
CREATE SCHEMA public;
```

## Project Architecture

### Backend Structure (`server/src/`)
- `main.rs` - Application entry point, router setup, CORS configuration
- `handlers/` - HTTP request handlers (auth, plugins, admin, search, health)
- `services/` - Business logic layer (auth, plugin management, storage, SMTP)
- `models/` - Data models and database entities
- `middleware/` - Auth middleware and rate limiting
- `utils/` - Configuration, validation utilities
- `migrations/` - Database schema migrations

### Key Components
- **Authentication**: JWT-based with email verification codes
- **File Upload**: Plugin tar.gz files with validation
- **Admin Panel**: User management, plugin management, SQL execution
- **Search**: Advanced search with filters and suggestions
- **Rate Limiting**: Built-in request rate limiting
- **SMTP Integration**: Email sending via Lettre crate

### Frontend Files
- `index.html` - Main plugin marketplace interface
- `admin.html` - Admin management panel
- `app.js` - Main application logic
- `admin.js` - Admin panel functionality
- `config.js` - Frontend configuration
- `proxy_server.py` - Development proxy for CORS handling

## Configuration

### Environment Variables (`server/.env`)
```bash
# Database
DATABASE_URL=postgres://username:password@localhost:5432/marketplace
DATABASE_MAX_CONNECTIONS=10

# JWT
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
JWT_ACCESS_TOKEN_EXPIRES_IN=3600
JWT_REFRESH_TOKEN_EXPIRES_IN=604800

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Storage
STORAGE_UPLOAD_PATH=./uploads
STORAGE_MAX_FILE_SIZE=104857600

# SMTP (optional)
SMTP_ENABLED=false
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password

# Logging
RUST_LOG=info
RUST_BACKTRACE=1
```

## API Endpoints

### Authentication
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - User login
- `POST /api/v1/auth/send-code` - Send email verification code
- `POST /api/v1/auth/verify-code` - Verify code and login
- `POST /api/v1/auth/refresh` - Refresh JWT token

### Plugins
- `GET /api/v1/plugins` - List all plugins with pagination
- `POST /api/v1/plugins` - Upload new plugin (authenticated)
- `POST /api/v1/plugins/upload` - Upload plugin (temporary, no auth)
- `GET /api/v1/plugins/:id` - Get plugin details
- `GET /api/v1/plugins/:id/download` - Download plugin file
- `POST /api/v1/plugins/:id/ratings` - Create plugin rating

### Admin
- `GET /api/v1/admin/dashboard` - Dashboard statistics
- `GET /api/v1/admin/users` - User management
- `GET /api/v1/admin/plugins` - Plugin management
- `POST /api/v1/admin/sql/execute` - Execute SQL queries

### Health & Monitoring
- `GET /api/v1/health` - Health check endpoint
- `GET /api/v1/metrics` - Application metrics

## Database Schema

Main tables:
- `users` - User accounts with email verification
- `plugins` - Plugin metadata and file information
- `ratings` - Plugin ratings and reviews
- `user_login_activities` - Login tracking for admin

## Testing

### Manual Testing
Access test pages:
- Frontend: http://localhost:8080
- Admin panel: http://localhost:8080/admin.html
- Health check: http://localhost:3000/api/v1/health

### Development Workflow
1. Start PostgreSQL database
2. Set up environment variables in `server/.env`
3. Run database migrations: `sqlx migrate run`
4. Start backend: `cargo run` (from server/ directory)
5. Start proxy: `python3 proxy_server.py` (from project root)
6. Access frontend at http://localhost:8080

## Key Dependencies

### Rust Crates
- `axum` - Web framework
- `sqlx` - Database toolkit
- `tokio` - Async runtime
- `jsonwebtoken` - JWT authentication
- `lettre` - SMTP email sending
- `bcrypt` - Password hashing
- `serde` - Serialization
- `validator` - Input validation

### Frontend
- Tailwind CSS - Utility-first CSS framework
- Vanilla JavaScript - No additional frameworks
- Font Awesome - Icon library

## Production Deployment

For production deployment:
1. Set strong JWT_SECRET in environment
2. Configure proper database credentials
3. Set up SMTP for email verification
4. Use reverse proxy (Nginx) for static file serving
5. Enable HTTPS/SSL certificates
6. Set up proper backup procedures for database and uploads