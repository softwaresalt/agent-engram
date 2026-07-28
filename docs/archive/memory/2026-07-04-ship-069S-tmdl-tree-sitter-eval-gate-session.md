---
date: 2026-07-04
agent: Ship
mode: measurement (test-first) + decision-gate + review + PR
shipment: 069-S
feature: 069-F
task: 066.008-T
pr: 196
pr_url: https://github.com/softwaresalt/agent-engram/pull/196
branch: 069-tmdl-tree-sitter-eval-gate
status: pr-open-awaiting-merge-approval
base: main
review_artifact: 069.001-R
plan: docs/exec-plans/2026-07-04-tmdl-tree-sitter-eval-gate-plan.md
finding: docs/decisions/2026-07-04-tmdl-eval-gate-finding.md
recommendation: DECLINE
head_commit: a1fa1a8
base_commit: fa2e1c3
---

# Ship — 069-S TMDL tree-sitter evaluation gate (066.008-T)

## Context

069-S is a **DECISION GATE, not a grammar build**. Manifest `[069-F, 066.008-T]`;
the only executable task was **066.008-T**: a **measurement-only**
differential-evaluation harness quantifying where the current *safe* line/indent
TMDL parser (`powerbi_tmdl_parser::parse_tmdl_document`) drops / truncates /
mis-scopes real TMDL structure, then a recorded FINDING with a decline-vs-promote
recommendation.

Hard boundaries honored: **NO new dependency, NO grammar, NO tree-sitter, NO
production parser change.** Crate stays `#![forbid(unsafe_code)]`. Ship executes
the measurement + records the finding; Ship does **NOT** create grammar tasks or
decide 069-F's fate (promotion, if warranted, is a follow-on Stage cycle).

Stage did NOT pre-create a branch this cycle — Ship branched fresh from `main`
(`fa2e1c3`): `069-tmdl-tree-sitter-eval-gate`.

## Deliverable

- `tests/unit/tmdl_differential_eval_test.rs` — differential harness, **11 tests
  (S-PTM-20..29)**. Inline `r"..."` fixtures continuing the `S-PTM-0x`/`S-PTM-2x`
  pattern; each run through `parse_tmdl_document` (parser crate) and, for the
  ingestion-impact case (S-PTM-28), through
  `engram::services::powerbi_tmdl`. Every verdict derived from **live** parser
  output; the aggregate gate (`s_ptm_29`) re-derives its counts so a fidelity
  change fails the anchor and forces the finding to be re-derived.
- `docs/decisions/2026-07-04-tmdl-eval-gate-finding.md` — recorded FINDING
  (conclusion: **decline**, confidence: high).
- `Cargo.toml` — registers `[[test]] name = "unit_tmdl_differential_eval"` so the
  harness runs under the root-package CI feature set.

## Measured delta (ground truth: 3 PASS / 6 MISS)

| Construct | Verdict | Failure mode | Class |
| --- | --- | --- | --- |
| `model.tmdl` metadata + 5 `ref`s | PASS | — | — |
| block relationship endpoints (quotes/`'Date'` normalized) | PASS | — | — |
| complex table core (columns+dataType, multiline DAX measure, fenced-M partition, scoped annotations/lineageTags) | PASS | — | — |
| relationship qualifiers (`isActive`/`crossFilteringBehavior`/`joinOnDateBehavior`) | MISS | dropped | model-richness |
| `hierarchy`/`level` member blocks | MISS | dropped (clean, no corruption) | model-richness |
| `calculationGroup`/`calculationItem` | MISS | dropped | model-richness |
| RLS `role`/`tablePermission` | MISS | dropped | model-richness |
| calculated column (`column X = <DAX>`) | MISS | mis-scoped (name absorbs `= DAX`) | heuristic bug |
| measure DAX containing `:` (e.g. `FORMAT(.., "HH:mm:ss")`) | MISS | truncated (body dropped) | heuristic bug |

## FINDING + recommendation: DECLINE

The 6 misses decompose into **4 model-richness gaps** (a grammar does not close
these on its own — they need `TmdlModel` type + adapter extensions, independent
of parse technology) plus **2 incrementally-fixable heuristic bugs**
(calc-column mis-scope; colon-in-DAX truncation via the
`looks_like_tmdl_property` "contains `:`" shortcut at
`crates/powerbi-tmdl-parser/src/lib.rs:1128`). **None is a material,
hard-to-fix-incrementally mis-parse.** Per the baked-in decision rule → the safe
parser is sufficient, a tree-sitter grammar is not ROI-positive, and **069-F is
retirable**. Boundary: retirement/promotion is a Stage decision, not taken here.

## Pipeline / gates

- fmt (`cargo fmt --all -- --check`) — clean.
- clippy (`--no-default-features --features cozo-backend,embeddings --all-targets
  -- -D warnings -D clippy::pedantic`) — clean.
- harness (`--test unit_tmdl_differential_eval`) — 11 passed; gate line prints
  `3 PASS / 6 MISS (4 model-richness, 2 incrementally-fixable heuristic) →
  recommend DECLINE`.
- **CI (PR #196) build check — PASS** (ubuntu-latest, CI feature set), 3m25s.
- Copilot review — **COMMENTED, "reviewed 5 of 5 files, generated no comments"**
  (zero line-level threads to resolve).

## Review remediation (self code-review agent, before PR)

- **P1 (High)**: `s_ptm_21_relationship_qualifiers_dropped` was tautological
  (`assert_eq!(inactive.from_table, active.from_table)` = `"Sales"=="Sales"`).
  Rewritten to pin the loss via `format!("{model:#?}")` NOT containing
  `bothDirections`/`datePartOnly`/`false`.
- **P2 (Medium)**: `s_ptm_29` asserted over a hardcoded array. Rewritten to
  re-derive every gate count from live `parse_tmdl_document` output via a
  `differential_gate_counts()` helper.
- Fixed as commit `a1fa1a8`. Extraction also resolved a
  `clippy::too_many_lines` (160→118→under-100) on `s_ptm_29`.

## Commits (branch base fa2e1c3)

| SHA | Message |
| --- | --- |
| `a0f356b` | test(069): harness + finding + Cargo target |
| `f2e2ba5` | chore(069): claim 069-S active; complete + archive 066.008-T |
| `a1fa1a8` | test(069): address review — live-derive S-PTM-21/29 verdicts |

## Backlog disposition

- 066.008-T → **done + archived** (`.backlogit/archive/066.008-T.md`:
  `status: archived`, `archived_status: done`, `archived_from`, `commit: a0f356b`).
- 069-S → left **active** (post-merge disposition per finding is closure-time).
- 069-F → **untouched** (retirement is a follow-on Stage cycle).
- Did NOT run `backlogit sync` (union landmine; cache clean, CLI mutations
  atomic). Ran in CLI-fallback degraded mode (no backlogit MCP tools in surface).
- `.gitignore` pre-existing drift left **untouched/unstaged** throughout.

## Landmines / notes for next agent

- Root `cargo test` operates on the **engram root package only** (members
  `[".", "crates/powerbi-tmdl-parser"]`; feature flags prove it). Parser-crate
  `#[cfg(test)]`/`tests/` do NOT run in CI — the harness MUST live under root
  `tests/` with a `[[test]]` registration. A root integration test CAN
  `use powerbi_tmdl_parser` (normal dep, Cargo.toml:66) and
  `engram::services::powerbi_tmdl`.
- `cargo dev-test` = `test --lib` only → will NOT run this harness. Always
  validate with the full CI feature-set `--all-targets` command. NEVER
  `--all-features` (pulls broken `otlp-export`/`src/server/observability.rs`).
- rustfmt expands multi-field tuple-array literals to one-line-per-field; that
  repeatedly tripped `too_many_lines`. Fix = extract derivation into a helper +
  small boolean arrays instead of a wide row literal.
- Raw-string gotcha: fixtures with `"#` (e.g. `"#,0"`) need `r##"..."##`.
- Copilot review request: `suggestedActors(CAN_BE_ASSIGNED)` did NOT list
  Copilot, but `POST /pulls/{n}/requested_reviewers` with
  `reviewers[]=copilot-pull-request-reviewer[bot]` DID trigger it (timeline
  `copilot_work_started` → `reviewed` ~5 min later). It moves out of
  `reviewRequests` once work starts.

## STOP gate

Stopped at the **user-approved-merge gate** (main is ruleset-protected;
merge-commit only, P-009). `mergeStateStatus: BLOCKED` (awaiting approval).
**Did NOT merge.** Final merge needs operator approval +
`gh pr merge 196 --merge --admin`.
