-- Таблица модов (метаданные репозитория)
CREATE TABLE IF NOT EXISTS mods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_repo_id BIGINT UNIQUE NOT NULL,
    owner VARCHAR(255) NOT NULL,
    repo VARCHAR(255) NOT NULL,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    summary TEXT,
    icon_url TEXT,
    default_branch VARCHAR(32) NOT NULL DEFAULT 'main',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Таблица версий (релизов), полученных через Webhook/API
CREATE TABLE IF NOT EXISTS mod_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    version_number VARCHAR(64) NOT NULL,
    game_versions VARCHAR(32)[] NOT NULL,
    loaders VARCHAR(32)[] NOT NULL,
    download_url TEXT NOT NULL,
    file_sha256 CHAR(64) NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_mod_version UNIQUE (mod_id, version_number)
);

-- Индексы для быстрой фильтрации лаунчерами
CREATE INDEX IF NOT EXISTS idx_mods_slug ON mods(slug);
CREATE INDEX IF NOT EXISTS idx_versions_lookup ON mod_versions USING GIN (game_versions, loaders);
