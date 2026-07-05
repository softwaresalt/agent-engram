---
date: 2026-07-04
agent: Stage
mode: deferred-task-harvest + impl-plan + plan-review + shipment-assembly
branch: stage-deferred-072-073
base: main @ 237595b
tasks: [064.004-T, 065.004-T]
features: [064-F, 065-F]
shipments: [072-S, 073-S]
reviews: [072.001-R, 073.001-R]
plans:
  - docs/exec-plans/2026-07-04-064-004-reactive-sync-verify-gate-plan.md
  - docs/exec-plans/2026-07-04-065-004-notready-direct-hint-plan.md
status: reviewed-backlog-ready (both shipments queued for Ship to claim)
---

# Stage session memory — 2026-07-04 — assemble 2 stale-DEFERRED tasks into shipments 072-S / 073-S

## Task

Operator-directed Stage run against `agent-engram` (main @ `237595b`, cache clean;
pre-existing ` M .gitignore` drift is NOT ours — left untouched). Assemble the two
confirmed-actionable, formerly-DEFERRED queued tasks into shipment(s), bias to
**width isolation** (two separate shipments, one per feature). Stage produced
reviewed structure only — plans + plan-reviews + queued shipments + task updates.
**No Ship/code/PR work.** Branch, commit all artifacts, push, NO PR (Orchestrator
lands it).

## Tool status

- **Manual/file-backed mode** — no `.autoharness/backlog-registry.yaml` present
  (intentional operating mode, not degraded). Backlog mutations via
  `C:\Tools\backlogit.exe` v1.3.0 (atomic CLI).
- `INDEX_SYNC` **skipped per operator** (cache freshly rebuilt; no reflexive
  `backlogit sync`). CLI `update`/`shipment create` update the index incrementally.

## Grounding (read the real modules @ 237595b — this changed the plans)

- **064.004-T reactive gate is bigger than "add a gate."** `adapt_event`
  (`src/daemon/debounce.rs:89`) currently maps markdown → `Skip`;
  `ServiceAction::ReingestContent` (`:50`) is **defined but never produced or
  consumed**; `verify_markdown` (`src/services/verify.rs:69`) is wired only into
  the CLI verify command (`src/cli/commands/verify.rs:89`) — **not the daemon**.
  So the task must first **produce** `ReingestContent` (adapt_event) and then
  **gate** it in the **live** `run_with_shutdown_v2` consumer loop
  (`src/daemon/ipc_server.rs:1081`, dispatched from `daemon/mod.rs:202`). A
  **legacy v1 loop** exists too (`run_with_shutdown:638`), `ReindexFile`-only.
  `ingest_single_file` (`src/services/ingestion.rs:641`) reads the file itself;
  `RegistryConfig` is loadable via `load_registry` (pattern `tools/lifecycle.rs:133`).
- **065.004-T is genuinely small.** `DaemonError::NotReady` at
  `src/errors/mod.rs:161-162`; `to_response` mapping (`:488-492`) is
  string-independent (message change is contract-safe); `mod tests` home at `:719`;
  precedent for a long actionable `#[error]` is `IpcError::VersionMismatch`
  (`:151-154`). IPC-timeout (`IpcError::Timeout:149`) is explicitly out of scope.

## Packaging decision — TWO shipments (width isolation), IDs match operator mnemonic

- **072-S** = **064.004-T only** (daemon event-loop width). Feature **064-F is
  already assigned to the done Phase 1a shipment 052-S** (features are
  single-shipment in backlogit), so it CANNOT be re-added — the shipment carries
  the task and references the parent. 064.004-T is 064-F's only remaining open
  child.
- **073-S** = **065-F + 065.004-T** (error-string/CLI width). 065-F was
  unassigned and its other children (001/002/003-T) are archived, so delivering
  065.004-T completes the feature; the feature rides the shipment.
- **ID-ordering hiccup (fixed):** the first `shipment create` (064) failed on the
  064-F-locked-to-052-S constraint, so the 065 shipment initially grabbed 072-S
  (inverting the mnemonic). Force-deleted that shipment (brand-new untracked file,
  safe/reversible) and recreated in order → 072-S=064 work, 073-S=065 work, as the
  operator intended.

## Artifacts produced

| Kind | 064 track | 065 track |
|---|---|---|
| impl-plan | docs/exec-plans/2026-07-04-064-004-reactive-sync-verify-gate-plan.md | docs/exec-plans/2026-07-04-065-004-notready-direct-hint-plan.md |
| plan-review | 072.001-R (docs/reviews/2026-07-04-064-004-...-plan-review.md) ACCEPTED | 073.001-R (docs/reviews/2026-07-04-065-004-...-plan-review.md) ACCEPTED |
| shipment | 072-S (queued) | 073-S (queued) |
| task update | 064.004-T retitled (drop DEFERRED), `deferred` label removed, gate-satisfied note | 065.004-T retitled, `deferred` label removed, gate-satisfied note |

Both plans are test-first (Constitution II). 064.004-T ran **plan-harden**
(elevated blast radius) → freeze-scope + pure injectable gate helper so no test
spins the daemon (avoids the Windows `run_with_shutdown_v2` SQLite flake); the
plan MUST NOT depend on or perturb daemon-startup timing.

## Open questions flagged for the operator

- **064.004-T Q2 — legacy v1 loop.** Gate only the live v2 loop + comment v1, and
  spawn a **separate** "confirm/remove v1 `run_with_shutdown` loop" item rather
  than widening 064.004-T? (Recommended.)
- **064.004-T Q1/Q3 — source resolution + extensions.** Longest-prefix match over
  `RegistryConfig.sources`, skip+log if unowned; extensions `md` + `markdown`.
- **064.004-T scope-honesty risk.** Because reactive markdown reingest does not
  exist yet, this is a ~2h feature (produce + gate), not a trivial gate. Kept
  within the acceptance criteria; flagged so the operator sizes it correctly.
- **065.004-T Q1 — optional help hint.** Default OFF (avoids `--help`
  snapshot/contract churn for a low-priority item); operator may opt in.

## Next steps (Ship)

- Claim 072-S then implement 064.004-T under the 072.001-R conditions (C1 pure
  helper / C2 source resolution / C3 freeze-scope / C4 fail-safe).
- Claim 073-S then implement 065.004-T under the 073.001-R conditions (brace-free
  hint / don't touch BoolishValueParser / NotReady-only / ≤3 files).
- Orchestrator lands this Stage branch (no PR opened by Stage).
