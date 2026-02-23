-- Migration for BibliZap tracking system
-- Creates tables for Biblitest/BibliZap integration

-- Table to link Biblitest tokens to BibliZap session IDs
CREATE TABLE IF NOT EXISTS token_link (
    id SERIAL PRIMARY KEY,
    biblitest_token VARCHAR(20) NOT NULL UNIQUE,  -- Format: BT-{12 alphanumeric}-{2 hex}
    bbz_sid UUID NOT NULL UNIQUE,
    linked_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for fast lookups by token
CREATE INDEX IF NOT EXISTS idx_token_link_biblitest_token ON token_link(biblitest_token);
CREATE INDEX IF NOT EXISTS idx_token_link_bbz_sid ON token_link(bbz_sid);

-- Table to log BibliZap events for analysis
CREATE TABLE IF NOT EXISTS bbz_events (
    id SERIAL PRIMARY KEY,
    bbz_sid UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    endpoint VARCHAR(100) NOT NULL,
    request_started_ms BIGINT NOT NULL,
    request_completed_ms BIGINT NOT NULL,
    request_duration_ms INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    metadata JSONB
);

-- If table already exists from previous runs, ensure new tracking columns are present.
ALTER TABLE bbz_events ADD COLUMN IF NOT EXISTS request_started_ms BIGINT NOT NULL DEFAULT 0;
ALTER TABLE bbz_events ADD COLUMN IF NOT EXISTS request_completed_ms BIGINT NOT NULL DEFAULT 0;
ALTER TABLE bbz_events ADD COLUMN IF NOT EXISTS request_duration_ms INTEGER NOT NULL DEFAULT 0;

-- Index for fast lookups by session ID and time-based queries
CREATE INDEX IF NOT EXISTS idx_bbz_events_bbz_sid ON bbz_events(bbz_sid);
CREATE INDEX IF NOT EXISTS idx_bbz_events_created_at ON bbz_events(created_at);
CREATE INDEX IF NOT EXISTS idx_bbz_events_event_type ON bbz_events(event_type);

-- Comments for documentation
COMMENT ON TABLE token_link IS 'Links Biblitest tokens to BibliZap session IDs for 2-hour student exercises';
COMMENT ON TABLE bbz_events IS 'Logs user events during BibliZap sessions for analytics';
COMMENT ON COLUMN token_link.biblitest_token IS 'Token from Biblitest (format: BT-{12 alphanumeric}-{2 hex CRC32})';
COMMENT ON COLUMN token_link.bbz_sid IS 'BibliZap session identifier (UUID v4)';
COMMENT ON COLUMN bbz_events.request_started_ms IS 'Request launch time as Unix epoch milliseconds';
COMMENT ON COLUMN bbz_events.request_completed_ms IS 'Request return time as Unix epoch milliseconds';
COMMENT ON COLUMN bbz_events.request_duration_ms IS 'Server-side request duration in milliseconds';
COMMENT ON COLUMN bbz_events.metadata IS 'JSON metadata with request inputs (ids/depth/direction/etc.) and outcome summary';
