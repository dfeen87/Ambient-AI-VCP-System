# Render.com Deployment Guide

This guide explains how to deploy the Ambient AI VCP API Server on Render.com with PostgreSQL.

## Current Deployment

**Live URL**: https://ambient-ai-vcp-system.onrender.com

> Note: Render service names (for example `ambient-ai-vcp-api`) may differ from the public URL slug.

## Prerequisites

- Render.com account (free tier available)
- GitHub repository connected to Render

## Deployment Methods

### Method 1: Using render.yaml (Recommended)

The repository includes a `render.yaml` blueprint that automatically configures:
- PostgreSQL database
- Web service with all environment variables
- Health checks

**Steps:**

1. **Connect Repository to Render**
   - Go to [Render Dashboard](https://dashboard.render.com/)
   - Click "New" → "Blueprint"
   - Connect your GitHub repository
   - Select `ambient-ai-vcp-system` repo

2. **Review Configuration**
   - Render will detect `render.yaml`
   - Review the services:
     - `ambient-ai-vcp-db` (PostgreSQL database)
     - `ambient-ai-vcp-api` (Web service)

3. **Set Secret Environment Variable**
   - The `JWT_SECRET` is set to auto-generate
   - You can manually set it to a secure value:
     ```bash
     # Generate a secure secret
     openssl rand -base64 32
     ```
   - In Render dashboard, go to your service → Environment
   - Update `JWT_SECRET` with your generated value

4. **Deploy**
   - Click "Apply"
   - Render will:
     - Create PostgreSQL database
     - Build Docker image
     - Deploy the service
     - Run migrations automatically

5. **Verify Deployment**
   - Check service logs for any errors
   - Visit health endpoint: `https://your-service.onrender.com/api/v1/health`
   - Visit Swagger UI: `https://your-service.onrender.com/swagger-ui`

### Method 2: Manual Setup

If you prefer manual setup:

**Step 1: Create PostgreSQL Database**

1. In Render Dashboard, click "New" → "PostgreSQL"
2. Configure:
   - Name: `ambient-ai-vcp-db`
   - Database: `ambient_vcp`
   - User: `vcp_user`
   - Region: Oregon (or your preferred region)
   - Plan: Starter (free tier)
3. Create database
4. Copy the "Internal Database URL"

**Step 2: Create Web Service**

1. Click "New" → "Web Service"
2. Connect GitHub repository
3. Configure:
   - Name: `ambient-ai-vcp-api`
   - Environment: Docker
   - Region: Oregon (same as database)
   - Branch: main
   - Docker Context: `.`
   - Dockerfile Path: `./Dockerfile`

**Step 3: Configure Environment Variables**

Add these environment variables:

```bash
# Required
DATABASE_URL=<paste Internal Database URL from Step 1>
JWT_SECRET=<generate with: openssl rand -base64 32>
ENVIRONMENT=production

# Server Configuration
PORT=10000
RUST_LOG=info

# Database Pool
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=2
DB_CONNECTION_TIMEOUT_SECS=30

# JWT
JWT_EXPIRATION_HOURS=24

# Rate Limiting
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10
```

**Step 4: Configure Health Check**

- Health Check Path: `/api/v1/health`

**Step 5: Deploy**

- Click "Create Web Service"
- Monitor build logs for any errors

## Troubleshooting Common Issues

### Issue: Service returns 404

**Symptoms:**
- `https://your-service.onrender.com/` returns "HTTP ERROR 404"

**Causes & Solutions:**

1. **Service not fully deployed yet**
   - Check: Go to Render Dashboard → Your Service → Events
   - Look for "Live" status
   - First deployment can take 5-15 minutes

2. **Build failed**
   - Check: Logs tab in Render dashboard
   - Look for compilation errors
   - Common issue: Missing dependencies

3. **Missing environment variables**
   - Check: Environment tab in dashboard
   - Ensure `DATABASE_URL` and `JWT_SECRET` are set
   - Restart service after adding variables

4. **Database connection failed**
   - Check: Logs for "Failed to create database connection pool"
   - Verify: DATABASE_URL is the Internal Database URL (not External)
   - Ensure: Database is in the same region as web service

5. **Migrations failed**
   - Check: Logs for "Failed to run database migrations"
   - Verify: Migrations directory is in Docker image
   - Manual fix: Connect to database and run migrations manually

### Issue: Service crashes on startup

**Check logs for:**

1. **JWT_SECRET error**
   ```
   PRODUCTION ERROR: JWT_SECRET must be a secure random string
   ```
   - Solution: Set a secure JWT_SECRET (min 32 chars)
   - Generate with: `openssl rand -base64 32`

2. **Database connection timeout**
   ```
   Failed to create database connection pool
   ```
   - Solution: Verify DATABASE_URL is correct
   - Check database is running and accessible

3. **Port binding error**
   ```
   Address already in use
   ```
   - Solution: Ensure PORT=10000 is set in environment variables

### Issue: Health check fails

**Symptoms:**
- Service shows as "Unhealthy" in dashboard

**Solutions:**

1. **Verify health endpoint works**
   ```bash
   curl https://your-service.onrender.com/api/v1/health
   ```
   - Should return: `{"status":"healthy","version":"1.0.0","timestamp":"..."}`

2. **Check health check path**
   - Must be: `/api/v1/health` (not `/health`)

3. **Wait for service to fully start**
   - Health checks may fail during startup (first 30-60 seconds)

## Accessing Your Deployed Service

Once deployed, your API is available at:

- **Base URL**: `https://ambient-ai-vcp-system.onrender.com`
- **Health Check**: `https://ambient-ai-vcp-system.onrender.com/api/v1/health`
- **API Docs**: `https://ambient-ai-vcp-system.onrender.com/swagger-ui`
- **OpenAPI JSON**: `https://ambient-ai-vcp-system.onrender.com/api-docs/openapi.json`

## Using the API

### 1. Register a User

```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "securepassword123"
  }'
```

### 2. Login

```bash
curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "password": "securepassword123"
  }'
```

Save the `access_token` from the response.

### 3. Access Protected Endpoints

```bash
curl https://ambient-ai-vcp-system.onrender.com/api/v1/cluster/stats \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

## Monitoring

### View Logs

1. Go to Render Dashboard
2. Select your service
3. Click "Logs" tab
4. Monitor for:
   - Startup messages
   - Database connection status
   - API requests
   - Errors

### Metrics

Render provides basic metrics:
- CPU usage
- Memory usage
- Request count
- Response times

Access via: Dashboard → Your Service → Metrics tab

## Updating the Deployment

### From Git

1. Push changes to your GitHub repository
2. Render automatically detects changes
3. Triggers new build and deployment
4. Zero-downtime deployment

### Manual Redeploy

1. Go to Render Dashboard
2. Select your service
3. Click "Manual Deploy" → "Deploy latest commit"

## Database Management

### Access Database

1. Go to Render Dashboard → Databases
2. Select `ambient-ai-vcp-db`
3. Click "Connect" for connection details

### Using psql

```bash
# Install PostgreSQL client
brew install postgresql  # macOS
sudo apt-get install postgresql-client  # Ubuntu

# Connect (use External Database URL)
psql <EXTERNAL_DATABASE_URL>
```

### View Tables

```sql
\dt  -- List tables
SELECT * FROM nodes;
SELECT * FROM tasks;
SELECT * FROM users;
```

### Backup Database

Render automatically backs up databases on paid plans. For free tier:

```bash
pg_dump <EXTERNAL_DATABASE_URL> > backup.sql
```

## Cost Considerations

### Free Tier Limitations

- Web service spins down after 15 minutes of inactivity
- First request after spin-down takes 30-60 seconds
- PostgreSQL: 1GB storage, 97 hours/month uptime

### Paid Plans

- **Starter Plan ($7/month)**: Always-on service, no spin-down
- **PostgreSQL ($7/month)**: Always-on database, 10GB storage

## Security Best Practices

1. **Never commit secrets**: JWT_SECRET, database passwords
2. **Use Render's secret management**: Auto-generated values
3. **Enable HTTPS**: Provided automatically by Render
4. **Restrict CORS**: Update `CORS_ALLOWED_ORIGINS` in production
5. **Monitor logs**: Watch for suspicious activity
6. **Regular updates**: Keep dependencies up to date

## Support

- **Render Docs**: https://render.com/docs
- **Render Support**: https://render.com/support
- **API Issues**: Check repository issues on GitHub

## Next Steps

1. Deploy using render.yaml (easiest method)
2. Verify health endpoint is accessible
3. Register a user via API
4. Test authentication flow
5. Monitor logs for any issues
6. Set up monitoring/alerting (optional)
