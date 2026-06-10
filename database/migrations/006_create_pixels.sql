CREATE TABLE IF NOT EXISTS pixels (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id  UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id    UUID REFERENCES users(id) ON DELETE SET NULL,
    code       VARCHAR(20) NOT NULL UNIQUE,
    name       VARCHAR(255),
    clicks     BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pixels_code ON pixels(code);
CREATE INDEX IF NOT EXISTS idx_pixels_tenant ON pixels(tenant_id);