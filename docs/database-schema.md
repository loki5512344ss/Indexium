# Схема БД Indexium

> PostgreSQL 16+ с расширениями `pg_trgm`, `pgvector` (опционально), `uuid-ossp`/`pgcrypto`.

---

## 1. Расширения

```sql
CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- fuzzy search
-- CREATE EXTENSION IF NOT EXISTS vector;    -- pgvector, когда нужен семантический поиск
```

## 2. Таблицы

### `authors` - авторы (зеркало GitHub users)

```sql
CREATE TABLE authors (
    github_id BIGINT PRIMARY KEY,            -- GitHub user ID
    login VARCHAR(39) NOT NULL UNIQUE,       -- GitHub login
    avatar_url TEXT,
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### `mods` - моды (один репозиторий = один мод на MVP)

```sql
CREATE TABLE mods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_repo_id BIGINT UNIQUE NOT NULL,
    slug VARCHAR(64) UNIQUE NOT NULL,        -- URL-friendly, напр. "sodium-extra"
    name VARCHAR(128) NOT NULL,
    summary TEXT,                            -- короткое описание из манифеста
    description TEXT,                        -- README.md (markdown, кэшируем)
    author_github_id BIGINT NOT NULL REFERENCES authors(github_id),
    github_repo_name VARCHAR(128) NOT NULL,  -- "owner/repo"
    default_branch VARCHAR(32) DEFAULT 'main',
    icon_url TEXT,
    verified BOOLEAN DEFAULT FALSE,
    suspicious BOOLEAN DEFAULT FALSE,
    search_vector TSVECTOR,                  -- материализованный FTS вектор
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_mods_search ON mods USING GIN (search_vector);
CREATE INDEX idx_mods_trgm ON mods USING GIN (name gin_trgm_ops, summary gin_trgm_ops);
CREATE INDEX idx_mods_author ON mods (author_github_id);
```

Триггер для `search_vector`:

```sql
CREATE OR REPLACE FUNCTION mods_search_vector_update() RETURNS trigger AS $$
BEGIN
  NEW.search_vector :=
    setweight(to_tsvector('english', coalesce(NEW.name,'')), 'A') ||
    setweight(to_tsvector('english', coalesce(NEW.summary,'')), 'B') ||
    setweight(to_tsvector('english', coalesce(NEW.description,'')), 'C');
  RETURN NEW;
END $$ LANGUAGE plpgsql;

CREATE TRIGGER trg_mods_search_vector
BEFORE INSERT OR UPDATE OF name, summary, description ON mods
FOR EACH ROW EXECUTE FUNCTION mods_search_vector_update();
```

### `mod_versions` - версии / релизы

```sql
CREATE TABLE mod_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    version_number VARCHAR(32) NOT NULL,     -- semver из манифеста/тега
    game_versions VARCHAR(32)[] NOT NULL,    -- e.g. ['1.20.1', '1.21']
    loaders VARCHAR(16)[] NOT NULL,          -- e.g. ['fabric','quilt','neoforge']
    download_url TEXT NOT NULL,              -- https://github.com/.../releases/download/...
    file_name VARCHAR(128) NOT NULL,         -- sodium-1.2.3.jar
    file_sha256 CHAR(64) NOT NULL,
    file_size BIGINT,
    published_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(mod_id, version_number)
);

CREATE INDEX idx_versions_mod ON mod_versions (mod_id, published_at DESC);
CREATE INDEX idx_versions_lookup ON mod_versions USING GIN (game_versions, loaders);
CREATE INDEX idx_versions_sha ON mod_versions (file_sha256);
```

### `webhook_deliveries` - идемпотентность webhook'ов (<50ms ответ)

```sql
CREATE TABLE webhook_deliveries (
    delivery_id VARCHAR(64) PRIMARY KEY,     -- X-GitHub-Delivery (UUID от GitHub)
    event VARCHAR(32) NOT NULL,              -- "release"
    action VARCHAR(32),                      -- "published"
    repo_id BIGINT,
    payload JSONB,
    processed_at TIMESTAMPTZ DEFAULT now()
);
-- Уникальный индекс уже есть как PK, но явно для дедупа:
-- INSERT INTO webhook_deliveries (...) VALUES (...) ON CONFLICT (delivery_id) DO NOTHING
-- В handler: если affected_rows == 0 → 409 Already Processed, иначе push в Redis Streams.
```

> **Почему так:** Ingestion API должен ответить `202` за <50ms. Сначала `INSERT ... ON CONFLICT DO NOTHING` в `webhook_deliveries`, только потом `XADD` в Redis. Если `delivery_id` уже есть - сразу `409` без очереди.

### `mod_authors` - M2M авторы/контрибьюторы

На MVP `mods.author_github_id` достаточно (1 репо = 1 owner). Для организаций и соавторов - нормализуем сразу, чтобы не мигрировать болезненно:

```sql
CREATE TABLE mod_authors (
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    github_id BIGINT NOT NULL REFERENCES authors(github_id) ON DELETE CASCADE,
    role VARCHAR(16) NOT NULL DEFAULT 'owner', -- 'owner' | 'contributor' | 'maintainer'
    created_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (mod_id, github_id)
);

CREATE INDEX idx_mod_authors_github ON mod_authors (github_id);
-- На MVP можно оставить mods.author_github_id как денормализованный owner
-- и дублировать его в mod_authors при создании мода (триггер или код).
```

> **MVP стратегия:** оставляем `mods.author_github_id` (как сейчас в `migrations/20260906000000_init_schema.sql`) для простых запросов, но добавляем `mod_authors` когда появится первый кейс организации. В `GET /mods/:slug` отдаём `authors: [{login, role}]` вместо одиночного `author`.

### `mod_versions.file_size` - откуда берётся

В `api-spec.md` поле `file_size` возвращается клиентам. Заполняется воркером из HTTP-заголовка:

```sql
-- уже в mod_versions: file_size BIGINT - bytes из Content-Length
```

Алгоритм воркера (`jar_parser.rs`):
1. `HEAD download_url` → `Content-Length` + `Accept-Ranges: bytes`.
2. Если `Content-Length` отсутствует - fallback на `GET` с `Range: bytes=0-0` и парсинг `Content-Range`.
3. Значение пишется в `mod_versions.file_size` при `INSERT`.

> GitHub CDN (`objects.githubusercontent.com`) всегда отдаёт `Content-Length` и поддерживает `Range` для release assets - проверено для `.jar` до 50MB.

### `dependencies` (опционально, нормализованная)

На MVP храним зависимости как `JSONB` в `mod_versions` или отдельной таблицей:

```sql
CREATE TABLE mod_dependencies (
    version_id UUID REFERENCES mod_versions(id) ON DELETE CASCADE,
    depends_on_mod_id UUID REFERENCES mods(id), -- nullable если внешний мод не в индексе
    mod_id_str VARCHAR(64) NOT NULL,         -- id из fabric.mod.json depends
    version_range VARCHAR(64),               -- ">=1.0.0"
    PRIMARY KEY (version_id, mod_id_str)
);
```

## 3. Телеметрия - аналог bStats (см. docs/analytics.md)

```sql
-- Полуагрегат: один пинг = одна строка, TTL 30 дней (DELETE via cron)
CREATE TABLE mod_telemetry_pings (
    id BIGSERIAL PRIMARY KEY,
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    server_hash CHAR(64) NOT NULL, -- sha256(server_uuid + daily_salt)
    mc_version VARCHAR(16) NOT NULL,
    loader VARCHAR(16) NOT NULL,
    os VARCHAR(16) NOT NULL,
    java_version VARCHAR(16) NOT NULL,
    player_count INT NOT NULL DEFAULT 0,
    pinged_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_telemetry_lookup ON mod_telemetry_pings (mod_id, pinged_at DESC);
CREATE INDEX idx_telemetry_hash ON mod_telemetry_pings (server_hash, pinged_at);

-- Суточный агрегат - хранится навсегда
CREATE TABLE mod_daily_stats (
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    active_servers INT NOT NULL DEFAULT 0, -- COUNT(DISTINCT server_hash) за день
    active_players INT NOT NULL DEFAULT 0,
    breakdown_json JSONB NOT NULL, -- { mc_versions:{}, loaders:{}, os:{}, java:{}, custom:{...} }
    PRIMARY KEY (mod_id, date)
);

-- Соль для анонимизации (ротация daily)
CREATE TABLE analytics_salts (
    date DATE PRIMARY KEY,
    salt CHAR(64) NOT NULL
);
-- Хеш: server_hash = sha256(server_uuid || salt_for_today) - позволяет считать уникальные за день, но не трекать сквозь дни.
-- Rate limit: Redis SET server_hash:mod_id NX EX 900 (1 пинг / 15 мин)
-- TTL: DELETE FROM mod_telemetry_pings WHERE pinged_at < NOW() - INTERVAL '30 days' (cron hourly)
-- Агрегация: кроном раз в час INSERT INTO mod_daily_stats ... ON CONFLICT DO UPDATE COUNT(DISTINCT server_hash)
```

> Postgres хватает до ~10M пингов/мес. При росте - `SELECT create_hypertable('mod_telemetry_pings','pinged_at')` (TimescaleDB) или ClickHouse без смены схемы.

## 4. Пример запросов

### Поиск с FTS + фильтры

```sql
SELECT id, name, summary, ts_rank(search_vector, query) AS rank
FROM mods, plainto_tsquery('english', $1) query
WHERE search_vector @@ query
  AND suspicious = false
  AND EXISTS (
    SELECT 1 FROM mod_versions v
    WHERE v.mod_id = mods.id
      AND v.game_versions && ARRAY[$2]::varchar[]
      AND v.loaders && ARRAY[$3]::varchar[]
  )
ORDER BY rank DESC
LIMIT 20 OFFSET $4;
```

### Fuzzy (опечатки)

```sql
SELECT name, similarity(name, 'sodim') AS sml
FROM mods
WHERE name % 'sodim'   -- оператор pg_trgm
ORDER BY sml DESC LIMIT 10;
```

## 5. Миграции

Хранятся в `indexium-backend/migrations/` (sqlx):

```
migrations/
  20260906000000_init_schema.sql  -- mods, mod_versions
  20260907000000_telemetry.sql    -- mod_telemetry_pings, mod_daily_stats, analytics_salts
```

Запуск: `sqlx migrate run` / `cargo sqlx migrate run`.

## 6. Сиды

Для дев-окружения: `migrations/seeds/dev.sql` - 5 фейковых модов + версии, чтобы фронт сразу имел данные.

## 7. Будущие расширения

- `pgvector` колонка `embedding vector(1536)` для семантического поиска по README.
- Партиционирование `mod_versions` по `published_at` если >1M строк.
- Материализованное представление `popular_mods` (top по скачиваниям).
