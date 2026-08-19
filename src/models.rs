use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Event {
    pub tx_hash: String,
    pub log_index: u32,
    pub contract_address: String,
    pub decoder_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Block {
    pub chain_id: u64,
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Fixture {
    pub chain_id: u64,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplyKind {
    Appended,
    Reorg,
    Idempotent,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ApplicationEvidence {
    pub kind: ApplyKind,
    pub block_hash: String,
    pub common_ancestor: Option<String>,
    pub orphaned_block_hashes: Vec<String>,
    pub canonical_tip: String,
    pub canonical_height: u64,
    pub state_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CanonicalEventEvidence {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub log_index: u32,
    pub contract_address: String,
    pub decoder_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceReport {
    pub schema: String,
    pub outcome: String,
    pub chain_id: u64,
    pub applications: Vec<ApplicationEvidence>,
    pub canonical_block_hashes: Vec<String>,
    pub canonical_events: Vec<CanonicalEventEvidence>,
    pub material_state: BTreeMap<String, String>,
    pub state_sha256: String,
    pub error: Option<String>,
    pub claim_boundary: String,
    pub report_sha256: String,
}
