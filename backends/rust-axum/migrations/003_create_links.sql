CREATE TABLE IF NOT EXISTS links (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id      UUID         REFERENCES users(id) ON DELETE SET NULL,
    short_code   VARCHAR(20)  NOT NULL UNIQUE,
    original_url TEXT         NOT NULL,
    clicks       BIGINT       DEFAULT 0,
    created_at   TIMESTAMPTZ  DEFAULT NOW(),
    expires_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_links_short_code ON links(short_code);
CREATE INDEX IF NOT EXISTS idx_links_tenant_id  ON links(tenant_id);
