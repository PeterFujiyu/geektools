#!/bin/bash

# GeekTools Plugin Marketplace Deployment Script
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="geektools-marketplace"
DOCKER_COMPOSE_FILE="docker-compose.yml"
BACKUP_DIR="./backups"
LOG_FILE="./logs/deploy.log"

# Functions
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1" >> "$LOG_FILE"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1" >> "$LOG_FILE"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1" >> "$LOG_FILE"
    exit 1
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed"
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        error "Docker Compose is not installed"
    fi
    
    if ! command -v git &> /dev/null; then
        error "Git is not installed"
    fi
    
    log "Prerequisites check passed"
}

# Create necessary directories
setup_directories() {
    log "Setting up directories..."
    
    mkdir -p "$BACKUP_DIR"
    mkdir -p "./logs"
    mkdir -p "./uploads"
    mkdir -p "./ssl"
    
    log "Directories created"
}

# Backup database
backup_database() {
    log "Creating database backup..."
    
    if docker-compose ps db | grep -q "Up"; then
        BACKUP_FILE="$BACKUP_DIR/db_backup_$(date +%Y%m%d_%H%M%S).sql"
        docker-compose exec -T db pg_dump -U postgres marketplace > "$BACKUP_FILE"
        
        if [ $? -eq 0 ]; then
            log "Database backup created: $BACKUP_FILE"
        else
            warn "Database backup failed, but continuing deployment"
        fi
    else
        warn "Database container not running, skipping backup"
    fi
}

# Pull latest code
update_code() {
    log "Updating code from repository..."
    
    if [ -d ".git" ]; then
        git fetch origin
        git pull origin main
        log "Code updated successfully"
    else
        warn "Not a git repository, skipping code update"
    fi
}

# Build and deploy
deploy() {
    log "Starting deployment..."
    
    # Stop existing containers gracefully
    log "Stopping existing containers..."
    docker-compose down --timeout 30
    
    # Build new images
    log "Building new images..."
    docker-compose build --no-cache
    
    # Start services
    log "Starting services..."
    docker-compose up -d
    
    # Wait for services to be healthy
    log "Waiting for services to be ready..."
    sleep 30
    
    # Check service health
    check_health
    
    log "Deployment completed successfully"
}

# Check service health
check_health() {
    log "Checking service health..."
    
    # Check app health
    if curl -f http://localhost:3000/api/v1/health &>/dev/null; then
        log "Application is healthy"
    else
        error "Application health check failed"
    fi
    
    # Check database connection
    if docker-compose exec -T db pg_isready -U postgres -d marketplace &>/dev/null; then
        log "Database is healthy"
    else
        error "Database health check failed"
    fi
    
    log "All services are healthy"
}

# Cleanup old images and containers
cleanup() {
    log "Cleaning up old Docker images and containers..."
    
    # Remove dangling images
    docker image prune -f
    
    # Remove old containers
    docker container prune -f
    
    # Keep only the last 5 backups
    if [ -d "$BACKUP_DIR" ]; then
        find "$BACKUP_DIR" -name "*.sql" -type f | sort -r | tail -n +6 | xargs rm -f
        log "Old backups cleaned up"
    fi
    
    log "Cleanup completed"
}

# Rollback function
rollback() {
    error "Deployment failed. Attempting rollback..."
    
    # Stop current containers
    docker-compose down --timeout 30
    
    # Start with previous images (if available)
    docker-compose up -d
    
    error "Rollback completed. Please check the logs for issues."
}

# Main deployment flow
main() {
    log "Starting GeekTools Marketplace deployment"
    
    # Set trap for error handling
    trap rollback ERR
    
    check_prerequisites
    setup_directories
    backup_database
    update_code
    deploy
    cleanup
    
    log "Deployment completed successfully!"
    log "Access the application at: http://localhost"
    log "API documentation: http://localhost/api/v1/health"
    log "Monitoring: http://localhost:9090 (Prometheus), http://localhost:3001 (Grafana)"
}

# Handle command line arguments
case "${1:-deploy}" in
    "deploy")
        main
        ;;
    "backup")
        log "Creating manual backup..."
        setup_directories
        backup_database
        ;;
    "health")
        check_health
        ;;
    "cleanup")
        cleanup
        ;;
    "logs")
        docker-compose logs -f "${2:-app}"
        ;;
    "stop")
        log "Stopping all services..."
        docker-compose down
        ;;
    "restart")
        log "Restarting services..."
        docker-compose restart
        ;;
    *)
        echo "Usage: $0 {deploy|backup|health|cleanup|logs|stop|restart}"
        echo ""
        echo "Commands:"
        echo "  deploy   - Full deployment (default)"
        echo "  backup   - Create database backup"
        echo "  health   - Check service health"
        echo "  cleanup  - Clean up old Docker resources"
        echo "  logs     - Show logs (optionally specify service)"
        echo "  stop     - Stop all services"
        echo "  restart  - Restart all services"
        exit 1
        ;;
esac