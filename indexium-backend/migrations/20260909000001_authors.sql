CREATE TABLE IF NOT EXISTS authors (
    github_id BIGINT PRIMARY KEY,
    login VARCHAR(39) NOT NULL UNIQUE,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
