-- Add collection fields to movies table
-- is_collection: marks a movie entry as a collection/box set
-- parent_collection_id: links individual movies to their parent collection

ALTER TABLE movies ADD COLUMN is_collection INTEGER NOT NULL DEFAULT 0;
ALTER TABLE movies ADD COLUMN parent_collection_id BLOB REFERENCES movies(id) ON DELETE SET NULL;

-- Index for efficient collection queries
CREATE INDEX IF NOT EXISTS idx_movies_is_collection ON movies(is_collection);
CREATE INDEX IF NOT EXISTS idx_movies_parent_collection_id ON movies(parent_collection_id);
