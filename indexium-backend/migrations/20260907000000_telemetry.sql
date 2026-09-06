-- Indexium Analytics: bStats аналог

CREATE TABLE IF NOT EXISTS mod_telemetry_pings (
    id BIGSERIAL PRIMARY KEY,
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    server_hash CHAR(64) NOT NULL,
    mc_version VARCHAR(16) NOT NULL,
    loader VARCHAR(16) NOT NULL,
    os VARCHAR(16) NOT NULL,
    java_version VARCHAR(16) NOT NULL,
    player_count INT NOT NULL DEFAULT 0,
    pinged_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_telemetry_lookup ON mod_telemetry_pings (mod_id, pinged_at DESC);
CREATE INDEX IF NOT EXISTS idx_telemetry_hash ON mod_telemetry_pings (server_hash, pinged_at);

CREATE TABLE IF NOT EXISTS mod_daily_stats (
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    active_servers INT NOT NULL DEFAULT 0,
    active_players INT NOT NULL DEFAULT 0,
    breakdown_json JSONB NOT NULL,
    PRIMARY KEY (mod_id, date)
);

CREATE TABLE IF NOT EXISTS analytics_salts (
    date DATE PRIMARY KEY,
    salt CHAR(64) NOT NULL
);
