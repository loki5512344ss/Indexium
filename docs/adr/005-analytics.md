# ADR-005: Собственный bStats-аналог для Indexium Analytics

Дата: 2026-09-06
Статус: Принято (дизайн) / К реализации в Phase 3

## Контекст
Скачивания накручиваются CI, нужна честная метрика популярности — активные установки в рантайме. bStats де-факто стандарт для Minecraft модов: lightweight SDK → POST gzip JSON → агрегация. Пользователь предложил полный дизайн с `server_uuid`, daily_salt, `mod_telemetry_pings` + `mod_daily_stats`, opt-out и сортировкой `active_servers`.

## Решение
- **SDK:** MIT Java/Kotlin модуль `dev.indexium:analytics` ~15KB, `IndexiumMetrics(slug)` + `SimplePie`, уважает `-Dindexium.analytics.disable=true` и `config/indexium.json`.
- **Ingestion:** `POST /api/v1/analytics/submit` (gzip, без IP логов) → валидация allow-list → `server_hash = sha256(uuid + daily_salt)` → Redis 1/15мин → `mod_telemetry_pings` (TTL 30d).
- **Storage:** Postgres `mod_telemetry_pings` + `mod_daily_stats (breakdown_json)` + `analytics_salts`. Кроном `COUNT(DISTINCT server_hash)` раз в час. Хватает до 10M пингов/мес, далее TimescaleDB hypertable без смены схемы.
- **Serving:** `GET /mods/:slug/analytics?range=7d|30d|90d` (кэш 5м), `GET /badges/:slug/servers.svg`, `GET /mods?sort=active_servers`.
- **Приватность:** не храним IP, daily_salt ротация (не трекать сквозь дни), `custom_charts` ≤5 ключей, opt-out на клиенте.

## Альтернативы
- Сторонний bStats.org — отклонён (внешняя зависимость, нет контроля, нет breakdown по нашим лоадерам).
- ClickHouse сразу — отклонён (оверхед для MVP, Postgres хватает).
- Хранить сырые пинги навсегда — отклонён (раздувание, достаточно daily агрегата).

## Последствия
- Плюс: честная сортировка `active_servers`, графики для авторов, бейджи, без сторонних сервисов.
- Минус: +2 таблицы, крон-агрегация, SDK нужно публиковать в Maven Central.
- План: сначала Axum handler + агрегация, потом SDK (или наоборот — можно параллельно).

## Ссылки
- `docs/analytics.md`
- `docs/api-spec.md` §Analytics
- `docs/database-schema.md` §3
