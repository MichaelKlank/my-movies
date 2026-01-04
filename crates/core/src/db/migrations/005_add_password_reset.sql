-- Add password reset fields to users table
ALTER TABLE users ADD COLUMN reset_token TEXT;
ALTER TABLE users ADD COLUMN reset_token_expires TEXT;

-- Index for reset token lookup
CREATE INDEX IF NOT EXISTS idx_users_reset_token ON users(reset_token);

