-- Hosts table
CREATE TABLE IF NOT EXISTS host (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    auth_header TEXT,
    guild_id INTEGER NOT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscription (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    host_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    channel_id INTEGER NOT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(guild_id, key)
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_subscription_key ON subscription(key);
CREATE INDEX IF NOT EXISTS idx_subscription_channel ON subscription(channel_id);
