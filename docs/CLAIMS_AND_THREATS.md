# Claims and threat model

## V0 claim

A successful V0 report means only that, for the supplied synthetic observation sequence, the implemented engine reconstructed the canonical branch according to the documented observation policy, removed orphaned material effects, preserved event lineage and emitted deterministic evidence.

## Explicitly unsupported claims

V0 does not prove:

- Ethereum consensus or fork-choice correctness;
- economic/finality guarantees;
- RPC provider honesty or completeness;
- production availability or durability;
- smart-contract correctness or audit status;
- multi-chain compatibility;
- high throughput or low latency;
- fraud detection, graph analytics, ML or zk verification.

## Threats considered in V0

### Competing branch / reorganisation

A previously canonical suffix may become orphaned. The engine must remove its material effects and preserve evidence naming the orphaned block hashes.

### Missing ancestry

A new observation may reference a parent that is unavailable locally. The engine must fail closed with `INDETERMINATE`; inventing an ancestor is prohibited.

### Duplicate observation

The same block may be observed again. Identical content is idempotent. Reusing one block hash for conflicting content is rejected.

### Ambiguous event ordering

Duplicate `log_index` values within one block are rejected in V0 because event lineage would be ambiguous.

### Naive append-only indexing

An implementation that only applies newly observed events can retain orphaned state after a reorg. A deliberately naive indexer is shipped solely as a negative control and must diverge on the reorg fixture.

## Threats deferred

- malicious or inconsistent RPC responses;
- partial provider outages;
- database transaction failures;
- checkpoint corruption;
- ABI spoofing or decoder upgrades;
- chain-id misconfiguration across external transports;
- contract-level adversarial behaviour;
- denial-of-service/resource exhaustion.

Those require later adapters and separate executable gates.
