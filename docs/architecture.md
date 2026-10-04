# Архитектура Indexium - Асинхронный событийный индексатор

> Цель: сделать сервис максимально лёгким, дешёвым в обслуживании и устойчивым к ограничениям GitHub.
> Принцип: бэкенд **не хранит** тяжёлые файлы - артефакты отдаются с GitHub Releases CDN, мы валидируем, индексируем метаданные и выдаём быстрые JSON-ответы.

---

## 1. Обзор системных компонентов

```
                  ┌───────────────────────────────────────────────┐
                  │                 GitHub Ecosystem              │
                  └───────┬───────────────────────────────▲───────┘
                          │                               │
                          │ 1. Webhook (release.published)│ 4. Read Assets / Metadata
                          ▼                               │
┌─────────────────────────────────────────────────────────┴────────────────────────────────┐
│  Ваш Бэкенд (Event-Driven Indexer)                                                       │
│                                                                                          │
│  ┌───────────────────────┐       ┌──────────────────────┐       ┌─────────────────────┐  │
│  │ Webhook Ingestion API │ ────> │ Async Job Queue      │ ────> │ Worker / Indexer    │  │
│  │ (Fast Signature Check)│       │ (Redis / NATS)       │       │ (Jar Parser & AST)  │  │
│  └───────────────────────┘       └──────────────────────┘       └──────────┬──────────┘  │
│                                                                            │             │
│                                                                            ▼             │
│  ┌───────────────────────┐       ┌──────────────────────┐       ┌─────────────────────┐  │
│  │ Edge Public REST/v1   │ <──── │ Read-Heavy Cache     │ <──── │ Relational DB       │  │
│  │ (High Throughput API) │       │ (Redis Key-Value)    │       │ (PostgreSQL + FTS)  │  │
│  └───────────────────────┘       └──────────────────────┘       └─────────────────────┘  │
└─────────────────────────────────────────────▲────────────────────────────────────────────┘
                                              │
                                              │ 5. Query Mods / Index
                                              │
                                  ┌───────────┴───────────┐
                                  │   Web UI / Launchers  │
                                  └───────────────────────┘
```

### Потоки данных
1. **Ingestion** - GitHub шлёт webhook → Ingestion API валидирует HMAC → кладёт job в очередь → отвечает `202`.
2. **Indexing** - Worker читает job → делает `Range Request` к `.jar` → парсит манифест → пишет в Postgres → инвалидирует кэш.
3. **Serving** - Лаунчер / Web UI дергает `GET /api/v1/mods` → читаем Redis → miss → Postgres FTS → кэшируем 60s → отдаём JSON.

---

## 2. Выбор технологического стека

| Слой | Технология | Почему именно это? |
| --- | --- | --- |
| **Backend Core** | **Rust (Axum)** | Минимальный memory footprint, высокий throughput, быстрый async I/O при парсинге. Альтернатива Go (Fiber) - допустима на Phase 2. |
| **Main Database** | **PostgreSQL** | Нативный FTS, `JSONB` для зависимостей, `pg_trgm` для fuzzy, `pgvector` опционально для семантики. |
| **Cache & Queue** | **Redis / Valkey** | Одновременно брокер очередей (Streams/PubSub) и L2-кэш популярных эндпоинтов. |
| **Worker Engine** | **Rust Background Worker (tokio)** | Потребляет webhook-события, скачивает только zip-header stream, валидирует байткод. |
| **Auth & Security** | **GitHub App / OAuth 2.0** | Вход только через GitHub, без локальных паролей. |
| **Frontend** | **SvelteKit + TypeScript** | SSR, маленький бандл, быстрый dev. |

> Обоснование против Elasticsearch/Meilisearch на старте: `to_tsvector` + `pg_trgm` переваривают десятки тысяч модов с <5ms без отдельного кластера. Миграция на внешний поиск - только если FTS упрётся.

---

## 3. Жизненный цикл релиза (Release Lifecycle)

### 3.1 Регистрация мода (Onboarding)
1. Разработчик логинится через GitHub OAuth.
2. Жмёт «Импортировать репозиторий» → сервис проверяет наличие манифеста (`fabric.mod.json`, `neoforge.mods.toml`, `quilt.mod.json`) в default branch.
3. Устанавливается GitHub App + Webhook на события `release` и `push`.

### 3.2 Обработка webhook (Ingestion & Job Dispatch)
1. GitHub шлёт `release.published`.
2. **Ingestion API** валидирует `X-Hub-Signature-256` (HMAC SHA-256 с `WEBHOOK_SECRET`), проверяет идемпотентность по `X-GitHub-Delivery`.
3. Кладёт задачу в Redis Queue, отвечает `202 Accepted` за <50ms (чтобы не висеть по таймауту GitHub - 10s).

### 3.3 Работа воркера-индексатора (Worker Execution) - детальный алгоритм `jar_parser.rs`

> Цель: не скачивать весь `.jar` (может быть 20–50MB), а прочитать только нужный манифест через 2–3 Range-запроса.

```
download_url = "https://github.com/owner/repo/releases/download/v1.2.3/mod-1.2.3.jar"
                    │
  Step 1: HEAD ─────┤
                    ▼
         Content-Length: 12345678
         Accept-Ranges: bytes
         (если нет Content-Length → GET Range: bytes=0-0 + парс Content-Range)
                    │
  Step 2: GET tail ─┤  Range: bytes=-65536  (последние 64KB)
                    ▼
         Найти EOCD (End of Central Directory) = 0x06054b50
         Из EOCD: central_dir_offset, central_dir_size, num_entries
                    │
  Step 3: GET central dir ─┤  Range: bytes=central_dir_offset..central_dir_offset+size
                           ▼
         Парс Central Directory headers (0x02014b50)
         Найти entry: fabric.mod.json | quilt.mod.json | neoforge.mods.toml | mcmod.info
         + icon (assets/<modid>/icon.png если указан в манифесте)
         → получить local_header_offset, compressed_size
                    │
  Step 4: GET manifest ─┤  Range: bytes=local_header_offset.. + compressed_size + header
                        ▼
         Распаковать (DEFLATE/STORE), парс JSON/TOML, валидация
         + SHA-256 сверка, file_size = Content-Length
```

**Детали реализации:**
1. `HEAD` - обязателен, чтобы получить `Content-Length` и убедиться `Accept-Ranges: bytes`. Таймаут 5s, retry 2.
2. Последние 64KB достаточно для EOCD даже для jar с 10k файлов (EOCD в конце). Если не найден - fallback к последним 128KB.
3. Central Directory читается одним запросом (обычно 5–30KB). Парсим `central_dir_offset/size` из EOCD.
4. Манифест - 4-й запрос только если нужен (часто 1–3KB). Иконка - опционально 5-й запрос, кэшируется и отдаётся через `GET /mods/:slug/icon`.
5. Все `GET` - `reqwest` с `header("Range", ...)`, проверка `206 Partial Content`, иначе fallback к полному скачиванию с лимитом 10MB.

**Ошибки:** `412` если `Accept-Ranges` != bytes → full download; `404` на Range → retry без Range; повреждённый ZIP → помечаем `suspicious` и DLQ.

Код: `indexium-backend/src/worker/jar_parser.rs` - чистые функции `find_eocd()`, `parse_central_dir()`, `fetch_manifest()` без I/O в тестах.

### 3.4 Агрегация и индексация (Storage & Cache Invalidation)
1. Сохраняет версию в `mod_versions` (см. `database-schema.md`).
2. `download_url` = прямая ссылка `https://github.com/.../releases/download/...` (CDN `objects.githubusercontent.com`).
3. Инвалидирует / обновляет Redis-кэш для поиска и карточки мода.

---

## 4. Обход ключевых ограничений (Edge Cases)

### GitHub Rate Limits
- Запросы воркеров - от имени **GitHub App Installs** (5k–12.5k RPH на инсталл vs 60 RPH анонимных).
- Скачивание не проксируем - выдаём клиентам прямые CDN-ссылки, трафик не идёт через нас.

### Безопасность (Malware Protection)
- Сверка `SHA-256` ассета с `.sha256` если есть.
- Сканирование байткода: флаг на `Runtime.getRuntime().exec()`, `ProcessBuilder`, `URLClassLoader`, сетевые вызовы в `<clinit>` / `FabricModInitializer`.
- Карантин: помечаем версию `suspicious = true`, не показываем в публичном поиске до ручной проверки.

### Поиск без Elasticsearch
- Postgres `to_tsvector('russian'|'english', name || summary)` + `GIN`.
- `pg_trgm` (`similarity()`, `%` оператор) для опечаток.
- Материализованный `search_vector` + триггер на update.

### Надёжность очереди
- Retry с exponential backoff (3 попытки), DLQ (dead-letter) для ручного разбора.
- Идемпотентный воркер: `ON CONFLICT (mod_id, version_number) DO UPDATE`.

### Телеметрия (bStats аналог, см. docs/analytics.md)
- **Ingestion:** `POST /api/v1/analytics/submit` (gzip JSON, без IP логов) → валидация allow-list → `server_hash = sha256(uuid + daily_salt)` → Redis rate limit 1/15мин → `INSERT mod_telemetry_pings`.
- **Aggregation:** кроном раз в час `COUNT(DISTINCT server_hash)` + `breakdown_json` → `mod_daily_stats`, TTL 30 дней для сырых пингов (`DELETE WHERE pinged_at < NOW()-30d`).
- **Serving:** `GET /mods/:slug/analytics?range=30d` (кэш 5 мин) + `GET /badges/:slug/servers.svg` + `sort=active_servers` для честной сортировки по реальным установкам, а не накрученным скачиваниям.
- **Масштаб:** Postgres хватает до 10M пингов/мес, далее TimescaleDB hypertable без смены схемы.

---

## 5. Схема структуры БД (Core Entity Relation)

См. детально в [`database-schema.md`](database-schema.md). Коротко:

```sql
-- Таблица модов
CREATE TABLE mods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    github_repo_id BIGINT UNIQUE NOT NULL,
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    summary TEXT,
    author_github_id BIGINT NOT NULL,
    default_branch VARCHAR(32) DEFAULT 'main',
    created_at TIMESTAMPTZ DEFAULT now()
);

-- Таблица версий (релизов)
CREATE TABLE mod_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mod_id UUID REFERENCES mods(id) ON DELETE CASCADE,
    version_number VARCHAR(32) NOT NULL,
    game_versions VARCHAR(32)[] NOT NULL, -- e.g. ['1.20.1', '1.20.2']
    loaders VARCHAR(16)[] NOT NULL,       -- e.g. ['fabric', 'quilt']
    download_url TEXT NOT NULL,           -- GitHub Release Direct Asset URL
    file_sha256 CHAR(64) NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    UNIQUE(mod_id, version_number)
);

CREATE INDEX idx_versions_lookup ON mod_versions USING GIN (game_versions, loaders);
```

Дополнительно: `authors`, `webhook_deliveries` (идемпотентность), `search_vector`, `mod_telemetry_pings`/`mod_daily_stats` (аналитика).

---

## 6. Масштабирование и эволюция

| Этап | Нагрузка | Действие |
|------|----------|----------|
| MVP | <10k модов, <100 RPS | Один инстанс Axum + Postgres + Redis, воркер в том же бинаре (tokio spawn) |
| Growth | 10k-100k модов, 1k RPS | Вынос воркера в отдельный деплой, реплика Postgres RO, Redis Cluster |
| Scale | 100k+ модов, 10k RPS | NATS JetStream вместо Redis Streams, read-replica + шардирование по `slug`, CDN перед API (Cloudflare) |

---

## 7. Нефункциональные требования

- **Latency**: `GET /mods` p95 < 80ms (cache hit), < 200ms (cache miss + FTS).
- **Availability**: 99.9% (допустим 43 мин downtime/мес на MVP).
- **Cost**: < $20/мес на MVP (1 VPS Hetzner + managed Postgres free tier).
- **Security**: HMAC, GitHub App, no local passwords, malware scan.

---

## 8. Диаграмма деплоя (MVP)

```
[GitHub] --webhook--> [Axum Ingestion :3000] --> [Redis :6379] --> [Worker]
                                                    |                |
                                                    v                v
                                              [Postgres :5432] <--+
                                                    |
[Browser/Launcher] --> [Axum Public API :3000] --> [Redis Cache]
                     \
                      --> [SvelteKit :5173] (SSR, fetch API)
```

Все сервисы - `docker compose` локально, один VPS в проде.
