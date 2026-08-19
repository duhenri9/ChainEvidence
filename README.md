# ChainEvidence

Reorg-aware EVM indexing and verifiable on-chain data lineage.

ChainEvidence is an open-source distributed-systems project for reconstructing canonical event history, surviving chain reorganisations and producing evidence that explains exactly which observed chain state supports an indexed result.

> Indexed data is useful. Indexed data with canonical-chain provenance is defensible.

## V0 status

The first executable gate is fixture-backed and deterministic. It proves a bounded canonical/reorg mechanism before adding PostgreSQL, RPC clients, Solidity contracts or analytics.

```text
ordered block observations
          ↓
chain + identity validation
          ↓
known block graph
          ↓
parent continuity / common ancestor
          ↓
canonical branch policy
          ↓
reorg + orphan invalidation
          ↓
material-state replay
          ↓
canonical event lineage + SHA-256 evidence
```

### What V0 proves

- straight canonical append;
- shorter competing branches are stored without silently replacing the tip;
- one-block and multi-block reorganisation;
- common-ancestor discovery from locally known ancestry;
- orphaned block effects are removed from material state;
- deterministic replay from the same observation sequence;
- duplicate observation is idempotent;
- conflicting block identity and ambiguous duplicate log indexes are rejected;
- missing ancestry fails closed as `INDETERMINATE`;
- every canonical event preserves chain id, block hash/number, transaction hash, log index, contract address and decoder identity;
- a deliberately naive append-only indexer diverges after reorg and is required to fail the control expectation.

### What V0 does not prove

- Ethereum consensus or fork-choice correctness;
- universal finality or settlement guarantees;
- RPC-provider honesty/completeness;
- persistence or crash-safe database recovery;
- audited smart contracts;
- production readiness, multi-chain support or throughput;
- zk verification, graph analytics, ML or fraud detection.

## Run the evidence fixture

Requires the Rust toolchain pinned in `rust-toolchain.toml`.

```bash
cargo run -- fixtures/reorg.json artifacts/reorg-report.json
```

A safe missing-ancestor control returns exit code `3` and writes an `INDETERMINATE` report:

```bash
cargo run -- fixtures/missing-ancestor.json artifacts/missing-ancestor-report.json
```

The JSON report includes canonical block identities, event lineage, material state, reorg/orphan evidence, a state SHA-256, an overall report SHA-256 and the exact claim boundary.

## Engineering gate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

CI additionally executes the reorg and missing-ancestor fixtures, proves deterministic replay, runs the known-bad indexer control and uploads machine-readable evidence artifacts.

## Design notes

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — canonicality, observation policy, reorg/replay semantics.
- [`docs/CLAIMS_AND_THREATS.md`](docs/CLAIMS_AND_THREATS.md) — bounded claims and threat assumptions.
- [Issue #1](../../issues/1) — staged roadmap from V0 to persistence, local EVM, live tail and analytics.

## Clean-room boundary

This repository is independently authored from public specifications and synthetic/local fixtures. It does not reuse private WM3 ChainLab implementation, business logic, curriculum or data.

## Licence

Apache-2.0. See [`LICENSE`](LICENSE).
