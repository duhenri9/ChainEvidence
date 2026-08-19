# Contributing

Contributions are welcome when they strengthen a bounded, reproducible engineering claim.

## Before opening a PR

Run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

For changes to canonicality, reorg or evidence semantics, add or update a deterministic fixture and include at least one failure/negative control where the mechanism could otherwise appear correct on the happy path.

## Design rules

- Do not describe fixture behaviour as Ethereum consensus/finality behaviour.
- Do not add network, wallet or signing authority implicitly.
- Preserve explicit `INDETERMINATE` behaviour when required ancestry/evidence is unavailable.
- Keep cryptographic lineage fields intact when adding event adapters.
- Add performance claims only with a documented benchmark methodology and reproducible evidence.
- Keep private/commercial WM3 implementations and data out of this repository.

Large architectural changes should start with an issue or ADR-sized design note before implementation.
