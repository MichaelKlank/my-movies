-- Series table
CREATE TABLE IF NOT EXISTS series (
    id BLOB PRIMARY KEY NOT NULL,
    user_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Identifiers
    collection_number TEXT,
    barcode TEXT,
    tmdb_id INTEGER,
    imdb_id TEXT,
    
    -- Titles
    title TEXT NOT NULL,
    original_title TEXT,
    sort_title TEXT,
    personal_title TEXT,
    personal_sort_title TEXT,
    
    -- Description
    description TEXT,
    tagline TEXT,
    
    -- Production Info
    production_year INTEGER,
    first_aired TEXT,
    air_time TEXT,
    network TEXT,
    status TEXT,
    production_companies TEXT,
    production_countries TEXT,
    studios TEXT,
    
    -- Episode Info
    episodes_count INTEGER,
    running_time INTEGER,
    
    -- Cast
    actors TEXT,
    
    -- Ratings
    rating TEXT,
    personal_rating REAL,
    
    -- Media Info
    disc_type TEXT,
    media_type TEXT,
    discs INTEGER,
    region_codes TEXT,
    video_standard TEXT,
    aspect_ratio TEXT,
    audio_tracks TEXT,
    subtitles TEXT,
    is_3d INTEGER NOT NULL DEFAULT 0,
    mastered_in_4k INTEGER NOT NULL DEFAULT 0,
    
    -- Categorization
    genres TEXT,
    categories TEXT,
    tags TEXT,
    series_group TEXT,
    
    -- User Status
    watched INTEGER NOT NULL DEFAULT 0,
    digital_copies TEXT,
    
    -- Physical Info
    condition TEXT,
    slip_cover INTEGER NOT NULL DEFAULT 0,
    cover_type TEXT,
    edition TEXT,
    extra_features TEXT,
    
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
    
    -- Languages
    spoken_languages TEXT,
    
    -- Timestamps
    added_date TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_series_user_id ON series(user_id);
CREATE INDEX IF NOT EXISTS idx_series_barcode ON series(barcode);
CREATE INDEX IF NOT EXISTS idx_series_tmdb_id ON series(tmdb_id);
CREATE INDEX IF NOT EXISTS idx_series_title ON series(title);

-- Trigger to update updated_at
CREATE TRIGGER IF NOT EXISTS series_updated_at
    AFTER UPDATE ON series
    FOR EACH ROW
BEGIN
    UPDATE series SET updated_at = datetime('now') WHERE id = NEW.id;
END;
