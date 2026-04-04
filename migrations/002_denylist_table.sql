CREATE TABLE bbz_denylists (
    hash bytea PRIMARY KEY CHECK (octet_length(hash) = 32),
    data BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);