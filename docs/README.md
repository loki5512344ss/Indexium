# docs — Индекс документации Indexium

| Документ | Описание |
|----------|----------|
| [architecture.md](architecture.md) | Архитектура асинхронного событийного индексатора, компоненты, lifecycle, edge cases |
| [database-schema.md](database-schema.md) | Схема PostgreSQL, индексы FTS/pg_trgm, миграции |
| [api-spec.md](api-spec.md) | Public REST API v1, webhooks, auth, ошибки, rate limiting |
| [git-strategy.md](git-strategy.md) | Почему монорепо, workflow веток/коммитов |
| [deployment.md](deployment.md) | Деплой MVP на VPS, docker-compose, бэкапы, CI/CD |
| [adr/001-monorepo.md](adr/001-monorepo.md) | ADR-001: монорепо vs полирепо |
| [adr/002-async-indexer.md](adr/002-async-indexer.md) | ADR-002: асинхронный индексатор поверх GH Releases |
| [analytics.md](analytics.md) | bStats-аналог: SDK, ingestion, daily агрегаты, приватность |
| [adr/003-search-engine.md](adr/003-search-engine.md) | ADR-003: Postgres FTS + pg_trgm вместо Meilisearch |
| [adr/004-auth-strategy.md](adr/004-auth-strategy.md) | ADR-004: GitHub-only + PAT (+ Device Flow Phase 2) |
| [adr/005-analytics.md](adr/005-analytics.md) | ADR-005: bStats аналог — Postgres + daily_salt |

## Как добавлять доки

- Новые ADR: `docs/adr/NNN-kebab-title.md` по шаблону ниже.
- Диаграммы — Mermaid внутри markdown (рендерится в GitHub).

### Шаблон ADR

```markdown
# ADR-NNN: Заголовок

Дата: 2026-09-06
Статус: Принято | Отклонено | Отложено

## Контекст
...

## Решение
...

## Последствия
...
```
