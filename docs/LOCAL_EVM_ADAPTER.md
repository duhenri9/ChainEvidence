# V0.3 local EVM adapter

ChainEvidence V0.3 adds one bounded bridge from a real local EVM JSON-RPC snapshot into the deterministic canonical-chain core delivered in V0/V0.2.

## Tooling decision

The gate uses Foundry Anvil only as a local in-memory EVM node. The Rust adapter intentionally talks to a small JSON-RPC surface directly instead of introducing a broad Web3 framework:

- `web3_clientVersion`;
- `eth_chainId`;
- `eth_blockNumber`;
- `eth_getBlockByNumber`;
- `eth_getLogs`;
- `eth_getTransactionReceipt`.

The CI fixture never targets a public RPC endpoint and does not require a private key. Transactions are sent only from accounts already unlocked by the local Anvil process.

## Reference contract

`evm/src/ReferenceLifecycle.sol` is test equipment, not product contract logic. It emits exactly one versioned event:

```solidity
event LifecycleSet(bytes32 key, bytes32 value);
```

The trusted decoder identity is `reference-lifecycle-v1`. The checked ABI lives at `evm/abi/ReferenceLifecycle.v1.json`; CI compiles the Solidity source with the pinned Foundry/Solidity configuration and requires the compiler ABI to equal the checked ABI before running adapter evidence.

The evidence report records SHA-256 identities for the canonicalised ABI and Solidity source, plus the event signature/topic identity.

## Adapter contract

For every accepted log, V0.3 preserves:

- observed chain id;
- block number and block hash;
- transaction hash;
- log index;
- contract address;
- decoder id;
- event topic identity;
- SHA-256 of the raw log data field.

The adapter verifies each unique event transaction receipt and requires receipt block identity to agree with the log. It then fetches every current canonical block from genesis to the local head and constructs the existing `Block`/`Event` core model. Canonicality and material-state reconstruction still belong to the core engine; the adapter does not redefine them.

## Fail-closed conditions

The report becomes `INDETERMINATE` for declared failures including:

- non-loopback RPC target;
- wrong chain id;
- unknown decoder identity;
- missing/invalid ABI or source identity;
- failed/missing deployment receipt;
- deployment contract mismatch;
- missing required log/block/receipt fields;
- removed logs;
- wrong event topic or shape;
- malformed bytes32 payload;
- conflicting duplicate log identity;
- receipt/log block mismatch;
- core reconstruction failure.

The decoder accepts only the exact checked reference event. Unknown ABI/event shapes are not guessed into trusted application state.

## Local fork control

Anvil exposes deterministic state snapshot/revert methods. CI therefore:

1. deploys the reference contract;
2. snapshots the local chain;
3. emits `owner=alice` and `route=legacy` and captures adapter evidence;
4. reverts to the post-deployment snapshot;
5. emits `owner=alice`, `route=replacement`, `settled=yes` and captures evidence again;
6. proves that the replacement canonical snapshot has a different tip identity and the expected replacement material state.

This is evidence that V0.3 correctly adapts the current replacement snapshot. It is **not** a claim that V0.3 implements continuous live-tail reorg detection. That belongs to V0.4, where overlapping backfill/live observations and resumable checkpoints will have their own authority and failure contracts.

## Reproducibility and evidence

The CI gate records:

- exact Foundry/Anvil client identity in the run;
- local chain id;
- deployment transaction, block and contract identity;
- ABI/source digests;
- all adapted log and receipt source identities;
- event and observed-block counts;
- deterministic core state/report SHA-256;
- V0.3 adapter report SHA-256;
- before-revert, replacement and deterministic replay reports;
- wrong-chain and wrong-decoder negative-control reports.

## Claim boundary

V0.3 proves only bounded adaptation of the checked reference event from a declared **local** EVM JSON-RPC snapshot into ChainEvidence's existing deterministic core.

It does not establish public RPC correctness, Ethereum consensus/fork choice, universal finality, production durability, smart-contract audit status, hostile-provider safety, continuous live-tail reorg handling, multi-chain compatibility or production readiness.
