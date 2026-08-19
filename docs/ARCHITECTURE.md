# ChainEvidence V0 architecture

ChainEvidence V0 is a deterministic, fixture-backed canonical-chain engine. It is deliberately smaller than a production blockchain indexer: the first gate proves reorganisation and lineage semantics before adding databases, RPC clients or contracts.

```text
ordered block observations
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
```

## Block and event identity

Every block carries `chain_id`, block number, block hash and parent hash. Every indexed event keeps block identity plus transaction hash, log index, contract address and decoder identity.

V0 rejects duplicate log indexes within one block because an ambiguous event position would weaken lineage evidence.

## Observation policy

V0 does **not** implement network consensus or universal finality.

Its deterministic fixture policy is:

1. the first accepted observation must be block 0;
2. a block extending the current canonical tip is appended;
3. a competing branch whose observed tip is shorter than the current canonical tip is stored but not promoted;
4. a competing branch whose observed tip reaches or exceeds the canonical height is promoted;
5. equal-height replacement therefore follows last-observed fixture order.

That rule exists to make reorg mechanics falsifiable in V0. It must not be described as Ethereum fork-choice or finality logic.

## Reorganisation

When a competing branch becomes eligible for promotion, the engine walks parent hashes until it finds a locally known canonical ancestor. It then:

- records the common ancestor;
- marks the old canonical suffix as orphaned evidence;
- replaces that suffix with the observed competing branch;
- rebuilds canonical event lineage;
- replays material state from the canonical sequence.

If ancestry required for safe reconciliation is unavailable, V0 returns `INDETERMINATE` with `MISSING_ANCESTOR`. It does not guess.

## Material state

Events in canonical block order, then ascending `log_index`, update a key/value material state. This is intentionally synthetic state-transition equipment, not an EVM state model.

A state digest covers:

- canonical block hashes;
- canonical event evidence;
- material key/value state.

The report digest identifies the complete redacted V0 evidence payload. These digests are integrity identifiers, not signatures or consensus proofs.

## Negative control

`NaiveIndexer` deliberately applies every observed event and never invalidates orphaned effects. The V0 reorg fixture leaves an `orphan-only` key in the naive state while the canonical engine removes it. CI requires that divergence to remain observable.

## Next architecture gates

Only after this V0 remains stable:

1. transactional PostgreSQL persistence and restart recovery;
2. local EVM + Solidity reference-event integration;
3. bounded backfill and live-tail adapters;
4. explicit confirmation/finality policy;
5. query API and analytics layers.
