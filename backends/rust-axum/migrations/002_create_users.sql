CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email         VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    totp_secret   VARCHAR(32),
    totp_enabled  BOOLEAN      DEFAULT FALSE,
    created_at    TIMESTAMPTZ  DEFAULT NOW(),
    UNIQUE(tenant_id, email)
);
