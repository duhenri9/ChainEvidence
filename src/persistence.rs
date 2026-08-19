use crate::engine::report_fixture;
use crate::models::{CanonicalEventEvidence, Fixture};
use postgres::{Client, NoTls};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

pub const SCHEMA_VERSION: &str = "0001_persistence";
pub const PERSISTENCE_CLAIM_BOUNDARY: &str = "V0.2 proves transactional persistence and restart/recovery semantics only against the declared PostgreSQL schema and synthetic fixtures. It does not establish production durability, replication/failover guarantees, RPC correctness, blockchain finality or disaster recovery.";
const MIGRATION: &str = include_str!("../migrations/0001_persistence.sql");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistMode {
    Commit,
    AbortBeforeCommit,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PersistenceEvidence {
    pub schema: String,
    pub outcome: String,
    pub schema_version: String,
    pub chain_id: u64,
    pub canonical_tip: String,
    pub canonical_height: u64,
    pub canonical_block_count: usize,
    pub canonical_event_count: usize,
    pub state_sha256: String,
    pub source_report_sha256: String,
    pub evidence_sha256: String,
    pub claim_boundary: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecoveryEvidence {
    pub schema: String,
    pub outcome: String,
    pub schema_version: String,
    pub chain_id: u64,
    pub checkpoint_tip: String,
    pub checkpoint_height: u64,
    pub canonical_block_hashes: Vec<String>,
    pub canonical_event_count: usize,
    pub material_state: BTreeMap<String, String>,
    pub state_sha256: String,
    pub source_report_sha256: String,
    pub recovery_sha256: String,
    pub claim_boundary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistenceError {
    code: &'static str,
    message: String,
}

impl PersistenceError {
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PersistenceError {}

#[derive(Serialize)]
struct MaterialSnapshot<'a> {
    canonical_block_hashes: &'a [String],
    canonical_events: &'a [CanonicalEventEvidence],
    material_state: &'a BTreeMap<String, String>,
}

fn db_error(context: &str, error: postgres::Error) -> PersistenceError {
    PersistenceError::new("DATABASE_ERROR", format!("{context}: {error}"))
}

fn i64_value(value: u64, field: &str) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| {
        PersistenceError::new(
            "INTEGER_RANGE",
            format!("{field} value {value} cannot be represented by PostgreSQL BIGINT"),
        )
    })
}

fn i32_value(value: u32, field: &str) -> Result<i32, PersistenceError> {
    i32::try_from(value).map_err(|_| {
        PersistenceError::new(
            "INTEGER_RANGE",
            format!("{field} value {value} cannot be represented by PostgreSQL INTEGER"),
        )
    })
}

fn u64_value(value: i64, field: &str) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| {
        PersistenceError::new(
            "INTEGER_RANGE",
            format!("database {field} value {value} is negative/out of range"),
        )
    })
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable persistence evidence");
    format!("{:x}", Sha256::digest(bytes))
}

fn state_sha256(
    canonical_block_hashes: &[String],
    canonical_events: &[CanonicalEventEvidence],
    material_state: &BTreeMap<String, String>,
) -> String {
    hash_json(&MaterialSnapshot {
        canonical_block_hashes,
        canonical_events,
        material_state,
    })
}

pub fn connect(database_url: &str) -> Result<Client, PersistenceError> {
    Client::connect(database_url, NoTls).map_err(|error| db_error("connect", error))
}

pub fn migrate(client: &mut Client) -> Result<(), PersistenceError> {
    client
        .batch_execute(MIGRATION)
        .map_err(|error| db_error("apply migration", error))
}

pub fn clear_chain(client: &mut Client, chain_id: u64) -> Result<(), PersistenceError> {
    migrate(client)?;
    let chain_id = i64_value(chain_id, "chain_id")?;
    let mut tx = client
        .transaction()
        .map_err(|error| db_error("begin clear transaction", error))?;
    tx.execute(
        "DELETE FROM chain_evidence_checkpoints WHERE chain_id = $1",
        &[&chain_id],
    )
    .map_err(|error| db_error("delete checkpoint", error))?;
    tx.execute(
        "DELETE FROM chain_evidence_events WHERE chain_id = $1",
        &[&chain_id],
    )
    .map_err(|error| db_error("delete events", error))?;
    tx.execute(
        "DELETE FROM chain_evidence_blocks WHERE chain_id = $1",
        &[&chain_id],
    )
    .map_err(|error| db_error("delete blocks", error))?;
    tx.commit()
        .map_err(|error| db_error("commit clear transaction", error))
}

pub fn persist_fixture(
    client: &mut Client,
    fixture: &Fixture,
    mode: PersistMode,
) -> Result<PersistenceEvidence, PersistenceError> {
    migrate(client)?;
    let source = report_fixture(fixture);
    if source.outcome != "PASS" {
        return Err(PersistenceError::new(
            "SOURCE_INDETERMINATE",
            format!(
                "fixture cannot be persisted as canonical evidence: {}",
                source
                    .error
                    .unwrap_or_else(|| "unknown source error".to_owned())
            ),
        ));
    }

    let canonical_tip = source
        .canonical_block_hashes
        .last()
        .cloned()
        .ok_or_else(|| PersistenceError::new("EMPTY_CANONICAL_CHAIN", "no canonical tip"))?;
    let canonical_height = fixture
        .blocks
        .iter()
        .find(|block| block.hash == canonical_tip)
        .map(|block| block.number)
        .ok_or_else(|| {
            PersistenceError::new(
                "CANONICAL_BLOCK_MISSING",
                format!("canonical tip {canonical_tip} is not present in fixture"),
            )
        })?;

    let chain_id_db = i64_value(fixture.chain_id, "chain_id")?;
    let height_db = i64_value(canonical_height, "canonical_height")?;
    let mut tx = client
        .transaction()
        .map_err(|error| db_error("begin persistence transaction", error))?;

    for block in &fixture.blocks {
        let block_number = i64_value(block.number, "block_number")?;
        tx.execute(
            "INSERT INTO chain_evidence_blocks \
             (chain_id, block_hash, block_number, parent_hash, canonical) \
             VALUES ($1, $2, $3, $4, FALSE) \
             ON CONFLICT (chain_id, block_hash) DO NOTHING",
            &[&chain_id_db, &block.hash, &block_number, &block.parent_hash],
        )
        .map_err(|error| db_error("insert block", error))?;

        let identity = tx
            .query_one(
                "SELECT block_number, parent_hash FROM chain_evidence_blocks \
                 WHERE chain_id = $1 AND block_hash = $2",
                &[&chain_id_db, &block.hash],
            )
            .map_err(|error| db_error("read persisted block identity", error))?;
        let persisted_number: i64 = identity.get(0);
        let persisted_parent: String = identity.get(1);
        if persisted_number != block_number || persisted_parent != block.parent_hash {
            return Err(PersistenceError::new(
                "BLOCK_IDENTITY_CONFLICT",
                format!("persisted identity conflicts for block {}", block.hash),
            ));
        }

        for event in &block.events {
            let log_index = i32_value(event.log_index, "log_index")?;
            tx.execute(
                "INSERT INTO chain_evidence_events \
                 (chain_id, block_hash, log_index, tx_hash, contract_address, decoder_id, \
                  state_key, state_value, canonical) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE) \
                 ON CONFLICT (chain_id, block_hash, log_index) DO NOTHING",
                &[
                    &chain_id_db,
                    &block.hash,
                    &log_index,
                    &event.tx_hash,
                    &event.contract_address,
                    &event.decoder_id,
                    &event.key,
                    &event.value,
                ],
            )
            .map_err(|error| db_error("insert event", error))?;

            let identity = tx
                .query_one(
                    "SELECT tx_hash, contract_address, decoder_id, state_key, state_value \
                     FROM chain_evidence_events \
                     WHERE chain_id = $1 AND block_hash = $2 AND log_index = $3",
                    &[&chain_id_db, &block.hash, &log_index],
                )
                .map_err(|error| db_error("read persisted event identity", error))?;
            let observed: (String, String, String, String, String) = (
                identity.get(0),
                identity.get(1),
                identity.get(2),
                identity.get(3),
                identity.get(4),
            );
            let expected = (
                event.tx_hash.clone(),
                event.contract_address.clone(),
                event.decoder_id.clone(),
                event.key.clone(),
                event.value.clone(),
            );
            if observed != expected {
                return Err(PersistenceError::new(
                    "EVENT_IDENTITY_CONFLICT",
                    format!(
                        "persisted event identity conflicts for block {} log {}",
                        block.hash, event.log_index
                    ),
                ));
            }
        }
    }

    tx.execute(
        "UPDATE chain_evidence_events SET canonical = FALSE WHERE chain_id = $1",
        &[&chain_id_db],
    )
    .map_err(|error| db_error("reset event canonical flags", error))?;
    tx.execute(
        "UPDATE chain_evidence_blocks SET canonical = FALSE WHERE chain_id = $1",
        &[&chain_id_db],
    )
    .map_err(|error| db_error("reset block canonical flags", error))?;

    for block_hash in &source.canonical_block_hashes {
        let updated = tx
            .execute(
                "UPDATE chain_evidence_blocks SET canonical = TRUE \
                 WHERE chain_id = $1 AND block_hash = $2",
                &[&chain_id_db, block_hash],
            )
            .map_err(|error| db_error("promote canonical block", error))?;
        if updated != 1 {
            return Err(PersistenceError::new(
                "CANONICAL_BLOCK_MISSING",
                format!("canonical block {block_hash} was not persisted exactly once"),
            ));
        }
        tx.execute(
            "UPDATE chain_evidence_events SET canonical = TRUE \
             WHERE chain_id = $1 AND block_hash = $2",
            &[&chain_id_db, block_hash],
        )
        .map_err(|error| db_error("promote canonical events", error))?;
    }

    tx.execute(
        "INSERT INTO chain_evidence_checkpoints \
         (chain_id, schema_version, canonical_tip, canonical_height, state_sha256, report_sha256) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (chain_id) DO UPDATE SET \
             schema_version = EXCLUDED.schema_version, \
             canonical_tip = EXCLUDED.canonical_tip, \
             canonical_height = EXCLUDED.canonical_height, \
             state_sha256 = EXCLUDED.state_sha256, \
             report_sha256 = EXCLUDED.report_sha256, \
             updated_at = now()",
        &[
            &chain_id_db,
            &SCHEMA_VERSION,
            &canonical_tip,
            &height_db,
            &source.state_sha256,
            &source.report_sha256,
        ],
    )
    .map_err(|error| db_error("write checkpoint", error))?;

    let outcome = match mode {
        PersistMode::Commit => {
            tx.commit()
                .map_err(|error| db_error("commit persistence transaction", error))?;
            "COMMITTED"
        }
        PersistMode::AbortBeforeCommit => {
            tx.rollback()
                .map_err(|error| db_error("rollback injected pre-commit crash", error))?;
            "ABORTED_BEFORE_COMMIT"
        }
    };

    let unsigned = (
        "chain-evidence.persistence.v0.2",
        outcome,
        SCHEMA_VERSION,
        fixture.chain_id,
        &canonical_tip,
        canonical_height,
        source.canonical_block_hashes.len(),
        source.canonical_events.len(),
        &source.state_sha256,
        &source.report_sha256,
        PERSISTENCE_CLAIM_BOUNDARY,
    );
    let evidence_sha256 = hash_json(&unsigned);
    Ok(PersistenceEvidence {
        schema: "chain-evidence.persistence.v0.2".to_owned(),
        outcome: outcome.to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        chain_id: fixture.chain_id,
        canonical_tip,
        canonical_height,
        canonical_block_count: source.canonical_block_hashes.len(),
        canonical_event_count: source.canonical_events.len(),
        state_sha256: source.state_sha256,
        source_report_sha256: source.report_sha256,
        evidence_sha256,
        claim_boundary: PERSISTENCE_CLAIM_BOUNDARY.to_owned(),
    })
}

pub fn recover(client: &mut Client, chain_id: u64) -> Result<RecoveryEvidence, PersistenceError> {
    migrate(client)?;
    let chain_id_db = i64_value(chain_id, "chain_id")?;
    let checkpoint = client
        .query_opt(
            "SELECT schema_version, canonical_tip, canonical_height, state_sha256, report_sha256 \
             FROM chain_evidence_checkpoints WHERE chain_id = $1",
            &[&chain_id_db],
        )
        .map_err(|error| db_error("read checkpoint", error))?
        .ok_or_else(|| PersistenceError::new("CHECKPOINT_MISSING", "no checkpoint exists"))?;

    let schema_version: String = checkpoint.get(0);
    let checkpoint_tip: String = checkpoint.get(1);
    let checkpoint_height_db: i64 = checkpoint.get(2);
    let expected_state_sha256: String = checkpoint.get(3);
    let source_report_sha256: String = checkpoint.get(4);
    if schema_version != SCHEMA_VERSION {
        return Err(PersistenceError::new(
            "SCHEMA_VERSION_MISMATCH",
            format!("checkpoint schema {schema_version} != {SCHEMA_VERSION}"),
        ));
    }
    let checkpoint_height = u64_value(checkpoint_height_db, "canonical_height")?;

    let rows = client
        .query(
            "SELECT block_hash, block_number, parent_hash \
             FROM chain_evidence_blocks \
             WHERE chain_id = $1 AND canonical = TRUE \
             ORDER BY block_number ASC, block_hash ASC",
            &[&chain_id_db],
        )
        .map_err(|error| db_error("read canonical blocks", error))?;
    if rows.is_empty() {
        return Err(PersistenceError::new(
            "CORRUPT_CHECKPOINT",
            "checkpoint exists without canonical blocks",
        ));
    }

    let mut canonical_block_hashes = Vec::with_capacity(rows.len());
    let mut previous: Option<(u64, String)> = None;
    for row in rows {
        let hash: String = row.get(0);
        let number = u64_value(row.get::<_, i64>(1), "block_number")?;
        let parent_hash: String = row.get(2);
        match &previous {
            None if number != 0 => {
                return Err(PersistenceError::new(
                    "CORRUPT_CANONICAL_CHAIN",
                    format!("first canonical block is {number}, expected 0"),
                ));
            }
            Some((previous_number, previous_hash)) => {
                if number != previous_number + 1 || parent_hash != *previous_hash {
                    return Err(PersistenceError::new(
                        "CORRUPT_CANONICAL_CHAIN",
                        format!("canonical continuity breaks at block {hash}"),
                    ));
                }
            }
            None => {}
        }
        previous = Some((number, hash.clone()));
        canonical_block_hashes.push(hash);
    }

    let (observed_height, observed_tip) = previous
        .ok_or_else(|| PersistenceError::new("CORRUPT_CHECKPOINT", "canonical chain empty"))?;
    if observed_height != checkpoint_height || observed_tip != checkpoint_tip {
        return Err(PersistenceError::new(
            "STALE_OR_CORRUPT_CHECKPOINT",
            format!(
                "checkpoint tip/height {checkpoint_tip}@{checkpoint_height} != persisted canonical {observed_tip}@{observed_height}"
            ),
        ));
    }

    let event_rows = client
        .query(
            "SELECT b.block_number, b.block_hash, e.tx_hash, e.log_index, \
                    e.contract_address, e.decoder_id, e.state_key, e.state_value, e.canonical \
             FROM chain_evidence_blocks b \
             JOIN chain_evidence_events e \
               ON e.chain_id = b.chain_id AND e.block_hash = b.block_hash \
             WHERE b.chain_id = $1 AND b.canonical = TRUE \
             ORDER BY b.block_number ASC, e.log_index ASC",
            &[&chain_id_db],
        )
        .map_err(|error| db_error("read canonical events", error))?;

    let mut canonical_events = Vec::with_capacity(event_rows.len());
    let mut material_state = BTreeMap::new();
    for row in event_rows {
        let event_canonical: bool = row.get(8);
        if !event_canonical {
            return Err(PersistenceError::new(
                "CANONICAL_FLAG_MISMATCH",
                "canonical block contains event not marked canonical",
            ));
        }
        let block_number = u64_value(row.get::<_, i64>(0), "block_number")?;
        let log_index_db: i32 = row.get(3);
        let log_index = u32::try_from(log_index_db).map_err(|_| {
            PersistenceError::new(
                "INTEGER_RANGE",
                format!("database log_index {log_index_db} is negative/out of range"),
            )
        })?;
        let key: String = row.get(6);
        let value: String = row.get(7);
        material_state.insert(key.clone(), value.clone());
        canonical_events.push(CanonicalEventEvidence {
            chain_id,
            block_number,
            block_hash: row.get(1),
            tx_hash: row.get(2),
            log_index,
            contract_address: row.get(4),
            decoder_id: row.get(5),
            key,
            value,
        });
    }

    let observed_state_sha256 =
        state_sha256(&canonical_block_hashes, &canonical_events, &material_state);
    if observed_state_sha256 != expected_state_sha256 {
        return Err(PersistenceError::new(
            "STATE_DIGEST_MISMATCH",
            format!(
                "checkpoint state digest {expected_state_sha256} != recovered {observed_state_sha256}"
            ),
        ));
    }

    let unsigned = (
        "chain-evidence.recovery.v0.2",
        "RECOVERED",
        SCHEMA_VERSION,
        chain_id,
        &checkpoint_tip,
        checkpoint_height,
        &canonical_block_hashes,
        canonical_events.len(),
        &material_state,
        &observed_state_sha256,
        &source_report_sha256,
        PERSISTENCE_CLAIM_BOUNDARY,
    );
    let recovery_sha256 = hash_json(&unsigned);
    Ok(RecoveryEvidence {
        schema: "chain-evidence.recovery.v0.2".to_owned(),
        outcome: "RECOVERED".to_owned(),
        schema_version,
        chain_id,
        checkpoint_tip,
        checkpoint_height,
        canonical_block_hashes,
        canonical_event_count: canonical_events.len(),
        material_state,
        state_sha256: observed_state_sha256,
        source_report_sha256,
        recovery_sha256,
        claim_boundary: PERSISTENCE_CLAIM_BOUNDARY.to_owned(),
    })
}
