# ChainEvidence

Reorg-aware EVM indexing and verifiable on-chain data lineage.

ChainEvidence is an open-source distributed-systems project for reconstructing canonical event history, surviving chain reorganisations and producing evidence that explains exactly which chain state supports an indexed result.

> Indexed data is useful. Indexed data with canonical-chain provenance is defensible.

## Status

Early public foundation. The V0 implementation is being built clean-room from public specifications and synthetic/local-chain fixtures. It does not reuse private WM3 ChainLab implementation, business logic or data.

## Design principles

- Canonicality before convenience.
- Reorgs are normal system behaviour, not edge-case decoration.
- Every material indexed event keeps cryptographic lineage.
- Replay and recovery must be deterministic.
- Missing ancestry fails closed instead of manufacturing certainty.
- Negative controls must prove that naive indexing can diverge.
- Performance, finality and production claims require their own evidence.

## V0 target

```text
EVM blocks + logs + receipts
          ↓
parent-hash continuity
          ↓
canonical-chain engine
          ↓
reorg detection / common ancestor
          ↓
orphan invalidation + replay
          ↓
material state + cryptographic lineage
          ↓
deterministic evidence manifest
```

The first public gate focuses on canonical-chain correctness using deterministic fixtures. PostgreSQL, local EVM integration, Solidity reference contracts, live tail/backfill and analytics are staged after the core reorg/replay contract is proven.

## Claim boundary

V0 will not claim production readiness, audited smart contracts, multi-chain support, universal finality guarantees, zk verification, ML fraud detection or sustained throughput until those capabilities have independent executable evidence.

## Licence

Apache-2.0 planned for the public V0. Licence file will be added with the first implementation PR.
