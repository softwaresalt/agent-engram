---
title: "PR 363 OTLP task-width remediation memory"
type: session-memory
doc_type: memory
source: "operator-requested remediation of reviews 5014749042 and 5014783124"
date: 2026-08-24
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
pull_request: 363
starting_head: 543e378be9bc7a2541889b2f011dd2c69b7ca154
planning_commit: 57e88065322d14c82aba7ad0542672a785196cfd
---

# PR 363 OTLP task-width remediation memory

## Outcome

Planning remediation is recorded in commit `57e88065322d14c82aba7ad0542672a785196cfd`. Stage retrieved the exact current review bodies and both unresolved threads before editing. Suppressed review 5014783124 identified the over-wide `131.002-T`; thread `discussion_r3849425228` required the RED title to name lifecycle scope; thread `discussion_r3849392979` identified contradictory compact-context totals. No source, test, config, build, linter, shipment claim, shipment close, PR merge, amend, force push, or blocked security-shipment mutation occurred.

## Decisions

- Keep one complete RED harness in `131.001-T` before every dependency or production change.
- Use seven tasks because the mandatory fewer-than-3-files rule requires Cargo manifest/lock alignment to remain separate from Rust provider construction.
- Exact chain: `131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T`.
- Cap every task at 105 minutes, 2 files, 4 functions, 3 scenarios, one skill domain, and one atomic milestone.
- PR #362 ordering is satisfied by merged commit `685f62668ac273a41a1f93fc9be2571510decae2`; `125-S` remains queued and unclaimed behind the exact-head PR #363 review/integration guard.
- Keep blocked security/durability features and shipments unchanged.

## Backlog changes

- Updated: `131-F`, `131.001-T`, `131.002-T`, `131.003-T`, and `125-S`.
- Created under parent `131-F`: `131.004-T`, `131.005-T`, `131.006-T`, `131.007-T`.
- Exact 125-S roster: `131-F`, then `131.001-T` through `131.007-T` in dependency order.
- Archived stash provenance remains exactly one `44E573BC` record with `reason: harvested` and `harvested_artifact_id: 131-F`, and zero active records.

## Planning gates

The OTLP plan was re-hardened for contract, external-export, owner-lifetime, and shutdown risk. Standard persona review was rerun for constitution, Rust/API, architecture, scope, tests, operations, learnings, and the external-boundary security lens. The gate is PASS with no unresolved P0/P1 finding; endpoint plumbing is a bounded P2 stop condition. Intercom and cross-model persona tooling were unavailable and were disclosed.

## Compact-context correction

The sole authoritative baseline is terminal HEAD `543e378be9bc7a2541889b2f011dd2c69b7ca154`. Definition: count tracked `*.md` blobs under exactly `docs/memory`, `docs/exec-plans`, and `docs/closure`, then sum the blob-size column from `git ls-tree -r -l 543e378be9bc7a2541889b2f011dd2c69b7ca154 -- <scope>`. This immutable, line-ending-independent source yields memory 148 files / 435,868 bytes; plans 71 / 1,143,652; closure 112 / 826,421. The two referenced reports now use this same source and supersede older working-tree totals. Files compacted: 0.

## Files modified

Backlog artifacts under `.backlogit/queue/` for 125-S and 131-F/children; the OTLP decision and reviewed plan; compact-context and current PR memories; this session memory. No source tree file was modified.

## Validation

- Backlog sync: 1,120 artifacts, zero parse failures.
- Target doctor: `125-S`, `131-F`, and all seven child tasks pass schema validation.
- Full doctor: exit 0; only 43 historical `archived_from_self_ref` and 38 historical `missing_shipped_event` advisories; no duplicate, orphan, partial mutation, or workspace-root conflict.
- Hierarchy/dependencies: exactly seven queued children under `131-F`; the linear chain from `131.001-T` through `131.007-T` matches every direct edge.
- Widths: estimates 45 to 105 minutes; maxima 2 files, 4 functions, and 3 scenarios; every task declares one skill domain and atomic milestone.
- Shipment state: `125-S` is the sole queued shipment, no shipment is active, and blocked `126-S` through `129-S` are unchanged.
- Stash provenance: zero active and exactly one archived `44E573BC` record with `reason: harvested` and `harvested_artifact_id: 131-F`.
- Documentation: 17 changed Markdown artifacts have valid frontmatter delimiters, titles, final newlines, balanced fences, no unresolved templates, and 19 existing frontmatter references.
- Compact totals: exact Git-tree recount at `543e378be9bc7a2541889b2f011dd2c69b7ca154` matches both referenced reports.
- Ordering: PR #362 merge commit is the current `origin/main` head and passes the ancestor check.
- Scope/diff: all 17 changed paths are OTLP backlog or planning/memory artifacts; no source, test, manifest, lockfile, config, blocked security shipment, or unrelated path changed; `git diff --check` passes.
- No build, test suite, or linter was run because Stage is planning-only.

## Next steps

Commit normally with PR/shipment/feature/review/reference trailers; push the same branch; reply to both unresolved threads with commit evidence and resolve only if fully addressed; add a PR comment recording suppressed finding remediation. Ship must not claim 125-S until the remaining exact-head guards pass.
