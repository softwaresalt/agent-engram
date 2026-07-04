---
date: 2026-07-04
agent: Ship
mode: test-first (TDD) + review + PR
shipment: 070-S
feature: 070-F
tasks:
  - 070.001-T
  - 070.002-T
pr: 199
pr_url: https://github.com/softwaresalt/agent-engram/pull/199
branch: 070-tmdl-parser-fixes
status: pr-open-awaiting-merge-approval
base: main
base_commit: 61b548a
head_commit: 17e1985
review_artifact: 070.001-R
plan: docs/exec-plans/2026-07-04-tmdl-parser-correctness-fixes-plan.md
finding: docs/decisions/2026-07-04-tmdl-eval-gate-finding.md
predecessor_shipment: 069-S
---

# Ship — 070-S TMDL safe-parser correctness fixes

## Context

070-S executes the **optional, bounded follow-on** the 069-S eval-gate finding
(`docs/decisions/2026-07-04-tmdl-eval-gate-finding.md`, DECLINE) proposed: fix
the **two incrementally-fixable heuristic bugs** in the SAFE line/indent TMDL
parser (`crates/powerbi-tmdl-parser/src/lib.rs`). No grammar, no new dependency;
crate stays `#![forbid(unsafe_code)]`. The differential harness
`tests/unit/tmdl_differential_eval_test.rs` already **pinned the buggy behavior**,
so TDD was half-done: each task flips its pinned assertion buggy→correct, then the
parser fix makes it green. Manifest `[070-F, 070.001-T, 070.002-T]`; dependency
edge is a **serialization** dependency (shared aggregate + module-doc table), not
a data dependency. Executed 070.001-T → 070.002-T. Branched fresh from `main`
(`61b548a`): `070-tmdl-parser-fixes`.

## What shipped

### 070.001-T — calculated-column expression capture (S-PTM-24 / S-PTM-28) → `ef89a00`
- Added additive `TmdlColumn.expression: Option<String>` (crate has **no serde**;
  `Default` → `None`; purely additive, back-compat).
- Added `parse_column_declaration` mirroring `parse_measure_declaration`
  (`splitn(2, '=')` → `(name, Option<expr>)`).
- **Generalized** the multiline measure-body machinery into a shared
  member-body mechanism: `PendingMeasureBody` → `PendingMemberBody` keyed by the
  existing `TmdlMemberKind` (`Column(idx)` / `Measure(idx)`); `finish_pending_*`
  now `match`es the member kind to write `columns[i].expression` **or**
  `measures[i].expression`. `start_column` opens a body capture **only** when the
  declaration carries an `=` (`has_assignment = rest.contains('=')`) and the
  inline expression is `None` — so plain columns (`column Amount`) never open a
  body and their `dataType:`/`lineageTag:` attach normally.
- Adapter (`src/services/powerbi_tmdl.rs`) reads `column.name/.data_type/
  .lineage_tag` by field access only — adding a field is safe, **no adapter
  change**; the indexed `PowerBiColumn` name corrects automatically (S-PTM-28).
- Aggregate: `heuristic_bugs` 2→1, PASS 3→4 (intermediate, internally consistent).

### 070.002-T — measure-DAX colon heuristic (S-PTM-25) → `3986783`
- `looks_like_tmdl_property` was a bare `trimmed.contains(':')`, so a DAX body
  line `FORMAT ( NOW (), "HH:mm:ss" )` was treated as a property boundary and the
  body capture ended early (expression dropped).
- Refined to require a **property-shaped `key:`**: `split_once(':')`, then the key
  must be a **non-empty bare TMDL identifier** (`is_ascii_alphanumeric() || '_'`).
  `FORMAT ( NOW (), "HH` → has spaces/parens/quote → not a property → body
  captured. `dataType:`, `lineageTag:`, `fromColumn:`, `formatString: "HH:mm:ss"`
  → bare-identifier key → still recognized. One centralized change covers both
  call sites (`should_finish_member_capture` + `capture_member_body_line`).
- Aggregate reconciled to final: **`heuristic_bugs` 1→0, PASS 4→5**, model-richness
  misses unchanged (4), total misses 6→4.

### Backlog state → `17e1985`
- 070-S claimed **active**; 070.001-T/070.002-T/070-F **done + archived** with
  commit provenance (`archived_from`, `archived_status: done`, `commit`).

## Aggregate anchor (s_ptm_29) — final

`heuristic_bugs == 0` confirmed. `differential_gate_counts()` re-derives every
verdict from live `parse_tmdl_document` output; the two `heur_misses` checks are
**kept as live regression guards** (both now `false`), so any regression
re-inflates the count and re-fails the anchor. Gate line now prints
`5 PASS / 4 MISS (4 model-richness, 0 incrementally-fixable heuristic) → recommend
DECLINE` — the DECLINE recommendation stands (remaining misses are all
model-richness gaps).

## Gates (CI feature set — NOT `--all-features`)

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --no-default-features --features cozo-backend,embeddings
  --all-targets -- -D warnings -D clippy::pedantic` — clean (both commits).
- `unit_tmdl_differential_eval` — 11 passed (`heuristic_bugs == 0`).
- `unit_powerbi_extract_tmdl` — 12 passed.
- `integration_powerbi_search_ingestion` — 24 passed (parse→ingest→index path).
- `powerbi-tmdl-parser` lib `#[cfg(test)]` — 7 passed (no regression from the
  member-body generalization; multiline measure + partition + hierarchy tests
  still green).
- **CI (PR #199) build check — PASS** (ubuntu-latest, CI feature set), ~3m26s.
- Copilot review — **COMMENTED, "reviewed 6 of 6 files, generated no comments"**
  (zero line-level threads to resolve).
- Runtime verification — `cargo build --bin engram` (CI features) OK;
  `engram --version`/`--help` exit 0. TMDL parse→ingest→index covered green by the
  24 integration tests.

## Commits (branch base 61b548a)

| SHA | Message |
| --- | --- |
| `ef89a00` | feat(tmdl): capture calculated-column expressions (070.001-T) |
| `3986783` | fix(tmdl): capture measure/column DAX bodies containing colons (070.002-T) |
| `17e1985` | chore(070): record backlog state for TMDL parser-fixes shipment |

## Backlog disposition

- 070.001-T → **done + archived** (`commit: ef89a00`).
- 070.002-T → **done + archived** (`commit: 3986783`).
- 070-F → **done + archived** (both tasks complete).
- 070-S → left **active** (ships post-merge via `backlogit shipment ship`).
- Did **NOT** run `backlogit sync` (union landmine; cache clean, CLI mutations
  atomic). Ran in CLI-fallback degraded mode (no backlogit MCP tools in surface).
- `.gitignore` pre-existing drift left **untouched/unstaged** throughout.

## Landmines / notes for next agent

- **Member-body generalization**: `PendingMemberBody` now serves both columns and
  measures via `TmdlMemberKind`. If adding another member type with a body,
  extend the `match` in `finish_pending_member_body` (compiler will force it).
- **`start_column` body gate**: the `has_assignment = rest.contains('=')` guard is
  what distinguishes a calculated column (`column X =`) from a plain column
  (`column Amount`). Do not drop it, or plain columns would swallow their
  following property lines.
- **`looks_like_tmdl_property` shape rule**: keys are bare identifiers
  (letters/digits/`_`). If TMDL ever gains a property whose key is not a bare
  identifier, this heuristic would misclassify it — revisit then.
- CI runs the **engram root package only** with exactly
  `--no-default-features --features cozo-backend,embeddings --all-targets`. The
  parser crate's own `#[cfg(test)]` runs via `cargo test -p powerbi-tmdl-parser`
  (no features). `cargo dev-test` = `--lib` only → won't run the unit harness.
  NEVER `--all-features` (pulls broken `otlp-export`).
- Copilot review request: `POST /pulls/199/requested_reviewers` with
  `reviewers[]=copilot-pull-request-reviewer[bot]` triggered it (~5 min to post);
  it leaves `reviewRequests` once work starts and the review lands as `COMMENTED`.

## STOP gate

Stopped at the **user-approved-merge gate** (main is ruleset-protected;
merge-commit only, P-009). **Did NOT merge.** Final merge needs operator approval
+ `gh pr merge 199 --merge --admin`. Post-merge closure remaining (deferred until
merge confirmed): `backlogit shipment ship 070-S`, session-memory head-commit
refresh, closure index resync.
