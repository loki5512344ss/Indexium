CREATE TABLE IF NOT EXISTS webhook_deliveries (
    delivery_id VARCHAR(64) PRIMARY KEY,
    event VARCHAR(32) NOT NULL,
    action VARCHAR(32),
    repo_id BIGINT,
    payload JSONB,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
