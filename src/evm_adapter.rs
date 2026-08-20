use crate::{report_fixture, Block, Event, EvidenceReport, Fixture};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::{Digest as Sha3Digest, Keccak256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const REFERENCE_DECODER_ID: &str = "reference-lifecycle-v1";
pub const EVENT_SIGNATURE: &str = "LifecycleSet(bytes32,bytes32)";
pub const EVM_ADAPTER_CLAIM_BOUNDARY: &str = "V0.3 proves only bounded adaptation of one versioned reference event from a declared local EVM JSON-RPC snapshot into the existing ChainEvidence core model. It does not establish RPC-provider correctness, network consensus, universal finality, smart-contract correctness, production safety, live-tail reorg handling or multi-chain compatibility.";

#[derive(Clone, Debug)]
pub struct EvmAdapterConfig {
    pub rpc_url: String,
    pub expected_chain_id: u64,
    pub contract_address: String,
    pub deployment_tx_hash: String,
    pub decoder_id: String,
    pub abi_path: PathBuf,
    pub source_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DeploymentEvidence {
    pub transaction_hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub contract_address: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LogSourceEvidence {
    pub transaction_hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub log_index: u32,
    pub contract_address: String,
    pub topic0: String,
    pub data_sha256: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ReceiptSourceEvidence {
    pub transaction_hash: String,
    pub block_number: u64,
    pub block_hash: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvmAdapterReport {
    pub schema: String,
    pub outcome: String,
    pub node_client: Option<String>,
    pub expected_chain_id: u64,
    pub observed_chain_id: Option<u64>,
    pub contract_address: String,
    pub deployment: Option<DeploymentEvidence>,
    pub decoder_id: String,
    pub abi_sha256: Option<String>,
    pub source_sha256: Option<String>,
    pub event_signature: String,
    pub event_topic0: String,
    pub log_sources: Vec<LogSourceEvidence>,
    pub receipt_sources: Vec<ReceiptSourceEvidence>,
    pub adapted_event_count: usize,
    pub observed_block_count: usize,
    pub core_report: Option<EvidenceReport>,
    pub error_code: Option<String>,
    pub error: Option<String>,
    pub limitations: Vec<String>,
    pub claim_boundary: String,
    pub report_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterError {
    code: &'static str,
    message: String,
}

impl AdapterError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RpcFailure {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcFailure>,
}

struct RpcClient {
    url: String,
    client: Client,
}

impl RpcClient {
    fn new(url: &str) -> Result<Self, AdapterError> {
        if !url.starts_with("http://127.0.0.1:") && !url.starts_with("http://localhost:") {
            return Err(AdapterError::new(
                "NON_LOCAL_RPC",
                "V0.3 accepts only loopback HTTP RPC endpoints",
            ));
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|error| AdapterError::new("RPC_CLIENT_ERROR", error.to_string()))?;
        Ok(Self {
            url: url.to_owned(),
            client,
        })
    }

    fn call(&self, method: &str, params: Value) -> Result<Value, AdapterError> {
        let response = self
            .client
            .post(&self.url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }))
            .send()
            .map_err(|error| AdapterError::new("RPC_TRANSPORT_ERROR", error.to_string()))?;
        if !response.status().is_success() {
            return Err(AdapterError::new(
                "RPC_HTTP_ERROR",
                format!("{method} returned HTTP {}", response.status()),
            ));
        }
        let envelope: RpcEnvelope = response
            .json()
            .map_err(|error| AdapterError::new("RPC_DECODE_ERROR", error.to_string()))?;
        if let Some(error) = envelope.error {
            return Err(AdapterError::new(
                "RPC_METHOD_ERROR",
                format!("{method}: {} ({})", error.message, error.code),
            ));
        }
        envelope.result.ok_or_else(|| {
            AdapterError::new(
                "RPC_MISSING_RESULT",
                format!("{method} did not return a result"),
            )
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DecodedLog {
    event: Event,
    block_number: u64,
    block_hash: String,
    data_sha256: String,
    topic0: String,
}

#[derive(Default)]
struct PartialEvidence {
    node_client: Option<String>,
    observed_chain_id: Option<u64>,
    deployment: Option<DeploymentEvidence>,
    abi_sha256: Option<String>,
    source_sha256: Option<String>,
    log_sources: Vec<LogSourceEvidence>,
    receipt_sources: Vec<ReceiptSourceEvidence>,
    adapted_event_count: usize,
    observed_block_count: usize,
    core_report: Option<EvidenceReport>,
}

fn normalize_hex(value: &str, field: &str) -> Result<String, AdapterError> {
    let trimmed = value.trim();
    if !trimmed.starts_with("0x") || trimmed.len() <= 2 {
        return Err(AdapterError::new(
            "MALFORMED_RPC_FIELD",
            format!("{field} is not a non-empty 0x-prefixed hex value"),
        ));
    }
    if !trimmed[2..].bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AdapterError::new(
            "MALFORMED_RPC_FIELD",
            format!("{field} contains non-hex characters"),
        ));
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn required_string<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str, AdapterError> {
    object.get(field).and_then(Value::as_str).ok_or_else(|| {
        AdapterError::new(
            "MALFORMED_RPC_FIELD",
            format!("required field {field} is missing or not a string"),
        )
    })
}

fn as_object(value: &Value, context: &str) -> Result<&Map<String, Value>, AdapterError> {
    value.as_object().ok_or_else(|| {
        AdapterError::new(
            "MALFORMED_RPC_OBJECT",
            format!("{context} is not a JSON object"),
        )
    })
}

fn parse_hex_u64(value: &str, field: &str) -> Result<u64, AdapterError> {
    let normalized = normalize_hex(value, field)?;
    u64::from_str_radix(&normalized[2..], 16).map_err(|error| {
        AdapterError::new(
            "MALFORMED_RPC_FIELD",
            format!("{field} is outside the supported u64 range: {error}"),
        )
    })
}

fn parse_hex_u32(value: &str, field: &str) -> Result<u32, AdapterError> {
    let parsed = parse_hex_u64(value, field)?;
    u32::try_from(parsed).map_err(|_| {
        AdapterError::new(
            "MALFORMED_RPC_FIELD",
            format!("{field} is outside the supported u32 range"),
        )
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_json_sha256(value: &Value) -> Result<String, AdapterError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| AdapterError::new("EVIDENCE_ENCODE_ERROR", error.to_string()))?;
    Ok(sha256_bytes(&encoded))
}

pub fn reference_event_topic0() -> String {
    format!("0x{:x}", Keccak256::digest(EVENT_SIGNATURE.as_bytes()))
}

fn bytes32_text(hex_value: &str, field: &str) -> Result<String, AdapterError> {
    if hex_value.len() != 64 || !hex_value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AdapterError::new(
            "MALFORMED_EVENT_DATA",
            format!("{field} must contain exactly 32 bytes"),
        ));
    }
    let mut bytes = Vec::with_capacity(32);
    for index in (0..64).step_by(2) {
        let byte = u8::from_str_radix(&hex_value[index..index + 2], 16).map_err(|error| {
            AdapterError::new("MALFORMED_EVENT_DATA", format!("{field}: {error}"))
        })?;
        bytes.push(byte);
    }
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    if bytes.is_empty() {
        return Err(AdapterError::new(
            "MALFORMED_EVENT_DATA",
            format!("{field} cannot decode to an empty value"),
        ));
    }
    String::from_utf8(bytes).map_err(|error| {
        AdapterError::new(
            "MALFORMED_EVENT_DATA",
            format!("{field} is not valid UTF-8: {error}"),
        )
    })
}

fn decode_reference_log(
    raw: &Value,
    contract_address: &str,
    decoder_id: &str,
    expected_topic0: &str,
) -> Result<DecodedLog, AdapterError> {
    let object = as_object(raw, "log")?;
    if object.get("removed").and_then(Value::as_bool) == Some(true) {
        return Err(AdapterError::new(
            "REMOVED_LOG",
            "removed log cannot be trusted as current canonical evidence",
        ));
    }

    let address = normalize_hex(required_string(object, "address")?, "log.address")?;
    if address != contract_address {
        return Err(AdapterError::new(
            "CONTRACT_IDENTITY_MISMATCH",
            format!("expected {contract_address}, observed {address}"),
        ));
    }
    let transaction_hash = normalize_hex(
        required_string(object, "transactionHash")?,
        "log.transactionHash",
    )?;
    let block_hash = normalize_hex(required_string(object, "blockHash")?, "log.blockHash")?;
    let block_number = parse_hex_u64(
        required_string(object, "blockNumber")?,
        "log.blockNumber",
    )?;
    let log_index = parse_hex_u32(required_string(object, "logIndex")?, "log.logIndex")?;

    let topics = object
        .get("topics")
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::new("MALFORMED_RPC_FIELD", "log.topics is missing"))?;
    if topics.len() != 1 {
        return Err(AdapterError::new(
            "UNKNOWN_EVENT_SHAPE",
            format!("reference event requires exactly one topic, observed {}", topics.len()),
        ));
    }
    let topic0 = normalize_hex(
        topics[0]
            .as_str()
            .ok_or_else(|| AdapterError::new("MALFORMED_RPC_FIELD", "log topic0 is not a string"))?,
        "log.topic0",
    )?;
    if topic0 != expected_topic0 {
        return Err(AdapterError::new(
            "UNKNOWN_EVENT_SIGNATURE",
            format!("expected topic0 {expected_topic0}, observed {topic0}"),
        ));
    }

    let data = normalize_hex(required_string(object, "data")?, "log.data")?;
    let payload = &data[2..];
    if payload.len() != 128 {
        return Err(AdapterError::new(
            "MALFORMED_EVENT_DATA",
            format!("reference event requires 64 data bytes, observed {}", payload.len() / 2),
        ));
    }
    let key = bytes32_text(&payload[..64], "event.key")?;
    let value = bytes32_text(&payload[64..], "event.value")?;

    Ok(DecodedLog {
        event: Event {
            tx_hash: transaction_hash,
            log_index,
            contract_address: address,
            decoder_id: decoder_id.to_owned(),
            key,
            value,
        },
        block_number,
        block_hash,
        data_sha256: sha256_bytes(data.as_bytes()),
        topic0,
    })
}

fn receipt_evidence(
    rpc: &RpcClient,
    tx_hash: &str,
    expected_contract: Option<&str>,
) -> Result<ReceiptSourceEvidence, AdapterError> {
    let value = rpc.call("eth_getTransactionReceipt", json!([tx_hash]))?;
    if value.is_null() {
        return Err(AdapterError::new(
            "MISSING_RECEIPT",
            format!("receipt is unavailable for {tx_hash}"),
        ));
    }
    let object = as_object(&value, "transaction receipt")?;
    let observed_tx = normalize_hex(
        required_string(object, "transactionHash")?,
        "receipt.transactionHash",
    )?;
    if observed_tx != tx_hash {
        return Err(AdapterError::new(
            "RECEIPT_IDENTITY_MISMATCH",
            format!("requested {tx_hash}, observed {observed_tx}"),
        ));
    }
    let block_hash = normalize_hex(
        required_string(object, "blockHash")?,
        "receipt.blockHash",
    )?;
    let block_number = parse_hex_u64(
        required_string(object, "blockNumber")?,
        "receipt.blockNumber",
    )?;
    let status = normalize_hex(required_string(object, "status")?, "receipt.status")?;
    if status != "0x1" {
        return Err(AdapterError::new(
            "TRANSACTION_FAILED",
            format!("transaction {tx_hash} has status {status}"),
        ));
    }
    if let Some(expected_contract) = expected_contract {
        let observed_contract = normalize_hex(
            required_string(object, "contractAddress")?,
            "receipt.contractAddress",
        )?;
        if observed_contract != expected_contract {
            return Err(AdapterError::new(
                "CONTRACT_IDENTITY_MISMATCH",
                format!("expected {expected_contract}, observed {observed_contract}"),
            ));
        }
    }
    Ok(ReceiptSourceEvidence {
        transaction_hash: observed_tx,
        block_number,
        block_hash,
        status,
    })
}

fn deployment_evidence(
    rpc: &RpcClient,
    transaction_hash: &str,
    contract_address: &str,
) -> Result<DeploymentEvidence, AdapterError> {
    let receipt = receipt_evidence(rpc, transaction_hash, Some(contract_address))?;
    Ok(DeploymentEvidence {
        transaction_hash: receipt.transaction_hash,
        block_number: receipt.block_number,
        block_hash: receipt.block_hash,
        contract_address: contract_address.to_owned(),
        status: receipt.status,
    })
}

fn fetch_block(
    rpc: &RpcClient,
    chain_id: u64,
    number: u64,
    events: Vec<Event>,
) -> Result<Block, AdapterError> {
    let tag = format!("0x{number:x}");
    let value = rpc.call("eth_getBlockByNumber", json!([tag, false]))?;
    if value.is_null() {
        return Err(AdapterError::new(
            "MISSING_BLOCK",
            format!("block {number} is unavailable"),
        ));
    }
    let object = as_object(&value, "block")?;
    let observed_number = parse_hex_u64(required_string(object, "number")?, "block.number")?;
    if observed_number != number {
        return Err(AdapterError::new(
            "BLOCK_IDENTITY_MISMATCH",
            format!("requested block {number}, observed {observed_number}"),
        ));
    }
    let hash = normalize_hex(required_string(object, "hash")?, "block.hash")?;
    let parent_hash = normalize_hex(required_string(object, "parentHash")?, "block.parentHash")?;
    Ok(Block {
        chain_id,
        number,
        hash,
        parent_hash,
        events,
    })
}

fn collect_inner(config: &EvmAdapterConfig, partial: &mut PartialEvidence) -> Result<(), AdapterError> {
    if config.decoder_id != REFERENCE_DECODER_ID {
        return Err(AdapterError::new(
            "UNKNOWN_DECODER",
            format!("unsupported decoder identity: {}", config.decoder_id),
        ));
    }
    let rpc = RpcClient::new(&config.rpc_url)?;
    let contract_address = normalize_hex(&config.contract_address, "contract_address")?;
    let deployment_tx = normalize_hex(&config.deployment_tx_hash, "deployment_tx_hash")?;

    let abi_bytes = fs::read(&config.abi_path).map_err(|error| {
        AdapterError::new(
            "ABI_READ_ERROR",
            format!("{}: {error}", config.abi_path.display()),
        )
    })?;
    let abi_json: Value = serde_json::from_slice(&abi_bytes)
        .map_err(|error| AdapterError::new("ABI_DECODE_ERROR", error.to_string()))?;
    partial.abi_sha256 = Some(canonical_json_sha256(&abi_json)?);

    let source_bytes = fs::read(&config.source_path).map_err(|error| {
        AdapterError::new(
            "SOURCE_READ_ERROR",
            format!("{}: {error}", config.source_path.display()),
        )
    })?;
    partial.source_sha256 = Some(sha256_bytes(&source_bytes));

    let client_version = rpc.call("web3_clientVersion", json!([]))?;
    partial.node_client = Some(
        client_version
            .as_str()
            .ok_or_else(|| {
                AdapterError::new("MALFORMED_RPC_FIELD", "web3_clientVersion is not a string")
            })?
            .to_owned(),
    );

    let chain_value = rpc.call("eth_chainId", json!([]))?;
    let chain_id = parse_hex_u64(
        chain_value
            .as_str()
            .ok_or_else(|| AdapterError::new("MALFORMED_RPC_FIELD", "eth_chainId is not a string"))?,
        "eth_chainId",
    )?;
    partial.observed_chain_id = Some(chain_id);
    if chain_id != config.expected_chain_id {
        return Err(AdapterError::new(
            "CHAIN_ID_MISMATCH",
            format!(
                "expected chain id {}, observed {chain_id}",
                config.expected_chain_id
            ),
        ));
    }

    partial.deployment = Some(deployment_evidence(&rpc, &deployment_tx, &contract_address)?);

    let topic0 = reference_event_topic0();
    let logs_value = rpc.call(
        "eth_getLogs",
        json!([{
            "address": contract_address,
            "fromBlock": "0x0",
            "toBlock": "latest",
            "topics": [topic0],
        }]),
    )?;
    let raw_logs = logs_value.as_array().ok_or_else(|| {
        AdapterError::new("MALFORMED_RPC_FIELD", "eth_getLogs result is not an array")
    })?;
    if raw_logs.is_empty() {
        return Err(AdapterError::new(
            "NO_REFERENCE_EVENTS",
            "no reference lifecycle events were observed",
        ));
    }

    let mut decoded_by_identity: BTreeMap<(String, u32), DecodedLog> = BTreeMap::new();
    for raw in raw_logs {
        let decoded = decode_reference_log(raw, &contract_address, &config.decoder_id, &topic0)?;
        let identity = (decoded.event.tx_hash.clone(), decoded.event.log_index);
        if let Some(existing) = decoded_by_identity.get(&identity) {
            if existing != &decoded {
                return Err(AdapterError::new(
                    "CONFLICTING_LOG_IDENTITY",
                    format!("conflicting observations for {}:{}", identity.0, identity.1),
                ));
            }
            continue;
        }
        decoded_by_identity.insert(identity, decoded);
    }

    let mut receipt_hashes = BTreeSet::new();
    for decoded in decoded_by_identity.values() {
        partial.log_sources.push(LogSourceEvidence {
            transaction_hash: decoded.event.tx_hash.clone(),
            block_number: decoded.block_number,
            block_hash: decoded.block_hash.clone(),
            log_index: decoded.event.log_index,
            contract_address: decoded.event.contract_address.clone(),
            topic0: decoded.topic0.clone(),
            data_sha256: decoded.data_sha256.clone(),
        });
        if receipt_hashes.insert(decoded.event.tx_hash.clone()) {
            let receipt = receipt_evidence(&rpc, &decoded.event.tx_hash, None)?;
            if receipt.block_number != decoded.block_number || receipt.block_hash != decoded.block_hash {
                return Err(AdapterError::new(
                    "RECEIPT_LOG_MISMATCH",
                    format!("receipt identity does not match log {}", decoded.event.tx_hash),
                ));
            }
            partial.receipt_sources.push(receipt);
        }
    }

    partial.log_sources.sort_by_key(|item| (item.block_number, item.log_index));
    partial
        .receipt_sources
        .sort_by_key(|item| (item.block_number, item.transaction_hash.clone()));
    partial.adapted_event_count = decoded_by_identity.len();

    let latest_value = rpc.call("eth_blockNumber", json!([]))?;
    let latest = parse_hex_u64(
        latest_value
            .as_str()
            .ok_or_else(|| AdapterError::new("MALFORMED_RPC_FIELD", "eth_blockNumber is not a string"))?,
        "eth_blockNumber",
    )?;

    let mut events_by_block: BTreeMap<u64, Vec<Event>> = BTreeMap::new();
    for decoded in decoded_by_identity.values() {
        events_by_block
            .entry(decoded.block_number)
            .or_default()
            .push(decoded.event.clone());
    }
    for events in events_by_block.values_mut() {
        events.sort_by_key(|event| event.log_index);
    }

    let mut blocks = Vec::with_capacity(usize::try_from(latest + 1).unwrap_or(0));
    for number in 0..=latest {
        let events = events_by_block.remove(&number).unwrap_or_default();
        blocks.push(fetch_block(&rpc, chain_id, number, events)?);
    }
    partial.observed_block_count = blocks.len();

    let core_report = report_fixture(&Fixture { chain_id, blocks });
    if core_report.outcome != "PASS" {
        return Err(AdapterError::new(
            "CORE_RECONSTRUCTION_FAILED",
            core_report
                .error
                .clone()
                .unwrap_or_else(|| "core report was not PASS".to_owned()),
        ));
    }
    partial.core_report = Some(core_report);
    Ok(())
}

fn limitations() -> Vec<String> {
    vec![
        "local loopback HTTP JSON-RPC only".to_owned(),
        "one current canonical snapshot; live-tail/reorg observation remains a later gate".to_owned(),
        "only the checked reference-lifecycle-v1 bytes32 event decoder is trusted".to_owned(),
        "bytes32 key/value payloads must decode to non-empty UTF-8".to_owned(),
        "no network consensus or universal finality claim".to_owned(),
    ]
}

fn digest_report(report: &EvmAdapterReport) -> String {
    let mut unsigned = report.clone();
    unsigned.report_sha256.clear();
    let bytes = serde_json::to_vec(&unsigned).expect("EVM adapter evidence is serializable");
    sha256_bytes(&bytes)
}

#[must_use]
pub fn collect_evm_evidence(config: &EvmAdapterConfig) -> EvmAdapterReport {
    let mut partial = PartialEvidence::default();
    let result = collect_inner(config, &mut partial);
    let contract_address = normalize_hex(&config.contract_address, "contract_address")
        .unwrap_or_else(|_| config.contract_address.clone());
    let topic0 = reference_event_topic0();

    let (outcome, error_code, error) = match result {
        Ok(()) => ("PASS".to_owned(), None, None),
        Err(adapter_error) => (
            "INDETERMINATE".to_owned(),
            Some(adapter_error.code.to_owned()),
            Some(adapter_error.message),
        ),
    };
    let mut report = EvmAdapterReport {
        schema: "chain-evidence.evm-adapter.v0.3".to_owned(),
        outcome,
        node_client: partial.node_client,
        expected_chain_id: config.expected_chain_id,
        observed_chain_id: partial.observed_chain_id,
        contract_address,
        deployment: partial.deployment,
        decoder_id: config.decoder_id.clone(),
        abi_sha256: partial.abi_sha256,
        source_sha256: partial.source_sha256,
        event_signature: EVENT_SIGNATURE.to_owned(),
        event_topic0: topic0,
        log_sources: partial.log_sources,
        receipt_sources: partial.receipt_sources,
        adapted_event_count: partial.adapted_event_count,
        observed_block_count: partial.observed_block_count,
        core_report: partial.core_report,
        error_code,
        error,
        limitations: limitations(),
        claim_boundary: EVM_ADAPTER_CLAIM_BOUNDARY.to_owned(),
        report_sha256: String::new(),
    };
    report.report_sha256 = digest_report(&report);
    report
}

pub fn write_evm_report(path: &Path, report: &EvmAdapterReport) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut encoded = serde_json::to_vec_pretty(report)?;
    encoded.push(b'\n');
    fs::write(path, encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded(value: &str) -> String {
        let mut encoded = String::new();
        for byte in value.as_bytes() {
            encoded.push_str(&format!("{byte:02x}"));
        }
        encoded.push_str(&"00".repeat(32 - value.len()));
        encoded
    }

    fn valid_log() -> Value {
        json!({
            "address": "0x1111111111111111111111111111111111111111",
            "transactionHash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "blockHash": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "blockNumber": "0x2",
            "logIndex": "0x0",
            "topics": [reference_event_topic0()],
            "data": format!("0x{}{}", padded("owner"), padded("alice")),
            "removed": false
        })
    }

    #[test]
    fn decodes_reference_event_with_exact_lineage() {
        let decoded = decode_reference_log(
            &valid_log(),
            "0x1111111111111111111111111111111111111111",
            REFERENCE_DECODER_ID,
            &reference_event_topic0(),
        )
        .expect("valid reference log");
        assert_eq!(decoded.event.key, "owner");
        assert_eq!(decoded.event.value, "alice");
        assert_eq!(decoded.event.log_index, 0);
        assert_eq!(decoded.block_number, 2);
    }

    #[test]
    fn malformed_missing_transaction_hash_fails_closed() {
        let mut value = valid_log();
        value
            .as_object_mut()
            .expect("object")
            .remove("transactionHash");
        let error = decode_reference_log(
            &value,
            "0x1111111111111111111111111111111111111111",
            REFERENCE_DECODER_ID,
            &reference_event_topic0(),
        )
        .expect_err("missing transaction identity must fail");
        assert_eq!(error.code, "MALFORMED_RPC_FIELD");
    }

    #[test]
    fn unknown_topic_fails_closed() {
        let mut value = valid_log();
        value["topics"] = json!([
            "0xcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        ]);
        let error = decode_reference_log(
            &value,
            "0x1111111111111111111111111111111111111111",
            REFERENCE_DECODER_ID,
            &reference_event_topic0(),
        )
        .expect_err("unknown event signature must fail");
        assert_eq!(error.code, "UNKNOWN_EVENT_SIGNATURE");
    }

    #[test]
    fn removed_log_is_not_promoted_to_canonical_evidence() {
        let mut value = valid_log();
        value["removed"] = json!(true);
        let error = decode_reference_log(
            &value,
            "0x1111111111111111111111111111111111111111",
            REFERENCE_DECODER_ID,
            &reference_event_topic0(),
        )
        .expect_err("removed log must fail closed");
        assert_eq!(error.code, "REMOVED_LOG");
    }

    #[test]
    fn event_topic_is_deterministic() {
        assert_eq!(reference_event_topic0().len(), 66);
        assert_eq!(reference_event_topic0(), reference_event_topic0());
    }
}
