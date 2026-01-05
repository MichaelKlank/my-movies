-- Add avatar_path field to users table
ALTER TABLE users ADD COLUMN avatar_path TEXT DEFAULT NULL;

