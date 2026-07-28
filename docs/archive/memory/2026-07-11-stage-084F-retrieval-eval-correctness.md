# Stage session — 2026-07-11 — 084-F Retrieval-Eval Correctness Hardening → shipment 079-S

**Agent:** Stage · **Repo:** softwaresalt/agent-engram · **Branch:** `main` @ `0507559` (synced, no code branch)
**Trigger:** Orchestrator directive — produce ONE reviewed, queued shipment for a "Retrieval-Eval
Correctness Hardening" release unit from the 081-F/082-F eval-review stash cluster, then STOP.
**Scope:** planning + backlog decomposition ONLY. No code, no cargo, no PR, no Ship manifest ops.
**Tools:** backlogit MCP + CLI both reachable (TOOL_OK); engram CLI reachable (TOOL_OK). Index synced.

## Deliverables produced

- **Deliberation:** `docs/decisions/2026-07-11-retrieval-eval-correctness-deliberation.md`
- **Plan:** `docs/exec-plans/2026-07-11-retrieval-eval-correctness-plan.md` — plan-review **PASS**
  (0 blocking, 0 critical, 5 advisory all resolved in cycle 1).
- **Feature:** **084-F** "Retrieval-Eval Correctness Hardening (retrieval_eval)"
- **Shipment:** **079-S** (status `queued`, covering_feature = 084-F) — for Ship to claim.

## Backlog hierarchy (all `queued`)

| Task | Concern | Stash | Deps |
|---|---|---|---|
| 084.001-T | report/config model surface (all new fields, additive) | foundation | — |
| 084.002-T | resolution_recall unit + language-scope consistency | D6F70DCC + 88B5FAFD | 001 |
| 084.003-T | resolution_recall index/generation consistency gate | 54848E3D | 001, 002 |
| 084.004-T | false-edge target-correctness (fixture manifest) | D07F0919 (⇄49561F22) | 001 |
| ├ 084.004.001-ST | fixture workspace + expected-target manifest | — | — |
| └ 084.004.002-ST | provenance retention + manifest target assertion | — | 004.001 |
| 084.005-T | canonical TSX language gate | 2894ACB5 | — |
| 084.006-T | thresholds enforced in run_retrieval_eval + report | 14B33F9F (svc) | 001 |
| 084.007-T | thresholds CLI exit-code surfacing | 14B33F9F (cli) | 006 |
| 084.008-T | retrieval-mode fidelity / fallback detection | 00C7F3CC | 001 |
| 084.009-T | semantic corpus completeness (LEFT JOIN + name fallback) | 78AA205D | 001 |
| 084.010-T | semantic eval top-k selection (perf) | 4CF046A5 | 009 |
| 084.011-T | graph eval bounded parsing (memory) | CA401F5F | 003 |
| 084.012-T | graph regression tier exercises real path | F137D72E | 002, 003, 004 |
| 084.013-T | align eval plan doc (narrowed + corrected contract) | 635EE7C0 | 004, 009 |
| 084.014-T | carry-forward: merge-gate compound learning + PR-automation rule | 3A280A4E | — |

## Key decisions

- **Dedup D07F0919 ⇄ 49561F22:** same false-edge lower-bound issue → consolidated into
  **084.004-T**. D07F0919 kept as canonical source; **49561F22 archived** from stash (superseded).
  Link/rationale recorded in the deliberation dedup table + 084.004-T body.
- **resolution_recall cluster (D6F70DCC/88B5FAFD/54848E3D) grouped**, not three divergent patches:
  unit reconciliation (distinct caller,callee) + language-gate join + index-generation staleness
  flag (emit-with-flag, never silent [0,1] clamp).
- **Scope-freeze honored:** all fixes stay in the eval read path (retrieval_eval.rs, tools/eval.rs,
  cozo_queries.rs read queries, models, CLI eval, tests, plan doc). The heavier "persist call-site
  inventory at index generation" fix (touches indexer write path code_graph.rs) is DEFERRED and
  noted on 084.003-T + deliberation.
- **Target-correctness acceptance nuance** (from `2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`)
  folded into the plan + 084.004-T: false_edge_rate is a dangling-only **lower bound**; correctness
  requires manifest target assertions by **exact identity**.
- **Carry-forward 3A280A4E** folded as its own docs/instructions task (084.014-T) to preserve
  width-isolation on the code tasks. Its stash entry was **left active** (belt-and-suspenders;
  content fully captured in 084.014-T).

## Deferred / excluded (left active in stash)

- **30CE5DD6** — connect_db SQLITE_BUSY reliability residual (U015-FLK1): different domain (DB
  reliability), belongs to a reliability shipment. Excluded per operator.
- **2C420C96** — get_workspace_status capability-discovery atomicity: a lifecycle-handler
  (`src/tools/lifecycle.rs`) concurrency concern that only *reads* an eval field; does not fit the
  eval config-surface scope cleanly → left active in stash (would mix lifecycle-concurrency width).
- Harvested core stash entries (D6F70DCC, 88B5FAFD, 54848E3D, D07F0919, 2894ACB5, 14B33F9F,
  00C7F3CC, F137D72E, 4CF046A5, 78AA205D, CA401F5F, 635EE7C0) were **left active** in stash for
  traceability rather than purged (operator gave a narrow archival instruction for only 49561F22).
  They are fully represented as 084.* tasks; a follow-up may archive them.

## Hygiene notes for Ship / next session

- Two pre-existing uncommitted agent-file deletions (`auto-mergeinstall.agent.md`,
  `auto-tune.agent.md`) were **left untouched** (never staged). Use explicit per-file `git add`.
- Merge policy is merge-commit only (P-009/P-011). Stage did not merge/branch/PR.
- 084.014-T source draft preserved at
  `.copilot/session-state/e571e150-14e3-4ff8-b6a0-6290b8c3c0c4/files/2026-07-11-merge-gate-compound-learning-draft.md`.

## Next step

Orchestrator routes **079-S** to **Ship**. Ship claims 079-S, executes each task test-first
(compiling-but-failing harness before code), honoring the dependency order above.
