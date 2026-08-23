# ChainEvidence — Repository Hygiene Baseline 2026-08

**Status:** active hygiene baseline / first safe correction executed  
**Base SHA:** `73b75d9b5f7615025393347669a5d7f1d6ffdd3b`  
**Current audit branch:** `audit/repository-hygiene-pre-v0-4`  
**Owner:** issue #6  
**Next product gate:** V0.4 bounded backfill + live tail

## 1. Goal

Make sure V0.4 extends the current V0/V0.2/V0.3 evidence system rather than accumulating contradictory docs, abandoned scaffolding or unnecessary compatibility code.

This is not a LOC-reduction exercise. Current negative controls and evidence paths may look old while remaining strategically required.

## 2. Classification

`DEAD / LEGACY / MISPOSITIONED / TECH_DEBT`

Action remains separate: `KEEP / DELETE / ARCHIVE / REPLACE / INVESTIGATE`.

## 3. Evidence-backed findings

| Path / subsystem | Class | Evidence | Action | V0.4 impact | Executed? |
|---|---|---|---|---|---|
| `src/naive.rs` / `NaiveIndexer` | CURRENT REFERENCE CONTROL | Architecture, README, engine/tests and CI deliberately use the naive indexer to retain orphaned effects and prove canonical reorg divergence. | `KEEP` | Protect as an adversarial/negative baseline. It is not dead code. | yes — keep decision recorded |
| `docs/ARCHITECTURE.md` prior “Next architecture gates” | `LEGACY` documentation | The document still described PostgreSQL persistence and local EVM integration as future gates even though V0.2 and V0.3 are delivered on main. | `REWRITE` | Prevent Memory Reality before V0.4 and make the extension boundary explicit. | **yes** — rewritten on this branch |
| V0 canonical engine | CURRENT | README/current architecture identify it as the deterministic canonical/reorg core consumed by later persistence/EVM layers. | `KEEP` | V0.4 must feed observations into this boundary rather than duplicate fork/reorg semantics. | yes — protected boundary |
| V0.2 PostgreSQL persistence + migration | CURRENT | Main history/README expose transactional restart/reorg recovery as a delivered evidence gate. | `KEEP` | Backfill/live-tail checkpoints should reuse persistence semantics rather than add a parallel cursor store unless evidence requires one. | yes — protected boundary |
| V0.3 local EVM adapter | CURRENT / boundary to protect | Current README/history pin a bounded local Anvil/Foundry adapter and explicit ABI. | `KEEP` | V0.4 should extend provider ingestion without leaking local-EVM assumptions into the canonical chain engine. | yes — protected boundary |
| `src/bin/**` multiple evidence CLIs | `INVESTIGATE` | Distinct V0/V0.2/V0.3 evidence commands may be deliberate reproducibility surfaces. No deletion proof yet. | `INVESTIGATE` | V0.4 should add one bounded command or subcommand only after checking existing CLI overlap. | no |
| fixtures across delivered gates | `INVESTIGATE` | Historical and negative-control fixtures may support deterministic evidence and CI. | `INVESTIGATE` | Do not deduplicate by filename/shape alone; map test/CI consumers first. | no |
| dependencies/features in `Cargo.toml` | `INVESTIGATE` | V0.2 and V0.3 added PostgreSQL/RPC dependencies. Current reachability needs cargo/module audit before pruning. | `INVESTIGATE` | Prune only after current binaries/tests and V0.4 adapter shape are known. | no |

## 4. First executed correction

The architecture document has now been truth-synchronised on this audit branch.

It explicitly records:

```text
V0 canonical/reorg core — delivered
V0.2 PostgreSQL persistence/recovery — delivered
V0.3 bounded local EVM adapter — delivered
V0.4 bounded backfill + live tail — next
```

The corrected V0.4 extension boundary is:

```text
bounded EVM RPC provider
→ backfill/live-tail observation adapter
→ canonical Block/Event observations
→ existing canonical/reorg engine
→ existing persistence/evidence path
```

This removes a proven documentation contradiction without deleting runtime evidence or changing executable semantics.

## 5. V0.4 invariants carried forward

V0.4 must not introduce:

- a second canonical chain implementation;
- a second event identity format;
- hidden fork-choice/finality semantics;
- a cursor/checkpoint path that bypasses current persistence evidence without a documented reason;
- public-RPC correctness or universal-finality claims from a bounded provider experiment.

It must prove:

- bounded range backfill;
- resumable checkpoint behavior;
- backfill/live overlap idempotency;
- provider failure/reconnect behavior;
- explicit live reorg observation/recovery;
- exact incomplete/failure states when continuity cannot be established.

## 6. Remaining inventory before destructive cleanup

- enumerate `src/bin/**` and map each to README/docs/CI;
- map fixture consumers across unit/integration/CI;
- compare migrations with current persistence structs/queries;
- inspect local EVM adapter for provider-neutral vs Anvil-only code;
- inspect Cargo dependencies/features for current consumers;
- review remaining docs for V0.2/V0.3 truth sync;
- inspect CI for duplicated setup/evidence commands;
- check generated evidence/artifact directories against `.gitignore`.

## 7. No-delete boundary

No source file, fixture, migration or dependency is authorised for deletion from this baseline alone.

Any destructive follow-up must provide:

- direct/reference search;
- runtime/CLI/test/CI/config evidence;
- exact replacement if behavior remains required;
- full affected validation suite on exact SHA.

## 8. Current decision

The repository currently looks more like a **mature layered prototype with documentation drift** than a codebase with obvious dead-code accumulation.

First proven cleanup result:

`LEGACY DOCUMENTATION → REWRITE → EXECUTED`

First protected false-positive cleanup candidate:

`NaiveIndexer → KEEP → NEGATIVE CONTROL`

Proceed with the remaining inventory and then V0.4. Do not spend the active engineering window trying to manufacture deletion candidates when the stronger next value is bounded live/backfill ingestion evidence.
