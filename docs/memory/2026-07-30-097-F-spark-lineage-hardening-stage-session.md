# Session Memory — Stage: 097-F Spark lineage v1 hardening harvest + 099-S assembly

- **Date:** 2026-07-30
- **Agent:** Stage (planning/backlog only — no build/PR/push-main)
- **Branch:** `099-spark-lineage-v1-hardening` (cut from `main` @ `df01b498`)
- **Feature:** 097-F "Spark lineage v1 hardening (deferred PR #284 review findings)"
- **Shipment assembled:** 099-S (status `queued`) — ready for Ship to claim

## Task

Run the Stage pipeline (impl-plan → plan-harden → plan-review → harvest →
shipment assembly) over the five deferred PR #284 hardening items enumerated on
card `097-F` + reference plan
`docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md`.

## Tool state (degraded)

- **backlogit MCP: DOWN** all session ("Transport closed" on every call). Used
  CLI fallback `C:\Tools\backlogit.exe sync` + direct card authoring + read-only
  SQLite queries against `.backlogit/backlogit.db`. Registry
  `.autoharness/backlog-registry.yaml` present → sanctioned CLI/file-backed mode.
- **engram:** daemon green, workspace bound, indexed today. `search` works
  (surfaced compound learnings); **code-graph symbol index only holds test
  fixtures** (`src/a.rs::alpha`, `src/lib.rs::build_widget`) — symbol lookup
  insufficient for real Rust sites, so code sites were confirmed by targeted
  grep + direct file reads (documented fallback order).

## Confirmed real code sites (line numbers vs drifted card)

- **V2** `src/services/parsing/sql.rs` → `normalize_spark_insert` L425,
  `insert_table_prefix_end` L448 (raw-byte scan, not quote/comment-aware).
- **V5** `src/models/lineage.rs` → `resolve_path` L211, `uri_matches_authority`
  L289 (storage-authority prefixes trusted verbatim).
- **W2** `src/models/lineage.rs` → `resolve_table` L167 (split at L177) (no
  component grammar check).
- **W1** `src/services/parsing/python.rs` → `resolve_cell_candidates` ReadBind
  arm ~L1296-1315 (second read into a bound var rebinds instead of invalidating).
- **X1** `src/models/lineage.rs` → `CURRENT_EXTRACTOR_VERSION` L38 = "1.0.0";
  `lineage_freshness_token` L278 (C4 freshness gate).

## Decisions

1. **6 tasks, not 5.** Added **X1** (extractor-version bump + re-extraction
   validation) — tightening extractor output (V2/V5/W2/W1) without bumping the
   freshness stamp leaves stale/false lineage on already-indexed notebooks (C4).
   Its absence would be a P1 review gap. Real fan-in dependency.
2. **V5 and W2 kept separate** despite sharing `lineage.rs` — distinct
   predicates/call-sites/negative-test matrices; single-branch build serializes
   the file edits (no `blocks` edge warranted).
3. **V4 not merged with any code task** (width isolation: test vs code). The
   operator-suggested V4+V5 pairing declined — no shared fixture, cross-domain.
4. **Dependencies:** only the real fan-in encoded — `097.006-T` (X1) blocked by
   `097.001/002/003/004-T`. V2/V5/W2/W1/V4 left parallel.
5. **plan-harden required + done** (freshness/re-extraction rollout + observable
   output-behavior change; ProposedAction/ActionRisk recorded). **plan-review
   gate: PASS** (no P0/P1; P2/P3 recorded: V2 escaped-quote fixture, W1 single-
   caller confirm, V5 query/fragment reject, X1 scope acknowledgment).

## Artifacts created (all on branch 099-spark-lineage-v1-hardening)

- Plan: `docs/exec-plans/2026-07-30-spark-lineage-v1-hardening-plan.md`
  (impl-plan + `## Plan Hardening` + `## Plan Review` PASS).
- Tasks under 097-F: `097.001-T` (V2), `097.002-T` (V5), `097.003-T` (W2),
  `097.004-T` (W1), `097.005-T` (V4), `097.006-T` (X1).
- Shipment: `099-S` (queued; items = [097-F, 097.001..097.006-T], feature first).

## Guardrails honored

- Did NOT build/implement code, run harness-architect/build-feature, create a PR,
  or push main.
- Did NOT touch the still-`active` `097-S` manifest (P-011) — verified `active`
  and unchanged post-session.

## Next steps (for Ship)

1. Claim shipment 099-S.
2. Generate compiling-but-failing TDD harness per task ACs (harness-architect).
3. Build V2/V5/W2/W1/V4 (parallelizable), then X1 last (fan-in).
4. Enforce lineage precision floor (0 false edges) + `lineage_precision_recall_test.rs`.
5. On X1: confirm exactly one `CURRENT_EXTRACTOR_VERSION` edit (1.0.0→1.1.0) and
   re-extraction of stale-stamped notebooks.
