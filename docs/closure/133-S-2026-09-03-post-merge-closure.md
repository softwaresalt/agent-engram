---
title: "133-S post-merge operational closure"
doc_type: closure
shipment_id: "133-S"
feature_id: "142-F"
mode: post-merge
date: 2026-09-03
author: ship
verdict: "READY — shipment record manually safe-closed (archived_status=done); Windows durability residual risk remains an accepted, documented releasability condition"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
pr_number: 376
closure_pr_number: 377
manual_closure_pr_number: 378
merge_commit: "33a0a41e345cef8965b707346728d44fa5492daf"
closure_pr_merge_commit: "224539ff4da60e477f4a93bff729cc42401ec4f8"
head_commit_merged: "2005b3db94752dbe37946a98532c46dde1aad674"
runtime_verification_report: "docs/closure/133-S-2026-09-03-runtime-verification.md"
follow_up_stash:
  - "A7C0BA5F"
  - "5A7FBC37"
  - "58B33C45"
  - "7B270F79"
  - "F2E84E15"
  - "28C0E138"
  - "F9D1C495"
  - "F9767C12"
  - "B761AFA7"
blocking_stash: null
duplicate_stash_flagged_for_stage_triage:
  - "F9D1C495"
  - "28C0E138"
cascade_mechanism_correction_stash: "F9767C12"
shipment_record_status: "archived (archived_status=done)"
---

# 133-S post-merge operational closure

## Summary

`133-S` (feature `142-F`) delivered read-server foundations: F00 (49
placeholder test-manifest registrations), F01 (storage feasibility spike,
GO verdict, accepted Windows durability residual risk), F02 (strict
`DaemonMode` mode-contract parser), F03 (immutable `mode` field on
`AppState`), and F12a (`crates/engram-indexer` empty stub crate + workspace
membership). No user-facing runtime behavior changed with this shipment.

PR #376 merged as a merge commit (repo policy: merge-only, squash/rebase
disabled) with explicit operator approval recorded in-session ("Keep
working autonomously until the task is truly finished" — treated as a
one-time approval scoped to PR #376 only, per the operator's own
instruction, not a blanket future-PR authorization).

**UPDATE (2026-09-03, continuation session): this closure is no longer
BLOCKED.** The shipment-record-archival step was completed via an
explicit, operator-approved manual safe-close (not `backlogit shipment
ship`) after PR #377 (this closure PR) merged. See "Manual Closure
Completion" below for the full evidence chain. The original blocking
analysis is preserved unedited below for audit continuity — it remains
accurate as a description of why the *cascade* path (`backlogit shipment
ship`) is unsafe, which is still true and still governs how `134-S`
through `142-S` must each be closed in turn.

**Known duplicate stash entries requiring Stage triage** (see "Stash
duplicate — flagged, not resolved" below): this session inadvertently
created a second entry, `F9D1C495`, describing the same defect as the
pre-existing `28C0E138`; both remain active and unedited pending Stage's
own duplicate-detection/harvest disposition, per Ship's role boundary.
This remains an open Stage follow-up and does not block this closure.

## Manual Closure Completion (2026-09-03, continuation session)

After PR #377 merged (merge commit
`224539ff4da60e477f4a93bff729cc42401ec4f8`, confirmed ancestor of
`origin/main`), the operator granted narrow, explicit approval for exactly
two actions: (1) merging PR #377 after an exact-HEAD gate recheck, and (2)
the manual closure sequence below for `133-S` — not a blanket approval for
future PRs or other destructive actions.

Sequence performed (official `backlogit` CLI seams only — `update`,
`comment add`, `archive`, and `sync`; no direct file
edits, no `backlogit shipment ship`):

1. **Precondition verification**: `133-S` manifest's 10 task-level items
   (`142.001-T`, `142.001.001-ST`..`142.001.005-ST`, `142.002-T`,
   `142.004-T`, `142.006-T`, `142.007-T`) were already physically present
   in `.backlogit/archive/` at `status: done` with correct `parent_id`
   chains — moved there as a raw `git mv` inside feature-implementation
   commit `3f890662` (an ancestor of PR #376's merge commit
   `33a0a41e...`), predating the official-CLI-archival assumption in the
   original closure plan. This precondition difference was recorded before
   any further mutation; no queue-side copies existed, no double-archive
   risk was present, and the difference did not block completion of the
   requested outcome (commit attribution + shipment closure), so work
   proceeded rather than halting.
2. **Commit attribution**: `backlogit update <id> --commit
   33a0a41e345cef8965b707346728d44fa5492daf` was run against each of the
   10 already-archived task items (official update seam; works on archived
   records). Diff-verified: exactly one `commit:` line added and
   `updated_at` bumped per file, no other field touched. **Correction**
   (Copilot review, this PR): the commit line was the only field this PR
   added — but it was not the only metadata these ten records lack
   relative to this workspace's canonical archive convention. All ten
   remain `status: done` without the `archived_status`/`archived_from`
   wrapper fields that the official `backlogit archive` command stamps
   elsewhere (e.g. the `133-S.md` shipment record itself, and 255 of 741
   other task-type archive files in this workspace); their filesystem
   location under `.backlogit/archive/` is the only signal of archival,
   not their frontmatter. This predates this PR — they were relocated by a
   raw `git mv` inside earlier feature-implementation commit `3f890662`,
   not via the official archive command — and normalizing them is a
   separate, materially larger mutation outside this PR's narrowly
   operator-approved manual-closure sequence (P-021 C1). Deferred as
   stash `B761AFA7` for Stage's disposition; not fixed here.
3. **Audit rationale**: `backlogit comment add 133-S --actor ship
   --commit-sha 224539ff4da60e477f4a93bff729cc42401ec4f8` recorded a
   detailed rationale explaining why `backlogit shipment ship` /
   `ShipShipment` is unsafe for `133-S`, citing PR #376, PR #377, and stash
   `F9767C12`. (Note: `.backlogit/logs/` is git-ignored in this workspace,
   so this comment is durable in the local index/log but not in git
   history; this document is the git-durable copy of that rationale.)
4. **Shipment status transition**: `backlogit update 133-S --status done`
   (live-verified `status: done` before archival), then `backlogit archive
   133-S` (live-verified `status: archived`, `archived_status: done`, no
   longer present in `.backlogit/queue/`).
5. **Postconditions verified** (before and after `backlogit sync`):
   `142-F` remains `active` with its queue file present and all 59 direct
   children retaining `parent_id: 142-F` (zero orphans, verified by
   enumerating every child and re-reading its `parent_id`); all 77
   remaining `142-F` descendants across the other nine covering shipments
   (`134-S` 12, `135-S` 4, `136-S` 9, `137-S` 6, `138-S` 14, `139-S` 6,
   `140-S` 7, `141-S` 7, `142-S` 12 — task-item counts excluding the
   covering feature itself, summing to 77) remain `queued` and attached to
   their manifests, unchanged by this closure.
6. **Index resync**: `backlogit sync` completed successfully
   (`Indexed 1292 artifacts`).
7. **Topology gate for `134-S`**: `autoharness gate pipeline-topology
   --mode agent --shipment 134-S --phase pre_claim --json` was re-run
   after closure. Result: still blocked, token
   `PREDECESSOR_CLOSURE_INCOMPLETE`, because the gate's `closure_complete`
   check additionally requires this closure document's own
   `closure_status` frontmatter to read `READY` (or `READY_WITH_CONDITIONS`
   with a satisfied machine-readable `conditions:` block) — which this
   same edit now sets (`closure_status: READY`, no `conditions:` block
   needed). `134-S` claim eligibility is therefore expected to pass once
   this document (on this branch) reaches `main`; re-verification of the
   gate after merge is a remaining follow-up, not performed by this
   session (no shipment claim was made or attempted for `134-S`).

No mutation touched `142-F`, any of its 59 direct children beyond the 10
manifest items receiving commit attribution, or any of the other nine
covering shipments' manifests. Rollback material (pre-mutation copies of
`133-S.md` and `142-F.md`) was captured to a local temp path before any
mutation and is available if reversal is ever required; git history on
this branch (each step as a discrete, revertable commit) is the primary
rollback mechanism.

## Merge / PR Evidence

| Gate | Result |
|---|---|
| PR #376 state | `MERGED` at `2026-09-03T17:54:11Z` |
| Merge commit | `33a0a41e345cef8965b707346728d44fa5492daf` |
| Reviewed/merged HEAD | `2005b3db94752dbe37946a98532c46dde1aad674` (unchanged from the operator's specified HEAD through merge) |
| Ancestry | `git fetch origin main` then `git merge-base --is-ancestor 33a0a41e... origin/main` → exit `0` |
| Merge strategy | `--merge` (merge commit); squash/rebase disabled repo-wide (`allow_merge_commit: true`, `allow_squash_merge: false`, `allow_rebase_merge: false`) |
| Local review readiness at HEAD | `READY_WITH_FOLLOWUPS`, P0=0, P1=0 |
| P-018 Copilot review gate | `autoharness gate copilot-review 376 --enforcement auto --max-wait 0 --json` → `SATISFIED` |
| Unresolved review threads at merge | 0 of 5 (`isResolved: true` on all, verified via GraphQL) |
| Last Copilot review vs HEAD | `commit_id` == HEAD; state `COMMENTED` (not `CHANGES_REQUESTED`); 0 new comments, only previously-suppressed items cited |
| `mergeStateStatus` / `mergeable` (pre-merge) | `CLEAN` / `MERGEABLE` |
| CI checks | `build`: SUCCESS, `start-launcher-windows`: SUCCESS |
| Pipeline-topology lifecycle gate (pre-merge, last-mile re-check) | passed both times |

## Runtime Verification

Verdict: **PASS WITH FOLLOW-UP** (full report:
[`133-S-2026-09-03-runtime-verification.md`](./133-S-2026-09-03-runtime-verification.md)).

### Validator Evidence (structured)

| Field | Value |
|---|---|
| Surface / adapter | CLI (release build, no-daemon commands); `cargo test` for MCP/CLI contract suites |
| Verdict | `PASS_WITH_FOLLOW_UP` |
| Probe outcomes | `cargo build --release`: ok (5m15s). `engram.exe --version`/`manifest`: ok. `unit_app_state_mode` (F03): 6/6 passed. `unit_plugin_config` (F02): 21/21 passed. `contract_mcp_catalog_oracle`: 9/9 passed. `contract_mcp_tool_catalog_parity` / `contract_mcp_envelope` / `contract_read_server_cli_mcp_parity` (F00 placeholders): 1/1 each. `contract_shim_stdio_initialize`: 18/19 passed |
| Manual checkpoint evidence | Isolated diagnostic worktree (`git worktree add`) checked out at pre-merge `main` tip `c66d320ee2ce8b0aab90e73bc07d4f81c3059862`, same test re-run, identical failure reproduced — confirms the one `contract_shim_stdio_initialize` failure (`shim_aborts_unresolved_startup_after_client_disconnects`) is pre-existing and unrelated to this shipment; worktree removed after comparison |
| Blocked prerequisites | `daemon-status`/`workspace-status`/`sync_workspace` probes require a bound running daemon session — not applicable to this foundations-only, no-daemon-behavior-change shipment; explicitly skipped, not silently omitted |

Release build succeeds; MCP tool catalog and existing MCP/CLI contract
suites unaffected. No new runtime behavior is introduced by this
shipment (F04's call-site migration and F06–F09/F12's real logic are
explicitly deferred to later shipments).

## Reconciliation

* **Pre-archive reconciliation**: manual verification (this backlogit
  version's `shipment-reconcile pre` mode assumptions about a direct
  `move --status shipped` path are now stale — see Blocking Finding) showed
  all 10 task-level manifest items (`142.001-T` + 5 subtasks, `142.002-T`,
  `142.004-T`, `142.006-T`, `142.007-T`) present in `.backlogit/archive/`
  with `status: done`. Covering feature `142-F` correctly remains in
  `.backlogit/queue/` at `status: active`. **Verified descendant count**
  (all files under the `142.*` ID namespace in `.backlogit/queue/` +
  `.backlogit/archive/`, corrected after Copilot review flagged the
  original draft's figure as inaccurate): `142-F` has **59 direct task
  children plus 28 nested subtask descendants (87 total)**. The manifest
  contains **5 of each** (5 direct tasks, 5 nested subtasks — 10 total),
  leaving **77 descendants** (54 direct tasks + 23 subtasks) outside the
  manifest — not fully covered, correctly untouched by the merged build
  work.
* Orphan scan (grep for `shipment_id: 133-S` across queue/archive):
  no matches — expected, this backlogit version does not store
  shipment back-references on task records.
* No `source_stash_id` or `source_deliberation_id` custom fields exist on
  `133-S` or `142-F` — no source-artifact cleanup required (see dedicated
  section below).

## Source artifact cleanup

- Archived stash (`source_stash_id`): none — `133-S` and `142-F` carry no
  `source_stash_id` custom field
- Archived deliberations (`source_deliberation_id`): none — `133-S` and
  `142-F` carry no `source_deliberation_id` custom field
- Skipped (already archived or not found): none — no candidate fields
  existed to act on

## Blocking Finding: Shipment Record Cannot Be Safely Archived (RESOLVED — see "Manual Closure Completion" above)

**Status: RESOLVED via manual safe-close, this session.** The analysis
below is preserved unedited as the audit record of *why* the cascade path
(`backlogit shipment ship`) is unsafe; it remains fully accurate and
continues to govern how `134-S` through `142-S` must each be closed.

**Evidence chain** (full detail; Ship independently reproduced every claim
below against the live `.backlogit/` state and the installed `backlogit`
binary in this session):

1. `backlogit move 133-S --status shipped` → **fails** (exit 9):
   `"shipment must be shipped via ShipShipment, not a direct status
   update"`. This error is produced by an unconditional guard
   (`ErrShipmentShippedRequiresEnvelope`, introduced by backlogit feature
   `144-F`, described in that repo's own code comments as intentionally
   unbypassable — "no legitimate caller... even an operator `--force`").
   `backlogit shipment ship --help` confirms no scope-limiting flag exists
   (`--author`, `--message`, `--sha` only).
2. The only remaining CLI path to `status: shipped` is
   `backlogit shipment ship 133-S`, which performs a **cascade**: for every
   ancestor feature reachable from the shipment's manifest items
   (`featureScopeRoots`), every non-manifest, non-terminal descendant is
   force-set to `status: queued` with `parent_id` cleared (detached), and
   if the feature itself is an explicit manifest member, it is
   **unconditionally** marked `done` regardless of actual completion.
3. `133-S`'s manifest lists `142-F` (the covering feature) as an explicit
   item, alongside only 5 of its 59 direct task children and 5 of its 28
   nested subtask descendants (10 of 87 total descendants — verified by
   direct enumeration of every file under the `142.*` ID namespace).
   Invoking `backlogit shipment ship 133-S` would therefore force-mark
   `142-F` `done` (77 of 87 descendants still incomplete) and
   force-requeue-and-detach those 77 non-manifest descendants from their
   parent chain.
4. This workspace's own P-015 policy permits the cascade close path
   **only** when every feature member of a shipment manifest is a root and
   fully covered (100% of its live children are manifest members) —
   `142-F` fails this decisively. **Invoking the cascade here would be a
   P-015 policy violation**, not merely an inconvenient side effect.
5. backlogit's own design-decision record
   (`docs/decisions/2026-07-31-shipshipment-partial-feature-archive-cascade-deliberation.md`
   in the `backlogit` repository) confirms this is the *intended* design:
   "the covering feature closes/archives iff it is an explicit manifest
   member" — meaning a multi-shipment covering feature is expected to be
   **omitted** from a partial shipment's `items` in the first place
   (relying on `parent_id` resolution, the same task-only-manifest pattern
   already used elsewhere in this workspace, e.g. `097-S`).

**Conclusion**: `133-S`'s manifest was assembled with `142-F` present as an
explicit item despite the shipment covering only a fraction of `142-F`'s
scope. This is a manifest-assembly correctness issue, not a Ship-side
execution defect, and correcting shipment planning fields (`custom_fields.items`)
is outside Ship's role boundary (Stage-only). This defect was first
identified during PR #372's review (stash `28C0E138`, created
2026-09-03T04:37:02Z, predates this session). This session's own evidence
chain independently confirmed and hardened the same defect with the exact
87/77 descendant counts and the `ErrShipmentShippedRequiresEnvelope` CLI
blocker, but — before discovering `28C0E138` already existed — captured it
as a second entry, `F9D1C495`. See "Stash duplicate — flagged for Stage
triage" below for the full disposition of that duplicate. **Recorded as
follow-up stash `28C0E138`** (priority: high, pre-existing) and
`F9D1C495` (priority: high, session-created, duplicate — left active for
Stage to triage) with the recommended remediation: Stage removes `142-F`
from `133-S`'s `custom_fields.items`, after which `backlogit shipment ship
133-S` becomes safe to invoke (or a corrected safe-close path becomes
available).

**Independent safety net**: the `pipeline-topology` gate's predecessor
check (`_is_shipped_terminal`) already fails closed on `133-S`'s current
`active` status ahead of even reading this closure artifact's
`closure_status` field — so `134-S` cannot be claimed regardless of this
document's content. `134-S` must not be claimed until `133-S` reaches a
genuinely shipped/archived terminal state.

## Stash duplicate — flagged for Stage triage, not resolved by Ship

During this session's first pass, the discovery-lookup step (searching
active + archived stash for an existing entry describing the same
expansion before capturing a new one) was performed insufficiently, and a
new entry `F9D1C495` was captured before `28C0E138` (the pre-existing
entry from PR #372's review, describing the identical `142-F`
manifest/cascade defect) was found. Copilot review correctly flagged this
as a duplicate.

**First remediation attempt (reverted — role-boundary violation)**: Ship's
initial fix archived `F9D1C495` via `backlogit stash archive` and edited
`28C0E138`'s text via `backlogit stash edit` to consolidate the hardened
evidence onto it. Copilot review correctly flagged this as a **P-010 role
boundary violation**: Ship's role boundary (`.github/agents/_ship.agent.md`
Backlog row) explicitly forbids "discretionary removal or archival of
stash entries" and "triage, prioritize/re-prioritize, re-classify, edit,
harvest, or deliberate on stash entries" outside two narrow exceptions
(capture-only creation for a follow-up/P-021-C2 step, and retiring the
specific `custom_fields.source_stash_id`-linked source entry at post-merge
Step 7) — neither of which covers archiving or editing an unrelated
duplicate. The discovery-failure protocol's own guidance is explicit that
"both fail-safe modes rely on Stage's unconditional duplicate detection
... to remediate any resulting duplicate" — Ship is not authorized to
perform that remediation itself.

**Correction applied**: both mutations were reverted. `F9D1C495` was
restored to `.backlogit/stash.jsonl` (active, unarchived) in its original
captured form; `28C0E138`'s text was restored to its original pre-session
content (the PR #372-era text, without this session's consolidated
evidence appended). **Current true state**: both `F9D1C495` and `28C0E138`
exist as active, unedited stash entries, both describing the same `142-F`
manifest/cascade defect. This duplicate is flagged here for **Stage's**
own unconditional duplicate-detection/harvest triage to resolve — Ship
takes no further stash-mutation action on either entry.

**Important correction to both entries' recommended remediation** (Copilot
review round 5, `F9767C12`): the "remove `142-F` from `133-S`'s manifest"
remediation recommended by both `28C0E138` and `F9D1C495` is **not
sufficient** to make `backlogit shipment ship 133-S` safe — see "Cascade
mechanism correction" below. Stage's triage of the duplicate pair must
account for this when deciding how to act on either entry.

## Cascade mechanism correction (Copilot review round 5, verified against backlogit source)

This session initially recommended (in both `28C0E138` and `F9D1C495`)
that removing `142-F` from `133-S`'s `custom_fields.items` would make
`backlogit shipment ship 133-S` safe to invoke. **This recommendation is
incorrect** and must not be followed as-is. Verified by reading
`backlogit`'s own source directly (`C:\Source\GitHub\backlogit`,
`internal/core/shipment_lifecycle.go`):

* `featureScopeRoots` (~line 1164) discovers a covering feature by walking
  **up** the `parent_id` chain from every item already in the shipment's
  release scope (its manifest) — this discovery is **independent of
  whether the feature itself is an explicit manifest member**.
* The `ShipShipment` call site (~lines 633–649) then **unconditionally**
  invokes `returnUnreleasedFeatureItems` (~line 734) for every feature so
  discovered. That function force-sets every descendant of the feature
  that is **not** in the release scope to `queued` and **clears its
  `parent_id`** ("returned to backlog after release") — regardless of
  whether the feature itself is an explicit manifest member.
* Only the **separate** decision to mark the feature itself `done`/archive
  it is gated on explicit membership (`explicitScopeSet`, same file
  ~line 641 and in `collectArchiveCandidateIDs` ~line 274).

**Consequence**: removing `142-F` from `133-S`'s manifest would stop
`142-F` itself from being wrongly force-marked `done`, but would **not**
stop the cascade — `backlogit shipment ship 133-S` would still
force-requeue-and-detach (clear `parent_id` on) all 77 of `142-F`'s
descendants outside `133-S`'s 10-item manifest, corrupting the hierarchy
for the other nine as-yet-unshipped shipments that also partially cover
`142-F`. **This is not unique to `133-S`**: any of the ten shipments
partially covering `142-F` would hit the identical cascade if it ever
calls `backlogit shipment ship` instead of a manual safe-close, regardless
of manifest edits.

**Corrected guidance for Stage**: do not treat "remove `142-F` from the
manifest" as sufficient to unblock `backlogit shipment ship` for `133-S`
or any sibling shipment; continue using the manual safe-close path
(individually archive manifest items, then transition the shipment's own
status directly) for every one of the ten `142-F`-covering shipments until
either `backlogit` changes `returnUnreleasedFeatureItems` to skip
non-explicit-member features, or `142-F` becomes fully covered by whichever
shipment ships last. Recorded as new follow-up stash `F9767C12`
(discovery-checked against active + archived stash for
`returnUnreleasedFeatureItems`/`featureScopeRoots`/`cascade` before
capture — no existing duplicate found).

**Correction note on the `F9767C12` stash entry text itself**: the
captured entry's text names four example sibling shipment IDs (`134-S`,
`137-S`, `139-S`, `141-S`) and cites a stash pair (`982B0B01`/`284285B5`)
as the source for "ten shipments partially covering `142-F`." Those two
specific stash IDs were not independently confirmed in this session
and per the single-write invariant for a captured stash entry (P-021
C2/C5: "Ship MUST NOT edit, amend, back-fill, re-classify, or
re-prioritize a captured entry afterwards"), `F9767C12` is **not** being
edited to correct that detail. However, the underlying "ten shipments"
figure itself **is** now independently verified in this closure
document (not merely asserted): a direct query of
`.backlogit/queue/*.md` cross-referencing `142-F`'s 59 direct children
(`parent_id: 142-F`) against every shipment's manifest confirms
**exactly ten** shipments jointly and exhaustively partition all 59
direct children, with **no overlap and no gap**:

| Shipment | Direct-child items covered | Count |
|---|---|---|
| `133-S` (this shipment) | `142.001-T`, `142.002-T`, `142.004-T`, `142.006-T`, `142.007-T` | 5 |
| `134-S` | `142.003-T`, `142.005-T`, `142.008-T`, `142.009-T`, `142.010-T` | 5 |
| `135-S` | `142.023-T`, `142.024-T`, `142.025-T`, `142.026-T` | 4 |
| `136-S` | `142.011-T`, `142.012-T`, `142.013-T`, `142.014-T`, `142.017-T` | 5 |
| `137-S` | `142.015-T`, `142.016-T`, `142.020-T`, `142.021-T`, `142.022-T`, `142.027-T` | 6 |
| `138-S` | `142.018-T`, `142.019-T`, `142.028-T`...`142.033-T` | 8 |
| `139-S` | `142.034-T`...`142.039-T` | 6 |
| `140-S` | `142.040-T`...`142.046-T` | 7 |
| `141-S` | `142.047-T`...`142.053-T` | 7 |
| `142-S` | `142.054-T`...`142.059-T` | 6 |
| **Total** | | **59** |

All ten are still `queue`-status shipment records except `133-S` (this
one, closure blocked). **Corrected, verified guidance for Stage**: the
manual safe-close path (not `backlogit shipment ship`) must be used for
`134-S` through `142-S` as well, in whatever order they complete, until
`142-F` is fully covered — the cascade risk is real and applies to all
nine remaining sibling shipments exactly as described above, now with an
exact, verified shipment list rather than an illustrative example. This
verified table supersedes the imprecise example-ID citation inside
`F9767C12`'s own (unedited) text; Stage should treat this closure
document's table as authoritative over the stash entry's illustrative
text for this specific detail.

## Risky Action Record

| Field | Value |
|---|---|
| `ProposedAction` | Invoke `backlogit shipment ship 133-S` (the only remaining CLI path to close the shipment record) |
| `ActionRisk` | **HIGH** — cascade would unconditionally force-mark covering feature `142-F` `done` while 77 of its 87 descendants remain incomplete, and would force-requeue-and-detach those 77 descendants from their parent chain — **verified to occur independent of whether `142-F` is an explicit manifest member** (see "Cascade mechanism correction" above); violates this workspace's own P-015 fully-covered-root policy |
| Approval / containment | **Not approved.** Ship assessed the action against P-015 during this session (evidence chain above), determined it would be a policy violation, and did not invoke it. No operator approval was sought for this specific mutation because Ship independently halted before executing it — consistent with Ship's role boundary (no authority to override P-015) and its Step 6 obligation to halt rather than force a cascade |
| `ActionResult` | **NOT EXECUTED / ABORTED.** No mutation was made to `142-F` or any of its descendants. `133-S` remains `active` (unchanged). Recorded as follow-up stash `28C0E138`/`F9D1C495` (duplicate pair, flagged for Stage triage) and `F9767C12` (corrected cascade-mechanism finding) instead |
| *(second risky action, self-detected and reverted)* | `ProposedAction`: archive `F9D1C495` + edit `28C0E138` to resolve the duplicate found above. `ActionRisk`: **role-boundary violation (P-010)** — discretionary stash archival/edit is Stage-only. `ActionResult`: **REVERTED** within this same session before merge; both entries restored to their pre-mutation state; see "Stash duplicate — flagged for Stage triage" above |

## Invariants to Preserve


* The strict `DaemonMode` parser (`DaemonMode::resolve`) must continue to
  hard-error (`DaemonModeParseError::Unrecognized`) on any value outside
  `managed`/`read_server` — no silent fallback to `managed` — **when it is
  eventually wired into the startup path**. Verified this session
  (Copilot review round 5 correction) that this resolver is **not yet
  wired into production config loading**: `PluginConfig` carries a
  permissive `mode: Option<String>` field, but no production call site
  currently routes it through `DaemonMode::resolve` (every current call
  site is a unit-test assertion in `unit_plugin_config`/
  `unit_app_state_mode`, both passing — 27/27 tests). An invalid
  configured mode therefore does not yet hard-fail daemon startup; F04 (or
  a dedicated later plan unit) must wire this before the invariant has any
  live runtime effect.
* Existing `AppState` construction call sites (`new`,
  `with_stale_strategy`, `with_options`) must continue to forward to
  `with_mode(DaemonMode::Managed, ...)` unchanged until F04 migrates them.
* The new `engram-indexer` crate must remain inert (no wired daemon
  participation) until its real supervisor logic ships under a later,
  reviewed shipment (F12).

## Pre-Deploy Audits

* No schema, migration, or config-flag changes ship with observable
  runtime effect — the new `DaemonMode`/`mode` field is additive and
  defaults to preserving current (`Managed`) behavior at every existing
  call site.
* No new external dependency, port, or credential surface introduced.
* Scope confirmed via merge-diff to be limited to the F00/F01/F02/F03/F12a
  files described in `docs/architecture.md`'s updated Module boundaries
  section, plus test/backlog/documentation artifacts.

## Deployment / Rollout Path

Merge-only. `engram` is distributed as a per-workspace binary/plugin; there
is no separate deploy or canary step beyond the merge landing on
`origin/main` and downstream consumers picking up the next build. No
maintenance window required.

## Post-Deploy Checks

* MCP tool catalog (`engram manifest`) continues to return the full,
  well-formed catalog on `main` (already verified this session at the
  merge commit).
* `contract_mcp_catalog_oracle` and related F00-placeholder contract tests
  continue to pass on CI.

## Healthy Signals

* Release build continues to succeed on `main`.
* No new failures introduced in the existing MCP/CLI/shim contract test
  suites beyond the confirmed pre-existing, unrelated
  `shim_aborts_unresolved_startup_after_client_disconnects` failure.

## Failure Signals (Rollback Trigger)

* A CI regression in `contract_mcp_catalog_oracle`,
  `contract_mcp_tool_catalog_parity`, `contract_mcp_envelope`, or
  `contract_read_server_cli_mcp_parity` attributable to this shipment's
  changes.
* Any report of an existing `AppState` construction call site behaving
  differently post-merge (would indicate the `with_mode` forwarding
  default is broken).

## Rollback Procedure

Revert merge commit `33a0a41e345cef8965b707346728d44fa5492daf` on `main`
via a standard `git revert -m 1` PR (merge-commit revert, preserving
history), gated through the same PR review/CI/approval pipeline as any
other change. No data migration or external state requires separate
rollback.

## Validation Window & Monitoring Plan

No external metrics/APM backend exists for this CLI/daemon tool.
Monitoring is CI/build-based:

| SLI | Source | Baseline | Threshold (escalate) | Owner |
|---|---|---|---|---|
| Existing contract suite stability | CI `build` check on every `main` push | 100% pass except the known pre-existing `shim_aborts_unresolved_startup_after_client_disconnects` gap | Any *new* contract-suite failure attributable to this shipment's files | Repository maintainer (`softwaresalt`) |
| Windows generation-publish durability (residual risk, stash `F2E84E15`) | Manual re-review by F07/F08 implementers before treating Windows publication as crash-durable equivalent to POSIX | N/A (accepted, unverified residual risk) | Any field/support report of a torn or missing generation directory on Windows after a crash during publish | Repository maintainer (`softwaresalt`), F07/F08 implementers |

Observation window: through the next shipment that touches
`src/db/cozo_backend/` or the generation-publish path (expected in a later
F06–F09 shipment).

## Owner

Ship agent / repository maintainer (`softwaresalt`) for monitoring; Stage
owns the manifest-correction remediation for the blocking finding above.

## Compaction Status (P-020)

`compact-context --target all` was invoked this session (mandatory,
unconditional on merge). Result: **done** (scan-only, no-op for
133-S/142-F memory). On first pass this session, the three 133-S session
memory checkpoints (`2026-09-03-ship-pr-372-stage-133-s-merge-closure.md`,
`2026-09-03-ship-133-s-mid-session-checkpoint.md`,
`2026-09-03-ship-133-s-pr-ready-checkpoint.md`) were incorrectly treated as
a completed-release-unit candidate and consolidated/archived — this was a
process error caught during Copilot review of this closure PR: the
compact-context skill's own eligibility rule excludes checkpoints for
active work items, and both the `133-S` shipment record and its covering
feature `142-F` remain `active` (this closure is explicitly `BLOCKED`, not
complete). The compaction was **reverted**: the three checkpoint files were
restored to `docs/memory/`, and
`docs/memory/compacted/2026-09-03-133-s-read-server-foundations-compacted.md`
plus its `docs/archive/memory/` copies were removed. `docs/exec-plans/` and
`docs/closure/` were also scanned for 133-S/142-F-specific candidates: the
one related exec-plan
(`2026-09-02-separate-indexer-read-server-plan.md`) governs feature `142-F`
as a whole, which remains open across multiple future shipments, so it does
not meet the "feature/chore complete" compaction precondition and was
correctly left uncompacted. **Net outcome after correction: compact-context
was invoked (P-020 mandate satisfied) but found zero eligible candidates
this session** — a valid scan-only no-op, since neither `133-S` nor `142-F`
has actually reached a completed/shipped state yet. Compaction of this
session's memory should be revisited only after Stage resolves the
manifest blocker and `133-S` genuinely reaches a terminal shipped/archived
state.

## Releasability Evidence (structured)

| Requirement | Status | Detail |
|---|---|---|
| Code merged, CI green, review clean | **Satisfied** | PR #376 merged as merge commit; CI (`build`, `start-launcher-windows`) both SUCCESS; local review `READY_WITH_FOLLOWUPS`; P-018 Copilot gate `SATISFIED` |
| Runtime verification | **Satisfied** | `PASS WITH FOLLOW-UP` — see Validator Evidence above; no new runtime regression attributable to this shipment |
| Windows generation-publish durability | **Conditional** | Accepted, unverified residual risk (stash `F2E84E15`); condition: F07/F08 implementers must explicitly re-review before treating Windows publication as crash-durable equivalent to POSIX |
| Shipment-record archival (`133-S` → archived) | **Satisfied** | Manual safe-close completed this session: `backlogit update 133-S --status done` → verified live `status: done` → `backlogit archive 133-S` → verified `status: archived`, `archived_status: done`. See "Manual Closure Completion" above. Stash `28C0E138`/`F9D1C495` duplicate-triage remains an open Stage follow-up but does not block this closure |

Overall releasability: `READY_WITH_CONDITIONS` — the shipped code itself
is fully ready and verified; the Windows durability item is a
satisfiable, accepted follow-up condition (tracked as stash `F2E84E15`,
requiring F07/F08 implementer re-review; see the "Windows generation-
publish durability" row above). Shipment closure itself is `READY` (no
open blocker) — the shipment-record archival item, previously a hard
block, is now resolved via manual safe-close.

## Verdict

**Closure: READY. Releasability: READY_WITH_CONDITIONS.** The code was
merged, verified, and is production-ready; runtime verification is PASS
WITH FOLLOW-UP; all task-level manifest items are done and archived; the
shipment record `133-S` itself is now `archived` with `archived_status:
done`, closed via an explicit, operator-approved manual safe-close (never
`backlogit shipment ship`, which remains unsafe for `142-F`'s partial
coverage — see "Cascade mechanism correction" above). Remaining
releasability conditions: the Windows durability residual risk
(accepted, documented, tracked as stash `F2E84E15`) and Stage's
duplicate-stash triage (`28C0E138`/`F9D1C495`,
informational follow-up, non-blocking). **The nine sibling shipments
(`134-S` through `142-S`) still partially cover `142-F` and must each use
the same manual safe-close path — never `backlogit shipment ship` — when
their own manifests reach completion.**
