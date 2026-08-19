# PostgreSQL persistence and recovery — V0.2

V0.2 adds a real PostgreSQL durability boundary to the deterministic canonical-chain core. The gate is intentionally local and bounded: it proves transaction/restart semantics against the checked-in schema and synthetic reorg fixtures before any RPC or live-chain adapter is introduced.

## Core invariant

A checkpoint must never advance independently of the canonical data it describes.

```text
observed fixture
      ↓
canonical V0 evidence
      ↓
BEGIN TRANSACTION
      ↓
store/verify block + event identities
      ↓
reset/promote canonical flags
      ↓
write checkpoint + state/report digests
      ↓
COMMIT
```

If the transaction does not commit, the previous durable checkpoint and canonical flags remain the recovery source of truth.

## Schema

Migration `0001_persistence` creates four evidence surfaces:

- `chain_evidence_migrations` — migration identity;
- `chain_evidence_blocks` — observed block identity plus canonical flag;
- `chain_evidence_events` — event lineage plus canonical flag;
- `chain_evidence_checkpoints` — schema version, canonical tip/height and core state/report digests.

Observed fork/orphan rows are retained. Canonicality is a state that can change; deleting every orphan would destroy useful reorg evidence.

## Identity conflict policy

`ON CONFLICT DO NOTHING` is not treated as proof that duplicate input is safe. After an existing block/event identity is encountered, V0.2 reads it back and verifies the immutable identity fields.

A reused block hash or `(block_hash, log_index)` with conflicting persisted content returns a bounded error instead of silently accepting the new payload.

## Recovery algorithm

A fresh PostgreSQL connection:

1. reads the checkpoint and verifies `schema_version`;
2. loads canonical blocks in height order;
3. requires genesis at height 0;
4. verifies contiguous heights and parent-hash continuity;
5. requires persisted canonical tip/height to equal the checkpoint;
6. loads canonical event lineage and verifies event canonical flags;
7. replays the material key/value state in block/log order;
8. recomputes the same V0 state SHA-256 shape used by the in-memory engine;
9. rejects the recovery if the checkpoint digest and reconstructed digest differ.

A recovery report is evidence about this bounded reconstruction. It is not a database backup certificate or blockchain-finality proof.

## Crash controls

### Pre-commit abort

`PersistMode::AbortBeforeCommit` deliberately executes the persistence transaction and rolls it back before commit. CI first persists a previous canonical state, injects the abort while applying a replacement reorg state, reconnects, then proves the previous checkpoint/state is unchanged.

### Durable commit + reconnect

CI commits a full state, drops the client, reconnects and reconstructs the evidence. Re-ingesting the same fixture after restart must not duplicate block/event rows or alter the recovered digest.

## Persisted reorg control

CI first commits the old canonical branch and then commits the full observation sequence containing the replacement branch. Recovery must show:

- old `a1/a2` blocks retained but `canonical = false`;
- replacement `b1/b2` blocks `canonical = true`;
- canonical chain `g → b1 → b2`;
- `orphan-only` absent from material state;
- replacement state present.

## Known-bad non-transactional control

A deliberately bad test mutates the checkpoint to point at the replacement branch **without** atomically persisting the corresponding canonical flags/data.

`recover()` must reject this with `STALE_OR_CORRUPT_CHECKPOINT`.

The control exists because a happy-path persistence test would not prove that checkpoint/data atomicity is actually enforced.

## Evidence pack

`persistence_evidence` produces a machine-readable pack containing:

- schema/migration version;
- chain id;
- persisted canonical tip/height;
- canonical block/event counts;
- persistence outcome;
- restart/recovery outcome;
- core state SHA-256;
- source report SHA-256;
- persistence/recovery evidence SHA-256 identities;
- pack SHA-256;
- explicit claim boundary.

## What V0.2 does not prove

- production-grade durability configuration;
- replication, failover or point-in-time recovery;
- disaster recovery;
- hostile concurrent writers;
- RPC/provider correctness;
- live-chain fork choice or finality;
- smart-contract correctness;
- production throughput/latency.

Those properties require separate mechanisms and evidence gates.
