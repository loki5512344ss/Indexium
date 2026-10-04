# Indexium Analytics - собственный аналог bStats

> Даём мододелам встроенную аналитику рантайма (активные серверы/клиенты, MC/Java/OS) без сторонних сервисов. Indexium получает честную метрику популярности - не по скачиваниям (накручиваются CI), а по реальным установкам.

Основано на твоей схеме + правки под KISS/SOLID/приватность.

---

## 1. Как работает bStats (база)

1. **SDK в моде** - фоновый таймер каждые 30–60 мин собирает `mc_version, loader, java_version, os, player_count, server_uuid, custom_charts` → `POST` gzip JSON асинхронно, не блокируя главный поток.
2. **Ingestion** - бэкенд валидирует, rate-limit по `server_hash`, анонимизирует `server_uuid`.
3. **Aggregation** - сырые пинги → часовые/суточные агрегаты (Time Series), сырые удаляются по TTL.

---

## 2. Indexium реализация

### 2.1 Клиент - Lightweight Java/Kotlin модуль

```java
// Fabric/NeoForge initialize()
IndexiumMetrics metrics = new IndexiumMetrics("sodium-extra", 12345); // slug + projectId (опц)
metrics.addCustomChart(new SimplePie("config_type", () -> config.getType()));
// respects: -Dindexium.analytics.disable=true, config/indexium.json { enabled: false }
```

**Что собираем (allow-list, ничего лишнего):**
- `mc_version` (1.20.1), `loader` (fabric/neoforge/forge/quilt), `loader_version`
- `java_version` (21.0.2), `os` (linux/windows/macos - без детальной версии), `arch` (x64/arm64)
- `player_count` (0 на клиенте, N на сервере), `server_uuid` (генерим раз, храним в `config/indexium-uuid.txt`)
- `mod_version` (из `fabric.mod.json`), `custom_charts` (String→String, до 5 ключей, до 32 символов)

**Что НЕ собираем:** IP (не логируем), ник игрока, путь к файлам, список других модов (опционально по согласию, off по умолчанию).

SDK: ~15KB, без зависимостей, `CompletableFuture` + `HttpURLConnection`, gzip. Лицензия MIT, публикуем в Maven Central как `dev.indexium:analytics:1.0.0`.

### 2.2 API

- `POST /api/v1/analytics/submit` - пинг от мода (gzip JSON, `Content-Encoding: gzip` опционально)
- `GET /api/v1/mods/:slug/analytics?range=7d|30d|90d` - графики для SvelteKit
- `GET /api/v1/badges/:slug/servers.svg` - бейдж активных серверов

Пример payload (как в твоём ТЗ):
```json
{
  "mod_slug": "sodium-extra",
  "server_uuid": "e8d9a0f1-4b2c-...",
  "metrics": {
    "mc_version": "1.20.1",
    "loader": "fabric",
    "java_version": "21.0.2",
    "os": "Linux",
    "player_count": 12,
    "custom_charts": { "gui_theme": "dark" }
  }
}
```

### 2.3 Хранение - PostgreSQL (MVP) → TimescaleDB/ClickHouse при росте

На MVP хватает Postgres + daily агрегат (как ты предложил). Сырые пинги храним 30 дней, агрегаты - навсегда.

```sql
-- Полуагрегат: один пинг = одна строка, TTL 30 дней через cron
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

-- Суточный агрегат (хранится навсегда)
CREATE TABLE mod_daily_stats (
    mod_id UUID NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    active_servers INT NOT NULL DEFAULT 0, -- COUNT(DISTINCT server_hash)
    active_players INT NOT NULL DEFAULT 0, -- SUM(player_count) по последним пингам сервера за день
    breakdown_json JSONB NOT NULL, -- { mc_versions:{}, loaders:{}, os:{}, java:{}, custom:{gui_theme:{dark: 120}} }
    PRIMARY KEY (mod_id, date)
);
```

**Агрегация:** воркер-кроном раз в час: `INSERT INTO mod_daily_stats ... ON CONFLICT DO UPDATE` группировкой по `server_hash` (последний пинг сервера за день). Через `pg_cron` или tokio `interval` в бэкенде.

**Масштаб:** при >10M пингов/мес - мигрируем на TimescaleDB hypertable (`create_hypertable('mod_telemetry_pings','pinged_at')`) или ClickHouse. Схема не меняется.

### 2.4 Защита и анонимность (критично)

- **Хеш + daily_salt:** `server_hash = sha256(server_uuid + salt_for_today)`. Соль ротируется в `analytics_salts(date, salt)`, храним 2 дня. Нельзя трекать сервер сквозь дни, но можно считать уникальные за день.
- **Не храним IP:** `tower_http::TraceLayer` без IP, `X-Forwarded-For` игнорируем, в логах - `/analytics/submit 200` без IP.
- **Opt-Out:** SDK проверяет в порядке: JVM флаг `-Dindexium.analytics.disable=true` → `global_privacy.json` (`.minecraft/config/indexium.json { enabled:false }`) → `config/<modid>/indexium.json`. Если любой `false` - не шлём.
- **Rate limit:** Redis `SET server_hash:mod_slug NX EX 900` - 1 пинг / 15 мин. Ответ `429` с `Retry-After`, SDK бэкофф 30 мин.
- **Валидация:** `mod_slug` должен существовать, `mc_version`/`loader` из allow-list, `custom_charts` ≤5 ключей, `player_count` 0–10000. Иначе `400`.

---

## 3. Фичи для профиля и карточки мода

1. **Live Charts (SvelteKit + LayerChart/Chart.js):** `GET /mods/:slug/analytics?range=30d` → `{ daily: [{date, active_servers, active_players}], breakdown: {mc_versions, loaders, os} }`. Графики: активные серверы (линия), разбивка по MC (пончик), лоадерам (бар).
2. **Badge:** `https://api.indexium.example.com/v1/badges/sodium-extra/servers.svg` → `Active Servers: 1.2k` (из `mod_daily_stats` за вчера, кэш 1h).
3. **Сортировка "Real-world Usage":** `GET /mods?sort=active_servers` - `ORDER BY (SELECT active_servers FROM mod_daily_stats WHERE date = CURRENT_DATE -1)`, а не по скачиваниям. Фильтр против накрутки CI.

---

## 4. Что не делаем (чтобы не стать spyware)

- Не собираем ник, чат, координаты, список всех модов без явного согласия (если включим - отдельный `custom_charts` с opt-in).
- Не fingerprint'им по железу.
- SDK открыт (MIT) - любой может проверить что шлём (как bStats - код на GitHub).

См. `adr/005-analytics.md`, `api-spec.md` §Analytics, `database-schema.md` §telemetry.
