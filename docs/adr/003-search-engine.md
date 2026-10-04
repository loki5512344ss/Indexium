# ADR-003: Использование PostgreSQL FTS и pg_trgm вместо Meilisearch/Elasticsearch

Дата: 2026-09-06  
Статус: Принято

## Контекст
Для поиска модов по названию, описанию и авторам требуется полнотекстовый поиск и устойчивость к опечаткам (fuzzy search). Введение отдельного движка поиска (Meilisearch, OpenSearch, Elasticsearch) усложняет инфраструктуру и увеличивает потребление RAM (ещё один сервис в `docker-compose`, отдельный индекс, синхронизация).

На MVP ожидается <50k модов, поисковый трафик <100 RPS, VPS за $5–10.

## Решение
Использовать возможности PostgreSQL 16:
- `tsvector` + `GIN`-индексы для ранжированного полнотекстового поиска (`to_tsvector`, `ts_rank`, `plainto_tsquery`).
- Расширение `pg_trgm` для поиска с опечатками (Trigram Similarity, оператор `%`, `similarity()`).
- Материализованный `search_vector` с триггером на `INSERT/UPDATE` (см. `docs/database-schema.md`).

Запрос: `search_vector @@ plainto_tsquery` + фильтр по `mod_versions` (`GIN (game_versions, loaders)`) + `ORDER BY ts_rank DESC` + fallback `similarity()` при 0 результатах.

## Последствия

- **Плюсы:** Нет дополнительных сервисов в `docker-compose`, экономия памяти (~0 доп. RAM vs +500MB–1GB у Meilisearch), атомарные транзакции при обновлении индекса (нет лагосинка), проще бэкапы.
- **Минусы:** При объёме >500k записей или >500 RPS скорость FTS в Postgres начинает уступать специализированным движкам (latency >100ms, нет typo-tolerance из коробки как в Meilisearch).
- **План миграции:** Если p95 latency поиска превысит 100ms (метрика в `tracing` + Grafana), вынести индекс в Meilisearch: добавить воркер-синк `mods` → Meilisearch, переключить `GET /mods` на Meilisearch с fallback на Postgres. Схема БД не меняется.

## Альтернативы (отклонены на MVP)

- **Meilisearch** - отличный typo-tolerance, но +1 сервис, нужен отдельный деплой и синк.
- **Elasticsearch / OpenSearch** - оверхед по RAM/диску, нужен кластер даже для малого объёма.
- **SQLite FTS** - не подходит, уже Postgres как основной.

## Ссылки
- `docs/database-schema.md` - секция 2 (триггер `mods_search_vector_update`), секция 3 (примеры FTS + fuzzy).
- `docs/architecture.md` - секция 4 (Поиск без Elasticsearch).
