-- Add poster_data BLOB column to movies table for storing poster images directly in database
ALTER TABLE movies ADD COLUMN poster_data BLOB DEFAULT NULL;

