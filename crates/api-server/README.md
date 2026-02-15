# API Server - PostgreSQL, Authentication & Security Features

This document describes the new features added to the API server including PostgreSQL persistence, JWT authentication, rate limiting, and structured error handling.

## Features

### 1. PostgreSQL Database Persistence

All node and task data is now persisted to a PostgreSQL database instead of in-memory storage.

**Database Schema:**
- `nodes` - Network node registrations
- `tasks` - Task submissions and status
- `task_assignments` - Many-to-many relationship between tasks and nodes
- `users` - User authentication data

**Migration Files:**
- Located in `crates/api-server/migrations/`
- Automatically applied on server startup using sqlx migrations

### 2. Authentication System

Two authentication methods are supported:

#### JWT (JSON Web Tokens)
- Token-based authentication with configurable expiration
- Secure password hashing using bcrypt
- Token validation middleware

#### API Keys
- Generated on user registration
- Alternative to JWT for service-to-service communication

**Endpoints:**
- `POST /api/v1/auth/register` - Register new user
- `POST /api/v1/auth/login` - Login and receive JWT token

### 3. Rate Limiting

Custom token bucket rate limiter to prevent API abuse:
- Configurable requests per minute
- Burst capacity for traffic spikes
- IP-based tracking
- Automatic cleanup of old rate limit buckets

### 4. Structured Error Handling

Security-focused error system:
- Prevents information leakage in error messages
- Comprehensive error types (400, 401, 403, 404, 409, 422, 429, 500, 503)
- Proper logging of internal errors
- Sanitized user-facing error messages

## Configuration

### Environment Variables

Create a `.env` file in the project root with the following variables:

```bash
# Database Configuration
DATABASE_URL=postgres://username:password@localhost:5432/database_name
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=2
DB_CONNECTION_TIMEOUT_SECS=30

# JWT Configuration
JWT_SECRET=your-jwt-secret-key-change-this-in-production
JWT_EXPIRATION_HOURS=24

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10

# Server Configuration
PORT=3000
RUST_LOG=info
```

### Generate JWT Secret

Generate a secure JWT secret:

```bash
openssl rand -base64 32
```

## Database Setup

### Local Development with PostgreSQL

1. Install PostgreSQL:
```bash
# Ubuntu/Debian
sudo apt-get install postgresql postgresql-contrib

# macOS
brew install postgresql
```

2. Create database and user:
```sql
CREATE DATABASE ambient_vcp;
CREATE USER vcp_user WITH ENCRYPTED PASSWORD 'vcp_password';
GRANT ALL PRIVILEGES ON DATABASE ambient_vcp TO vcp_user;
```

3. Update `.env` with your database URL:
```bash
DATABASE_URL=postgres://vcp_user:vcp_password@localhost:5432/ambient_vcp
```

### Render.com Deployment

For deployment on Render.com (https://ambient-ai-vcp-system.onrender.com):

1. Create a PostgreSQL database in Render:
   - Go to Render Dashboard → New → PostgreSQL
   - Copy the Internal Database URL

2. Add environment variables in Render:
   - `DATABASE_URL` - from PostgreSQL instance
   - `JWT_SECRET` - generate using `openssl rand -base64 32`
   - Other optional variables as needed

3. Migrations run automatically on startup

### Docker Compose (Local Development)

```bash
# Start PostgreSQL with Docker
docker-compose up -d postgres

# Run migrations
cargo run --package api-server

# Or use sqlx CLI
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run --source crates/api-server/migrations
```

## API Usage Examples

### 1. Register a User

```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "securepassword123"
  }'
```

Response:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "testuser",
  "api_key": "vcp_aBc123XyZ456...",
  "message": "User registered successfully. Save your API key - it won't be shown again."
}
```

### 2. Login

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "securepassword123"
  }'
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

### 3. Register a Node (Protected Endpoint)

```bash
curl -X POST http://localhost:3000/api/v1/nodes \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "node_id": "node-001",
    "region": "us-west",
    "node_type": "compute",
    "capabilities": {
      "bandwidth_mbps": 1000.0,
      "cpu_cores": 8,
      "memory_gb": 32.0,
      "gpu_available": true
    }
  }'
```

### 4. Submit a Task (Protected Endpoint)

```bash
curl -X POST http://localhost:3000/api/v1/tasks \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "task_type": "computation",
    "inputs": {"data": "example"},
    "requirements": {
      "min_nodes": 1,
      "max_execution_time_sec": 300,
      "require_gpu": false,
      "require_proof": false
    }
  }'
```

## Security Best Practices

### Production Deployment

1. **JWT Secret**: Use a strong, randomly generated secret
2. **Database**: Use SSL/TLS connections (add `?sslmode=require` to DATABASE_URL)
3. **Environment Variables**: Never commit `.env` to version control
4. **Rate Limiting**: Adjust limits based on your traffic patterns
5. **HTTPS**: Always use HTTPS in production (Render.com provides this)

### Password Requirements

- Minimum 8 characters
- Enforced in validation

### Error Security

- Internal errors are logged but not exposed to users
- Database errors are sanitized
- JWT errors provide minimal information

## API Documentation

Interactive API documentation is available at:
- **Swagger UI**: http://localhost:3000/swagger-ui
- **OpenAPI JSON**: http://localhost:3000/api-docs/openapi.json

## Testing

Run the test suite:

```bash
# Unit tests
cargo test --package api-server

# With database (requires DATABASE_URL)
cargo test --package api-server -- --ignored
```

## Troubleshooting

### Database Connection Issues

```bash
# Test database connection
psql $DATABASE_URL

# Check if migrations ran
psql $DATABASE_URL -c "\dt"
```

### JWT Token Issues

- Ensure `JWT_SECRET` is set
- Check token expiration time
- Verify `Authorization: Bearer TOKEN` header format

### Rate Limiting

- Check IP address extraction (works with reverse proxies)
- Adjust `RATE_LIMIT_REQUESTS_PER_MINUTE` if needed
- Monitor rate limit bucket cleanup in logs

## Architecture

```
┌─────────────────┐
│   API Client    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Rate Limiter   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Auth Layer    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   API Handlers  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  AppState + DB  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   PostgreSQL    │
└─────────────────┘
```

## License

MIT License - See LICENSE file for details
