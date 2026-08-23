# ChainEvidence — Repository Hygiene Baseline 2026-08

**Status:** audit baseline / no destructive cleanup yet  
**Base SHA:** `73b75d9b5f7615025393347669a5d7f1d6ffdd3b`  
**Owner:** issue #6  
**Next product gate:** V0.4 bounded backfill + live tail

## 1. Goal

Make sure V0.4 extends the current V0/V0.2/V0.3 evidence system rather than accumulating contradictory docs, abandoned scaffolding or unnecessary compatibility code.

This is not a LOC-reduction exercise. Current negative controls and evidence paths may look old while remaining strategically required.

## 2. Classification

`DEAD / LEGACY / MISPOSITIONED / TECH_DEBT`

Action remains separate: `KEEP / DELETE / ARCHIVE / REPLACE / INVESTIGATE`.

## 3. First evidence-backed findings

| Path / subsystem | Class | Evidence | Action | V0.4 impact |
|---|---|---|---|---|
| `src/naive.rs` / `NaiveIndexer` | CURRENT REFERENCE CONTROL | `docs/ARCHITECTURE.md`, README, engine/tests and CI deliberately use the naive indexer to retain orphaned effects and prove canonical reorg divergence. | `KEEP` | Protect as an adversarial/negative baseline. It is not dead code. |
| `docs/ARCHITECTURE.md` “Next architecture gates” | `LEGACY` documentation | The document still says PostgreSQL persistence and local EVM integration are future gates, while V0.2 and V0.3 are already delivered on main. | `REWRITE` after inventory | Avoid misleading contributors before V0.4; preserve V0 historical architecture while marking delivered gates. |
| V0 canonical engine | CURRENT | README/current architecture identify it as the deterministic canonical/reorg core consumed by later persistence/EVM layers. | `KEEP` | V0.4 must feed observations into this boundary rather than duplicate fork/reorg semantics. |
| V0.2 PostgreSQL persistence + migration | CURRENT | Main history/README expose transactional restart/reorg recovery as a delivered evidence gate. | `KEEP` | Backfill/live-tail checkpoints should reuse persistence semantics rather than add a parallel cursor store unless evidence requires one. |
| V0.3 local EVM adapter | CURRENT / boundary to protect | Current README/history pin a bounded local Anvil/Foundry adapter and explicit ABI. | `KEEP` | V0.4 should extend provider ingestion without leaking local-EVM assumptions into the canonical chain engine. |
| `src/bin/**` multiple evidence CLIs | `INVESTIGATE` | Distinct V0/V0.2/V0.3 evidence commands may be deliberate reproducibility surfaces. No deletion proof yet. | `INVESTIGATE` | V0.4 should add one bounded command or subcommand only after checking existing CLI overlap. |
| fixtures across delivered gates | `INVESTIGATE` | Historical and negative-control fixtures may support deterministic evidence and CI. | `INVESTIGATE` | Do not deduplicate by filename/shape alone; map test/CI consumers first. |
| dependencies/features in `Cargo.toml` | `INVESTIGATE` | V0.2 and V0.3 added PostgreSQL/RPC dependencies. Current reachability needs cargo/module audit before pruning. | `INVESTIGATE` | Prune only after current binaries/tests and V0.4 adapter shape are known. |

## 4. V0.4 boundary inferred from current architecture

The safest extension point is:

```text
bounded RPC provider
→ backfill/live-tail observation adapter
→ canonical Block/Event observations
→ existing canonical/reorg engine
→ existing persistence/evidence path
```

V0.4 should not introduce a second canonical chain implementation, second event identity format or hidden fork-choice policy.

## 5. Remaining inventory before cleanup

- enumerate `src/bin/**` and map each to README/docs/CI;
- map fixture consumers across unit/integration/CI;
- compare migrations with current persistence structs/queries;
- inspect local EVM adapter for provider-neutral vs Anvil-only code;
- inspect Cargo dependencies/features for current consumers;
- review all docs for V0.2/V0.3 truth sync;
- inspect CI for duplicated setup/evidence commands;
- check generated evidence/artifact directories against `.gitignore`.

## 6. No-delete boundary

No source file, fixture, migration or dependency is authorised for deletion from this baseline alone.

Any destructive follow-up must provide:

- direct/reference search;
- runtime/CLI/test/CI/config evidence;
- exact replacement if behavior remains required;
- full affected validation suite on exact SHA.

## 7. Preliminary decision

The repository currently looks more like a **mature layered prototype with documentation drift** than a codebase with obvious dead-code accumulation. The first proven cleanup target is documentation truth, not runtime deletion.

Proceed toward V0.4 after the remaining inventory confirms that no duplicate ingestion/cursor boundary already exists.
