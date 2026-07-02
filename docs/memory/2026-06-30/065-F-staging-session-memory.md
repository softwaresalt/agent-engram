---
title: "065-F Staging Session Memory — Document daemonless --direct mode"
type: session-memory
date: 2026-06-30
agent: stage
feature: 065-F
shipment: 053-S
source_stash: 20477A6A
plan: docs/exec-plans/2026-06-30-document-daemonless-direct-mode-plan.md
---

## Objective

Process stash `20477A6A` (chore, medium) end-to-end through the Stage pipeline:
triage → impl-plan → plan-review gate → harvest → queued shipment. Docs-first,
width-isolated. No Ship work (no build/branch/PR).

## Outcome

- **Exec-plan:** `docs/exec-plans/2026-06-30-document-daemonless-direct-mode-plan.md`
  (includes explicit Constitution Check + Plan Hardening Signals: `Requires plan
  hardening: no`).
- **Plan-review gate:** **PASS**. 1 P1 (missing explicit Constitution Check
  section) resolved before harvest; F2/F3 P2 accepted with scoping notes; F4/F5
  P3 advisory. No P0.
- **Feature:** `065-F` — "Surface & document the daemonless --direct indexing
  mode" (labels: chore, docs, discoverability).
- **Tasks:**
  - `065.001-T` (docs) — configuration.md + troubleshooting.md canonical anchor. **Root of docs chain.**
  - `065.002-T` (docs) — README escape-hatch quickstart callout. Blocked by 065.001-T.
  - `065.003-T` (docs/scripts) — start.ps1 + start.sh cross-references. Blocked by 065.001-T.
  - `065.004-T` (code, rust, deferred, priority low) — daemon-startup-timeout
    (`DaemonError::NotReady`) error hint → `--direct`. **Excluded from the docs
    shipment** (width isolation; Test-First + cargo gates).
- **Shipment:** `053-S` (status `queued`) = `065-F` (parent, added first) +
  `065.001-T`, `065.002-T`, `065.003-T`. Ready for Ship to claim.
- **Stash:** `20477A6A` archived after harvest; feature linked (`informs`) to
  deliberation `010-D`.

## Key decisions

- **Skipped a fresh deliberation:** 010-D already closed the decision (document
  the shipped 045-F/030-S escape hatch, don't re-build it) and named the exact
  surfaces. Only sequencing remained → lightweight impl-plan sufficed.
- **Parent is a `feature`** (backlogit has no `chore` artifact type); `chore`
  captured as a label.
- **Docs vs code split (deliverable 3):** the timeout-error hint touches compiled
  Rust (`src/errors/mod.rs`), a different skill domain bound by Test-First +
  cargo gates. Harvested as a **separate deferred code task** kept OUT of the
  docs shipment, per stash guidance ("defer rather than bloat the docs
  shipment").
- **Grounding correction:** docs are NOT a total void — `docs/configuration.md`
  already has a terse `ENGRAM_DIRECT` env-var row (~L47) and
  `docs/troubleshooting.md:70` mentions `engram sync --direct` (debugging-framed).
  README has **zero** mentions (primary gap). Work framed as consolidation +
  README net-new + escape-hatch framing, NOT first-ever coverage.
- **Canonical anchor first:** `configuration.md` chosen as the docs home (it
  already owns CLI flags/env vars); harvested first so README + start-script
  cross-references stay valid (cross-reference integrity gate).

## Grounding evidence

- `src/bin/engram.rs`: `Sync { full, direct }` (~L85–94), `Index { direct }`
  (~L96–104), both `#[arg(long, env = "ENGRAM_DIRECT", value_parser =
  BoolishValueParser::new())]` with doc comments (per-subcommand `--help` shows
  the flag; top-level `engram --help` does not signpost it).
- `src/errors/mod.rs`: `DaemonError::NotReady { timeout_ms }` (~L161) is the
  daemon-startup-timeout message; `IpcTimeout` (~L149) is a separate surface
  (optional follow-up, not in scope for 065.004-T).
- `README.md`: QuickStart shows only `engram sync` (~L62); zero `--direct` /
  `ENGRAM_DIRECT` mentions.
- `start.ps1`: already USES `engram sync --direct` pre-warm (~L101–102) + daemon
  fallback, but no doc cross-ref. `start.sh`: does NOT invoke engram at all.
- Framing to reuse (do not re-derive): `docs/decisions/2026-05-08-cli-direct-daemonless-mode-deliberation.md`,
  `docs/closure/2026-05-08-030-S-cli-direct-mode-closure.md`.
- Compound learnings (3, all implementation): the clap-bool-env-var learning is
  a Task 065.004-T caveat — do NOT change the `ENGRAM_DIRECT` `BoolishValueParser`.

## Risks / assumptions carried forward

- 065.004-T may require updating CLI help/error snapshot or contract tests if the
  top-level help summary changes — flagged in the task and plan.
- Closure doc compound note #1 ("use `value_parser!(bool)` not
  `BoolishValueParser`") conflicts with shipped code (which uses
  `BoolishValueParser`) — do NOT propagate that into new docs.

## Next steps (for Ship)

1. Claim shipment `053-S`; build docs tasks in dependency order: `065.001-T`
   first, then `065.002-T` and `065.003-T` (parallel).
2. Verify docs quality gates: markdown structure, no unresolved `{{...}}`, all
   cross-references resolve, commands match `src/bin/engram.rs`.
3. Separately schedule the deferred code task `065.004-T` in its own code
   shipment (test-first; full cargo fmt/clippy/test/audit gates).

## Boundary note

All actions were within the Stage Role Boundary: planning + backlog artifacts
only (exec-plan, backlog items, shipment manifest, stash archive, session
memory). No source/test/config code was written; no build/branch/PR executed.
