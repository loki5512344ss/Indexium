# Деплой Indexium

## MVP (один VPS, <$20/мес)

### Инфра
- **VPS**: Hetzner CX22 (2 vCPU, 4GB) или аналог.
- **Postgres**: Supabase Free / Neon Free / или Docker на том же VPS.
- **Redis**: Valkey/Redis в Docker.
- **Reverse proxy**: Caddy (авто TLS) или Nginx.

### Компоновка
```
VPS
 ├─ Caddy :80/:443 → :3000 (Axum) + :5173 (SvelteKit SSR)
 ├─ indexium-backend (systemd / docker)
 ├─ indexium-frontend (adapter-node)
 ├─ postgres:5432
 └─ redis:6379
```

### Docker Compose (prod)

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: indexium
      POSTGRES_USER: indexium
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes: [pgdata:/var/lib/postgresql/data]
    healthcheck: { test: ["CMD-SHELL", "pg_isready -U indexium"] }

  redis:
    image: valkey/valkey:8-alpine
    command: ["valkey-server", "--save", ""]
    healthcheck: { test: ["CMD", "valkey-cli", "ping"] }

  backend:
    build: ./indexium-backend
    env_file: ./indexium-backend/.env
    depends_on: [postgres, redis]
    ports: ["3000:3000"]

  frontend:
    build: ./indexium-frontend
    environment: { PUBLIC_API_URL: "https://api.indexium.example.com" }
    ports: ["5173:3000"]

volumes: { pgdata: {} }
```

### Env (backend)

```
DATABASE_URL=postgres://indexium:***@postgres:5432/indexium
REDIS_URL=redis://redis:6379
GITHUB_APP_ID=...
GITHUB_APP_PRIVATE_KEY=...
WEBHOOK_SECRET=...
RUST_LOG=info
```

### Деплой шаги

```bash
# на VPS
git pull origin main
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d
# миграции
docker compose exec backend sqlx migrate run
# проверка
curl https://api.indexium.example.com/api/v1/health
```

### Бэкапы

- Postgres: ежедневный `pg_dump` + WAL (PITR если managed).
- Хранить 7 дней в S3/R2.
- Тест восстановления раз в месяц.

### Мониторинг (MVP минимум)

- `/health` + Uptime Kuma / Hetrix.
- Логи: `journalctl -u indexium-backend` или `docker logs`.
- Позже: Prometheus + Grafana + Loki.

### CI/CD (GitHub Actions)

```yaml
# .github/workflows/ci.yml
on: [push, pull_request]
jobs:
  backend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - run: bun install --cwd indexium-frontend
      - run: bun run check --cwd indexium-frontend
```

Деплой: `on: push: branches: [main]` → SSH в VPS → `git pull && docker compose up -d` (или через Watchtower).

### Масштабирование (когда >1k RPS)

- Вынести Worker в отдельный сервис/реплики.
- Postgres read-replica.
- Redis Cluster / NATS JetStream.
- Cloudflare перед API (кэш GET).
