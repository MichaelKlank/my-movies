-- Add include_adult field to users table
ALTER TABLE users ADD COLUMN include_adult INTEGER NOT NULL DEFAULT 0;

