# Indexium — TODO / Roadmap

> Сервис: асинхронный событийный индексатор модов Minecraft поверх GitHub Releases CDN.
> Бэкенд не хранит тяжёлые артефакты, только метаданные + индексация + быстрый JSON API.

---

## Правила проекта (обязательно к соблюдению)

> Эти правила — не чекбоксы, а инварианты. Любой PR, нарушающий их, не принимается.

### 1. KISS — Keep It Simple, Stupid
- Выбирай самое простое решение, которое закрывает задачу. Никаких абстракций «на будущее» (YAGNI).
- Один модуль — одна ответственность. Если не можешь объяснить функцию в одном предложении — дроби.
- Предпочитай явный код неявной магии (никаких макросов ради макросов).

### 2. DRY — Don't Repeat Yourself
- Повтор >2 раз → выноси в функцию/модуль. Но не DRY ради DRY: дублирование лучше неправильной абстракции.
- Общие типы/утилиты — в `common`/`shared`, доменная логика — в своём модуле.

### 3. SOLID (применительно к Rust)
- **S** — один файл/модуль = одна причина для изменений (см. лимиты ниже).
- **O** — открыт для расширения через трейты, закрыт для модификации (feature-flag, а не `if` на типы).
- **L** — любой `impl Trait` должен заменять другой без поломки контракта.
- **I** — узкие трейты лучше жирных (`Readable`, `Validatable` вместо `GodService`).
- **D** — зависимость от абстракций (`PgPool` через `AppState`, а не глобаль).

### 4. Лимиты структуры (жёстко)
- **Макс 250 строк на файл** — если больше, дроби файл на подмодули.
- **Макс 4 файла на папку** — если больше, вводи подпапки по домену (`api/mods/`, `worker/parsers/`).
- Исключение: `mod.rs`/`lib.rs` не считаются, но должны быть тонкими реэкспортами.
- CI будет ругаться (`cargo clippy` + кастомный скрипт `scripts/check-limits.sh`).

### 5. Дополнительные инварианты
- **Чистота ошибок**: никаких `unwrap()`/`expect()` вне `main.rs` и тестов. Везде `Result` + `thiserror`/`anyhow`.
- **Типы вместо строк**: `Slug`, `GameVersion`, `Loader` — newtype, а не `String`.
- **Миграции только вперёд**: никаких `DROP` без ADR и бэкапа. Каждая миграция — идемпотентна (`IF NOT EXISTS`).
- **Логика без сайд-эффектов**: парсеры/валидаторы — чистые функции, I/O только на границах (handler/worker).
- **Документация рядом с кодом**: публичная функция без `///` — не готова к мерджу.
- **Тест на каждый баг**: регрессия покрывается тестом до фикса.

---

## Легенда статусов
- `[ ]` — не начато
- `[~]` — в процессе
- `[x]` — готово
- `[!]` — заблокировано / требует решения

---

## Phase 0 — Фундамент монорепо (Текущий приоритет)

- [x] Объединить `indexium-backend` + `indexium-frontend` в один git-монорепо (корень `/`)
- [x] Настроить корневой `.gitignore` + локальные `.gitignore`
- [x] Создать структуру `docs/` и заполнить базовую архитектуру
- [x] Создать `todo.md` и `README.md` в корне + прописать правила KISS/DRY/SOLID и лимиты файлов
- [x] Первая миграция `20260906000000_init_schema.sql` (mods, mod_versions, GIN индексы)
- [x] Базовый `src/main.rs` (Axum + SQLx + CORS + /health + миграции при старте)
- [x] Настройка `.env` / `.env.example` (DATABASE_URL, SERVER_PORT)
- [ ] Добавить `docker-compose.yml` (Postgres + Redis/Valkey) для локальной разработки
- [ ] Добавить `Makefile` / `justfile` с командами `dev`, `migrate`, `lint`, `test`
- [ ] Настроить CI (GitHub Actions): `cargo clippy + test`, `svelte-check`, `sqlx migrate check`
- [ ] Скрипт `scripts/check-limits.sh` — проверка 250 строк / 4 файла на папку

## Phase 1 — Backend Core (Rust / Axum)

### 1.1 Инфраструктура
- [x] `config` — загрузка `.env` (DATABASE_URL, REDIS_URL, GITHUB_APP_ID, WEBHOOK_SECRET) — база в `main.rs` через `dotenvy`
- [x] `db` — пул `sqlx::PgPool`, миграции (`sqlx::migrate!`), health-check `/health`
- [x] `tracing` — структурированные логи (EnvFilter + fmt layer)
- [x] Axum роутер: `GET /health`, CORS (5173), TraceLayer

### 1.2 Схема БД (PostgreSQL + FTS + pg_trgm)
- [x] Миграция `20260906000000_init_schema.sql` — таблицы `mods`, `mod_versions` + `GIN (game_versions, loaders)`
- [ ] Миграция `002_fts` — `search_vector`, `pg_trgm`, триггер (см. `docs/database-schema.md`)
- [ ] Таблица `authors` + `webhook_deliveries` (идемпотентность)
- [ ] Сиды / фикстуры для локального дев-окружения

### 1.3 Webhook Ingestion API
- [ ] `POST /api/v1/webhooks/github` — проверка `X-Hub-Signature-256` (HMAC SHA-256)
- [ ] Валидация эвента `release.published` / `release.released`, идемпотентность по `delivery_id`
- [ ] Пуш задачи в очередь (Redis Streams) + ответ `202 Accepted` < 50ms
- [ ] Тест на replay-атаку и неверную подпись

### 1.4 Async Worker / Indexer
- [ ] Консьюмер очереди (tokio task)
- [ ] Скачивание через HTTP Range Request — чтение только ZIP central directory `.jar`
- [ ] Парсинг `fabric.mod.json` / `quilt.mod.json` / `neoforge.mods.toml` / `mcmod.info`
- [ ] Валидация: `mod_id`, `version`, `game_versions`, `loaders`, иконка
- [ ] SHA-256 сверка (если приложен `.sha256`), отбраковка битого артефакта
- [ ] Бейсик malware-скан: поиск `Runtime.exec`, `URLClassLoader`, сетевых вызовов в `<clinit>`
- [ ] Сохранение в `mod_versions`, инвалидация Redis-кэша

### 1.5 Public REST API (Read-Heavy, Cache-First)
- [ ] `GET /api/v1/mods?query=&gameVersion=&loader=&page=&limit=` — FTS + фильтры, кэш Redis 60s
- [ ] `GET /api/v1/mods/:slug` — карточка мода + список версий
- [ ] `GET /api/v1/mods/:slug/versions/:version` — детали версии + `download_url` (прямая CDN ссылка GitHub)
- [ ] Пагинация cursor/offset, ETag, `Cache-Control`
- [ ] Rate limiting (tower_governor / redis-cell)

### 1.6 Auth & Profiles (см. docs/auth-profiles.md, adr/004)
- [ ] GitHub OAuth 2.0 (read:user, user:email) + JWT httpOnly — вход для авторов (MVP)
- [ ] PAT `personal_access_tokens` (hash, scopes, expires) — `POST /auth/tokens` для CLI/лаунчеров (MVP)
- [ ] `POST /api/v1/mods/import` — импорт репозитория (проверка LICENSE + public + манифест)
- [ ] Профили `/u/:login`, `/org/:login` — кэш ISR, sponsors, verified badge, SVG `/v1/badges/:slug/*.svg` (MVP)
- [ ] Star/Follow `stars`, `follows` (с фильтром game_version/loader) — in-app уведомления (MVP-лайт)
- [ ] Device Flow RFC8628 (`/oauth/device/code` → `/activate`) — спроектировать, реализация Phase 2
- [ ] Discord linked_accounts + бот роли Verified Modder — Phase 2
- [ ] Установка Webhook'а через GitHub App API (автоматически) — миграция с OAuth на App в Phase 2

## Phase 2 — Frontend (SvelteKit) + Social

- [ ] Дизайн-система: Tailwind / UnoCSS + токены
- [ ] Страницы: `/` (поиск + фильтры), `/mod/[slug]`, `/mods/import`, `/u/[login]`, `/org/[login]`, `/activate` (device flow)
- [ ] Компоненты: `ModCard`, `VersionTable`, `SearchBar`, `LoaderBadge`, `ProfileHeader`, `SponsorsBar`
- [ ] Клиент API (`src/lib/api.ts`) — типизированные fetch-обёртки
- [ ] SSR + кэширование, skeletons, error boundaries
- [ ] SEO / OpenGraph для карточек модов
- [ ] **Collections / Modlists** `collections`, `collection_stars` + экспорт `?format=prism|packwiz` (Phase 2 хит)
- [ ] **Activity Feed** — лента по подпискам (releases + collections + stars)
- [ ] **Org/Teams** — `/org/:login` агрегатор, `role=maintainer`
- [ ] **Геймификация** — `badges` (Early Adopter, Bug Hunter, Veteran) + Showcase SVG

## Phase 3 — Поиск, качество данных и аналитика (см. docs/analytics.md, adr/005)

- [ ] PostgreSQL FTS (`to_tsvector` + `ts_rank`) по `name`, `summary`, `README`
- [ ] `pg_trgm` для неточных совпадений / опечаток
- [ ] Опционально: `pgvector` для семантического поиска по описанию (эмбеддинги README)
- [ ] Админ-панель / ручная модерация, флаг `verified` / `suspicious`
- [ ] **Indexium Analytics (bStats аналог):**
  - [ ] Миграция `20260907000000_telemetry.sql` (`mod_telemetry_pings`, `mod_daily_stats`, `analytics_salts`)
  - [ ] `POST /api/v1/analytics/submit` (gzip, валидация, daily_salt hash, Redis 1/15мин, без IP)
  - [ ] Крон агрегация `COUNT(DISTINCT server_hash)` → `mod_daily_stats` + TTL 30д
  - [ ] `GET /mods/:slug/analytics?range=30d` + `GET /badges/:slug/servers.svg` + `sort=active_servers`
  - [ ] Легковесный Java/Kotlin SDK `dev.indexium:analytics` (MIT, SimplePie, opt-out флаг)

## Phase 4 — Надёжность и ограничения GitHub

- [ ] GitHub App Install token — 5k-12.5k RPH вместо 60 RPH анонимных
- [ ] Прямые редиректы на `objects.githubusercontent.com` — не проксировать трафик
- [ ] Retry + exponential backoff, DLQ для воркера
- [ ] Метрики: Prometheus / `tracing` + Grafana, алерты на lag очереди

## Phase 5 — Деплой и эксплуатации

- [ ] Dockerfile multi-stage для backend (distroless / alpine)
- [ ] Dockerfile для frontend (adapter-node / adapter-static)
- [ ] `docker-compose.prod.yml` / Fly.io / Railway / Hetzner
- [ ] Бэкапы Postgres (PITR), миграции в CI
- [ ] Документация деплоя (`docs/deployment.md`)

## Phase 6 — Расширения (Backlog)

- [ ] Поддержка CurseForge / Modrinth как доп. источников (опционально)
- [ ] Webhooks для лаунчеров (подписка на обновления мода)
- [ ] CLI для авторов (`indexium publish`)
- [x] Аналитика рантайма — аналог bStats (спроектирована, см. docs/analytics.md) → реализация в Phase 3
- [ ] Аналитика скачиваний (агрегация без хранения персоналки)

---

## Ближайшие 3 шага (Next Actions)

1. `docker-compose.yml` + первая миграция SQL
2. `POST /webhooks/github` с HMAC-проверкой и заглушкой очереди (in-memory channel)
3. `GET /mods` — мок-данные из БД + подключение фронта

---

## Как отмечать прогресс

- При завершении задачи ставь `[x]` и добавляй ссылку на PR/коммит: `[x] Задача (#12)`
- Если задача блочится — ставь `[!]` и опиши блокер в комментарии ниже.

## Блокеры / Вопросы

- [ ] Выбрать окончательно очередь: `Redis Streams` vs `NATS JetStream` vs `pg-queue` на старте?
  - Рекомендация: стартовать с `Redis` (уже нужен как кэш) → мигрировать на NATS если нужен strict ordering.
- [ ] Где хостить Postgres на старте — Supabase / Neon / self-hosted?
