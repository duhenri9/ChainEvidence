# ChainEvidence — Repository Hygiene Baseline 2026-08

**Status:** audit baseline / narrow truth-sync executed / no destructive runtime cleanup yet  
**Base SHA:** `73b75d9b5f7615025393347669a5d7f1d6ffdd3b`  
**Current audit branch:** `audit/repository-hygiene-pre-v0-4`  
**Owner:** issue #6  
**Next product gate:** issue #8 — V0.4 bounded backfill + live tail

## 1. Goal

Make sure V0.4 extends the current V0/V0.2/V0.3 evidence system rather than accumulating contradictory docs, abandoned scaffolding or unnecessary compatibility code.

This is not a LOC-reduction exercise. Current negative controls and evidence paths may look old while remaining strategically required.

## 2. Classification

`DEAD / LEGACY / MISPOSITIONED / TECH_DEBT`

Action remains separate: `KEEP / DELETE / ARCHIVE / REPLACE / INVESTIGATE`.

## 3. Evidence-backed findings

| Path / subsystem | Class | Evidence | Action | V0.4 impact |
|---|---|---|---|---|
| `src/naive.rs` / `NaiveIndexer` | CURRENT REFERENCE CONTROL | Architecture, README, engine/tests and CI deliberately use the naive indexer to retain orphaned effects and prove canonical reorg divergence. | `KEEP` | Protect as an adversarial/negative baseline. It is not dead code. |
| `docs/ARCHITECTURE.md` “Next architecture gates” | `LEGACY` documentation — FIXED ON AUDIT BRANCH | The document previously described PostgreSQL persistence and local EVM integration as future work although V0.2/V0.3 were delivered. | `REWRITE` — executed | The branch now marks V0.2/V0.3 delivered and V0.4 as next, preserving historical V0 boundaries. |
| V0 canonical engine | CURRENT | README/current architecture identify it as the deterministic canonical/reorg core consumed by later persistence/EVM layers. | `KEEP` | V0.4 must feed observations into this boundary rather than duplicate fork/reorg semantics. |
| V0.2 PostgreSQL persistence + migration | CURRENT | Main history/README expose transactional restart/reorg recovery as a delivered evidence gate. | `KEEP` | Backfill/live-tail checkpoints should reuse persistence semantics rather than add a parallel cursor store unless evidence requires one. |
| V0.3 local EVM adapter | CURRENT / boundary to protect | Current README/history pin a bounded local Anvil/Foundry adapter and explicit ABI. | `KEEP` | V0.4 should extend provider ingestion without leaking local-EVM assumptions into the canonical chain engine. |
| `src/bin/persistence_evidence.rs` | CURRENT REPRODUCIBILITY SURFACE | CLI requires `DATABASE_URL`, persists a fixture, reconnects, recovers canonical state and emits the versioned `chain-evidence.persistence-pack.v0.2` digest. It is an executable V0.2 evidence boundary, not an abandoned binary. | `KEEP` | V0.4 may extend the evidence pack or add one bounded V0.4 command, but must not silently replace V0.2 reproducibility. |
| `src/bin/evm_evidence.rs` | CURRENT REPRODUCIBILITY SURFACE | CLI executes the bounded local-EVM adapter with explicit loopback RPC / chain / contract / deployment identity and writes the V0.3 evidence report. | `KEEP` | Reuse adapter primitives; do not turn this command into an unbounded daemon or bury V0.3 proof inside V0.4. |
| `src/bin/**` count | CURRENT / intentionally small | Current source tree has only two evidence binaries: V0.2 persistence and V0.3 EVM. No CLI proliferation is currently proven. | `KEEP` | V0.4 should prefer one bounded evidence command/binary only if the new gate cannot be expressed safely with existing entry points. |
| `Cargo.toml` runtime dependencies | CURRENT / no deletion proof | Current package remains small: `postgres`, `reqwest`, `serde`, `serde_json`, `sha2`, `sha3`. Each dependency family corresponds to delivered persistence, JSON-RPC/HTTP, serialization/evidence hashing or EVM topic/hash semantics. | `KEEP / RECHECK AFTER V0.4` | Do not prune before the live-tail adapter shape is known. No dependency bloat is currently proven. |
| fixtures across delivered gates | `INVESTIGATE` | Historical and negative-control fixtures may support deterministic evidence and CI. | `INVESTIGATE` | Do not deduplicate by filename/shape alone; map test/CI consumers first. |
| CI evidence jobs | CURRENT / INVESTIGATE FOR DUPLICATION ONLY | V0.2/V0.3 depend on separate Postgres and local-EVM evidence gates. Their existence is expected from the delivered architecture. | `KEEP`, inspect command duplication only | V0.4 must preserve old gates and add one bounded new gate rather than collapsing proof history. |

## 4. V0.4 boundary from current architecture

The safe extension point is:

```text
bounded RPC provider
→ backfill/live-tail observation adapter
→ canonical Block/Event observations
→ existing canonical/reorg engine
→ existing PostgreSQL persistence/evidence path
```

V0.4 must not introduce:

- a second canonical chain implementation;
- a second event identity format;
- an independent cursor database without evidence that current persistence cannot own resume safely;
- an implicit/unrecorded finality policy;
- a new provider framework purely for abstraction symmetry.

Execution contract is tracked in issue #8.

## 5. Remaining inventory before any destructive cleanup

- map fixture consumers across unit/integration/CI;
- inspect migrations against current persistence structs/queries for superseded schema paths;
- inspect local EVM adapter for provider-neutral reusable boundary vs intentionally Anvil-only test equipment;
- inspect CI for duplicated setup commands that can be consolidated without reducing independent proof;
- check generated evidence/artifact directories against `.gitignore` and repository-tracked evidence policy.

The remaining inventory is deliberately narrow. No broad refactor is justified before V0.4.

## 6. No-delete boundary

No source file, fixture, migration or dependency is authorised for deletion from this baseline alone.

Any destructive follow-up must provide:

- direct/reference search;
- runtime/CLI/test/CI/config evidence;
- exact replacement if behaviour remains required;
- full affected validation suite on exact SHA.

## 7. Current decision

The repository currently looks like a **mature layered prototype with limited documentation drift**, not a codebase suffering from obvious dead-code accumulation.

The first proven cleanup target was documentation truth and that correction is executed on the audit branch. The two evidence CLIs are intentionally distinct and current. The dependency set is already small enough that premature pruning would create more risk than value.

Recommended sequence:

```text
finish narrow fixture/CI/migration inventory
→ close hygiene gate with no manufactured deletion
→ start issue #8 V0.4
```

The objective remains: **less accidental system, more intentional evidence infrastructure**.
