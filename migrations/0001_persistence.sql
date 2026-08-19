CREATE TABLE IF NOT EXISTS chain_evidence_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO chain_evidence_migrations(version)
VALUES ('0001_persistence')
ON CONFLICT (version) DO NOTHING;

CREATE TABLE IF NOT EXISTS chain_evidence_blocks (
    chain_id BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    parent_hash TEXT NOT NULL,
    canonical BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (chain_id, block_hash),
    UNIQUE (chain_id, block_number, block_hash)
);

CREATE TABLE IF NOT EXISTS chain_evidence_events (
    chain_id BIGINT NOT NULL,
    block_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    tx_hash TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    decoder_id TEXT NOT NULL,
    state_key TEXT NOT NULL,
    state_value TEXT NOT NULL,
    canonical BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (chain_id, block_hash, log_index),
    FOREIGN KEY (chain_id, block_hash)
        REFERENCES chain_evidence_blocks(chain_id, block_hash)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chain_evidence_checkpoints (
    chain_id BIGINT PRIMARY KEY,
    schema_version TEXT NOT NULL,
    canonical_tip TEXT NOT NULL,
    canonical_height BIGINT NOT NULL,
    state_sha256 TEXT NOT NULL,
    report_sha256 TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS chain_evidence_blocks_canonical_idx
    ON chain_evidence_blocks(chain_id, canonical, block_number);

CREATE INDEX IF NOT EXISTS chain_evidence_events_canonical_idx
    ON chain_evidence_events(chain_id, canonical, block_hash, log_index);
