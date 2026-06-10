# Product Requirements Document (PRD)
# URL Shortener — Multi-Backend Portfolio Project

**Version**: 1.0  
**Author**: Isaias Pereira  
**Date**: 2026-05-17  
**Status**: Draft

---

## 1. Executive Summary

**Problem Statement**: Desenvolvedores que buscam vagas internacionais precisam demonstrar proficiência em múltiplas linguagens, arquitetura limpa, segurança e infraestrutura em um único projeto tangível — algo que a maioria dos portfólios não consegue comunicar de forma unificada.

**Proposed Solution**: Um sistema de encurtamento de URLs com 3 backends poliglotas (Rust/Axum, Go/Gin+GORM, Python/FastAPI) operando em paralelo, compartilhando o mesmo banco de dados PostgreSQL + Redis, com autenticação JWT + 2FA TOTP opcional, multi-tenant auto-registrado, clean architecture, e proteções contra race conditions — orquestrados via Nginx e Podman.

**Success Criteria**:
- [ ] 3 backends funcionais com 100% de paridade de API (mesmos endpoints, mesmos payloads, mesmas respostas)
- [ ] Zero race conditions sob carga concorrente de 100 req/s simultâneas nos 3 backends
- [ ] Cobertura de testes >= 80% em cada backend
- [ ] Benchmark documentado: latência p50/p95 e throughput de cada backend sob carga idêntica
- [ ] `podman compose up` inicia todos os serviços sem erros em ambiente limpo
- [ ] Frontend em inglês com links diretos para Swagger de cada backend
- [ ] README profissional com diagrama de arquitetura, instruções de setup e screenshots

---

## 2. User Experience & Functionality

### 2.1 User Personas

| Persona | Descrição |
|---|---|
| **Recrutador Técnico** | Avalia proficiência técnica em < 5 minutos. Precisa ver código limpo, documentação clara, e o projeto rodando. |
| **Tech Lead / Engineering Manager** | Avalia profundidade: arquitetura, decisões técnicas, testes, segurança, e capacidade de escrever em múltiplas stacks. |
| **Desenvolvedor (Isaias)** | Constrói, demonstra e mantém o projeto como prova de competência para vagas internacionais. |

### 2.2 User Stories

#### US-01: Encurtar URL
**As a** registered user,  
**I want to** submit a long URL and receive a shortened version,  
**so that** I can share it easily.

**Acceptance Criteria**:
- POST `/api/shorten` aceita `{ "url": "https://..." }`
- Retorna `{ "short_url": "https://<domain>/<code>", "original_url": "https://...", "short_code": "<code>" }` com status 201
- `short_code` é gerado via Base62 (caracteres alfanuméricos, 6-8 chars, URL-safe)
- Validação: rejeita URLs inválidas com 400 + mensagem de erro
- Validação: rejeita requests sem autenticação com 401
- O mesmo URL original pode gerar short codes diferentes (sem deduplicação obrigatória)
- Latência p95 < 100ms (sem cache), < 20ms (com cache Redis)

#### US-02: Redirecionamento
**As a** visitor,  
**I want to** access a shortened URL and be redirected to the original,  
**so that** I reach the intended destination.

**Acceptance Criteria**:
- GET `/:short_code` retorna HTTP 302 com header `Location` apontando para a URL original
- Incrementa contador de clicks atomicamente
- Registra click event (IP, user-agent, timestamp, referer)
- Short code inexistente retorna 404 com JSON `{ "error": "Link not found" }`
- Latência p95 < 50ms (com cache Redis)

#### US-03: Listar Links
**As a** registered user,  
**I want to** view all links I created,  
**so that** I can manage and track them.

**Acceptance Criteria**:
- GET `/api/links` retorna lista paginada (default 20, max 100 por página)
- Cada item: `{ "short_url", "original_url", "short_code", "clicks", "created_at" }`
- Retorna apenas links do tenant + usuário autenticado (isolamento multi-tenant)
- Suporta query params: `?page=2&limit=10&sort=clicks&order=desc`
- Status 200 com `{ "links": [...], "total": N, "page": N, "limit": N }`

#### US-04: Deletar Link
**As a** registered user,  
**I want to** delete a shortened link I created,  
**so that** it can no longer be accessed.

**Acceptance Criteria**:
- DELETE `/api/links/:short_code` retorna 200 com `{ "message": "Link deleted successfully" }`
- Link deletado retorna 404 no redirect
- Apenas o dono do link pode deletar (verificação de ownership)
- Deleção em cascade dos click_events associados

#### US-05: Registro e Login
**As a** new user,  
**I want to** create an account and log in,  
**so that** I can use the URL shortener.

**Acceptance Criteria**:
- POST `/api/auth/register` com `{ "email", "password", "tenant_name" }` cria tenant + user
- POST `/api/auth/login` com `{ "email", "password" }` retorna `{ "access_token", "refresh_token", "totp_required" }`
- Senha: mínimo 8 chars, com validação de força
- Password hashing com bcrypt (cost factor 12) ou argon2id
- JWT access token com expiry de 15 minutos
- JWT refresh token com expiry de 7 dias
- POST `/api/auth/refresh` com refresh token gera novo access token
- Tenant é criado automaticamente no registro (auto-provisioned)
- Slug do tenant é gerado a partir do nome (lowercase, hyphenated, unique)

#### US-06: 2FA TOTP (Opcional)
**As a** security-conscious user,  
**I want to** enable two-factor authentication,  
**so that** my account is protected against credential theft.

**Acceptance Criteria**:
- POST `/api/auth/2fa/setup` retorna `{ "secret", "qr_code_uri", "backup_codes" }`
- QR code URI compatível com Google Authenticator, Authy, etc.
- POST `/api/auth/2fa/verify` com `{ "code" }` ativa 2FA (6 dígitos, janela de 30s)
- Login com 2FA ativado retorna 428 com `{ "totp_required": true }` e exige segundo passo
- POST `/api/auth/2fa/disable` desativa 2FA (requer senha + código TOTP)
- 8 backup codes gerados no setup, single-use, hash armazenado
- 2FA é **opt-in** — usuários podem usar sem 2FA

#### US-07: Multi-Tenant Isolation
**As a** tenant admin,  
**I want to** ensure my data is isolated from other tenants,  
**so that** there is no data leakage.

**Acceptance Criteria**:
- Todo query inclui `WHERE tenant_id = ?` automaticamente (middleware/filtro)
- Usuário do tenant A não consegue acessar links do tenant B (403 Forbidden)
- Registro cria tenant dedicado — não há shared tenants
- API key / JWT contém `tenant_id` no payload — validado em cada request

#### US-08: Dashboard Frontend
**As a** user,  
**I want to** a web interface to interact with the service,  
**so that** I can manage my links without using the API directly.

**Acceptance Criteria**:
- SPA em inglês (HTML/CSS/JS vanilla, sem frameworks)
- Página de login/registro com validação client-side
- Dashboard com: formulário de encurtar, lista de links, contador de clicks
- Seletor de backend ativo: indica qual backend está sendo usado (Rust / Go / Python)
- Health check visual: mostra status de cada backend (verde/vermelho)
- Links diretos para Swagger docs de cada backend
- Copy-to-clipboard para URLs curtas
- Setup de 2FA com QR code renderizado no frontend
- Responsivo (mobile-friendly)

#### US-09: Benchmark e Comparação
**As a** portfolio reviewer,  
**I want to** see performance comparisons between the 3 backends,  
**so that** I can evaluate technical depth.

**Acceptance Criteria**:
- Script de benchmark incluído no repositório (k6 ou wrk)
- Relatório em `BENCHMARK.md` com:
  - Latência p50, p95, p99 por backend
  - Throughput (req/s) por backend
  - Uso de memória (RSS) por backend
  - Tamanho da imagem Docker por backend
- Mesma carga aplicada a todos os 3 backends simultaneamente
- Resultados reproduzíveis (instruções de como rodar)

### 2.3 Non-Goals

- **Não** incluir analytics avançado (gráficos, dashboards de métricas de click)
- **Não** incluir custom domains (ex: user.com/xyz)
- **Não** incluir expiração automática de links (campo existe no schema, mas feature não é implementada no MVP)
- **Não** incluir QR code generation para links curtos
- **Não** incluir API de terceiros / webhooks
- **Não** incluir deploy em produção (apenas local/dev)
- **Não** incluir internacionalização do frontend (apenas inglês)

---

## 3. Technical Specifications

### 3.1 Architecture Overview

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

### 3.2 Data Flow

1. **Shorten**: Client → Nginx → Backend → Valida URL → Gera short_code (Base62) → Redis lock → INSERT no PostgreSQL → Cache no Redis → Retorna short_url
2. **Redirect**: Client → Nginx → Backend → GET `/:code` → Cache Redis (hit) → Retorna 302 / Cache miss → SELECT PostgreSQL → Cache no Redis → Incrementa clicks (async) → Retorna 302
3. **Auth**: Client → Backend → Valida credentials → Gera JWT → Retorna tokens
4. **2FA**: Client → Backend → Gera TOTP secret → Retorna QR URI → Client escaneia → Client envia código → Backend verifica → Ativa 2FA

### 3.3 Database Schema

```sql
CREATE TABLE tenants (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name       VARCHAR(255) NOT NULL,
    slug       VARCHAR(100) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ  DEFAULT NOW()
);

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email         VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    totp_secret   VARCHAR(32),
    totp_enabled  BOOLEAN      DEFAULT FALSE,
    created_at    TIMESTAMPTZ  DEFAULT NOW(),
    UNIQUE(tenant_id, email)
);

CREATE TABLE links (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id      UUID         REFERENCES users(id) ON DELETE SET NULL,
    short_code   VARCHAR(20)  NOT NULL UNIQUE,
    original_url TEXT         NOT NULL,
    clicks       BIGINT       DEFAULT 0,
    created_at   TIMESTAMPTZ  DEFAULT NOW(),
    expires_at   TIMESTAMPTZ
);

CREATE TABLE click_events (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    link_id    UUID        NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    ip         INET,
    user_agent TEXT,
    referer    TEXT,
    clicked_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_links_short_code  ON links(short_code);
CREATE INDEX idx_links_tenant_id   ON links(tenant_id);
CREATE INDEX idx_click_events_link ON click_events(link_id);
CREATE INDEX idx_users_tenant      ON users(tenant_id, email);
```

### 3.4 API Contract

Todos os 3 backends expõem **exatamente** os mesmos endpoints:

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/api/auth/register` | No | Register + create tenant |
| POST | `/api/auth/login` | No | Login, returns JWT |
| POST | `/api/auth/refresh` | No | Refresh access token |
| POST | `/api/auth/2fa/setup` | Yes | Generate TOTP secret |
| POST | `/api/auth/2fa/verify` | Yes | Verify + enable 2FA |
| POST | `/api/auth/2fa/disable` | Yes | Disable 2FA |
| POST | `/api/shorten` | Yes | Create short URL |
| GET | `/:short_code` | No | Redirect (302) |
| GET | `/api/links` | Yes | List user's links |
| DELETE | `/api/links/:short_code` | Yes | Delete link |
| GET | `/health` | No | Health check |

### 3.5 Integration Points

| Component | Protocol | Purpose |
|---|---|---|
| PostgreSQL | TCP 5432 | Persistent storage (shared) |
| Redis | TCP 6379 | Cache, distributed locks, rate limiting |
| Nginx | HTTP/80, HTTPS/443 | Reverse proxy, rate limiting, security headers |
| Frontend | HTTP | SPA served as static files |

### 3.6 Security & Privacy

| Threat | Mitigation |
|---|---|
| SQL Injection | Parameterized queries (SQLx, GORM, SQLAlchemy) — nunca string concat |
| XSS | Content-Security-Policy, X-Content-Type-Options, input sanitization |
| CSRF | SameSite cookies, CSRF tokens em forms |
| Brute Force Login | Rate limiting por IP (Redis sliding window, max 5 tentativas/min) |
| Credential Theft | bcrypt/argon2id password hashing, JWT short expiry (15min) |
| Account Takeover | 2FA TOTP opcional, backup codes |
| Race Conditions | Redis distributed locks (Redlock pattern) no shorten |
| Data Leakage | Multi-tenant isolation via middleware + row-level filtering |
| Open Redirect | Validar que original_url é http/https, bloquear javascript: e data: |
| DDoS | Nginx rate limiting (10 req/s por IP para shorten, 50 req/s para redirect) |
| Sensitive Data | Nunca logar passwords, tokens, ou TOTP secrets |

### 3.7 Tech Stack por Backend

| Camada | Rust/Axum | Go/Gin | Python/FastAPI |
|---|---|---|---|
| Framework | Axum 0.8 | Gin 1.10 | FastAPI 0.115 |
| ORM/DB | SQLx (compile-time) | GORM | SQLAlchemy 2.0 async |
| Redis | redis-rs | go-redis | redis-py async |
| Auth | jsonwebtoken | golang-jwt | PyJWT |
| TOTP | totp-rs | pquerna/otp | pyotp |
| Validation | validator | go-playground/validator | Pydantic v2 |
| Hashing | argon2 | golang.org/x/crypto/argon2 | argon2-cffi |
| Docs | utoipa (OpenAPI) | swaggo | FastAPI auto |
| Container | distroless/static | distroless/base | chainguard/python |

---

## 4. Risks & Roadmap

### 4.1 Phased Rollout

#### Phase 1: Foundation (Python First)
**Goal**: Validar arquitetura com o backend mais rápido de implementar.

- Setup do monorepo e `compose.yaml` (PostgreSQL + Redis)
- Schema do banco + migrations
- Backend Python/FastAPI completo: auth, shorten, redirect, links, 2FA
- Frontend MVP: login, register, encurtar, listar
- Testes de integração Python
- `podman compose up` funcional

**Duration**: ~5-6 horas

#### Phase 2: Rust Backend
**Goal**: Backend de alta performance com clean architecture.

- Backend Rust/Axum: mesma API do Python
- SQLx + Redis + distributed locks
- 2FA TOTP + multi-tenant
- Testes unitários + integração
- Distroless container

**Duration**: ~8-10 horas

#### Phase 3: Go Backend
**Goal**: Completar o trio poliglota.

- Backend Go/Gin + GORM: mesma API
- Redis + distributed locks
- 2FA TOTP + multi-tenant
- Swagger docs (swaggo)
- Distroless container

**Duration**: ~6-8 horas

#### Phase 4: Nginx + Frontend Completo
**Goal**: Integração final e polish.

- Nginx config: routing, rate limiting, security headers
- Frontend completo: dashboard, 2FA setup, health checks, backend selector
- Links para Swagger de cada backend
- Copy-to-clipboard, responsivo

**Duration**: ~4-5 horas

#### Phase 5: Security Hardening + Benchmarks
**Goal**: Produção-readiness e documentação de performance.

- Rate limiting em todos os backends
- CSRF, XSS, input validation
- Audit logging
- Benchmark com k6/wrk
- `BENCHMARK.md` com resultados
- Testes de concorrência (race conditions)

**Duration**: ~4-5 horas

#### Phase 6: Documentation
**Goal**: Portfólio-ready.

- README principal com arquitetura, setup, screenshots
- README por backend (decisões técnicas)
- Diagrama de arquitetura
- Cobertura de testes documentada

**Duration**: ~3-4 horas

### 4.2 Technical Risks

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| Inconsistência de API entre backends | Alto | Médio | Definir OpenAPI spec primeiro, gerar clientes, testar paridade |
| Race conditions no shorten | Alto | Médio | Redis distributed locks + unique constraint no DB |
| N+1 queries na listagem de links | Médio | Alto | Eager loading (SQLx joins, GORM Preload, SQLAlchemy selectinload) |
| Distroless images sem ferramentas de debug | Baixo | Alto | Multi-stage build com busybox para shell de debug |
| TOTP sync issues entre backends | Baixo | Baixo | TOTP é stateless — depende apenas do secret + tempo |
| Podman compatibility issues | Médio | Baixo | Testar com `podman compose` desde o início, não Docker |
| Cobertura de testes < 80% | Médio | Médio | Escrever testes junto com o código, não depois |

### 4.3 Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| Podman | >= 4.0 | Container runtime |
| Podman Compose | >= 1.0 | Orchestration |
| PostgreSQL | 16 | Database |
| Redis | 7 | Cache + locks |
| Nginx | 1.25+ | Reverse proxy |
| Rust | 1.75+ | Backend 1 |
| Go | 1.22+ | Backend 2 |
| Python | 3.12+ | Backend 3 |

### 4.4 Testing Strategy

| Type | Tool (Rust) | Tool (Go) | Tool (Python) |
|---|---|---|---|
| Unit | `cargo test` | `go test` | `pytest` |
| Integration | `sqlx` test containers | `testcontainers-go` | `pytest + testcontainers` |
| API | `reqwest` mock | `httptest` | `httpx` + `TestClient` |
| Load | \multicolumn{3}{c|}{k6 / wrk (shared)} |
| Concurrency | \multicolumn{3}{c|}{Test script com 100 concurrent requests} |

**Coverage Target**: >= 80% line coverage por backend.

---

## 5. Success Metrics (KPIs)

| KPI | Target | Measurement |
|---|---|---|
| API Parity | 100% | Mesmos inputs → mesmos outputs nos 3 backends |
| Race Condition Free | 0 failures em 10k concurrent requests | Script de teste de concorrência |
| Test Coverage | >= 80% | `cargo tarpaulin`, `go test -cover`, `pytest-cov` |
| Latency p95 (redirect) | < 50ms | k6 benchmark |
| Latency p95 (shorten) | < 100ms | k6 benchmark |
| `podman compose up` | Zero errors em ambiente limpo | CI script ou manual test |
| Lighthouse (frontend) | >= 90 Performance, 100 Accessibility | `lighthouse-ci` |
| Security Headers | A+ rating | Mozilla Observatory ou similar |

---

## Appendix A: Short Code Generation

**Algorithm**: Base62 encoding de um integer incremental ou UUID truncated.

**Opção recomendada**: NanoID com alphabet `0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ` (62 chars), tamanho 7 = ~3.5 trilhões de combinações, colisão probability < 0.001% com 1M de links.

**Collision handling**: Se colisão detectada (unique constraint violation), regenerar e retry (max 3 attempts).

---

## Appendix B: Redis Key Schema

| Key Pattern | TTL | Purpose |
|---|---|---|
| `url:{short_code}` | 24h | Cache de redirect (original_url) |
| `lock:shorten:{hash}` | 5s | Distributed lock para prevenir race condition |
| `ratelimit:ip:{ip}` | 60s | Sliding window rate limiter |
| `ratelimit:user:{user_id}` | 60s | Rate limiter por usuário autenticado |

---

## Appendix C: Nginx Security Headers

```
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 0
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
```
