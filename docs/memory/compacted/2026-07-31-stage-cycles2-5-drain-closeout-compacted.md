---
title: "Stage triage/planning cycle compaction: post-100-F cycles 2-5, drain closeout"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-07-28/stage-cycle2-index-freshness-triage-memory.md
  - docs/archive/memory/2026-07-29/stage-cycle3-pr293-followup-triage-memory.md
  - docs/archive/memory/2026-07-29/stage-cycle4-held-items-memory.md
  - docs/archive/memory/2026-07-30/087-powerbi-durability-stage-memory.md
  - docs/archive/memory/2026-07-30/stage-cycle5-daemon-sync-index-reconciliation-memory.md
  - docs/archive/memory/2026-07-31/drain-closeout-memory.md
---

# Stage triage/planning cycle compaction (planning-only, no code shipped)

These five Stage sessions (2026-07-28 through 2026-07-31) triaged stash
entries and produced queued shipments / parked deliberations, without any
build, PR, or push to main. All are planning artifacts; the resulting
shipments (093-S through 101-S) shipped separately and are compacted under
their own dates once merged.

## Cycle 2 (07-28) — post-100-F follow-up triage

4 stash entries: **8DD29746** advanced to feature 101-F (versioned
code-graph revalidation/backfill — content-hash skip keys on file content,
not extractor generation, so pre-100-F wrong edges stay stale until
`--force`; fix mirrors 096-F's T7 marker pattern) as shipment 094-S.
**F97D51DF** advanced to feature 102-F (cargo audit remediation, 10
pre-existing transitive advisories) as shipment 093-S. **5765BAAB**
deferred to spike 015-D (stash's claimed root cause at `direct.rs:162` was
refuted by static analysis — `engram index` routes to the unconditional
cross-file post-pass on both daemon and `--direct` paths; real defect
location unproven, needs hands-on daemon repro). **B94772CB** deferred to
low-priority deliberation 016-D (Python-only last-wins recall recovery).

## Cycle 3 (07-29) — PR #293 follow-up triage

5 stash entries: **685FAA80 + 92EE75BB** grouped into feature 103-F
(forced-index certify-path completeness — no global orphan-edge GC exists,
and newly-excluded still-on-disk files never self-heal on the forced
route) as shipment 096-S. **BE366218** advanced to feature 104-F (daemon
pending-sync drain hardening — cancel/DB-fail paths leaked companion
revalidate/backfill flags without draining, causing spurious heavy passes
or stalls) as shipment 095-S. **D2416925** resolved in-cycle as a Stage
bookkeeping fix (`backlogit stash archive` has no `--reason` flag; adopted
a harvest-provenance-comment convention instead). **99AFF44B** (cozo 0.8+
major-version bump for RUSTSEC-2026-0041) deferred to deliberation 017-D —
unbounded blast radius, not scopeable to 2 hours.

## Cycle 4 (07-29) — held investigative items, zero shipments produced

Closed three held items without building anything (honest "no shipment"
outcome): **015-D spike** reproduced the daemon IPC hang (`engram index`
via daemon hangs the CLI past its own timeout while the daemon completes
server-side — root cause: `ensure_daemon_running` daemon-spawn/model-load
happens outside the client's timed request) but found the non-persist
claim inconclusive pending a known-green corpus control; deferred to a
Ship-owned runtime-verification spike rather than fabricate a fix on an
unproven root cause. **016-D decided KEEP FAIL-CLOSED** — Python last-wins
recall recovery was judged unsound (non-linear redefinition, shared
Rust+Python resolver) or disproportionately complex; revisit only on
measured recall loss on a real corpus. **014-D archived as moot** — its
chosen direction had already shipped as 100-F.

## PowerBI durability planning (07-30) — shipment 100-S

Gave the two 083-S/PR#257 cycle-3 deferrals their own reviewed plan:
**087.006-T** (durability contract) kept as one task — a completion-marker
relation replaces content-row-derived hash-skip gating, written last.
**087.005-T** (deletion-semantics) decomposed into three width-isolated
subtasks (shared fail-closed reconciler → wire into notebook sweep → wire
into non-TMDL PowerBI sweep). pbip/backlog sweep parity explicitly
recorded as an out-of-scope P2 follow-up, not harvested.

## Daemon sync/index reconciliation cluster (07-30) — feature 105-F, shipment 097-S

Three PR #297/#299 Copilot-thread residual races grouped into one covering
feature: **105.001-T** generation-scoped pending-sync clear (a whole-queue
wipe could erase a newer generation's revalidate/backfill intent),
**105.002-T** atomic producer→lock-holder drain handoff (depends on
105.001-T), **105.003-T** forced-index eviction-before-post-pass ordering
fix (independent width). Explicitly evaluated and **rejected** folding
015-D's non-persist finding into 105.003-T — distinct root cause/trigger,
015-D stayed unpinned and out of width.

## Drain closeout (07-31) — shipment 101-S

Parked three tail follow-ups as future features rather than build-later
tasks (each needed its own plan-harden/adversarial review or benchmarking):
090.005-T (CLI/MCP routing-identity dispatch registry), 091.019-T
(canonical re-export tail), 091.021-T (single-snapshot perf). Dropped
091.017-T as refuted (`wont-fix`, single-reviewer speculation overturned by
a second reviewer). Softened the harvest-provenance-convention doc rather
than rewrite history, since `backlogit stash archive` cannot carry a
descriptive reason. Assembled a docs/backlog-hygiene closeout (feature
106-F, task 106.001-T) as shipment 101-S.

## Preserved, not compacted

015-D (daemon IPC hang, unpinned root cause), 016-D/017-D (parked
deliberations), and 090.005-T/091.019-T/091.021-T (parked future features)
remain open in the backlog, not memory — revisit only on their named
triggers.
