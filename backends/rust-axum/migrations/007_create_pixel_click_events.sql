CREATE TABLE IF NOT EXISTS pixel_click_events (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pixel_id   UUID NOT NULL REFERENCES pixels(id) ON DELETE CASCADE,
    ip         INET,
    user_agent TEXT,
    referer    TEXT,
    country    VARCHAR(2),
    city       VARCHAR(255),
    latitude   DOUBLE PRECISION,
    longitude  DOUBLE PRECISION,
    isp        VARCHAR(255),
    clicked_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pixel_click_events_pixel_id ON pixel_click_events(pixel_id);