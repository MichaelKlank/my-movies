-- Add poster_path field to movies table
ALTER TABLE movies ADD COLUMN poster_path TEXT;

-- Add poster_path field to series table  
ALTER TABLE series ADD COLUMN poster_path TEXT;

