---
title: "Compound refresh — 098-S / 099-F post-merge closure"
type: compound-refresh
date: 2026-07-30
scope: recent
context: "098-S (feature 099-F: Post-096-F Python canonical resolution hardening & parity) merged to main as merge-commit 4da43f9d; folded closure ride-along in the 097-S PR"
mode: apply
result: keep-all (no new/updated/stale entries required)
---

## Scope

Reconcile `docs/compound/` against the 099-F work stream (7 review follow-ups,
Python canonical resolution hardening & parity) that shipped in 098-S and merged
as `4da43f9d`. Candidate learnings flagged for potential capture:

1. Shipment-ID cross-branch collision → serialize-assembly discipline.
2. Copilot merge-gate "wait for a review at HEAD after every push".

## Evidence gathered

- `docs/compound/` full library review (subdirectory-organized).
- Merged 099-F work + the cross-branch backlog-JSON union that had to be
  reconciled during the 105-F feature-branch rebase onto `4da43f9d`.

## Classifications

| Entry | Classification | Evidence |
|---|---|---|
| `workflow-issues/ship-single-pr-serialization-and-stash-handoff-2026-05-14.md` | **keep** | Already codifies "one open Ship PR at a time; do not advance a second shipment until the prior reaches merged-and-closed; stash-first branch handoff." The shipment-ID cross-branch collision is a downstream symptom of running two assemblies concurrently — already covered. No drift. |
| `copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` | **keep** | Already codifies the load-bearing 4-point gate (Copilot review `commit_id == HEAD`, Copilot removed from `requested_reviewers`, 0 unresolved threads, `mergeable_state == clean`), re-checked after every push. Cross-referenced into `.github/instructions/github-pr-automation.instructions.md`. Accurate. |
| `gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` | **keep** | Companion `--paginate` caveat for the HEAD-review check. Still accurate. |
| `backlogit-sync-cache-union-landmine-2026-07-02.md` | **keep** | Actively load-bearing this session: after file-level archive of 098-S/099-F, a plain `backlogit sync` would have unioned stale queue/active rows back into Markdown. Applied the documented empty-cache rebuild (stop stale `backlogit mcp` PIDs → delete gitignored `backlogit.db*` → `sync`). Entry remains correct and valuable. |
| `packed-atomic-clear-requires-atomic-publish-2026-07-29.md` | **keep** | Anchor for the incoming 105.001-T (R1) generation-scoped pending-sync clear; unchanged by 099-F. |
| `atomicbool-drain-race-take-before-lock-2026-05-09.md`, `pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md` | **keep** | Anchors for incoming 105.002-T (R2); unchanged by 099-F. |
| `certify-completeness-reconcile-fileset-and-sweep-orphans-2026-07-29.md` | **keep** | Anchor for incoming 105.003-T (R3); unchanged by 099-F. |

## Outcome

**keep-all.** Both candidate learnings from the 099-F cycle are already durably
captured and accurate; no entry is stale, duplicated, or contradicted by the
merged work. No new compound entry fabricated (evidence-backed maintenance over
cosmetic addition). No files updated, consolidated, replaced, or archived.

## Follow-up

None. The R1/R2/R3 technical anchors are pre-positioned for the 105-F build in
the same PR.
