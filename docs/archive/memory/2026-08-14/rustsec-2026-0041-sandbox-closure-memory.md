---
title: "RustSec 2026-0041 sandbox closure"
type: session-memory
date: 2026-08-14
status: complete
---

# RustSec 2026-0041 sandbox closure

## Completed

* Rechecked U1 process, handle, disk, candidate, and containment gates
* Ran the explicitly approved `lz4_flex 0.11.6` candidate only under `tmp/rustsec-2026-0041/`
* Completed U2 compile, test, clippy, format, and audit validation
* Completed U3 synthetic Cozo verification with 53 focused tests passing
* Recorded the direct Cargo patch failure and manifest-only `swapvec 0.3.0` sandbox bridge
* Updated findings and backlog statuses/comments
* Committed and pushed `0c2d2bcde34b65e7661584c157f74dee3dd3df51`

## Decision

Recommend `pivot`, not an automatic production dependency change. A production fix needs a reviewed maintained compatibility bridge or an upstream `swapvec` release that widens its `lz4_flex` requirement. Production manifests, lockfile, source, and live data remain unchanged.

## Validation

* `cargo dev-test`: 599 passed
* `cargo test --all-targets`: passed
* Default-feature pedantic clippy, format check, and sandbox `cargo audit`: passed; audit found 0 vulnerabilities
* Focused Cozo U3 tests: 53 passed, 0 failed
* All-features clippy remains blocked by unrelated pre-existing OpenTelemetry API incompatibilities

## Remaining work

* Sandbox artifacts under `tmp/rustsec-2026-0041/` are intentionally retained
* Exact destructive cleanup approval is still required before deletion
* A separate production remediation plan must select and review the compatibility approach
* The all-features OpenTelemetry issue is unrelated and remains a separate task
