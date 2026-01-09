-- Add theme and card_size preferences to users table
ALTER TABLE users ADD COLUMN theme TEXT DEFAULT NULL;
ALTER TABLE users ADD COLUMN card_size TEXT DEFAULT NULL;
