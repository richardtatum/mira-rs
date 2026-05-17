-- Hosts table
CREATE TABLE IF NOT EXISTS host (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    auth_header TEXT,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS host_guild (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    host_id INTEGER NOT NULL,
    guild_id INTEGER NOT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        
    FOREIGN KEY (host_id) REFERENCES host(id),
    UNIQUE (host_id, guild_id)
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscription (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,
    host_guild_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    message_id INTEGER NULL,
    playing TEXT NULL,
    created_by INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (host_guild_id) REFERENCES host_guild(id) ON DELETE CASCADE,
    UNIQUE(host_guild_id, key, channel_id)
);
