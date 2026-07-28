---
type: compacted-memory
date: 2026-07-27
period: "2026-07-05 .. 2026-07-13"
source_count: 9
archive_path: docs/archive/memory/
---

# Phase 3: Autonomous Orchestration & Eval Subsystem (Shipments 071-S through 080-S)

## Overview
Orchestrator granted full autonomy (merge approval, adversarial review, circuit-breaker judgment). Delivered four autonomous shipments (071-S–074-S) end-to-end in a single July-5 run, then shipped two eval/rec1 harvests — retrieval+graph-recall eval subsystem (077-S/081-F, 7 tasks; merged PR #238) and rec1 call-edge cross-file resolution (078-S/082-F, 10 tasks; merged PR #241, after one re-harvest splitting three over-limit tasks following Copilot blocks) — and HARVESTED two further shipments left QUEUED for a later Ship: hardening (079-S/084-F, 14 tasks) and DAX intelligence (080-S/085-F, 7 tasks). Merged shipments used proper merge-commits (P-009/P-011), zero unresolved threads.

## Key shipments and outcomes

### 071-S (Autonomous execution): CI build-skip (July 5)
Orchestrator delivered: PR #201 (plan), #202 (code `paths-ignore` scoped globs), #203 (closure). Adversarial review caught P1 under-run (blanket `**/*.md` matched `tests/fixtures/verify/*.md`), scoped to specific globs. All subsequent doc/backlog-only PRs (#203, #206, #208, #209, #211) correctly skipped CI — validation end-to-end.

### 072-S & 073-S (Autonomous execution): Deferred task delivery (July 5)
Executed 064.004-T (daemon reactive-sync gate) + 065.004-T (NotReady hint) simultaneously. Adversarial review caught P1 on 072-S (DB divergence risk: markdown under dedicated-indexer source dirs post-restart). Fix: dedicated allowlist + active-source/size guards. All merged; reactive-sync now live in daemon watcher.

### 074-S: NotReady scope fix (July 5)
Post-merge Copilot finding on 073-S: `NotReady` is shared (startup path + respawn-wait path); `--direct` hint only applies to startup. Fix: new `DaemonError::ShutdownTimeout` variant (8010 wire code), scoped message omits `--direct`. Test-first both paths (startup retains hint, shutdown drops it). Shipped as planned + fix cycle.

### 077-S & 081-F: Retrieval + Graph-Recall Eval Subsystem (July 11)
Portable evaluation infrastructure: `[retrieval_eval]` config section (disabled default), models, MCP tools (`run_retrieval_eval`/`get_retrieval_eval_report`), `engram eval` CLI. Semantic metrics (precision@k/recall@k/MRR/nDCG via docstrings as ground truth). Graph metrics (resolution-recall, false-edge-rate via tree-sitter inventory). Persistence to `.engram/eval/{branch}/`. Test tier with graduated baseline. 7 tasks, 29 tests. Copilot 7 review cycles (unusual length): cycles 1–3 fixed logic (DB, atomic persist, sampling determinism), cycle 4 de-scoped own violation (retracted shared `connect_db` retry, hardened test-scoped), cycle 6 fixed P1 workspace-isolation security (symlink TOCTOU), cycle 7 deferred design/enhancement. Shipped PR #238, merge commit `0228de2`.

### 078-S & 082-F: rec1 Cross-File Call-Edge Resolution (July 11–12)
10 tasks decomposed across dual domains: parsing (capture method/receiver), staging (staged_call relation), provenance storage (resolution column migration), model export (CodeEdge.resolution), rehydration (ParsedEdge carry-through), post-pass (unambiguous-only singletons), lifecycle (clear-before-reindex, file-delete cleanup), rollback (down-migration + CLI trigger), acceptance (manifest target-correctness). Copilot 4 review cycles with fixes at 1/2/3, breaker invoked at 4 (concurrency harness + column-schema findings deferred). Shipped PR #241, merge commit `bf8d8a6`. Follow-up PR remediation added durable rollback marker + tests.

### 082-F Re-Harvest (July 11 after Copilot #239 block)
Copilot review of 082-F queue artifacts blocked 078-S: three over-limit tasks (082.002-T, 082.003-T, 082.004-T; >3 files or incomplete lifecycle specs). Stage re-harvested: rescoped 082.002-T/003-T and reduced 082.004-T to four scenarios, creating six new single-width tasks (082.008-T/009-T/010-T/011-T/012-T/013-T) across Copilot #240 cycles 2–3, propagated full contract into task bodies, rewired dependencies. 078-S re-queued with 10 items; cross-feature acceptance gate preserved (082.004-T depends on 081-F's eval metrics).

### 079-S & 084-F: Retrieval-Eval Correctness Hardening (July 11)
14 tasks consolidating Copilot post-077-S findings: resolution_recall unit/index-consistency gate, false-edge target-correctness via fixture manifest, canonical TSX language gate, threshold enforcement in service + CLI, retrieval-mode fidelity (silent keyword-only fallback detection), semantic corpus completeness, perf tuning (top-k selection, graph parsing memory bound), regression tier. Dependency DAG: 084.001-T is the foundation (report/config model surface); 002/004/006/008/009 fan out from 001; 003←{001,002}, 007←006, 010←009, 011←003, 012←{002,003,004}, 013←{004,009}; 005 (canonical TSX language gate) and 014 (merge-gate carry-forward learning) are independent (no deps). Deferred: durable call-site inventory persistence (separate reliability shipment), workspace-status atomicity (lifecycle-handler concern).

### 080-S & 085-F: DAX Intelligence (July 13)
7 tasks (high priority): P1 DAX reference extractor (parser crate), P2 carry calculated-column DAX (models + adapter), P3 reference edges (largest; split points P3.a/b), P4 impact_analysis Power BI span, P5 DAX lint Tier 1 + VerifyFinding.severity, P6 DAX lint Tier 2 + lint_dax MCP (largest; P6.a/b/c), P7 engram lint-dax CLI (bounded parity guard vs. full 30F372C8 audit). Dependencies: P3←{P1,P2}; P4←P3; P5←P1; P6←{P3,P5}; P7←P6. No schema migration (additive fields only). Fixtures must be committed (no tmp/ILSOS-* uncommitted samples). Open questions resolved: Q1 (standalone `lint_dax` MCP tool + Tier-1 verify gate), Q2 (reuse `pbi_uses_field`), Q3 (code↔Power BI bridge — OUT of scope/deferred). Operator REVERSED D4 (CLI-parity deferral): `engram lint-dax` CLI is now IN as P7 with a bounded 5-gap allowlist; 30F372C8 keeps the full-surface audit for a future harvest.

## Traceability & decisions

- **Adversarial gate efficacy** (071-S/072-S): caught 3 genuine P1s (blanket glob under-run, DB divergence, NotReady hint scope). Design: only after Ship + review caught issues, open-loop fixing before merge.
- **Copilot review-fix circuit breaker** (077-S, 078-S, 082-F): 3 cycles limit holds; de-scope own violations, security fixes, and backlog defects override limit. 078-S cycle 4 deferred concurrency edge-cases (separate hardening shipment).
- **Workspace isolation security** (077-S cycle 6): symlink resolution before containment check (both sides canonicalized); escaping paths skipped. Added to Constitution Check invariant III.
- **Rollback-marker durability** (082-F remediation): schema_meta relation flag prevents re-migration on next daemon start; durable across restarts.
- **Eval metrics ground truth** (081-F): docstrings/names as semantic query corpus (no manual labels); tree-sitter inventory as graph denominator (dangling callees only; false_edge_rate is lower bound, not completeness).

## Cross-domain dependencies

- 081-F eval subsystem gates 082.004-T acceptance (082-F cross-feature dep).
- 077-S CI skip live; downstream doc-only PRs cascade the skip (end-to-end validated).
- 078-S depends_on 081-F core tasks (eval metrics already archived when 078-S ships).
- 084-F absorbs 12 Copilot post-077-S stash findings (resolution_recall, false-edge, TSX gate, threshold, fidelity, corpus, perf, regression). Deferred: reliability (30CE5DD6) + lifecycle-concurrency (2C420C96).
- 085-F DAX references consumer to be determined (currently embedded in TMDL measures only; no symbolic DAX consumer in-repo yet).

## Orchestrator autonomous run summary (July 5)

Single comprehensive run: 4 shipments (071-S/072-S/073-S/074-S) end-to-end, all merged, 0 open PRs final. Adversarial review blocked 3 times with P1 findings; each fix-cycle completed before merge. Hard rules honored: sound judgment on circuit breaker, wait for Copilot + resolve all comments, respect circuit breakers. Pipeline drained (0 queued shipments at close). Main `70760b3`; index rebuilt (572 artifacts).

## Archived originals (traceability)

| File | Summary |
|---|---|
| 2026-07-05-orchestrator-autonomous-run-071-074.md | Autonomous pipeline execution: 4 shipments merged with adversarial gate catching 3 P1s; CI skip live; 072-S DB-divergence guard; 074-S NotReady scope. |
| 2026-07-05-stage-074S-notready-scope-fix.md | Bug-fix planning for 074-S: NotReady shared across startup + respawn; new ShutdownTimeout variant (8010) drops --direct hint on respawn-wait path. |
| 2026-07-10-stage-081F-082F-eval-and-rec1-harvest-session.md | Harvest 2 decided deliberations → exec-plans → plan-review PASS → shipments 077-S (eval subsystem) + 078-S (rec1 calls). |
| 2026-07-11-ship-077S-retrieval-eval-subsystem.md | 077-S shipped eval subsystem: 7 tasks, 29 tests, retrieval + graph metrics, config/MCP/CLI, persistence. Copilot 7-cycle review (cycles 1-3 logic, 4 de-scope, 6 security, 7 deferred). |
| 2026-07-11-ship-078S-rec1-calledges.md | 078-S shipped call-edge resolution: 10 tasks test-first, dual-domain (parsing/staging/storage/export/rehydrate/post-pass/lifecycle/rollback/trigger/acceptance). Copilot 4 cycles. |
| 2026-07-11-ship-rec1-remediation.md | Follow-up remediation: durable rollback marker + active-daemon refusal test; doc corrections on target-correctness gate + peer-language follow-on. |
| 2026-07-11-stage-082-reharvest-rec1.md | Re-harvest after Copilot #239 block: split over-limit tasks into 8 single-width items; propagate full contract; rewire deps; 078-S re-queued. |
| 2026-07-11-stage-084F-retrieval-eval-correctness.md | Consolidate 077-S Copilot post-findings into 084-F: 14 tasks (resolution_recall, false-edge manifest, TSX gate, threshold, fidelity, perf, regression). |
| 2026-07-13-stage-dax-intelligence.md | DAX intelligence (B0E2B374) harvested: 085-F 7 tasks, extractor/carry-column/references/impact-analysis/lint-tier1/lint-tier2/CLI-with-bounded-parity. |
