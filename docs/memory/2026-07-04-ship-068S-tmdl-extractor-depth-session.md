---
date: 2026-07-04
agent: Ship
mode: build (TDD) + review + PR
shipment: 068-S
feature: 068-F
pr: 192
pr_url: https://github.com/softwaresalt/agent-engram/pull/192
branch: 068-tmdl-extractor-depth
status: pr-open-awaiting-merge-approval
base: main
review_artifact: 068.001-R
plan: docs/exec-plans/2026-07-03-tmdl-extractor-depth-plan.md
---

# Ship — 068-S TMDL extractor depth (partitions, datasource props, lineage)

## Context

Shipment 068-S deepens the Power BI TMDL extractor along three
dependency-ordered tasks. Branch `068-tmdl-extractor-depth` already carried the
Stage planning commits (`404d07e`, `c6f61fa`) off main `6138bc0`; this Ship
session stacked the implementation on top (no rebranch). Strict TDD (red→green)
per task. Blocked follow-on `066.008-T` left blocked (out of shipment).

## Tasks — all done

| Task | Disposition | Commits |
| --- | --- | --- |
| 068.001-T partitions + fenced-M capture + `PowerBiNodeKind::Partition` | done | `02f10af` (red), `37d2d41` (green) |
| 068.002-T richer data-source props + `powerbi_data_source` summary | done | `e39ecc7` (red), `e384b4f` (green) |
| 068.003-T refs/annotations/lineage/culture/defaultMode | done | `c897fb3` (red), `955f2d3` (green) |

Additional commits:
- `143490b` fix(tmdl): P2 review fix — hierarchy/level-nested metadata skip window.
- `c882f79` chore(backlog): archive 068.001/002/003-T done; 068-S active.
- `dfd9516` docs(powerbi): correct partition source-body normalization docs (Copilot review).

068.001-T did NOT need the pre-authorized split — it landed as one red+green pair
within budget.

## Design notes

- Additive-only: NO CozoDB schema migration. `PowerBiNodeKind::Partition` is a
  new string variant in the existing `powerbi_node` relation; round-trip arm
  added in `parse_powerbi_node_kind` (`src/db/cozo_queries.rs`). All new model
  fields are `#[serde(default)]`.
- New parser types `TmdlAnnotation`/`TmdlRef` + additive fields on
  model/table/column/measure; mirrored across `src/models/powerbi.rs`, adapter
  `src/services/powerbi_tmdl.rs`, JSON path `src/services/powerbi_extract.rs`,
  and multi-fragment merge `src/services/pbip_tmdl.rs`.
- Indexer folds culture/lineage/refs/annotations into semantic-model/table/
  measure summaries and emits `powerbi_partition` + `powerbi_data_source`
  summary records + graph nodes/`Contains` edges.

## Review gate

1. **code-review agent** (diff `6138bc0...HEAD`) found ONE genuine P2: `hierarchy`/
   `level` were in `is_declaration_line()` but had no handler arm, so their nested
   `lineageTag:`/`annotation` overwrote the preceding column/measure metadata
   (fires on ubiquitous auto date tables). FIXED (not deferred — task 003 is about
   correct metadata attachment) via an indent "skip window" (`skip_below_indent`
   + `MetadataTarget::Skip`) that clears member scope on an unmodeled block and
   drops its deeper metadata, cleared on dedent. Regression test added. Extracted
   `start_table`/`enter_unmodeled_member_block` helpers to stay under
   clippy::too_many_lines (100). See compound doc
   `docs/compound/tmdl-declaration-keyword-without-handler-misattributes-nested-metadata-2026-07-04.md`.
2. **Copilot review** (auto, `copilot-pull-request-reviewer`): 2 inline comments,
   both the same doc-accuracy nit — partition `source` M body docs said "verbatim"
   but `capture_partition_source_line` trims each line and drops blanks. Fixed the
   docs (not the behavior — normalization is relied on by tests) in `dfd9516`,
   replied to both comments, and resolved both threads via GraphQL
   `resolveReviewThread`. Review-fix cycles used: 1 (P2) + 1 (docs), well under the
   circuit breakers.

## Quality gates (CI feature set — NOT --all-features)

- `cargo fmt --all -- --check` — clean
- `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` — clean
- Tests: parser crate 7, lib powerbi 9, `unit_powerbi_extract_tmdl` 12,
  `integration_powerbi_search_ingestion` 24 — all green.
- **CI (PR #192, ubuntu-latest, `build` check)**: PASS on both `c882f79` and
  `dfd9516` (~3m each).

## Runtime verification

`integration_powerbi_search_ingestion` (real cozo-backend TMDL ingestion via
TempDir/connect_db/index_powerbi_source/select_content_records): **24/24 pass**
at HEAD `dfd9516`, and green on Ubuntu CI. Partition/data-source/model-metadata
ingestion exercised end-to-end.

## Landmines encountered / re-confirmed

- Do NOT use `--all-features` — pulls in the pre-existing-broken
  `otlp-export`/`observability.rs` (opentelemetry 0.26). CI uses exactly
  `--no-default-features --features cozo-backend,embeddings --all-targets`.
- `cargo dev-test` == `--lib` only; would MISS the integration/contract tests.
  Always validate with the full CI feature-set command.
- Windows-only non-regression flakes (both pass on Ubuntu CI, I touch no
  daemon/db/telemetry code): `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
  (cozo SQLite `database is locked`), and a full-suite telemetry-parallelism
  contract flake (`c017_03_agents_have_required_subfields`) that passes in
  isolation. Kill orphaned `engram` processes between local runs.
- Requesting Copilot as reviewer: `gh pr edit --add-reviewer Copilot` fails with
  `'' not found`; the working path is REST
  `POST /pulls/{n}/requested_reviewers -f "reviewers[]=Copilot"`. Copilot then
  auto-reviews and the request drops from `reviewRequests`.
- `backlogit` commit tracking is `backlogit update <id> --commit <sha>` (there is
  NO `track-commit` subcommand). Did NOT run `backlogit sync` (union landmine).

## Open items / STOP gate

- **STOPPED at the user-approved-merge gate** — did NOT merge. main is protected
  by a ruleset ("Changes must be made through a pull request"); the eventual merge
  needs operator approval + `gh pr merge --merge --admin` (merge-commit only,
  P-009; squash/rebase disabled).
- Post-merge remaining (NOT done, gated on merge): confirm merge SHA in
  `origin/main`, `backlogit_ship_shipment` 068-S → done, closure index resync.
- `.gitignore` working-tree drift (unrelated operator change) intentionally left
  unstaged/uncommitted throughout.
- Follow-on `066.008-T` remains intentionally blocked.
