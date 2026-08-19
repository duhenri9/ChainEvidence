# Security policy

ChainEvidence is an experimental open-source engineering project and is **not production-ready**.

## Reporting

Please report potential security issues privately to the repository owner rather than publishing exploit details in a public issue before triage.

## V0 security boundary

V0 consumes synthetic/local JSON fixtures only. It has no wallet keys, signing path, network RPC client, contract deployment authority, database credentials or production settlement integration.

Security-sensitive additions such as RPC adapters, persistence, contract integrations or signing/account abstractions require their own threat model and executable failure controls before release.

## Claims

A green V0 CI run is evidence for the bounded canonical/reorg fixture contract only. It is not a security audit or certification of blockchain correctness.
