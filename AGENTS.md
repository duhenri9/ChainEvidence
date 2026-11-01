# ChainEvidence — Public contributor audit and delivery contract

**Project-specific authority:** Reorg-aware EVM indexing, canonical chain recovery and on-chain lineage. Preserve finality/reorg invariants, transactional rollback and distinction between deterministic Anvil fixtures and live RPC coverage.

This public-safe contract is an execution/acceptance rule, not a copy of a private WM3 playbook and not a substitute for the project's license, security model, domain invariants or existing tests.

## A0 / A1 / A2 / A3

- **A0 — before changes:** define approved objective, affected identity/invariant, base SHA, scope/out-of-scope, risk, evidence needed and a counterexample to success.
- **A1 — before review:** compare the actual diff and test plan to the stated objective. Execute appropriate positive, negative and recovery tests. Keep synthetic proof distinct from Production data; record UNKNOWN when coverage is absent.
- **A2 — before COMPLETE:** submit exact official artifact/commit, source and test receipts, current unresolved material risks, responsible owner and follow-up. A successful build or fixture alone cannot certify production reliability. A contributor cannot self-certify an unsupported customer/compliance claim.
- **A3 — where there is a real operated service:** monitor correctness, recovery and evidence quality, rather than inferring operational success from synthetic tests.

Read-only audits may deliver traceable Markdown without creating code or costly external effects. Implementation retains established CI and independent review requirements. An official PR/issue is usually sufficient for the compact receipt: task, approved outcome, SHA, artifact, PASS/FAIL/UNKNOWN/N/A_WITH_REASON, scope, limitations and decision.

**Status:** Documentation-only contributor instruction. No claim of branch-protection enforcement, CI improvement, product production readiness or new release testing is made by this file.
