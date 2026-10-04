# Профили, аккаунты и авторизация - дизайн Indexium

> Цель: максимально лёгкая, но крутая система профилей без паролей, где GitHub - источник правды.

---

## 1. TL;DR - рекомендуем для MVP

**Авторизация: только GitHub OAuth / GitHub App.** Никаких паролей, email+пароль, Google и т.д. на старте.

**Почему именно GitHub-only:**

| Плюс | Минус |
|---|---|
| 1 клик, нет форм регистрации | Отсекаем тех у кого нет GitHub (но они и моды не публикуют) |
| Доказуемое владение репозиторием (`GET /repos` с токеном) | Зависимость от GitHub OAuth (но у нас и так всё на GitHub) |
| Аватар, ник, био подтягиваются автоматически | Нет anon-публикаций (и это хорошо для open source) |
| Один токен - и публикация, и вебхуки, и профиль | Если GitHub лежит - логин не работает (редкость) |
| Нет хранения паролей, нет утечек |  |

> **Вывод:** для каталога где `1 мод = 1 GitHub репо` - GitHub-only это не ограничение, а фича. Пользователи-читатели (игроки) могут смотреть каталог **без логина вообще**. Логин нужен только авторам.

---

## 2. Роли и модель аккаунта

### Роли

- **Reader (anonymous)** - ищет, качает по прямым ссылкам, смотрит профили. Без аккаунта.
- **Author** - залогинен через GitHub, импортировал хотя бы один репо. Может публиковать релизы (через `git push` + webhook, без кнопки "загрузить jar").
- **Contributor** - указан в `mod_authors` с `role=contributor`, не обязательно owner репо. Получает бейдж на карточке мода.
- **Moderator / Admin** - ручная выдача, может ставить `verified` / `suspicious`, банить.

### Что храним (минимум GDPR)

```sql
-- уже есть authors, расширяем (см. database-schema.md)
CREATE TABLE authors (
    github_id BIGINT PRIMARY KEY,
    login VARCHAR(39) NOT NULL UNIQUE, -- github login
    display_name VARCHAR(128),         -- from GitHub name
    avatar_url TEXT,
    bio TEXT,                          -- from GitHub bio (кэш, обновляем раз в день)
    company VARCHAR(128),
    location VARCHAR(128),
    website TEXT,                      -- blog
    created_at TIMESTAMPTZ DEFAULT now(),
    last_synced_at TIMESTAMPTZ
);

CREATE TABLE mod_authors (
    mod_id UUID REFERENCES mods(id) ON DELETE CASCADE,
    github_id BIGINT REFERENCES authors(github_id) ON DELETE CASCADE,
    role VARCHAR(16) NOT NULL DEFAULT 'owner', -- owner | maintainer | contributor
    PRIMARY KEY (mod_id, github_id)
);
```

Никаких email в открытом виде (берём только для JWT, не показываем), никаких паролей. `github_id` - неизменяемый PK, `login` может смениться - обновляем по webhook `user.renamed` или при следующем логине.

### Сессии

- **JWT (httpOnly cookie)**: `sub: github_id`, `login`, `exp: 7d`. Подпись `HS256` с `JWT_SECRET` или `RS256` если хотим ротацию.
- **Не храним сессии в Redis на MVP** - stateless JWT достаточно. Позже - refresh token в `author_sessions`.
- **CSRF**: `SameSite=Lax` + `Origin` check для `POST /mods/import`.

---

## 3. Флоу авторизации (GitHub OAuth)

```
[User] → GET /auth/github → 302 https://github.com/login/oauth/authorize?client_id=...&scope=read:user,repo
  → GitHub login → 302 /auth/github/callback?code=...
  → Backend: POST https://github.com/login/oauth/access_token (code → access_token)
  → GET https://api.github.com/user (с токеном) → { id, login, avatar_url, name, bio }
  → UPSERT authors
  → Set-Cookie: indexium_token=<JWT>; HttpOnly; Secure; SameSite=Lax; Max-Age=604800
  → 302 /me или /?welcomed=1
```

**Для публикации модов нужен `repo` scope** только если хотим ставить webhook автоматически. На MVP можно `read:user` + `public_repo` (только публичные). Токен GitHub не храним долго - меняем на JWT и забываем (или храним encrypted `github_access_token` для будущих API вызовов, с возможностью revoke).

**GitHub App (альтернатива OAuth):**
- Плюс: `5k–12.5k RPH`, управление webhooks через App, `installation_id` per org.
- Минус: сложнее флоу установки.
- Рекомендация: старт с **OAuth** (проще), миграция на **GitHub App** когда упрёмся в rate limits или захотим `checks` API.

---

## 4. Профили - как сделать круто и по open source

### URL структура

- `/u/:login` - профиль пользователя (зеркало GitHub, но с модами)
- `/org/:login` - профиль организации (если `type: Organization`)
- `/mod/:slug` - карточка мода (показывает авторов с ролями)

Все профили **публичны и кэшируются** (ISR в SvelteKit).

### Что показываем на `/u/:login`

```
[avatar] flashy (@flashy) - "Minecraft modder"
  bio | 📍 Berlin | 🔗 flashy.dev | Joined 2024

Stats: 12 mods · 48 releases · 12k downloads (aggregated) · 342 stars (from GH)

Mods:
  [sodium-extra] 1.20.1 fabric - ★ 42 - MIT
  [lithium-fork] ...

Contributions: контрибьютил в 5 чужих модов (через mod_authors)

Activity: последние релизы таймлайн (из webhook_deliveries)

Links: GitHub → github.com/flashy | Indexium RSS → /u/flashy/feed.xml
```

Фишки:
- **Верификация:** бейдж `✓ Verified` если `mods` >0 и все репо публичные + лицензия. `✦ Staff` для модераторов.
- **Граф вклада:** как GitHub contributions, но по релизам модов.
- **Open Source score:** % модов с OSI лицензией, наличие `CONTRIBUTING.md`, `issues` открыты.
- **Не показываем email**, только то что уже публично на GitHub.

### Крутые идеи (backlog, но заложим)

- **Profile README** - рендерим `https://github.com/:login/:login/blob/main/README.md` если есть (как GitHub profile README).
- **Achievements:** `First Mod`, `10k Downloads`, `GPL Defender` (все моды GPL).
- **Follow:** подписка на автора (email / webhook) - `POST /u/:login/follow` → уведомляем о новых релизах (через `author_follows` таблицу).
- **Organizations:** группируем моды по `owner` (из `mods.owner`), страница `/org/:owner` агрегирует всех авторов организации.
- **Sponsors:** кнопка `Sponsor` → ссылка на `github.com/sponsors/:login` если у автора включён Sponsors.

---

## 5. Альтернативы - когда добавлять второй провайдер

| Провайдер | Когда добавлять | Как |
|---|---|---|
| **Discord OAuth** | Если заведём Discord сервер и хотим связать роли | `GET /auth/discord` → линк к `authors.discord_id`, не как замена GitHub, а как `linked_accounts` |
| **Google / Email magic link** | Если появятся читатели-комментаторы без GitHub | Только для `Reader` роли, без права публикации. Публикация всё равно требует GitHub линк (`GET /link/github`) |
| **Passkeys / WebAuthn** | Если хотим passwordless для модераторов | Избыточно на MVP |
| **Gitea / Codeberg / GitLab** | Если хотим тру-децентрализацию | Добавляем `provider: github|gitlab|codeberg` в `authors`, но каждый - отдельный OAuth. На MVP - только GitHub |

**Архитектура на будущее (не делаем сейчас, но не блокируем):**

```sql
CREATE TABLE linked_accounts (
    github_id BIGINT REFERENCES authors(github_id),
    provider VARCHAR(16) NOT NULL, -- discord | google
    provider_id VARCHAR(128) NOT NULL,
    PRIMARY KEY (provider, provider_id)
);
-- Публикация мода всё равно требует linked GitHub с доступом к репо
```

**Рекомендация:** MVP - **только GitHub**. Второй провайдер - Discord линк **после** первых 500 пользователей, если попросят.

---

## 6. Безопасность и приватность

- Никаких паролей - нечего утекать.
- `access_token` GitHub храним только в памяти/JWT, не в БД (или encrypted at rest).
- Rate limit на `/auth/*` - 10 req/min per IP.
- Удаление аккаунта: `DELETE /me` → удаляем `authors` + `mod_authors`, но `mods` остаются ( orphan → показываем `by @deleted` ), т.к. код уже open source и на GitHub.
- GDPR: `GET /me/export` → JSON со всеми данными, `DELETE` - право на забвение (кроме публичных модов).

---

## 7. Расширенная авторизация - твои идеи (оценка)

### 7.1 API Keys / PAT - **да, делаем в Phase 1**
Генерация в `/settings/tokens` с кастомными скоупами `read:mods`, `write:mods`, `webhooks:manage`.
- Хранение: `personal_access_tokens (id, github_id, token_hash, scopes[], expires_at)` - храним только `SHA256(token)` как у GitHub.
- Зачем: CI/CD (`github actions: indexium publish --token $INDEXIUM_TOKEN`), лаунчеры без браузера.
- Риск: утечка → лимит скоупов + `expires_at` 30/90 дней + `last_used_at` + revoke.
- **Вердикт:** берём в MVP - 1 таблица + 2 эндпоинта, без OAuth сервера.

### 7.2 OAuth2 Provider / Device Code Flow (RFC 8628) - **круто, но Phase 2**
Ты предлагаешь сделать Indexium IdP для лаунчеров: лаунчер показывает `ABCD-1234` → юзер на `indexium.example.com/activate` подтверждает.
- Плюс: идеален для Prism на Linux/TV/без браузера, как у GitHub CLI (`gh auth login --web`).
- Минус: нужно реализовать полноценный Authorization Server (`/oauth/authorize`, `/oauth/token`, `/oauth/device/code`, `/oauth/device/verify`) + consent screen + refresh tokens. Это +2-3 недели.
- Альтернатива на MVP: **PAT** - лаунчер просит вставить токен вручную (как `gh` с PAT). UX хуже, но без IdP.
- **Вердикт:** проектируем сейчас (закладываем `oauth_clients`, `device_codes`), реализуем после PAT когда попросят лаунчеры.

### 7.3 Discord линк - **да, но как linked_account, не как логин**
- Флоу: `GET /auth/discord` → `linked_accounts (github_id, provider='discord', provider_id)` → бот выдаёт `Verified Modder` на сервере Indexium, шлёт DM о релизах.
- Не делаем Discord как замену GitHub - публикация всё равно требует GitHub. Это синк ролей, не вход.
- **Вердикт:** делаем после MVP, когда заведём Discord сервер.

---

## 8. Фичи профиля - разбор твоих идей

### 8.1 Для разработчиков (оценка)

| Идея | Оценка | Комментарий |
|---|---|---|
| **Дашборд аналитики** (скачивания по версиям/лоадерам/OS, краш-логи) | **Phase 2** | Скачивания считаем агрегатом `downloads_daily` (без IP), OS - из `User-Agent` лаунчера если пришлёт. Краш-логи - отдельный `POST /telemetry/crash` с анонимизацией, опционально. |
| **Организации/команды** (Team CoFH) | **MVP-лайт** | Уже есть `mod_authors` + `mods.owner` (org). Делаем `/org/:login` как агрегатор, `role=maintainer` для команды. Без отдельного `teams` на старте. |
| **Спонсорство** (GitHub Sponsors, Patreon, Ko-fi) | **MVP** | Поле `authors.sponsors: JSONB { github, patreon, kofi, bmc }` + кнопки в шапке профиля/мода. Парсим из GitHub `sponsors` API или ручной ввод. |
| **Verified + PGP/GPG подпись** | **MVP / Phase 2** | `verified` уже в `mods` - ставим если репо через GitHub App и `license` ok. PGP - показываем `gpg_keys` из GitHub API (`GET /users/:login/gpg_keys`), проверка `.asc` рядом с `.jar` - Phase 2. |

### 8.2 Для игроков

| Идея | Оценка |
|---|---|
| **Коллекции / Модпаки** (`My OptiFine Alternatives`) с экспортом в Prism/CurseForge | **Phase 2, хит** | `collections (id, author_id, slug, title, mods[] JSONB, visibility)` + `collection_stars`. Экспорт - `GET /collections/:slug/export?format=prism|packwiz`. Виральная фича. |
| **Star / Follow + подписки** (уведомления о релизе под `1.20.1+fabric`) | **MVP-лайт** | `stars (github_id, mod_id)`, `follows (github_id, author_id)` + фильтр `notify_game_version/loader`. Уведомления - сначала in-app + Discord DM, email позже. |
| **Activity Feed** | **Phase 2** | Лента из `webhook_deliveries` + `collections` + `stars` по подпискам. |

### 8.3 Геймификация и виджет

- **Бейджи:** `Early Adopter` (id <1000), `Bug Hunter` (репорты), `Top Contributor` (N релизов/мес), `Open Source Veteran` (GitHub age >5 лет через `created_at` из API). Храним `badges (github_id, badge_id)` - выдаём воркером раз в день. Показываем на `/u/:login`.
- **Showcase Widget SVG:** `GET /v1/badges/:slug/downloads.svg` и `GET /v1/badges/:slug/version.svg` - генерируем SVG на лету (как `shields.io`), кэш 1h, без JS. Пример: `![Indexium](https://api.indexium.example.com/v1/badges/sodium-extra/downloads.svg)` - **делаем в MVP**, это маркетинг.

---

## 9. Итоговая приоритизация (что берём когда)

**MVP (следующие 2 недели):** GitHub OAuth only + PAT + `verified` + sponsors + star/follow (без email) + SVG badges + `/u/:login` + `/org/:login`
**Phase 2 (после 100 модов):** Device Flow + Discord linked + Collections + Activity Feed + аналитика + PGP
**Backlog:** краш-логи, `Top Contributor` лидерборд, Profile README

См. также: `catalog-philosophy.md`, `api-spec.md` (раздел Auth), `database-schema.md` (authors/mod_authors), `adr/004-auth-strategy.md` (создать).

## 10. Что решить сейчас

1. Подтверди: **PAT в MVP - да?** (я заложил, это быстро).
2. Device Flow - **проектируем сейчас, код позже** - ок?
3. Коллекции - делать сразу после MVP или откладываем до 500 юзеров?

