---
title: "110-S compacted closure context"
doc_type: closure
source: docs/closure/110-S-2026-08-08-post-merge-closure.md
date: 2026-08-08
shipment_id: "110-S"
status: done
---

Shipment `110-S` released feature `116-F` and tasks `116.001-T` through
`116.005-T` in PR #327. Approved HEAD
`fb0fc89b0e5a3e1d28c7b8f3c0b2f1a9cc435319` merged as two-parent commit
`b7c9fa1b5ba4fc2d3dca36e2069b6ef669969793`.

The released safety floor rejects out-of-workspace traversal roots, separates
physical `Unknown` from `Absent`, migrates PBIP/backlog to checked
reconciliation, carries one snapshot through index and sweep, and requires
notebook, backlog, PBIP, and Power BI materialization completeness before that
snapshot can authorize alias-stale deletion. Power BI bytes and parse results
are materialized once before dirty-scope deletion.

Three structured review cycles ended with no P0/P1. Copilot reviewed the exact
approved HEAD; its two valid findings were fixed, replied to, and resolved.
Formatting, pedantic Clippy, all-target tests, hosted CI, and the dependency
audit gate passed. The audit used the upstream-blocked
`RUSTSEC-2026-0041` command-line ignore without adding repository policy.

Post-merge focused runtime verification passed 59/59 scenarios. Incomplete or
unavailable controls removed zero rows; physical deletion and authoritative
alias supersession each removed one expected path. Windows directory symlinks
were supported, so cross-platform alias assertions executed.

Backlogit archived exactly shipment `110-S`, feature `116-F`, its five tasks,
and their plan review with merge evidence. Shipments `111-S` through `114-S`
remain queued. The seven-day removed-count and fail-closed-warning observation
window ends 2026-08-15.
