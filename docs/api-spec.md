# Public REST API — Indexium v1

Base URL: `https://api.indexium.example.com/api/v1` (локально `http://localhost:3000/api/v1`)

Все ответы — `application/json`. Пагинация — `page`/`limit` (MVP) → cursor позже. Кэш — `Cache-Control: public, max-age=60`, `ETag`.

---

## Health

### `GET /health`
Проверка живости + БД + Redis.

**200**
```json
{ "status": "ok", "db": "up", "redis": "up", "version": "0.1.0" }
```

---

## Webhooks (internal)

### `POST /webhooks/github`
Принимает GitHub Webhook `release`.

Headers:
- `X-GitHub-Delivery: uuid`
- `X-Hub-Signature-256: sha256=...`
- `X-GitHub-Event: release`

Body: raw JSON от GitHub.

**202** — принято в очередь
```json
{ "status": "accepted", "delivery_id": "..." }
```
**401** — неверная подпись
**409** — уже обработано (идемпотентность)

Логика: HMAC проверка → дедуп по `delivery_id` → push в Redis Streams → 202.

---

## Mods

### `GET /mods`

Query params:

| param | type | описание |
|-------|------|----------|
| `query` | string | FTS по name/summary/README |
| `gameVersion` | string | фильтр `1.20.1` |
| `loader` | string | `fabric` \| `quilt` \| `neoforge` \| `forge` |
| `page` | int | default 1 |
| `limit` | int | default 20, max 50 |
| `sort` | string | `relevance` \| `newest` \| `popular` \| `active_servers` |

**200**
```json
{
  "data": [
    {
      "slug": "sodium-extra",
      "name": "Sodium Extra",
      "summary": "Extra optimizations",
      "author": "flashy",
      "icon_url": "https://...",
      "game_versions": ["1.20.1"],
      "loaders": ["fabric"],
      "latest_version": "1.2.3",
      "download_url": "https://github.com/.../releases/download/...",
      "updated_at": "2026-09-01T12:00:00Z"
    }
  ],
  "pagination": { "page": 1, "limit": 20, "total": 142, "pages": 8 }
}
```

Кэшируется в Redis по ключу `mods:query=...:gv=...:loader=...:page=...` TTL 60s.

### `GET /mods/:slug`

**200**
```json
{
  "slug": "sodium-extra",
  "name": "Sodium Extra",
  "summary": "...",
  "description": "... (markdown)",
  "github_repo": "owner/repo",
  "author": { "login": "flashy", "avatar_url": "https://..." },
  "icon_url": "https://...",
  "verified": true,
  "versions": [
    {
      "version_number": "1.2.3",
      "game_versions": ["1.20.1"],
      "loaders": ["fabric"],
      "download_url": "https://github.com/.../releases/download/v1.2.3/sodium-extra-1.2.3.jar",
      "file_sha256": "abc...",
      "file_size": 123456,
      "published_at": "2026-09-01T12:00:00Z"
    }
  ]
}
```
**404** — `{"error":"mod_not_found"}`

### `GET /mods/:slug/icon`

Отдаёт иконку мода. Воркер при индексации извлекает `assets/<modid>/icon.png` (или `icon` из `fabric.mod.json` → путь внутри jar) → сохраняет в кэш/проксирует.

- **200** — `image/png` / `image/webp` с `Cache-Control: public, max-age=86400`, `ETag`. Если иконки нет → `302` на `raw.githubusercontent.com` fallback или дефолтная заглушка.
- **404** — мод не найден.

> Альтернатива на MVP: не хранить иконку у себя, а отдавать `icon_url` как прямую ссылку `https://raw.githubusercontent.com/<owner>/<repo>/<branch>/src/main/resources/assets/...`. Эндпоинт `/icon` тогда — 302 редирект + кэш заголовков.

### `POST /mods/resolve` — пакетный резолв для лаунчеров

Принимает список модов + окружение, возвращает дерево прямых скачиваний и зависимостей (для Prism / Modrinth-compatible клиентов).

**Request**
```json
{
  "game_version": "1.20.1",
  "loader": "fabric",
  "mods": [
    { "slug": "sodium-extra", "version": "1.2.3" },
    { "slug": "lithium", "version": "latest" }
  ]
}
```

**200**
```json
{
  "resolved": [
    {
      "slug": "sodium-extra",
      "version_number": "1.2.3",
      "download_url": "https://github.com/.../releases/download/.../sodium-extra-1.2.3.jar",
      "file_sha256": "abc...",
      "file_size": 123456,
      "dependencies": [{ "slug": "sodium", "version_range": ">=0.5.0", "resolved_version": "0.5.8" }]
    },
    {
      "slug": "sodium",
      "version_number": "0.5.8",
      "download_url": "https://github.com/.../sodium-0.5.8.jar",
      "file_sha256": "def...",
      "file_size": 654321,
      "dependencies": []
    }
  ],
  "unresolved": []
}
```

- `version: "latest"` → резолвит последнюю совместимую с `game_version` + `loader`.
- Транзитивные зависимости резолвятся рекурсивно (BFS, max depth 20, защита от циклов).
- **422** — несовместимая комбинация `game_version`/`loader`.
- Кэшируется по ключу `resolve:gv:loader:hash(mods)` TTL 60s.

### `GET /mods/:slug/versions/:version`

Детали конкретной версии. Аналогично элементу массива выше + зависимости:

```json
{
  "mod_slug": "sodium-extra",
  "version_number": "1.2.3",
  "game_versions": ["1.20.1"],
  "loaders": ["fabric"],
  "dependencies": [{ "mod_id": "sodium", "version_range": ">=0.5.0" }],
  "download_url": "https://github.com/...",
  "file_sha256": "...",
  "file_size": 123456,
  "published_at": "..."
}
```

### `POST /mods/import` (auth required)

Импорт репозитория по GitHub OAuth.

Headers: `Authorization: Bearer <github_token>`

Body:
```json
{ "repo": "owner/repo" }
```

Логика: проверить что токен имеет доступ к репо → fetch `fabric.mod.json` из default branch → создать запись `mods` → повесить webhook.

**201** — создан
**409** — уже импортирован
**422** — манифест не найден

---

## Auth

### `GET /auth/github` → 302 redirect на GitHub OAuth
### `GET /auth/github/callback?code=...` → обмен code→token, установка httpOnly cookie / JWT

### `POST /auth/tokens` (auth) — PAT creation
Body: `{ "name": "ci-token", "scopes": ["read:mods","write:mods"], "expires_in_days": 30 }` → `201 { token: "idx_...", id, expires_at }` (токен показывается 1 раз, храним hash). `Authorization: Bearer idx_...` для API.

### `GET /auth/tokens` / `DELETE /auth/tokens/:id` — список/отзыв.

### Device Flow (Phase 2, RFC 8628)
- `POST /oauth/device/code` → `{ device_code, user_code: "ABCD-1234", verification_uri: "https://indexium.example.com/activate", expires_in: 600 }`
- `GET /activate` (frontend) — ввод `user_code` → consent → `POST /oauth/device/verify { user_code }`
- `POST /oauth/token` grant_type=`urn:ietf:params:oauth:grant-type:device_code` → `{ access_token, refresh_token }`
- Лаунчер поллит `/oauth/token` до получения токена.

### `GET /auth/discord` → линк Discord (linked_account, не логин). `GET /auth/discord/callback` → запись в `linked_accounts`.

## Profiles & Social

### `GET /u/:login` / `GET /org/:login` — публичный профиль (кэш 60s, ISR)
### `POST /mods/:slug/star` / `DELETE /mods/:slug/star` — звезда (auth)
### `POST /u/:login/follow` / `DELETE /u/:login/follow` — подписка на автора с опционально `?game_version=1.20.1&loader=fabric`
### `GET /collections` / `POST /collections` (auth, body: `{ title, description, mods: [{slug, version}] }`)
### `GET /collections/:slug` / `GET /collections/:slug/export?format=prism|packwiz`
### `GET /v1/badges/:slug/downloads.svg` / `GET /v1/badges/:slug/version.svg` — SVG виджет для README (public, кэш 1h)

## Analytics (bStats аналог, см. docs/analytics.md)

### `POST /api/v1/analytics/submit` — пинг от мода (gzip опционально)
Headers: `Content-Type: application/json`, `Content-Encoding: gzip` (optional)
Body:
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
- Валидация: `mod_slug` exists, allow-list версий/лоадеров, `custom_charts` ≤5 ключей.
- Анонимизация: `server_hash = sha256(server_uuid + daily_salt)` — IP не храним.
- Rate limit: 1 пинг / 15 мин per `server_hash+mod_slug` (Redis `SET NX EX 900`) → `429`.
- Opt-Out: respect `-Dindexium.analytics.disable=true` на клиенте.
- **200** `{ "status": "ok" }` **400** validation **429** rate_limited

### `GET /api/v1/mods/:slug/analytics?range=7d|30d|90d`
Public, кэш `public, max-age=300`.
```json
{
  "mod_slug": "sodium-extra",
  "range": "30d",
  "daily": [
    { "date": "2026-09-01", "active_servers": 450, "active_players": 3200 },
    { "date": "2026-09-02", "active_servers": 470, "active_players": 3400 }
  ],
  "breakdown": {
    "mc_versions": { "1.20.1": 400, "1.21": 50 },
    "loaders": { "fabric": 420, "neoforge": 30 },
    "os": { "Linux": 200, "Windows": 250 },
    "java": { "21": 300, "17": 150 },
    "custom": { "gui_theme": { "dark": 120, "light": 30 } }
  }
}
```
**404** mod_not_found. Источник: `mod_daily_stats`.

### `GET /api/v1/badges/:slug/servers.svg` — бейдж активных серверов (как downloads.svg, кэш 1h)
SVG `Active Servers: 1.2k` из `mod_daily_stats` за вчера.

## Sponsors & Badges

- `GET /u/:login` отдаёт `sponsors: { github, patreon, kofi, bmc }` и `badges: ["early_adopter","verified"]`
- Бейджи выдаются воркером (`badges` таблица), SVG — динамически.

---

## Ошибки

Единый формат:

```json
{ "error": "validation_error", "message": "gameVersion must be semver", "details": {...} }
```

Коды:
- `400` validation_error
- `401` unauthorized
- `404` not_found
- `429` rate_limited (headers `Retry-After`)
- `500` internal_error

---

## Rate limiting

- Public API: 60 req/min per IP (Redis).
- Webhook: без лимита, но HMAC обязателен.

---

## Версионирование

- URL версионирование `/api/v1`.
- Breaking changes → `/api/v2` + 6 мес поддержка v1.

---

## OpenAPI

Спека будет жить в `indexium-backend/openapi.yaml` (генерировать из Axum через `utoipa` когда созреет). На MVP — этот markdown как источник правды.
