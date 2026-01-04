-- Settings table for app configuration
-- These settings can be overridden by environment variables
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value, description) VALUES
    ('tmdb_api_key', '', 'API key for The Movie Database (themoviedb.org)');

