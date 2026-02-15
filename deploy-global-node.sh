#!/bin/bash

# Ambient AI VCP - Global Node Deployment Script
# This script deploys a complete global VCP network

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored messages
info() {
    echo -e "${BLUE}ℹ ${1}${NC}"
}

success() {
    echo -e "${GREEN}✓ ${1}${NC}"
}

warn() {
    echo -e "${YELLOW}⚠ ${1}${NC}"
}

error() {
    echo -e "${RED}✗ ${1}${NC}"
    exit 1
}

# Banner
echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║        Ambient AI VCP - Global Node Deployment           ║"
echo "║                      v1.0.0                               ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Check prerequisites
info "Checking prerequisites..."

if ! command -v docker &> /dev/null; then
    error "Docker is not installed. Please install Docker first: https://docs.docker.com/get-docker/"
fi
success "Docker found"

if ! command -v docker-compose &> /dev/null; then
    error "Docker Compose is not installed. Please install Docker Compose: https://docs.docker.com/compose/install/"
fi
success "Docker Compose found"

# Parse command line arguments
MODE=${1:-full}

case $MODE in
    full)
        info "Deploying full global network (API + Coordinator + 4 Nodes)..."
        docker-compose up -d
        ;;
    api)
        info "Deploying API server only..."
        docker-compose up -d api-server
        ;;
    minimal)
        info "Deploying minimal setup (API + Coordinator + 1 Node)..."
        docker-compose up -d api-server coordinator node-1
        ;;
    build)
        info "Building Docker images..."
        docker-compose build
        success "Build complete"
        exit 0
        ;;
    down)
        info "Stopping all services..."
        docker-compose down
        success "All services stopped"
        exit 0
        ;;
    logs)
        info "Showing logs..."
        docker-compose logs -f
        exit 0
        ;;
    *)
        echo "Usage: $0 [full|api|minimal|build|down|logs]"
        echo ""
        echo "Modes:"
        echo "  full     - Deploy complete global network (default)"
        echo "  api      - Deploy API server only"
        echo "  minimal  - Deploy API + Coordinator + 1 Node"
        echo "  build    - Build Docker images"
        echo "  down     - Stop all services"
        echo "  logs     - Show logs"
        exit 1
        ;;
esac

# Wait for services to be healthy
info "Waiting for services to start..."
sleep 10

# Check API health
info "Checking API health..."
if curl -s -f http://localhost:3000/api/v1/health > /dev/null 2>&1; then
    success "API server is healthy!"
else
    warn "API server might still be starting. Give it a few more seconds..."
fi

# Display access information
echo ""
success "Deployment complete!"
echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                   Access Information                      ║"
echo "╠═══════════════════════════════════════════════════════════╣"
echo "║                                                           ║"
echo "║  API Endpoint:    http://localhost:3000                  ║"
echo "║  Swagger UI:      http://localhost:3000/swagger-ui       ║"
echo "║  Health Check:    http://localhost:3000/api/v1/health    ║"
echo "║  Metrics:         http://localhost:9090/metrics          ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

info "Test the API:"
echo "  curl http://localhost:3000/api/v1/health"
echo ""

info "View logs:"
echo "  docker-compose logs -f"
echo ""

info "Stop services:"
echo "  docker-compose down"
echo ""

info "View running services:"
echo "  docker-compose ps"
echo ""

# Show running services
docker-compose ps

success "Global VCP node is now running! 🚀"
