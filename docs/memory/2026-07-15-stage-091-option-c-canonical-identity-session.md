# Stage session — 091-F Option C canonical-identity spike → reviewed plan/backlog/shipments

- **Date:** 2026-07-15 (DARK_MODE) · **Agent:** stage
- **Worktree:** `.copilot/session-state/2c95481b-…/files/stage-followups` · **Branch:** `stage/stash-followups-2026-07-15`
- **Base:** `7346f86` (one commit atop origin/main `df77584`).

## Operator decision (authoritative)

At 2026-07-15 19:03 -07:00 the operator resolved the Option C blocker in favour of **absolute
precision**: KEEP the absolute no-false-edge invariant (013-D) and proceed with **Option C canonical
module/type identity**. No downgrade to best-effort. This cleared prerequisite #1 of 2; prerequisite #2
(feasibility spike 091.001-T) was executed this session.

## P-012 tool/state gate (degraded-mode determination)

- **backlogit MCP: DEGRADED/UNSAFE for writes.** The MCP server is rooted at the **main working tree**
  (branch `feat/088-rec1-call-resolution`, PR #248) — `get_item 091-F` → `not_found`. Writing via MCP
  would mutate another agent's branch. Used the **backlogit CLI pinned to the worktree** (`Push-Location`
  / `--cwd`) as the safe path. Note: `dep add` cross-item validation misreads with `--cwd`; `Push-Location`
  fixed it.
- **engram: DEGRADED.** No engram MCP tools exposed; engram CLI daemon bound to the main tree and
  returned "database is locked". Used best-effort only; authoritative code reads from the worktree
  (`df77584` baseline).
- `backlogit sync` (CLI) = INDEX_SYNC_OK.

## Spike 091.001-T — VERDICT: GO (Rust-first)

Canonical module/type identity is feasible on the existing substrate without unsafe assumptions, under a
strict **fail-closed** discipline. Key evidence (origin/main `df77584`):

- **Baseline reconciliation:** ALL 15 commits of the 088 qualified-resolution work are **unmerged** on
  `feat/088` (PR #248). origin/main ships only the rec1 **bare-name** subsystem (078-S/082-F):
  `staged_call` provenance (082.002-T), a full-index **post-pass** `reresolve_calls_edges` (082.008-T),
  and qualified/method calls **`continue`-dropped before staging** (code_graph.rs L487/L1138).
- Impl methods index as **source-spelling** `{ty}::method` (rust.rs `extract_impl` L197) → the RMeJ0
  root cause. `use` decls already extracted (dropped per 013-D). Resolver is pure **string-name** match
  on `function_meta.name` (cozo_queries.rs L1701). `schema_meta` = fingerprint home; `file_hash` =
  content-hash skip that hides format-only changes (safeguard #4); 087.001-T = fingerprint precedent.
- **Architecture:** additive `function_meta.canonical_path` (name UNTOUCHED → blast-radius isolation);
  module tree + enriched use-graph + fail-closed canonical resolver + re-export closure; unforgeable
  typed `Self` sentinel; generic normalization; fingerprint + one-time forced re-index; canonical
  singleton resolution in the post-pass emitting `calls_resolved_canonical` only on an **exactly-one**
  canonical match; identity-based precision/recall release gate (fixes 088-F4).
- **Scope:** Rust-first, NOT cross-language parity. Method resolution scoped to `self`/`Self` receivers
  only (no type inference for `x.foo()`).
- Report: `docs/decisions/2026-07-15-091-001-canonical-identity-spike.md`.

## Plan-harden + adversarial review

`docs/closure/2026-07-15-091-F-option-c-canonical-identity-adversarial-review.md` — verdict
**PROCEED-WITH-REMEDIATIONS**. Six HIGH P1 findings folded into the plan pre-harvest: D1 `#[path]`/cfg
module mapping fail-closed; D2 method scope = self/Self only; D3 workspace-crate-set classification; D4
empty `canonical_path` never a match target; D5 A8 reindex single-flight/BUSY-safe/non-blocking; D6
use/local shadowing precedence. P2/P3 dispositioned. Formal **multi-model panel** encoded as a required
**Unit-B release gate** before edges flip on. No P0.

## Backlog changes (all in the worktree)

- **091.001-T** spike → **done** (GO recorded).
- **091-F** blocked → **queued**; description + DoD updated (GO, plan refs, fail-closed, remediations).
- Harvested **A1–A8 = 091.003-T…091.010-T** (Unit A) and **B1–B4 = 091.011-T…091.014-T** (Unit B),
  each ≤2h, width-isolated, test-first; 20 dependency edges wired.
- **091.002-T** (reconcile 088.005-T) — added dep on **091.014-T**; remains **blocked**; must not touch
  088.005-T / 081-S manifest until formal resumption.
- **088-F** — informational comment (Option C supersedes; manifest UNCHANGED); still blocked.

## Shipments (queued)

- **087-S — Option C Unit A** (infra, precision-neutral): {091.003-T…091.010-T}. No external hard dep.
- **088-S — Option C Unit B** (gated flip-on): {091-F, 091.011-T…091.014-T}; covering feature 091-F is
  released here (final unit). **Blocks-on 087-S + 084-S** (encoded).
- Existing 083-S / 084-S / 085-S / 086-S manifests **UNCHANGED**.

## Recommended Ship order

`086-S` → `084-S` → `087-S` (Unit A) → `088-S` (Unit B, after 087-S+084-S, with the multi-model panel
gate) → `083-S` / `085-S` (last). 084-S must precede Unit B (durable staged_call substrate) — encoded as
a dependency, not prose.

## Integrity

`doctor` = 43 pre-existing `archived_from_self_ref` issues on old items (047/052/055/061/062); **none**
involve the new 091.x / 087-S / 088-S artifacts. No new orphans/dupes.

## Next steps for the next Stage/Ship session

- Ship bases the next branch on this commit's SHA (see handoff) or origin/main; claims 086-S/084-S first.
- Unit B (088-S) must not be claimed until 087-S and 084-S are shipped; run the multi-model adversarial
  panel on the Unit-B diff before enabling `calls_resolved_canonical` edges.
- 091.002-T reconciliation is adjudicable only after 091.014-T (B4) lands.
