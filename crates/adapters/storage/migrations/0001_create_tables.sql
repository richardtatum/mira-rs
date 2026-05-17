-- Hosts table
CREATE TABLE IF NOT EXISTS host (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    auth_header TEXT,
    guild_id INTEGER NOT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(url, guild_id)
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscription (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    host_id INTEGER NOT NULL,
    key TEXT NOT NULL,
    channel_id INTEGER NOT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (host_id) REFERENCES host(id) ON DELETE CASCADE,
    UNIQUE(host_id, key)
);

CREATE TABLE IF NOT EXISTS stream (
    id INTEGER NOT NULL PRIMARY KEY NOT NULL,
    subscription_id INTEGER NOT NULL,
    status INTEGER NOT NULL,
    viewer_count INTEGER DEFAULT 0 NOT NULL,
    message_id INTEGER NOT NULL,
    playing TEXT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NULL,

    FOREIGN KEY (subscription_id) REFERENCES subscription(id) ON DELETE CASCADE,
    UNIQUE (subscription_id)
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_subscription_key ON subscription(key);
CREATE INDEX IF NOT EXISTS idx_subscription_channel ON subscription(channel_id);
