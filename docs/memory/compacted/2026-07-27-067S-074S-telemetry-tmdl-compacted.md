---
type: compacted-memory
date: 2026-07-27
period: "2026-07-03 .. 2026-07-04"
source_count: 12
archive_path: docs/archive/memory/
---

# Phase 2: TMDL Pipeline & CI Infrastructure (Shipments 067-S through 071-S)

## Overview
Intensive two-day sprint delivering four shipments — telemetry completion (067-S), TMDL extractor depth (068-S), decision-gate evaluation harness (069-S), safe-parser correctness fixes (070-S) — plus Stage planning for the queued 071-S CI build-skip (which actually shipped July 5 in the orchestrator run; see the 074S-084F summary). Established a repeatable pattern: differential evaluation → decline grammar investment, incrementally fix safe parser.

## Key shipments and outcomes

### 067-S Continuation: backlog-hygiene chore (July 3)
Landed accumulated post-merge bookkeeping from prior sessions (053-S/065-F/064-F transitions, 067-F plan artifacts, compound learning) as `chore(backlog)` PR #189. Backlogit cache-union risk mitigated by staging markdown-only (source of truth). MCP transport down — used CLI fallback.

### 068-S: TMDL Extractor Depth — Partitions, Datasource Properties, Lineage (July 3–4)
Harvested three Stage-prepared tasks from orphaned 066-F (re-IDed TMDL). Tasks: (1) partitions + fenced-M source capture, (2) richer datasource props + `powerbi_data_source` summary, (3) refs/annotations/lineage metadata. All shipped test-first; the internal code-review agent caught 1 P2 (hierarchy/level nested-metadata skip window), and Copilot review caught 2 security findings (partition M bodies + connection strings scraped from summaries, only size hints retained). Landed in PR #192; all regression tests green.

### 069-S: TMDL Tree-Sitter Eval Gate (July 4)
Decision-gate, not grammar build. Measured safe line/indent parser against a differential harness: 3 constructs pass, 6 miss (4 model-richness gaps requiring new `TmdlModel` types; 2 incrementally-fixable heuristic bugs). Finding: grammar ROI-negative; recommend DECLINE. Boundary hard: no tree-sitter, no unsafe, no new dependency, no promotion decision (that's a future Stage call). Shipped as PR #196 with 11-test harness (S-PTM-20..29).

### 070-S: TMDL Safe-Parser Correctness Fixes (July 4)
Executed optional bounded follow-on from 069-S finding: fix the 2 heuristic bugs. (1) Calculated-column expression capture (mis-scope due to `parse_identifier(rest)` swallowing `= DAX`); added `TmdlColumn.expression` field + generalized multiline-body machinery. (2) Measure-DAX colon truncation (`looks_like_tmdl_property` bare `contains(':')` broke on `FORMAT("HH:mm:ss")`); refined to require property-shaped bare-identifier key. Differential harness flipped assertions buggy→correct; test-first. Shipped as PR #199; aggregate `heuristic_bugs: 0`.

### 071-S: CI Build-Skip on Doc/Backlog-Only PRs — Stage planning (July 4)
Grounding spike resolved: `build` is NOT a required status check on `main` (only 1 approval + code-owner + thread-resolution). Proposed mechanism: `paths-ignore` on `.backlogit/**`, `docs/**`, Markdown, and `.autoharness/**` for both `push` and `pull_request`; code re-arms full suite (Rust/Cargo/workflow changes). Rejected PR-title `if:` guard (fragile). Documented future-coupling guardrail (if `build` promoted to required, switch to companion always-passing job). NOTE: the plan's blanket `**/*.md` glob was later SCOPED to specific paths (so `tests/**/*.md` still triggers CI) during the July-5 shipping after adversarial review. 071-S actually shipped July 5 as PR #201 (plan), #202 (code), #203 (closure) — see the 074S-084F summary.

### 072-S & 073-S: Deferred Task Assembly (July 4)
Harvested two long-pending DEFERRED tasks: 064.004-T (daemon reactive-markdown reingest gate, Phase 1b) and 065.004-T (DaemonError::NotReady hint for `--direct` escape). Wide-isolated into separate shipments (width-per-feature, state-of-the-art). 064.004-T scoped to produce + gate `ReingestContent` without perturbing daemon startup (pure helper + source resolution + freeze-scope). 065.004-T tiny CLI-facing task (error string + optional hint).

## Traceability & decisions

- **Differential evaluation pattern** (069-S): measure against live parser output, derive verdict from harness assertions (not hard-coded arrays), gate-line re-derives counts to ensure any parser change fails the anchor and forces re-evaluation.
- **Safe-parser vs. grammar ROI** (070-S decision): 1404-line indent-aware parser now handles partitions, datasources, refs, annotations, lineage — the exact constructs the spike attributed to tree-sitter. Robustness/maintainability a gain, but coverage already delivered by safe parser.
- **CI paths-ignore semantics** (071-S): all-match → single code file re-arms full build. Guarantees code-PR coverage cannot be weakened.
- **Plan-harden deferred** (072-S): reactive-sync module is new (not yet written); test strategy defers to dry-run-free injection (test harness calls sync gate function, daemon unperturbed). Blast radius ELEVATED but scope FROZEN.

## Key engineering notes

- **TMDL member-body generalization** (070-S): `PendingMeasureBody` → `PendingMemberBody` keyed by `TmdlMemberKind`, matches on `finish_pending_*` — additive pattern holds for future member types.
- **Heuristic property detection** (070-S fix 2): keys are bare identifiers (letters/digits/`_`). If TMDL ever gains non-identifier-key properties, revisit.
- **Rollback safety** (072-S planning): reactive-sync doesn't exist yet; cold-start test skips spawning daemon (injectable gate helper only).

## Cross-domain dependencies

- 069-S eval gate enabled 070-S (decision data), but both are standalone PRs (no data dependency).
- 071-S CI skip (live July 5): every subsequent backlog-only closure PR (#203, #206, #208, #209, #211) correctly skipped CI — validated end-to-end.
- 072-S/073-S assembly unblocks Orchestrator autonomous pipeline (both queued).

## Archived originals (traceability)

| File | Summary |
|---|---|
| 2026-07-03-ship-backlog-hygiene-chore-session.md | Staged accumulated post-merge backlog/doc bookkeeping (053-S/065/064 transitions, 067 plan, compound learning) as chore PR #189. |
| 2026-07-03-ship-pr189-backlog-hygiene-postmerge-closure-session.md | Post-merge closure PR #189: `chore/backlog-hygiene` merged; local main synced; merged branch pruned; backlog state spot-checked. |
| 2026-07-03-stage-067-amend-cli-correlation-id-direct-emission.md | Amendment to 067-F: operator directive reversed CLI-direct out-of-scope → in-scope; added --correlation-id arg + 067.005-T/006-T. |
| 2026-07-03-stage-tmdl-depth-triage-and-068S-assembly.md | TMDL depth triage: 066.005-T/006-T/007-T re-parented → 068.001-T/002-T/003-T; harvested as 068-S; deliberations 010-D/011-D/012-D closed. |
| 2026-07-04-ship-068S-tmdl-extractor-depth-session.md | 068-S executed 3 TDD tasks: partitions (red/green), datasource props (red/green), refs/annotations/lineage (red/green); 7 commits, 24 integration tests. |
| 2026-07-04-ship-069S-tmdl-tree-sitter-eval-gate-session.md | 069-S evaluated safe TMDL parser: 3 PASS / 6 MISS (4 model-richness, 2 heuristic); recommended DECLINE grammar; PR #196 with harness. |
| 2026-07-04-ship-070S-tmdl-parser-fixes-session.md | 070-S fixed 2 heuristic bugs: calc-column expression capture, measure-DAX colon truncation; aggregate `heuristic_bugs: 0`; PR #199. |
| 2026-07-04-stage-066008-unblock-tmdl-tree-sitter-spike.md | Unblock + re-scope 066.008-T: safety blocker FALSE (forbid(unsafe_code) forbids own source, not deps); ROI shifted (safe parser delivered coverage). |
| 2026-07-04-stage-069S-tmdl-tree-sitter-eval-and-dax-correction.md | Refined 066.008-T → eval harness + `069-F` umbrella; parked DAX stash (F7E89921) with unsafe-myth correction (not shipped); created shipment 069-S. |
| 2026-07-04-stage-070S-tmdl-parser-fixes.md | Assembled 070-S from the 2 heuristic bugs 069-S found: fix calc-column (070.001-T), fix colon-in-DAX (070.002-T); both test-first. |
| 2026-07-04-stage-071S-ci-build-skip.md | Grounding spike: `build` NOT required; `paths-ignore` mechanism safest; future guardrail documented; 071-S queued for Ship. |
| 2026-07-04-stage-072S-073S-deferred-harvest.md | Harvested 2 DEFERRED tasks into 2 shipments (width isolation): 072-S=064.004-T (daemon reactive-sync gate), 073-S=065.004-T (NotReady hint). |
