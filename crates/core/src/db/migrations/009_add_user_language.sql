-- Add language field to users table
ALTER TABLE users ADD COLUMN language TEXT DEFAULT NULL;

-- Set default language based on system locale (will be handled in application code)
-- For now, we'll use NULL to indicate system default

