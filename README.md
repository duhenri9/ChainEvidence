# ChainEvidence

Reorg-aware EVM indexing and verifiable on-chain data lineage.

ChainEvidence is an open-source distributed-systems project for reconstructing canonical event history, surviving chain reorganisations and producing evidence that explains exactly which observed chain state supports an indexed result.

> Indexed data is useful. Indexed data with canonical-chain provenance is defensible.

## Current engineering baseline

ChainEvidence now has two executable evidence layers:

```text
V0 — canonical semantics
ordered block observations
          ↓
identity + ancestry
          ↓
reorg / orphan invalidation
          ↓
material-state replay
          ↓
canonical lineage + SHA-256

V0.2 — transactional persistence
canonical evidence
          ↓
PostgreSQL transaction
          ↓
blocks + events + canonical flags
          ↓
checkpoint + state/report digests
          ↓
commit / injected rollback
          ↓
new connection + deterministic recovery
```

The project deliberately proves these mechanisms before adding live RPC, Solidity integration, backfill/live-tail or analytics.

## V0 — canonical-chain engine

The fixture-backed Rust engine proves, within its documented deterministic observation policy:

- straight canonical append;
- shorter competing branches stored without silently replacing the tip;
- one-block and multi-block reorganisation;
- common-ancestor discovery from locally known ancestry;
- orphaned block effects removed from material state;
- deterministic replay from the same observation sequence;
- duplicate observation idempotency;
- conflicting block identity and ambiguous duplicate log indexes rejected;
- missing ancestry fails closed as `INDETERMINATE`;
- canonical event lineage preserves chain id, block hash/number, transaction hash, log index, contract address and decoder identity;
- a deliberately naive append-only indexer retains orphaned state and must diverge on the reorg negative control.

The V0 branch-selection rule is test equipment for deterministic reorg mechanics. It is **not** presented as Ethereum consensus/fork-choice or universal finality behaviour.

## V0.2 — PostgreSQL persistence and restart recovery

V0.2 adds a real PostgreSQL 16 gate around the canonical engine.

It proves, against the checked-in migration and synthetic fixtures:

- versioned block/event/checkpoint persistence;
- persisted block/event identity conflict detection rather than blind upsert acceptance;
- canonical flags and checkpoint advancement in one transaction;
- a pre-commit injected rollback preserves the previous durable checkpoint/state;
- a durable commit survives client teardown/reconnect;
- duplicate ingestion after restart remains idempotent;
- a persisted multi-block reorg keeps old fork rows as evidence but marks them non-canonical;
- orphaned material state is absent after recovery;
- recovery validates genesis, contiguous heights and parent-hash continuity;
- checkpoint tip/height must match reconstructed canonical state;
- recovered canonical events reproduce the same V0 state SHA-256;
- stale/corrupt checkpoint identity fails closed;
- a deliberately bad non-transactional checkpoint update is detected rather than accepted.

See [`docs/POSTGRES_RECOVERY.md`](docs/POSTGRES_RECOVERY.md) for the transaction invariant, crash controls and recovery algorithm.

## What is not claimed

The current evidence does **not** establish:

- Ethereum consensus or live-chain fork-choice correctness;
- universal finality or settlement guarantees;
- RPC-provider honesty/completeness;
- production-grade PostgreSQL durability configuration;
- replication, failover, point-in-time recovery or disaster recovery;
- hostile multi-writer concurrency safety;
- audited smart contracts;
- production readiness, multi-chain support or throughput;
- zk verification, graph analytics, ML or fraud detection.

Each of those properties requires a separate executable gate.

## Run the canonical evidence fixture

Requires the Rust toolchain pinned in `rust-toolchain.toml`.

```bash
cargo run --bin chain-evidence -- fixtures/reorg.json artifacts/reorg-report.json
```

A safe missing-ancestor control returns exit code `3` and writes an `INDETERMINATE` report:

```bash
cargo run --bin chain-evidence -- fixtures/missing-ancestor.json artifacts/missing-ancestor-report.json
```

The JSON report includes canonical block identities, event lineage, material state, reorg/orphan evidence, state SHA-256, report SHA-256 and the exact claim boundary.

## Run PostgreSQL recovery evidence

Set `DATABASE_URL` to a PostgreSQL instance, then:

```bash
cargo run --bin persistence_evidence -- \
  fixtures/reorg.json artifacts/postgres-recovery-report.json
```

The tool clears only the fixture chain id, persists the canonical evidence, closes the first client, reconnects, reconstructs the durable state and emits a persistence/recovery evidence pack.

CI runs the same layer against a fresh `postgres:16-alpine` service. Local PostgreSQL configuration outside this bounded fixture path is not certified by the project.

## Engineering gate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The CI gate additionally:

- boots a real PostgreSQL 16 service;
- runs all core and persistence/recovery tests serially;
- proves the in-memory known-bad indexer divergence;
- proves the non-transactional checkpoint negative control;
- executes reorg and missing-ancestor artifacts;
- generates a restart/recovery evidence pack;
- verifies machine-readable semantics;
- uploads evidence artifacts.

## Design notes

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — canonicality, observation policy, reorg/replay semantics.
- [`docs/CLAIMS_AND_THREATS.md`](docs/CLAIMS_AND_THREATS.md) — bounded claims and threat assumptions.
- [`docs/POSTGRES_RECOVERY.md`](docs/POSTGRES_RECOVERY.md) — transactional persistence, crash and restart invariants.
- [Issue #1](../../issues/1) — staged roadmap toward local EVM, backfill/live-tail and analytics.

## Clean-room boundary

This repository is independently authored from public specifications and synthetic/local fixtures. It does not reuse private WM3 ChainLab implementation, business logic, curriculum or data.

## Contributing and security

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)

## Licence

Apache-2.0. See [`LICENSE`](LICENSE).
