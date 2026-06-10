CREATE TABLE IF NOT EXISTS click_events (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    link_id    UUID        NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    ip         INET,
    user_agent TEXT,
    referer    TEXT,
    clicked_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_click_events_link_id ON click_events(link_id);
