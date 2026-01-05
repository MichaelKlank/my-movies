-- Add avatar_data BLOB column to users table for storing avatar images directly in database
ALTER TABLE users ADD COLUMN avatar_data BLOB DEFAULT NULL;

