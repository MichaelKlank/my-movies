-- Movies table
CREATE TABLE IF NOT EXISTS movies (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
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
    release_date TEXT,
    running_time INTEGER,
    director TEXT,
    actors TEXT,
    production_companies TEXT,
    production_countries TEXT,
    studios TEXT,
    
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
    movie_group TEXT,
    
    -- User Status
    watched INTEGER NOT NULL DEFAULT 0,
    digital_copies TEXT,
    status TEXT,
    
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
    
    -- TMDB data
    budget INTEGER,
    revenue INTEGER,
    spoken_languages TEXT,
    
    -- Timestamps
    added_date TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_movies_user_id ON movies(user_id);
CREATE INDEX IF NOT EXISTS idx_movies_barcode ON movies(barcode);
CREATE INDEX IF NOT EXISTS idx_movies_tmdb_id ON movies(tmdb_id);
CREATE INDEX IF NOT EXISTS idx_movies_title ON movies(title);
CREATE INDEX IF NOT EXISTS idx_movies_sort_title ON movies(sort_title);
CREATE INDEX IF NOT EXISTS idx_movies_production_year ON movies(production_year);

-- Trigger to update updated_at
CREATE TRIGGER IF NOT EXISTS movies_updated_at
    AFTER UPDATE ON movies
    FOR EACH ROW
BEGIN
    UPDATE movies SET updated_at = datetime('now') WHERE id = NEW.id;
END;
