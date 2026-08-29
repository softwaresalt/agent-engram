---
title: Stage re-entry — 138-F plan revision 2 and 131-S claim-readiness
date: 2026-08-27
type: session-memory
doc_type: memory
agent: stage
worktree: .worktrees/ship-137-late-readiness-proxy-recovery-20260826
feature: 138-F
shipment: 131-S
supersedes_gate: 138.001-R
active_gate: 138.002-R
plan_revision: 2
---

# Stage re-entry — 2026-08-27 (138-F revision 2)

> [!NOTE]
> **No RCA restated.** The authoritative root cause for the late-readiness
> sticky-proxy defect is
> `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`.
> Prior verification/closure evidence lives in the `137-F`/`130-S` artifacts.
> The first Stage session for `138-F` is
> `docs/memory/2026-08-27-stage-138-terminal-vs-transient-health-session.md`.
> This memory records **only** what changed in the revision-2 re-entry.

## Why re-entry was required

PR #365's Copilot review surfaced 12+ feasibility/correctness defects against
**plan revision 1** *after* Ship had exceeded the 3-cycle review-fix circuit
breaker. Ship correctly refused to redesign Stage-owned content, converted the
findings to a backlog handoff, and recorded a claim prohibition on
`131-S`/`138.002-T`. Revision 1 had passed its own gate (`138.001-R`) because
that gate probed *design intent* but never *mechanical executability*.

## Operating mode

* **P-012 CLI degraded mode declared.** The backlogit MCP server is bound to the
  **main** workspace root (`C:\Source\GitHub\engram\.backlogit`), which is stale
  for this work — it reports `130-S` queued, has no `138-F`, and its
  `stash.jsonl` fails to parse (`invalid character '<'`, i.e. conflict markers).
  All reads and mutations therefore used the backlogit **CLI** scoped to this
  worktree, per the `cli_command` fallbacks in
  `.autoharness/backlog-registry.yaml`. No ad-hoc filesystem editing of artifact
  IDs or frontmatter.
* **Engram discovery unavailable (again).** `engram daemon-status` failed with
  `Daemon failed to reach Ready state within 30000ms` — another in-the-wild
  instance of the failure class this feature addresses. The engram MCP surface
  was not exposed either, so source grounding fell to the third tier, which the
  ladder permits only after both prior tiers are unavailable.
* `INDEX_SYNC_OK` (CLI fallback) at session start and end.

## What changed

Revision 2 is a **re-grounded redesign, not an addendum**. Every design claim
now carries a `file:line` citation re-read at worktree HEAD.

| Artifact | Change |
|---|---|
| Plan | Rewritten as **revision 2** (same path) |
| Hardening | Rewritten as **revision 2**; three revision-1 claims formally **withdrawn as false**; 10-row ProposedAction/ActionRisk register added |
| Review | New `138.002-R` at `...-plan-review-r2.md`, verdict **approved**; `138.001-R` marked **superseded** with a caution banner |
| `138-F` | Description, Traceability, Goals, Non-Goals, DoD rewritten |
| Tasks | 7 → **14**; all 7 originals rescoped; 7 created (`138.008-T`…`138.014-T`) |
| Dependencies | 3 edges removed, 16 added; **19 total**, acyclic |
| `131-S` | 8 → **15** members; still **queued**, never claimed |

## The three defects revision 1 got materially wrong

Recorded because each was a *correctness* error, not a documentation gap.

1. **The latch did not latch (fail-open race).** Revision 1 claimed publishing
   `Degraded` under `recovery_lock` latched the session. It does not:
   `recovery_lock` lives on `ShimHandler` (`src/shim/transport.rs:66,129`) and
   does not bind `spawn_late_readiness_monitor`, which holds its own
   `Arc<watch::Sender>` (`src/shim/mod.rs:247`), never inspects the current
   value, and sends `Ready` **unconditionally** at `src/shim/mod.rs:263`. A
   request-path terminal latch could be silently overwritten — re-opening the
   exact hole the feature closes. **This was never raised by Copilot;** it was
   found during revision-2 grounding. Fixed by making `Degraded` an absorbing
   state via `watch::Sender::send_if_modified` (atomic under the channel write
   lock, no new shared state, no signature plumbing). Guard: **C5**.
2. **The transport framing claim was false.** Revision 1's residual-risk
   acceptance rested on length framing. Daemon IPC is **newline-delimited**
   (`read_line`, `src/shim/ipc_client.rs:87-93`) and `send_request` decodes the
   JSON **itself** at `:104`. Replaced with a stronger *structural* invariant:
   **every `Err` returned by `send_request` is `Transient` by construction**;
   only errors `fetch_health` builds after `send_request` returned `Ok` are
   terminal candidates. Guard: **R5**.
3. **T5 asserted a record nothing wrote.** `write_startup_failure_record` is
   private (`src/shim/mod.rs:348`), needs a `workspace_hint`, and runs only
   inside `compute_startup_outcome` *before* the late probe; the request handler
   has no validated workspace. Fixed: the **monitor** is the sole late-terminal
   writer, gains `workspace_hint` (already in scope at `src/shim/mod.rs:217`,
   used at `:219`), writes exactly once (structural — it returns immediately
   after), best-effort semantics stated rather than hidden.

## Three further defects found against revision 2 itself

Fixed in-cycle during the review pass:

* **N-1** — N1 (exact happy-path probe count) fit neither the "new-red" nor
  "pre-existing-green" category. Added a third category, **neutrality pin**: a
  new assertion that must be green at authoring; red-at-authoring now *proves*
  the phase-1 seams were not behavior-neutral.
* **N-2** — T5's "exactly one record" contradicted the `readiness_timeout`
  record **already** written at `src/shim/mod.rs:218-221`. Restated
  baseline-relative.
* **N-3** — T7 guarded only the JSON-RPC error text source; the
  undecodable-payload path is equally daemon-controlled
  (`HealthCheckResult.workspace` carries a real path,
  `src/daemon/ipc_server.rs:374`). Split into two cases.

## Decisions recorded

| Decision | Rationale |
|---|---|
| `Degraded` is absorbing, enforced by `send_if_modified` | Only mechanism that makes read-decide-write atomic across two independent publishers without new shared state or signature plumbing. tokio 1.49.0 (`Cargo.lock:4320`). |
| Terminal on `-32601` **only** | `_health` dispatches unconditionally (`src/daemon/ipc_server.rs:357`) and a hydrating daemon answers `status:"starting"` (`:362-367`), so `-32601` cannot fire against a healthy warm-up. `-32603` and all other codes stay transient. |
| `-32600`/`-32700` deliberately **not** terminal | Accepted under-classification (review P2-R1). Degrades to the status quo; over-classification permanently kills healthy sessions. |
| `TerminalKind` is a **closed enum**, not `Terminal { reason: String }` | Revision 1 would have propagated the daemon's arbitrary JSON-RPC `error.message` (embedded verbatim at `src/shim/lifecycle.rs:99-105`) into the permanent client payload — a path/env leak. |
| `expected`/`actual` only for `VersionMismatch` | The other three terminal kinds become terminal before any protocol version exists. Common fields are `endpoint` + `terminal_kind`. |
| Clock seam is a **production** change, gated as one | `tokio::time::pause` cannot move a `std::time::Instant` (`src/shim/transport.rs:11,31,138,144`), so revision 1's C2 was unprovable. `tokio::time::Instant` is wall-clock-identical unpaused, but the neutrality gate verifies it rather than assuming it. |
| Compatibility **veto deleted**, not re-specified | No consumer deserializes `failure_class`; the veto was unverifiable and its fallback was dead code that could have bounced the plan back to review for an impossible condition. Replaced by a golden-record additivity assertion. |
| Concurrency tests live **in-crate** under `#[cfg(test)]` | Keeps `ShimHandler`/`with_probe`/`StartupOutcome` crate-private — avoids a `pub` API widening. H-7 forbids `#[cfg(test)]` *branching in production logic*, not test modules; the seams stay default-installed. |
| Seams placed in **phase 1**, before the harness | Revision 1's fatal ordering defect: the harness had to compile against a seam built two tasks later. A seam is infrastructure, not logic; its behavior-neutrality gate is what keeps test-first honest. |
| 14 tasks, not 7 | Width isolation: helper, two seams, five harness groups, taxonomy, lifecycle, transport, mod, docs. Repo rule is 2-hour + single skill domain (`AGENTS.md:261-265`); there is no scenario-count rule. |
| No `131-S` → `130-S` block edge | `130-S` is shipped/archived; ordering is moot. |

## Verified outcomes

* `131-S` **queued, unclaimed**, exactly 15 members: `138-F` +
  `138.001-T`…`138.014-T`.
* Dependency graph: **19 `blocks` edges, acyclic**. Topological order —
  {`138.002-T`, `138.013-T`, `138.014-T`} → {`138.008-T`, `138.009-T`,
  `138.010-T`, `138.011-T`, `138.006-T`, `138.012-T`} → `138.003-T` →
  `138.001-T` → {`138.004-T`, `138.005-T`} → `138.007-T`. Every harness task
  gates every behavior task through the `138.003-T` fan-in, so TDD ordering is
  **mechanically** enforced.
* No other shipment is `active`. `125-S`…`129-S` remain pre-existing `blocked`;
  `131-S` is the only `queued` shipment.
* `backlogit doctor`: 43 findings, **all** pre-existing
  `archived_from_self_ref` on legacy archives; zero orphans, zero duplicates,
  none touching 130/131/137/138. Unchanged from the prior session's baseline.
* `git status`: only `.backlogit/**` and `docs/**` planning artifacts modified.
  **Zero source, test, or config files touched.**

## Boundary confirmation

No Git operations, no builds, no source/test edits, no commits, no pushes, no PR
actions, and `131-S` was **not** claimed. PR #365 was read via `gh` read-only;
no PR mutations. All work confined to the isolated worktree.

## Next steps (Ship)

1. Transfer/commit these Stage artifacts onto a clean `131-S` branch from
   `origin/main`.
2. Claim `131-S` — the circuit-breaker prohibition is **lifted**.
3. Execute in the three-phase order. Do **not** cite `138.001-R`; the active
   gate is `138.002-R`.
