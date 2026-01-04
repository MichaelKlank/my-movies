-- Collections table (for box sets)
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Identifiers
    collection_number TEXT,
    barcode TEXT,
    
    -- Titles
    title TEXT NOT NULL,
    sort_title TEXT,
    personal_title TEXT,
    
    -- Description
    description TEXT,
    
    -- Media Info
    disc_type TEXT,
    discs INTEGER,
    region_codes TEXT,
    
    -- Categorization
    genres TEXT,
    categories TEXT,
    tags TEXT,
    
    -- Physical Info
    condition TEXT,
    slip_cover INTEGER NOT NULL DEFAULT 0,
    cover_type TEXT,
    edition TEXT,
    
    -- Financial
    purchase_date TEXT,
    price REAL,
    currency TEXT,
    purchase_place TEXT,
    value_date TEXT,
    value_price REAL,
    value_currency TEXT,
    
    -- Lending
    lent_to TEXT,
    lent_due TEXT,
    
    -- Location
    location TEXT,
    
    -- Notes
    notes TEXT,
    
    -- Timestamps
    added_date TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Collection items (links movies/series to collections)
CREATE TABLE IF NOT EXISTS collection_items (
    id TEXT PRIMARY KEY NOT NULL,
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    item_type TEXT NOT NULL CHECK (item_type IN ('movie', 'series')),
    movie_id TEXT REFERENCES movies(id) ON DELETE CASCADE,
    series_id TEXT REFERENCES series(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    -- Ensure either movie_id or series_id is set based on item_type
    CHECK (
        (item_type = 'movie' AND movie_id IS NOT NULL AND series_id IS NULL) OR
        (item_type = 'series' AND series_id IS NOT NULL AND movie_id IS NULL)
    )
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_collections_user_id ON collections(user_id);
CREATE INDEX IF NOT EXISTS idx_collections_barcode ON collections(barcode);
CREATE INDEX IF NOT EXISTS idx_collection_items_collection_id ON collection_items(collection_id);
CREATE INDEX IF NOT EXISTS idx_collection_items_movie_id ON collection_items(movie_id);
CREATE INDEX IF NOT EXISTS idx_collection_items_series_id ON collection_items(series_id);

-- Trigger to update updated_at
CREATE TRIGGER IF NOT EXISTS collections_updated_at
    AFTER UPDATE ON collections
    FOR EACH ROW
BEGIN
    UPDATE collections SET updated_at = datetime('now') WHERE id = NEW.id;
END;
