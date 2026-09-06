# Git-стратегия — почему монорепо

## Решение (ADR-001)

**Выбрано: монорепо в корне `/` с двумя пакетами `indexium-backend/` и `indexium-frontend/`.**

Альтернатива — полирепо (два отдельных git) — отклонена на старте.

## Почему монорепо

| Критерий | Монорепо | Полирепо |
|----------|----------|----------|
| Onboarding нового разработчика | `git clone` один раз, `docker compose up` | 2 clone, синхронизация версий |
| Атомарные изменения API+UI | Один коммит/PR меняет `api-spec` + фронт-клиент | Два PR, риск рассинхрона |
| CI | Один pipeline, один статус | Два pipeline, дублирование |
| Версионирование контрактов | Фронт всегда соответствует бэку в `main` | Нужен отдельный версионинг |
| Стоимость поддержки | Минимальна для 1-3 человек | Оверхед: 2 набора настроек, 2 issue-треккера |

Монорепо оправдан пока команда <10 человек и релизный цикл единый. Если в будущем бэкенд и фронт разойдутся по командам/каденсу — легко разрезать через `git filter-repo` или `git subtree`.

## Что было сделано

1. Удалён пустой `.git` из `indexium-backend/` (коммитов не было — безопасно).
2. `git init --initial-branch=main` в корне `Indexium/`.
3. Корневой `.gitignore` + локальные.
4. Весь код теперь трекается как:
   ```
   Indexium/
     .git/
     .gitignore
     README.md
     todo.md
     docs/
     indexium-backend/
     indexium-frontend/
   ```

## Workflow

### Ветки
- `main` — защищённая, только через PR.
- `feat/<scope>-<short>` — фичи, напр. `feat/webhook-hmac`.
- `fix/<scope>-<short>`.

### Коммиты (Conventional Commits)
```
feat(api): add GET /mods with FTS
fix(worker): handle missing quilt.mod.json
docs(arch): describe queue retry
chore(frontend): bump svelte 5.56 → 5.57
```

### PR
- Один PR = одна фича/фикс.
- Если меняется API — в том же PR обновляется `docs/api-spec.md` и фронт-клиент.
- CI должен пройти: `cargo fmt --check`, `cargo clippy`, `cargo test`, `svelte-check`.

### Локально
```bash
git clone <url> Indexium && cd Indexium
git checkout -b feat/my-feature
# ... код ...
cargo fmt && cargo clippy -- -D warnings
git add -A && git commit -m "feat(scope): message"
git push -u origin feat/my-feature
# → создать PR
```

## Когда резать на полирепо

Сигналы что пора:
- >10 активных контрибьюторов, частые конфликты в `main`.
- Фронт деплоится 10× в день, бэк — 1× в неделю (разный каденс).
- Нужны разные права доступа (внешние контрибьюторы только к фронту).

Как резать: `git subtree split -P indexium-backend -b backend-only` и аналогично для фронта, либо `git filter-repo --path`.

## Альтернативы (для справки)

- **Git submodules** — не рекомендуется: сложны, легко сломать, плохой DX.
- **Polyrepo + shared package** — имеет смысл если выносить `openapi`/`types` в отдельный npm/crate.

## ADR

- ADR-001: Монорепо vs полирепо — принято монорепо (этот документ).
- Следующие ADR складывать в `docs/adr/NNN-title.md`.
