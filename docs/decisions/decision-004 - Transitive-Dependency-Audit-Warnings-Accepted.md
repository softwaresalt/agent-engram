---
id: decision-004
title: 'ADR-004: Transitive Dependency Audit Warnings Accepted'
date: '2026-02-13'
status: Accepted
source: docs/adrs/0004-transitive-audit-warnings-accepted.md
---
## Context

- **Phase**: 8 (Polish & Cross-Cutting Concerns), T105

During Phase 8 final hardening, `cargo audit` reported 5 warnings in
transitive dependencies:

| Crate | Advisory | Severity | Source |
|-------|----------|----------|--------|
| `atomic-polyfill` 1.0.3 | RUSTSEC-2023-0089 | Unmaintained | SurrealDB -> rstar -> heapless |
| `bincode` 1.3.3 | RUSTSEC-2025-0141 | Unmaintained | SurrealDB |
| `number_prefix` 0.4.0 | RUSTSEC-2025-0119 | Unmaintained | fastembed -> indicatif -> hf-hub |
| `paste` 1.0.15 | RUSTSEC-2024-0436 | Unmaintained | fastembed -> tokenizers |
| `lru` 0.12.5 | RUSTSEC-2026-0002 | Unsound | SurrealDB -> surrealkv |

None are direct dependencies of t-mem. All originate from SurrealDB 2.x
or fastembed 3.x dependency trees.

## Decision

Accept these transitive warnings for the v0 release. No direct code in
t-mem uses unsafe patterns from these crates. The `lru` unsound advisory
(Stacked Borrows violation in `IterMut`) is mitigated by t-mem's
`#![forbid(unsafe_code)]` policy and the fact that `surrealkv` manages
its own LRU cache internally.

## Consequences

- **Positive**: Unblocks v0 release without dependency forking.
- **Negative**: Inherited risk from upstream unmaintained crates.
- **Mitigation**: Monitor SurrealDB and fastembed releases for updates
  that resolve these advisories. Re-run `cargo audit` before any
  production deployment.

## Alternatives Considered

1. **Pin or patch transitive deps** -- Rejected; would require forking
   SurrealDB or fastembed, introducing maintenance burden.
2. **Wait for upstream fixes** -- Rejected; blocks release indefinitely
   for non-critical warnings.
