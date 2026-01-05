-- Remove poster_path column from movies table
-- All poster images are now stored as BLOB in poster_data column
ALTER TABLE movies DROP COLUMN poster_path;

