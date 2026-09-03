---
title: "133-S read-server foundations — compacted session memory"
date: 2026-09-03
type: session-memory-compacted
doc_type: memory
agent: ship
shipment: 133-S
feature: 142-F
status: done
compacted_from:
  - "docs/archive/memory/2026-09-03-ship-pr-372-stage-133-s-merge-closure.md"
  - "docs/archive/memory/2026-09-03-ship-133-s-mid-session-checkpoint.md"
  - "docs/archive/memory/2026-09-03-ship-133-s-pr-ready-checkpoint.md"
  - "docs/archive/memory/2026-09-03-ship-133-s-post-merge-closure-blocked.md"
---

## Outcome

`133-S` (read-server foundations: F00 test-manifest registration, F01
storage feasibility spike, F02 strict `DaemonMode` parsing, F03 immutable
`AppState.mode`, F12a `engram-indexer` stub crate) is **complete and
closed**. Shipment record `archived`, `archived_status: done`. Covering
feature `142-F` remains `active` (multi-shipment feature; 54 of its 59
direct children still owned by nine other shipments — see the closure
doc's Reconciliation section for the full 54 direct + 23 nested = 77
descendant breakdown). No user-facing runtime behavior changed.

## Timeline (across 4 sessions, all same shipment)

1. **PR #372** (staging gate, `chore/stage-133-s`) merged as merge commit
   `<see PR history>`; opened the Orchestrator staging gate for `133-S`.
   Shipment not claimed in that session.
2. **Build session**: claimed `133-S`, implemented all 10 manifest items
   (F00 49-target registration, F02 `DaemonMode` parser, F12a
   `engram-indexer` stub, F03 `AppState.mode`, F01 storage spike with GO
   verdict and accepted Windows durability residual risk). All quality
   gates green (`clippy -D warnings -D clippy::pedantic`, `fmt --check`,
   `cargo dev-test` full suite). PR #376 opened, brought to
   `READY_WITH_FOLLOWUPS` local review + `SATISFIED` P-018 Copilot gate.
3. **Merge + closure session (round 1)**: PR #376 merged as merge commit
   `33a0a41e345cef8965b707346728d44fa5492daf` (operator approval scoped
   to that PR only). Post-merge closure produced runtime-verification
   (PASS WITH FOLLOW-UP) and operational-closure (`closure_status:
   BLOCKED`) docs, `docs/architecture.md` updates, and PR #377. Shipment
   archival was **blocked**: `backlogit move 133-S --status shipped` is
   unconditionally rejected by backlogit 1.10.1, and the only remaining
   path (`backlogit shipment ship`) would cascade-force-requeue/detach 77
   of `142-F`'s 87 descendants outside `133-S`'s 10-item manifest —
   verified against backlogit's own Go source
   (`internal/core/shipment_lifecycle.go`): `featureScopeRoots` discovers
   a covering feature independent of explicit manifest membership, and
   `returnUnreleasedFeatureItems` runs unconditionally for every
   discovered feature. This is a **workspace-wide risk**: exactly ten
   shipments (`133-S` through `142-S`) jointly and exhaustively cover
   `142-F`'s 59 direct children with no overlap/gap; all ten require
   manual safe-close, never cascade `shipment ship`, until `142-F` is
   fully covered by whichever ships last or backlogit changes this
   behavior. Recorded as stash `28C0E138` (pre-existing, from PR #372
   review) / `F9D1C495` (session-duplicate, flagged for Stage triage,
   *not* resolved by Ship — a first attempt to reconcile the duplicate
   was reverted as a P-010 role-boundary violation) / `F9767C12`
   (cascade-mechanism correction). PR #377 brought to full readiness,
   not merged pending operator approval.
4. **Merge + manual-closure session (round 2, this compaction)**: operator
   approved (narrowly, non-blanket) (a) merging PR #377 after an
   exact-HEAD gate recheck, and (b) a manual safe-close of `133-S`.
   PR #377 merged as merge commit
   `224539ff4da60e477f4a93bff729cc42401ec4f8` (local main fast-forwarded).
   Manual closure performed via official `backlogit` CLI seams only
   (`update`, `comment add`, `archive`, `sync` — no `shipment ship`):
   attached commit `33a0a41e...` to all 10
   already-archived manifest items (they had been moved to
   `.backlogit/archive/` as a raw `git mv` inside an earlier feature
   commit, predating official-CLI archival — a precondition difference
   recorded and worked through rather than blocking); recorded a detailed
   audit-rationale comment on `133-S` citing PR #376/#377 and stash
   `F9767C12`; transitioned `133-S` `active -> done` then archived it
   (`archived_status: done`); verified `142-F` unchanged (`active`, 59
   children, zero orphans) and all 77 remaining `142-F` descendants across
   `134-S`..`142-S` unchanged (`queued`, attached); resynced the backlogit
   index. Updated `docs/closure/133-S-2026-09-03-post-merge-closure.md`
   in place (`closure_status: BLOCKED -> READY`; `releasability:
   READY_WITH_CONDITIONS`) on a new
   `post-merge/133-s-manual-shipment-archival` branch/PR, not merged
   pending separate operator approval. Re-ran the `pipeline-topology`
   pre_claim gate for `134-S`: still blocked
   (`PREDECESSOR_CLOSURE_INCOMPLETE`) until this closure-doc update lands
   on `main`.

## Durable follow-ups (Stage-owned, unresolved)

* `28C0E138` / `F9D1C495` — duplicate stash pair describing the `142-F`
  manifest/cascade defect; needs Stage's unconditional duplicate-detection
  triage (Ship is not authorized to archive/edit either entry).
* `F9767C12` — corrected cascade-mechanism finding; supersedes the
  "remove `142-F` from the manifest" remediation in the two entries above
  as insufficient on its own.
* `F2E84E15` — accepted Windows generation-publish durability residual
  risk; F07/F08 implementers must re-review before treating Windows
  publication as crash-durable equivalent to POSIX.
* `B761AFA7` — the ten already-archived `133-S` task records lack
  canonical `archived_status`/`archived_from` wrapper fields; normalizing
  them was out of scope for PR #378's narrow manual-closure sequence per
  P-021 C1.
* `58B33C45`, `7B270F79`, `A7C0BA5F`, `5A7FBC37` — pre-existing/out-of-scope
  items captured per P-021, not fixed in this shipment.
* **Process precedent for `134-S` through `142-S`**: each must use the
  same manual safe-close path (never `backlogit shipment ship`) until
  `142-F` reaches full coverage or backlogit's cascade behavior changes.

## Key artifacts (not compacted, still authoritative)

* `docs/closure/133-S-2026-09-03-post-merge-closure.md` — full evidence
  chain, cascade-mechanism analysis, releasability evidence
* `docs/closure/133-S-2026-09-03-runtime-verification.md` — validator
  evidence
* `docs/architecture.md` — `engram-indexer` stub + `DaemonMode`/`mode`
  plumbing documentation
