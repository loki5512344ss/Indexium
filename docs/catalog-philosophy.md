# Философия каталога Indexium - Open Source Only, Zero Storage

> **Тезис:** Indexium - не хостинг файлов. Ты даёшь свой GitHub, мы даём индексацию, поиск и доверие. Все моды в каталоге обязаны быть open source.

---

## 1. Принцип Zero Storage

| Храним у себя | НЕ храним у себя |
|---|---|
| Метаданные `fabric.mod.json` / `mods.toml` | `.jar` / `.zip` артефакты |
| `README.md`, `LICENSE`, иконка (кэш) | Скомпилированный байткод |
| `SHA256`, `file_size`, `game_versions`, `loaders` | Исходники (берём с GitHub) |
| `search_vector` для FTS | Логи скачиваний с IP |

**Как работает:** 
- Релиз публикуется в `github.com/<owner>/<repo>/releases` → webhook → воркер делает 2-3 `Range Request` к CDN (`objects.githubusercontent.com`) → парсит только центральную директорию ZIP → сохраняет метаданные в Postgres → отдаёт клиенту **прямую ссылку** `https://github.com/.../releases/download/...`
- Трафик не идёт через нас. Мы не платим за egress, не упираемся в лимиты хранения.

**Почему это круто:**
- Дешёво: VPS $5 + managed Postgres, без S3.
- Честно: автор контролирует файлы, может удалить релиз - он пропадёт и у нас (через webhook `release.deleted`).
- Устойчиво к DMCA: мы - индексатор, а не дистрибьютор (как `crates.io` vs `GitHub`).

---

## 2. Open Source Only - честь и правило

### Что значит "обязан быть open source"

Мод принимается в каталог только если:

1. **Репозиторий публичный** (`private: false` через GitHub API).
2. **Есть файл лицензии** в корне: `LICENSE` / `COPYING` / `LICENSE.md`. Проверяем через `GET /repos/{owner}/{repo}/license` - поле `license.spdx_id != null` и `license.spdx_id != "NOASSERTION"`.
3. **Лицензия из allow-list OSI:** `MIT`, `Apache-2.0`, `GPL-2.0`, `GPL-3.0`, `LGPL-2.1`, `LGPL-3.0`, `MPL-2.0`, `BSD-2/3-Clause`, `CC0-1.0`, `Unlicense`, `EUPL-1.2`, `AGPL-3.0`. Список расширяется через ADR.
4. **Исходники соответствуют артефакту** (best-effort): проверяем что в репо есть `fabric.mod.json` / `gradle.properties` с тем же `mod_id`/`version` что и в `.jar`. Полная reproducible-build проверка - в backlog.
5. **Нет обфускации/шифрования** в релизе без исходников: если воркер находит `Runtime.exec` без открытого кода - флаг `suspicious`.

> **На MVP** достаточно п.1 + п.2 (любая распознанная лицензия GitHub). Строгий OSI allow-list включаем после первых 100 модов.

### Как проверяем при импорте

```
POST /mods/import { repo: "owner/repo" }
  → GitHub API: GET /repos/{repo} → private? reject 422
  → GET /repos/{repo}/license → null? reject 422 "LICENSE required - open source only"
  → GET /repos/{repo}/contents/fabric.mod.json?ref=main → not found? reject
  → Создаём mods + ставим webhook
```

При каждом `release.published` повторно проверяем лицензию - если автор сменил на `NOASSERTION`/сделал приватным → мод помечается `deprecated`, скрывается из поиска, но старые версии доступны (кэш).

### Что показываем пользователю

- Бейдж `OSI: MIT` на карточке мода, ссылка на `LICENSE` на GitHub.
- Фильтр `license:MIT` в поиске.
- Страница `/manifesto` - манифест: "Почему только open source" (прозрачность, безопасность, форки, обучение).

### Edge cases

- **Форки:** разрешены, но `slug` уникален, показываем `fork_of: owner/repo`. Оригинал помечается `upstream`.
- **Мульти-мод репо (монорепо):** на MVP 1 репо = 1 мод. Позже - поддержка `mods.toml` с несколькими `modId`.
- **Организация vs личный акк:** оба ок, если репо публичное и лицензия есть.
- **Что если автор закрыл репо?** Webhook `repository.privatized` → скрываем мод, чистим кэш, храним метаданные 30 дней для восстановления.

---

## 3. Что это даёт экосистеме

- **Доверие:** любой может `git clone`, проверить код, собрать самому - нет "левый jar с майнером".
- **Долговечность:** даже если Indexium умрёт, моды живут на GitHub.
- **Культура:** стимулируем PR'ы, а не "скачал и забыл". Профили показывают контрибьюторов, а не только owner.

---

## 4. Что НЕ делаем

- Не принимаем бинарники без исходников (даже если автор "обещает" открыть позже).
- Не зеркалируем закрытые репозитории, даже с токеном.
- Не храним `.jar` у себя даже кэшем (кроме 64KB хвоста для парсинга - эфемерно).

См. также: `docs/auth-profiles.md` - как профили усиливают open source (контрибьюторы, верификация), `docs/adr/004-open-source-only.md`.
