# ADR-004: Стратегия авторизации и профилей — GitHub-only + PAT (+ Device Flow позже)

Дата: 2026-09-06
Статус: Принято (MVP) / Запроектировано (Phase 2)

## Контекст
Каталог open-source модов где 1 мод = 1 GitHub репо. Нужна минимальная, безопасная авторизация без паролей, но с поддержкой CLI/лаунчеров (Prism) и будущих Discord/коллекций. Пользователь предложил: GitHub App, PAT, Device Flow, Discord синк, дашборды, коллекции, геймификацию, SVG виджет.

## Решение

**MVP:**
- Вход только GitHub OAuth (`read:user`, `user:email`), JWT httpOnly cookie, без паролей. Читатели — anonymous.
- PAT с `SHA256` хранением и скоупами `read:mods|write:mods|webhooks:manage` для CI/лаунчеров.
- `verified` бейдж если репо публичное + лицензия + GitHub App установлен.
- Sponsors (GitHub/Patreon/Ko-fi) + star/follow (in-app) + SVG badges (`/v1/badges/:slug/downloads.svg`).

**Phase 2 (спроектировано, не кодим сейчас):**
- Device Code Flow (RFC 8628) — Indexium как OAuth2 Provider для лаунчеров (`/oauth/device/code` → `/activate`).
- Discord linked_account + бот роли `Verified Modder`.
- Collections/Modlists с экспортом Prism/packwiz, Activity Feed, аналитика по версиям/лоадерам, PGP проверка.

**Backlog:** краш-логи, лидерборды, Profile README.

## Альтернативы
- Google/email логин — отклонён для MVP (публикация всё равно требует GitHub, лишняя сложность).
- Сразу Device Flow — отклонён ( +2 недели, PAT покрывает 80% кейсов).
- Discord как логин — отклонён (только linked).

## Последствия
- Плюс: минимум GDPR, нет паролей, доказуемое владение репо, CLI готов через PAT.
- Минус: без GitHub аккаунта не опубликовать (осознанно, соответствует open-source философии).
- Миграция: таблицы `personal_access_tokens`, `linked_accounts`, `oauth_clients/device_codes` добавятся без breaking change.

## Ссылки
- `docs/auth-profiles.md` §7-10
- `docs/catalog-philosophy.md`
- `docs/api-spec.md` §Auth/Profiles
