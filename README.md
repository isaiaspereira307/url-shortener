# URL Shortener — Multi-Backend Portfolio Project

A URL shortener system with 3 polyglot backends (Rust/Axum, Go/Gin, Python/FastAPI) running in parallel, sharing PostgreSQL + Redis, with JWT + 2FA TOTP authentication, multi-tenant auto-registration, and clean architecture.

## Quick Start

### 1. Configure Environment

Copy the example environment file and generate secure secrets:

```bash
cp .env.example .env

# Generate a strong JWT secret
openssl rand -base64 48 >> .env  # or use any 32+ char random string

# Edit .env with your preferred password
nano .env
```

**Required variables in `.env`:**
- `POSTGRES_PASSWORD` — Database password (required)
- `JWT_SECRET` — JWT signing key, min 32 chars (required)

### 2. Start Services

```bash
podman compose up -d --build
```

### 3. Access the Application

- **Frontend**: http://localhost
- **Python API docs**: http://localhost/api/python/docs/
- **Rust API docs**: http://localhost/api/rust/docs/
- **Go API docs**: http://localhost/api/go/docs/

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Nginx (:80/:443)                       │
│  ┌─────────────────┐  ┌──────────────────────────────────┐   │
│  │   Frontend SPA  │  │         API Routing               │   │
│  │   (static)      │  │  /api/rust/*    → :8001          │   │
│  │                 │  │  /api/go/*      → :8002          │   │
│  │                 │  │  /api/python/*  → :8003          │   │
│  │                 │  │  /:short_code   → round-robin    │   │
│  └─────────────────┘  └──────────────────────────────────┘   │
└──────────────────────────┬───────────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
     ┌────────────┐ ┌────────────┐ ┌────────────┐
     │ Rust/Axum  │ │  Go/Gin    │ │ Python/    │
     │   :8001    │ │   :8002    │ │ FastAPI    │
     │            │ │            │ │   :8003    │
     └──────┬─────┘ └──────┬─────┘ └──────┬─────┘
            │              │              │
            └──────────────┼──────────────┘
                           ▼
              ┌────────────────────────┐
              │    PostgreSQL (:5432)   │
              │    (shared database)    │
              └────────────────────────┘
                           ▲
              ┌────────────────────────┐
              │     Redis (:6379)       │
              │  cache + distributed    │
              │  locks + rate limiting  │
              └────────────────────────┘
```

## Tech Stack

| Layer | Rust/Axum | Go/Gin | Python/FastAPI |
|-------|-----------|--------|----------------|
| Framework | Axum 0.8 | Gin 1.10 | FastAPI 0.115 |
| ORM/DB | SQLx | GORM | SQLAlchemy 2.0 async |
| Redis | redis-rs | go-redis | redis-py async |
| Auth | jsonwebtoken | golang-jwt | PyJWT |
| TOTP | totp-rs | pquerna/otp | pyotp |
| Container | distroless/cc | distroless/base | chainguard/python |

## Security

- Password hashing with Argon2
- JWT access tokens (15min) + refresh tokens (7 days)
- Optional 2FA TOTP (Google Authenticator compatible)
- Multi-tenant isolation via `tenant_id` filtering
- Redis distributed locks to prevent race conditions
- Rate limiting at Nginx level (10 req/s shorten, 50 req/s redirect, 5 req/s auth)
- Security headers: HSTS, CSP, X-Frame-Options, X-Content-Type-Options
- Input validation on all endpoints
- Parameterized queries (no SQL injection)
- URL scheme validation (blocks javascript:, data:, internal hosts)

## Project Structure

```
├── backends/
│   ├── rust-axum/       # Rust backend (Axum + SQLx)
│   ├── go-gin/          # Go backend (Gin + GORM)
│   └── python-fastapi/  # Python backend (FastAPI + SQLAlchemy)
├── frontend/            # Vanilla JS SPA
├── nginx/               # Reverse proxy config
├── database/migrations/ # SQL migrations
├── scripts/             # Setup scripts
├── compose.yaml         # Podman/Docker Compose
└── .env.example         # Environment template
```

## License

MIT
