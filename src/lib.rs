pub mod engine;
pub mod evm_adapter;
pub mod models;
pub mod naive;
pub mod persistence;

pub use engine::{report_fixture, CanonicalEngine, EngineError, CLAIM_BOUNDARY};
pub use evm_adapter::{
    collect_evm_evidence, reference_event_topic0, write_evm_report, EvmAdapterConfig,
    EvmAdapterReport, EVM_ADAPTER_CLAIM_BOUNDARY, EVENT_SIGNATURE, REFERENCE_DECODER_ID,
};
pub use models::{ApplyKind, Block, Event, EvidenceReport, Fixture};
pub use persistence::{
    clear_chain, connect, migrate, persist_fixture, recover, PersistMode, PersistenceError,
    PersistenceEvidence, RecoveryEvidence, PERSISTENCE_CLAIM_BOUNDARY, SCHEMA_VERSION,
};
