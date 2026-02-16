-- Migration: remove plaintext API key storage from users table
ALTER TABLE users DROP COLUMN IF EXISTS api_key;
