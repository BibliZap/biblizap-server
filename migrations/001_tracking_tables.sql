-- Migration for BibliZap tracking system
-- Creates tables for Biblitest/BibliZap integration

-- Table to log BibliZap events for analysis
CREATE TABLE IF NOT EXISTS bbz_events (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    endpoint VARCHAR(100) NOT NULL,
    request_started_ms BIGINT NOT NULL,
    request_completed_ms BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    metadata JSONB
);

-- Index for fast lookups by session ID and time-based queries
CREATE INDEX IF NOT EXISTS idx_bbz_events_created_at ON bbz_events(created_at);
CREATE INDEX IF NOT EXISTS idx_bbz_events_event_type ON bbz_events(event_type);

-- Comments for documentation
COMMENT ON TABLE bbz_events IS 'Logs user events during BibliZap sessions for analytics';
COMMENT ON COLUMN bbz_events.request_started_ms IS 'Request launch time as Unix epoch milliseconds';
COMMENT ON COLUMN bbz_events.request_completed_ms IS 'Request return time as Unix epoch milliseconds';
COMMENT ON COLUMN bbz_events.metadata IS 'JSON metadata with request inputs (ids/depth/direction/etc.) and outcome summary';
