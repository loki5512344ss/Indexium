# Indexium

> Лёгкий, дешёвый и устойчивый к лимитам GitHub индексатор модов Minecraft.  
> Бэкенд - **асинхронный событийный индексатор**: не хранит тяжёлые `.jar`, индексирует метаданные из GitHub Releases CDN и отдаёт быстрые JSON-ответы.

## Архитектура (TL;DR)

```
GitHub --webhook release.published--> Ingestion API (HMAC check) --> Redis Queue --> Worker (Range-Request .jar → parse manifest) --> PostgreSQL --> Redis Cache --> Public REST API --> Web UI / Launchers
```

Подробно: [`docs/architecture.md`](docs/architecture.md)

## Монорепо

```
/
├── indexium-backend/   # Rust + Axum + sqlx + tokio
├── indexium-frontend/  # SvelteKit + TypeScript + Vite
├── docs/               # Архитектура, API, схема БД, ADR
├── todo.md             # Роадмап по фазам
└── docker-compose.yml  # (WIP) Postgres + Redis
```

Почему монорепо: см. [`docs/git-strategy.md`](docs/git-strategy.md) - один clone, атомарные изменения API+UI, один CI.

## Быстрый старт (локально)

```bash
# 1. Инфра
docker compose up -d          # postgres + redis (когда будет compose файл)

# 2. Бэкенд
cd indexium-backend
cp .env.example .env
sqlx migrate run
cargo run

# 3. Фронт
cd ../indexium-frontend
bun install    # или npm install
bun run dev
```

- Backend: `http://localhost:3000/api/v1/health`
- Frontend: `http://localhost:5173`

## Стек

| Слой | Технология | Зачем |
|------|-----------|-------|
| Backend Core | Rust (Axum) | Минимальный footprint, высокий throughput, async I/O |
| DB | PostgreSQL + FTS + pg_trgm | `JSONB`, полнотекстовый поиск без Elasticsearch, `pgvector` опционально |
| Cache & Queue | Redis / Valkey | Очередь (Streams) + L2-кэш популярных эндпоинтов |
| Worker | Rust tokio task | Парсинг `.jar` (zip-header stream), валидация, malware-чек |
| Auth | GitHub App / OAuth 2.0 | Только GitHub-вход, без локальных паролей |
| Frontend | SvelteKit | SSR, лёгкий бандл |

## Документация

- [`docs/architecture.md`](docs/architecture.md) - компоненты, lifecycle релиза, обход лимитов
- [`docs/database-schema.md`](docs/database-schema.md) - схема БД + индексы
- [`docs/api-spec.md`](docs/api-spec.md) - Public REST API v1
- [`docs/deployment.md`](docs/deployment.md) - деплой, бэкапы
- [`docs/git-strategy.md`](docs/git-strategy.md) - почему монорепо и как работать с ним
- [`todo.md`](todo.md) - роадмап по фазам

## Лицензия

TBD

## Контакты / Issues

Используй GitHub Issues для багов и фич. Перед PR - `cargo fmt && cargo clippy`.
