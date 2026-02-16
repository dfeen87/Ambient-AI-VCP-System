-- Forward-only reconciliation migration for changes moved out of the initial schema.
-- Safe to run in production and idempotent.

-- The initial migration originally created users.api_key and idx_users_api_key.
-- These were later removed from 20240101000000_initial_schema.sql and should be
-- applied as a forward change instead of rewriting migration history.
DROP INDEX IF EXISTS idx_users_api_key;

ALTER TABLE users
    DROP COLUMN IF EXISTS api_key;
