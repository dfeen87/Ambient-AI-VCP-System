# Quick Fix for Render.com 404 Error

## Problem
Your site https://ambient-ai-vcp-system.onrender.com returns HTTP 404.

## Root Cause
The deployment is missing:
1. PostgreSQL database configuration
2. Required environment variables (DATABASE_URL, JWT_SECRET, etc.)
3. Database migrations in Docker image

## Solution

### Option 1: Redeploy with Updated Configuration (Recommended)

1. **Merge this PR** to your main branch

2. **In Render Dashboard:**
   - Go to https://dashboard.render.com/
   - Find your `ambient-ai-vcp-api` service
   - Click "Manual Deploy" → "Clear build cache & deploy"

3. **The updated `render.yaml` will automatically:**
   - Create a PostgreSQL database
   - Set all environment variables
   - Configure health checks
   - Run migrations on startup

### Option 2: Manual Fix (if you need to keep current setup)

1. **Create PostgreSQL Database:**
   - In Render Dashboard → New → PostgreSQL
   - Name: `ambient-ai-vcp-db`
   - Copy the "Internal Database URL"

2. **Update Service Environment Variables:**
   - Go to your service → Environment tab
   - Add these variables:

   ```
   DATABASE_URL=<paste internal database URL>
   JWT_SECRET=<generate with: openssl rand -base64 32>
   ENVIRONMENT=production
   DB_MAX_CONNECTIONS=10
   DB_MIN_CONNECTIONS=2
   DB_CONNECTION_TIMEOUT_SECS=30
   JWT_EXPIRATION_HOURS=24
   RATE_LIMIT_REQUESTS_PER_MINUTE=60
   RATE_LIMIT_BURST=10
   ```

3. **Redeploy:**
   - Click "Manual Deploy" → "Deploy latest commit"

## Verify Fix

After deployment completes (5-15 minutes):

1. **Check health endpoint:**
   ```bash
   curl https://ambient-ai-vcp-system.onrender.com/api/v1/health
   ```
   
   Should return:
   ```json
   {
     "status": "healthy",
     "version": "1.0.0",
     "timestamp": "2024-..."
   }
   ```

2. **Check Swagger UI:**
   Visit: https://ambient-ai-vcp-system.onrender.com/swagger-ui

3. **Register a test user:**
   ```bash
   curl -X POST https://ambient-ai-vcp-system.onrender.com/api/v1/auth/register \
     -H "Content-Type: application/json" \
     -d '{"username":"testuser","password":"password123"}'
   ```

## If Still Getting 404

Check the Render logs:

1. Go to Render Dashboard → Your Service → Logs
2. Look for errors like:
   - "JWT_SECRET not configured"
   - "Failed to create database connection pool"
   - "Failed to run database migrations"

Common fixes:
- Ensure `ENVIRONMENT=production` is set
- Ensure `JWT_SECRET` is at least 32 characters
- Ensure `DATABASE_URL` uses the Internal Database URL
- Wait 30-60 seconds for service to fully start

## Timeline

- **Merge PR**: 1 minute
- **Render detects changes**: 1-2 minutes  
- **Build Docker image**: 5-10 minutes
- **Deploy & migrations**: 2-5 minutes
- **Total**: ~15 minutes

## Need Help?

See the full deployment guide: `RENDER_DEPLOYMENT.md`

Or check Render logs for specific error messages.
