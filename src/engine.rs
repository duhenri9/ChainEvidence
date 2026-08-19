use crate::models::{
    ApplicationEvidence, ApplyKind, Block, CanonicalEventEvidence, EvidenceReport, Fixture,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

pub const CLAIM_BOUNDARY: &str = "V0 proves only deterministic canonical-chain reconstruction over the supplied synthetic fixture sequence and the implemented observation policy. It does not establish network consensus, universal finality, production safety, throughput, smart-contract correctness or multi-chain compatibility.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    ChainIdMismatch { expected: u64, observed: u64 },
    MissingGenesis { observed_number: u64 },
    MissingAncestor { hash: String },
    InvalidBlockNumber {
        parent_number: u64,
        block_number: u64,
    },
    DuplicateLogIndex { block_hash: String, log_index: u32 },
    ConflictingBlockIdentity { hash: String },
    AncestryCycle { hash: String },
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ChainIdMismatch { .. } => "CHAIN_ID_MISMATCH",
            Self::MissingGenesis { .. } => "MISSING_GENESIS",
            Self::MissingAncestor { .. } => "MISSING_ANCESTOR",
            Self::InvalidBlockNumber { .. } => "INVALID_BLOCK_NUMBER",
            Self::DuplicateLogIndex { .. } => "DUPLICATE_LOG_INDEX",
            Self::ConflictingBlockIdentity { .. } => "CONFLICTING_BLOCK_IDENTITY",
            Self::AncestryCycle { .. } => "ANCESTRY_CYCLE",
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChainIdMismatch { expected, observed } => {
                write!(formatter, "expected chain id {expected}, observed {observed}")
            }
            Self::MissingGenesis { observed_number } => {
                write!(formatter, "first observed block must be number 0, got {observed_number}")
            }
            Self::MissingAncestor { hash } => {
                write!(formatter, "required ancestor is unavailable: {hash}")
            }
            Self::InvalidBlockNumber {
                parent_number,
                block_number,
            } => write!(
                formatter,
                "block number {block_number} does not follow parent number {parent_number}"
            ),
            Self::DuplicateLogIndex {
                block_hash,
                log_index,
            } => write!(
                formatter,
                "block {block_hash} contains duplicate log index {log_index}"
            ),
            Self::ConflictingBlockIdentity { hash } => {
                write!(formatter, "block hash {hash} was observed with conflicting content")
            }
            Self::AncestryCycle { hash } => {
                write!(formatter, "ancestry cycle detected while resolving {hash}")
            }
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Serialize)]
struct MaterialSnapshot<'a> {
    canonical_block_hashes: &'a [String],
    canonical_events: &'a [CanonicalEventEvidence],
    material_state: &'a BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct CanonicalEngine {
    chain_id: u64,
    blocks: HashMap<String, Block>,
    canonical: Vec<String>,
    canonical_events: Vec<CanonicalEventEvidence>,
    material_state: BTreeMap<String, String>,
}

impl CanonicalEngine {
    #[must_use]
    pub fn new(chain_id: u64) -> Self {
        Self {
            chain_id,
            blocks: HashMap::new(),
            canonical: Vec::new(),
            canonical_events: Vec::new(),
            material_state: BTreeMap::new(),
        }
    }

    pub fn apply(&mut self, block: Block) -> Result<ApplicationEvidence, EngineError> {
        self.validate_block(&block)?;

        if let Some(existing) = self.blocks.get(&block.hash) {
            if existing != &block {
                return Err(EngineError::ConflictingBlockIdentity {
                    hash: block.hash.clone(),
                });
            }
            return Ok(self.application_evidence(
                ApplyKind::Idempotent,
                block.hash,
                None,
                Vec::new(),
            ));
        }

        if self.canonical.is_empty() {
            if block.number != 0 {
                return Err(EngineError::MissingGenesis {
                    observed_number: block.number,
                });
            }
            let hash = block.hash.clone();
            self.blocks.insert(hash.clone(), block);
            self.canonical.push(hash.clone());
            self.rebuild_material_state();
            return Ok(self.application_evidence(
                ApplyKind::Appended,
                hash,
                None,
                Vec::new(),
            ));
        }

        let parent = self
            .blocks
            .get(&block.parent_hash)
            .ok_or_else(|| EngineError::MissingAncestor {
                hash: block.parent_hash.clone(),
            })?;
        if parent.number + 1 != block.number {
            return Err(EngineError::InvalidBlockNumber {
                parent_number: parent.number,
                block_number: block.number,
            });
        }

        let hash = block.hash.clone();
        let parent_hash = block.parent_hash.clone();
        let block_number = block.number;
        self.blocks.insert(hash.clone(), block);

        let tip_hash = self
            .canonical
            .last()
            .expect("canonical chain is non-empty after genesis");
        if &parent_hash == tip_hash {
            self.canonical.push(hash.clone());
            self.rebuild_material_state();
            return Ok(self.application_evidence(
                ApplyKind::Appended,
                hash,
                None,
                Vec::new(),
            ));
        }

        let (common_index, common_hash, mut branch) = self.resolve_branch(&hash)?;
        let canonical_height = self.canonical_height();
        if block_number < canonical_height {
            return Ok(self.application_evidence(
                ApplyKind::StoredFork,
                hash,
                Some(common_hash),
                Vec::new(),
            ));
        }

        let orphaned = self.canonical.split_off(common_index + 1);
        self.canonical.append(&mut branch);
        self.rebuild_material_state();
        Ok(self.application_evidence(
            ApplyKind::Reorg,
            hash,
            Some(common_hash),
            orphaned,
        ))
    }

    fn validate_block(&self, block: &Block) -> Result<(), EngineError> {
        if block.chain_id != self.chain_id {
            return Err(EngineError::ChainIdMismatch {
                expected: self.chain_id,
                observed: block.chain_id,
            });
        }

        let mut log_indexes = HashSet::new();
        for event in &block.events {
            if !log_indexes.insert(event.log_index) {
                return Err(EngineError::DuplicateLogIndex {
                    block_hash: block.hash.clone(),
                    log_index: event.log_index,
                });
            }
        }
        Ok(())
    }

    fn resolve_branch(&self, tip_hash: &str) -> Result<(usize, String, Vec<String>), EngineError> {
        let tip = self
            .blocks
            .get(tip_hash)
            .expect("new block is stored before branch resolution");
        let mut cursor = tip.parent_hash.clone();
        let mut branch_reversed = vec![tip_hash.to_owned()];
        let mut visited = HashSet::new();

        loop {
            if let Some(index) = self.canonical.iter().position(|hash| hash == &cursor) {
                branch_reversed.reverse();
                return Ok((index, cursor, branch_reversed));
            }

            if !visited.insert(cursor.clone()) {
                return Err(EngineError::AncestryCycle { hash: cursor });
            }

            let ancestor = self
                .blocks
                .get(&cursor)
                .ok_or_else(|| EngineError::MissingAncestor {
                    hash: cursor.clone(),
                })?;
            branch_reversed.push(cursor.clone());
            cursor = ancestor.parent_hash.clone();
        }
    }

    fn rebuild_material_state(&mut self) {
        self.material_state.clear();
        self.canonical_events.clear();

        for block_hash in &self.canonical {
            let block = self
                .blocks
                .get(block_hash)
                .expect("canonical block must exist in the block store");
            let mut events = block.events.clone();
            events.sort_by_key(|event| event.log_index);

            for event in events {
                self.material_state
                    .insert(event.key.clone(), event.value.clone());
                self.canonical_events.push(CanonicalEventEvidence {
                    chain_id: block.chain_id,
                    block_number: block.number,
                    block_hash: block.hash.clone(),
                    tx_hash: event.tx_hash,
                    log_index: event.log_index,
                    contract_address: event.contract_address,
                    decoder_id: event.decoder_id,
                    key: event.key,
                    value: event.value,
                });
            }
        }
    }

    fn application_evidence(
        &self,
        kind: ApplyKind,
        block_hash: String,
        common_ancestor: Option<String>,
        orphaned_block_hashes: Vec<String>,
    ) -> ApplicationEvidence {
        ApplicationEvidence {
            kind,
            block_hash,
            common_ancestor,
            orphaned_block_hashes,
            canonical_tip: self.canonical.last().cloned().unwrap_or_default(),
            canonical_height: self.canonical_height(),
            state_sha256: self.state_sha256(),
        }
    }

    #[must_use]
    pub fn canonical_height(&self) -> u64 {
        self.canonical
            .last()
            .and_then(|hash| self.blocks.get(hash))
            .map_or(0, |block| block.number)
    }

    #[must_use]
    pub fn canonical_block_hashes(&self) -> Vec<String> {
        self.canonical.clone()
    }

    #[must_use]
    pub fn canonical_events(&self) -> Vec<CanonicalEventEvidence> {
        self.canonical_events.clone()
    }

    #[must_use]
    pub fn material_state(&self) -> BTreeMap<String, String> {
        self.material_state.clone()
    }

    #[must_use]
    pub fn state_sha256(&self) -> String {
        hash_json(&MaterialSnapshot {
            canonical_block_hashes: &self.canonical,
            canonical_events: &self.canonical_events,
            material_state: &self.material_state,
        })
    }
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable evidence payload");
    format!("{:x}", Sha256::digest(bytes))
}

#[must_use]
pub fn report_fixture(fixture: &Fixture) -> EvidenceReport {
    let mut engine = CanonicalEngine::new(fixture.chain_id);
    let mut applications = Vec::new();
    let mut error_code = None;
    let mut error = None;

    for block in fixture.blocks.clone() {
        match engine.apply(block) {
            Ok(application) => applications.push(application),
            Err(engine_error) => {
                error_code = Some(engine_error.code().to_owned());
                error = Some(engine_error.to_string());
                break;
            }
        }
    }

    let outcome = if error.is_some() { "INDETERMINATE" } else { "PASS" };
    let mut report = EvidenceReport {
        schema: "chain-evidence.report.v0".to_owned(),
        outcome: outcome.to_owned(),
        chain_id: fixture.chain_id,
        applications,
        canonical_block_hashes: engine.canonical_block_hashes(),
        canonical_events: engine.canonical_events(),
        material_state: engine.material_state(),
        state_sha256: engine.state_sha256(),
        error_code,
        error,
        claim_boundary: CLAIM_BOUNDARY.to_owned(),
        report_sha256: String::new(),
    };
    report.report_sha256 = hash_json(&report);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Event;
    use crate::naive::NaiveIndexer;

    fn event(log_index: u32, key: &str, value: &str) -> Event {
        Event {
            tx_hash: format!("0xtx{log_index}{key}"),
            log_index,
            contract_address: "0xasset".to_owned(),
            decoder_id: "asset-v1".to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    fn block(number: u64, hash: &str, parent_hash: &str, events: Vec<Event>) -> Block {
        Block {
            chain_id: 31_337,
            number,
            hash: hash.to_owned(),
            parent_hash: parent_hash.to_owned(),
            events,
        }
    }

    fn reorg_fixture() -> Fixture {
        Fixture {
            chain_id: 31_337,
            blocks: vec![
                block(0, "0xg", "0xroot", vec![event(0, "owner", "alice")]),
                block(1, "0xa1", "0xg", vec![event(0, "route", "legacy")]),
                block(2, "0xa2", "0xa1", vec![event(0, "orphan-only", "present")]),
                block(1, "0xb1", "0xg", vec![event(0, "route", "replacement")]),
                block(2, "0xb2", "0xb1", vec![event(0, "settled", "yes")]),
            ],
        }
    }

    #[test]
    fn appends_straight_chain() {
        let fixture = Fixture {
            chain_id: 31_337,
            blocks: vec![
                block(0, "0xg", "0xroot", vec![]),
                block(1, "0xa1", "0xg", vec![event(0, "status", "issued")]),
            ],
        };
        let report = report_fixture(&fixture);
        assert_eq!(report.outcome, "PASS");
        assert_eq!(report.canonical_block_hashes, vec!["0xg", "0xa1"]);
        assert_eq!(report.material_state.get("status"), Some(&"issued".to_owned()));
    }

    #[test]
    fn stores_shorter_fork_without_replacing_canonical_tip() {
        let mut engine = CanonicalEngine::new(31_337);
        engine.apply(block(0, "0xg", "0xroot", vec![])).unwrap();
        engine.apply(block(1, "0xa1", "0xg", vec![])).unwrap();
        engine.apply(block(2, "0xa2", "0xa1", vec![])).unwrap();
        let evidence = engine.apply(block(1, "0xb1", "0xg", vec![])).unwrap();
        assert_eq!(evidence.kind, ApplyKind::StoredFork);
        assert_eq!(engine.canonical_block_hashes(), vec!["0xg", "0xa1", "0xa2"]);
    }

    #[test]
    fn performs_multi_block_reorg_and_removes_orphaned_state() {
        let report = report_fixture(&reorg_fixture());
        assert_eq!(report.outcome, "PASS");
        assert_eq!(report.canonical_block_hashes, vec!["0xg", "0xb1", "0xb2"]);
        assert_eq!(report.material_state.get("route"), Some(&"replacement".to_owned()));
        assert_eq!(report.material_state.get("settled"), Some(&"yes".to_owned()));
        assert!(!report.material_state.contains_key("orphan-only"));
        let reorg = report
            .applications
            .iter()
            .find(|item| item.kind == ApplyKind::Reorg)
            .expect("reorg evidence must exist");
        assert_eq!(reorg.common_ancestor.as_deref(), Some("0xg"));
        assert_eq!(reorg.orphaned_block_hashes, vec!["0xa1", "0xa2"]);
    }

    #[test]
    fn one_block_reorg_is_supported() {
        let fixture = Fixture {
            chain_id: 31_337,
            blocks: vec![
                block(0, "0xg", "0xroot", vec![]),
                block(1, "0xa1", "0xg", vec![event(0, "status", "old")]),
                block(1, "0xb1", "0xg", vec![event(0, "status", "new")]),
            ],
        };
        let report = report_fixture(&fixture);
        assert_eq!(report.canonical_block_hashes, vec!["0xg", "0xb1"]);
        assert_eq!(report.material_state.get("status"), Some(&"new".to_owned()));
    }

    #[test]
    fn missing_ancestor_fails_closed() {
        let fixture = Fixture {
            chain_id: 31_337,
            blocks: vec![
                block(0, "0xg", "0xroot", vec![]),
                block(2, "0xunknown-child", "0xmissing", vec![]),
            ],
        };
        let report = report_fixture(&fixture);
        assert_eq!(report.outcome, "INDETERMINATE");
        assert_eq!(report.error_code.as_deref(), Some("MISSING_ANCESTOR"));
        assert_eq!(report.canonical_block_hashes, vec!["0xg"]);
    }

    #[test]
    fn replay_from_empty_state_is_deterministic() {
        let fixture = reorg_fixture();
        let first = report_fixture(&fixture);
        let second = report_fixture(&fixture);
        assert_eq!(first.state_sha256, second.state_sha256);
        assert_eq!(first.report_sha256, second.report_sha256);
    }

    #[test]
    fn duplicate_block_is_idempotent() {
        let genesis = block(0, "0xg", "0xroot", vec![]);
        let mut engine = CanonicalEngine::new(31_337);
        engine.apply(genesis.clone()).unwrap();
        let evidence = engine.apply(genesis).unwrap();
        assert_eq!(evidence.kind, ApplyKind::Idempotent);
        assert_eq!(engine.canonical_block_hashes(), vec!["0xg"]);
    }

    #[test]
    fn known_bad_naive_indexer_diverges_after_reorg() {
        let fixture = reorg_fixture();
        let canonical = report_fixture(&fixture);
        let mut naive = NaiveIndexer::default();
        for block in fixture.blocks {
            naive.apply(&block);
        }
        assert!(!canonical.material_state.contains_key("orphan-only"));
        assert_eq!(naive.material_state().get("orphan-only"), Some(&"present".to_owned()));
        assert_ne!(canonical.material_state, naive.material_state());
    }
}
