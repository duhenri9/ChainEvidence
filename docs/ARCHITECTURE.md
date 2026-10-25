# ChainEvidence architecture

ChainEvidence is a layered evidence system for reconstructing canonical EVM event history under explicit, bounded assumptions. The deterministic V0 canonical-chain engine remains the core. V0.2 and V0.3 are delivered layers around that core; V0.4 is the next ingestion/runtime gate.

The project is deliberately smaller than a production blockchain indexer. Each layer proves a narrow property before the next one is allowed to expand the system.

## Current architecture

```text
bounded observations / provider adapter
        ↓
identity + chain validation
        ↓
known block graph
        ↓
parent continuity / ancestry resolution
        ↓
canonical branch selection policy
        ↓
reorg + orphan invalidation
        ↓
full deterministic material-state replay
        ↓
canonical event lineage + SHA-256 evidence
        ↓
transactional PostgreSQL persistence/recovery
```

Current delivered provider boundary:

```text
local Anvil / EVM JSON-RPC snapshot
        ↓
V0.3 bounded EVM adapter
        ↓
canonical Block/Event observations
        ↓
existing V0 canonical/reorg engine
        ↓
existing V0.2 persistence/evidence path
```

The next gate extends the **observation/ingestion boundary**. It does not replace the canonical engine or persistence semantics.

## V0 — canonical-chain core

### Block and event identity

Every block carries `chain_id`, block number, block hash and parent hash. Every indexed event keeps block identity plus transaction hash, log index, contract address and decoder identity.

V0 rejects duplicate log indexes within one block because an ambiguous event position would weaken lineage evidence.

### Observation policy

V0 does **not** implement network consensus or universal finality.

Its deterministic fixture policy is:

1. the first accepted observation must be block 0;
2. a block extending the current canonical tip is appended;
3. a competing branch whose observed tip is shorter than the current canonical tip is stored but not promoted;
4. a competing branch whose observed tip reaches or exceeds the canonical height is promoted;
5. equal-height replacement therefore follows last-observed fixture order.

That rule exists to make reorg mechanics falsifiable in V0. It must not be described as Ethereum fork-choice or finality logic.

### Reorganisation

When a competing branch becomes eligible for promotion, the engine walks parent hashes until it finds a locally known canonical ancestor. It then:

- records the common ancestor;
- marks the old canonical suffix as orphaned evidence;
- replaces that suffix with the observed competing branch;
- rebuilds canonical event lineage;
- replays material state from the canonical sequence.

If ancestry required for safe reconciliation is unavailable, V0 returns `INDETERMINATE` with `MISSING_ANCESTOR`. It does not guess.

### Material state

Events in canonical block order, then ascending `log_index`, update a key/value material state. This is intentionally synthetic state-transition equipment, not an EVM state model.

A state digest covers:

- canonical block hashes;
- canonical event evidence;
- material key/value state.

The report digest identifies the complete redacted evidence payload. These digests are integrity identifiers, not signatures or consensus proofs.

### Negative control

`NaiveIndexer` deliberately applies every observed event and never invalidates orphaned effects. The V0 reorg fixture leaves an `orphan-only` key in the naive state while the canonical engine removes it. CI requires that divergence to remain observable.

`NaiveIndexer` is therefore an intentional adversarial/reference control, not obsolete code.

## V0.2 — transactional PostgreSQL persistence and recovery — delivered

V0.2 adds the durable boundary around the canonical engine.

Delivered properties include:

- versioned PostgreSQL migration;
- observed block/event lineage persisted independently from canonical flags;
- canonical flags and checkpoint advancement in one transaction;
- restart/reconnect recovery from persisted canonical state;
- rollback/failure controls that preserve the prior durable checkpoint;
- persisted reorg evidence with orphaned rows retained while canonical effects are invalidated;
- fail-closed recovery when persisted checkpoint/state is inconsistent.

V0.2 does not establish production database replication, disaster recovery, hostile concurrent-writer safety, network/provider correctness or blockchain finality.

## V0.3 — bounded local EVM adapter — delivered

V0.3 adds one real observation adapter while keeping canonicality in the existing core.

Delivered properties include:

- versioned Solidity reference event + ABI identity;
- loopback-only JSON-RPC boundary for a controlled local EVM;
- preservation of chain/block/transaction/log/contract/decoder identity into the existing core model;
- receipt/log consistency checks;
- rejection of wrong chain, malformed/removed logs, unknown decoder and conflicting identities;
- deterministic replay against the same local snapshot;
- Anvil snapshot/revert replacement control.

This remains a **local current-snapshot adapter**. It does not prove public-RPC correctness, continuous live-tail reorg handling, universal finality, hostile-provider safety, smart-contract correctness or multi-chain compatibility.

## V0.4 — next gate: bounded backfill + live tail

V0.4 should extend the provider/ingestion layer only as far as required to prove resumable historical catch-up and controlled live-head ingestion.

Target shape:

```text
bounded EVM RPC provider
        ↓
range backfill
        ↓
resumable persisted ingestion checkpoint
        ↓
overlap/idempotency boundary
        ↓
live-head observation loop
        ↓
provider failure/reconnect handling
        ↓
explicit reorg observation
        ↓
existing canonical engine
        ↓
existing persistence/evidence path
```

Required constraints:

- no second canonical-chain implementation;
- no second event-identity contract;
- no hidden fork-choice/finality policy;
- backfill/live overlap must be idempotent;
- retry/reconnect must not duplicate canonical evidence;
- provider failure must produce an explicit incomplete/failure state rather than guessed continuity;
- confirmation/finality, if introduced, is explicit configuration and never described as universal network truth;
- public RPC correctness remains outside the claim unless separately evidenced.

## Later gates — not current claims

Only after V0.4 is stable:

1. explicit confirmation/finality policy with bounded semantics;
2. query API over canonical state;
3. reproducible analytics layers;
4. anomaly/ML analysis only as separately versioned evidence;
5. multi-chain or verifiable/zk extensions only behind their own evidence gates.

The architecture must remain evidence-driven: a new layer may consume the canonical engine, but it may not silently replace the identity, reorg, lineage or uncertainty semantics already proven by earlier gates.
