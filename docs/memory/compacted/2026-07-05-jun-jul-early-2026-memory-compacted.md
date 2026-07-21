---
title: June and Early July 2026 Memory Compaction
type: compacted-memory
date: 2026-07-05
compacted-from:
  - docs/memory/2026-06-12/tmdl-expression-slice-memory.md
  - docs/memory/2026-06-12/tmdl-parser-crate-start-memory.md
  - docs/memory/2026-06-15/ship-050-S-post-merge-closure-memory.md
  - docs/memory/2026-06-30/065-F-staging-session-memory.md
  - docs/memory/2026-07-01/ship-052-S-verify-cli-rereview-session-memory.md
  - docs/memory/2026-07-01/ship-052-S-verify-cli-session-memory.md
  - docs/memory/2026-07-01/ship-session-memory.md
  - docs/memory/2026-07-05/orchestrator-usage-measurement-and-f1-fix-memory.md
archived-to: docs/archive/memory/
---

# Compacted Memory - June and Early July 2026

## Summary

This period advanced the Power BI TMDL/PBIP track, closed PBIP project
definition indexing, staged and shipped documentation for daemonless `--direct`,
delivered the `engram verify` CLI through several review-remediation rounds, and
completed the 075-S/076-S autonomous AFK run. Major durable lessons involved safe
parser boundaries, additive telemetry report surfaces, Copilot re-review
circuit-breakers, backlog ID collision avoidance, explicit-path staging, and
merge-gated PR discipline.

## TMDL parser and Power BI expression work

* `tmdl-parser-crate-start-memory.md` created the internal
  `powerbi-tmdl-parser` workspace crate for feature `064-F` / stash `59039891`.
  Files changed: `Cargo.toml`, `Cargo.lock`,
  `crates/powerbi-tmdl-parser/Cargo.toml`,
  `crates/powerbi-tmdl-parser/src/lib.rs`, `src/services/powerbi_tmdl.rs`,
  `src/services/powerbi_indexer.rs`, unit and integration tests, and
  `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md`
* Decisions: start with a safe internal crate boundary, avoid tree-sitter FFI
  until a constitution-compliant safety story exists, use fixture-driven parsing
  first, close relationship and multiline-measure fixture gaps, keep ref-only
  `model.tmdl` shells indexable, and emit first-class
  `powerbi_semantic_model` records for JSON and TMDL models
* `tmdl-expression-slice-memory.md` extended the same track with top-level TMDL
  expressions, JSON parity, and graph wiring. Files changed:
  `crates/powerbi-tmdl-parser/src/lib.rs`, `src/models/powerbi.rs`,
  `src/models/powerbi_graph.rs`, `src/services/powerbi_tmdl.rs`,
  `src/services/powerbi_extract.rs`, `src/services/powerbi_indexer.rs`,
  `src/db/cozo_queries.rs`, and targeted tests
* Decisions: model expressions as shared semantic-model entities, emit
  `powerbi_expression` content records plus `expression` graph nodes, and keep
  Power BI-specific parsing outside the generic code-graph parser pipeline.
  Follow-ups remained for partition blocks, richer data-source properties, and a
  safe grammar-backed parser strategy

## PBIP project-definition closure

* `ship-050-S-post-merge-closure-memory.md` closed `050-S` / `062-F` after PR
  `#177` merged with merge commit `275faa4`. The repository was verified to allow
  merge commits only, and the merge used `gh pr merge 177 --merge --admin` after
  operator approval
* Scope completed tasks `062.002-T`, `062.005-T`, `062.006-T`, and `062.007-T`.
  Implementation commits extracted `.pbip` workspace and `.pbir` linkage
  entities, page/visual entities, PBIP content records and graph edges, boundary
  docs, a `report_display_name` char-boundary panic fix, snapshot-cap borrowing in
  `build_model`, and skipped-file counter correctness
* Durable learnings: `pbip` and `powerbi` are independent content-source
  boundaries; migration from `powerbi` is deferred, not deprecated; skipped files
  must be excluded from ingested/unchanged counts; borrowing snapshot content
  avoids redundant allocation on whole-project rebuilds

## Daemonless `--direct` docs staging and shipping

* `065-F-staging-session-memory.md` staged stash `20477A6A` into feature `065-F`
  and shipment `053-S`. Plan-review passed after resolving a missing Constitution
  Check P1. Tasks `065.001-T`, `065.002-T`, and `065.003-T` formed the docs
  shipment; `065.004-T` was deferred as a separate Rust code task
* Decisions: skip fresh deliberation because `010-D` already settled the shipped
  escape hatch, model the chore as a feature with labels, keep docs and code split
  for width isolation, use `configuration.md` as the canonical anchor, and do not
  propagate an outdated closure-note recommendation to replace
  `BoolishValueParser`
* `ship-session-memory.md` recorded PR `#187` shipping `053-S`: docs were grounded
  against `src/bin/engram.rs` (`Sync { full, direct }`, `Index { direct }`,
  `ENGRAM_DIRECT` binding). Files covered `docs/configuration.md`,
  `docs/troubleshooting.md`, `README.md`, `start.ps1`, `start.sh`, closure note
  `docs/closure/2026-07-01-053-S-daemonless-direct-docs-closure.md`, and backlog
  artifacts. Copilot fixes corrected README's daemon-management implication and
  added archive provenance. The operator-directed `start.ps1 --timeout 3000`
  drift was preserved and described as intentional

## Verify CLI delivery and re-review remediation

* `ship-052-S-verify-cli-session-memory.md` delivered `engram verify <path>` for
  `064-F` / `052-S` in PR `#185`, stopping before operator-gated merge. It added
  `src/services/verify.rs`, `src/cli/commands/verify.rs`, CLI wiring in
  `src/bin/engram.rs`, module exports, three `[[test]]` entries in `Cargo.toml`,
  unit/contract/integration tests, fixtures, and closure
  `docs/closure/2026-07-01-052-S-engram-verify-cli-closure.md`
* Lessons: backlog IDs `064-F` and `064.001-T` through `064.004-T` collided with
  archived Power BI TMDL items on `main`, so verify task specs stayed in
  `.backlogit/queue/` with `status: done`; use
  `git merge-tree --write-tree --name-only origin/main HEAD` to detect add/add
  conflicts before pushing. Explicit-path staging isolated 052-S artifacts from a
  29-file dirty tree. Local Windows and Ubuntu CI flakes were triaged with
  isolation runs and CI reruns rather than code changes
* Operator later accepted a containment bug: relative paths were read relative to
  CWD rather than `--workspace` / `ENGRAM_WORKSPACE`. Test-first fix introduced
  `ResolvedTarget { read, display }`, joined relative paths under canonicalized
  workspace roots, rejected `..`, preserved display strings, and returned exit `2`
  on resolve/canonicalize errors. Windows lesson: canonicalize both operands for
  `starts_with` containment because verbatim prefixes and 8.3 names otherwise
  break comparisons
* `ship-052-S-verify-cli-rereview-session-memory.md` handled re-review nits on
  `src/cli/commands/verify.rs`. Nit 2 added RED test
  `nonmarkdown_missing_file_exits_error` and fixed non-markdown missing targets to
  exit `2` before format branching. Nit 3 documented the global `--quiet` summary
  suppression exception. Later doc fixes updated the integration-test module
  header and clap help after behavior changed
* Review mechanics: each substantive push can trigger new Copilot findings; fix
  only clearly trivial in-scope findings, reply/decline false positives and
  out-of-scope gaps, and stop at the review-fix breaker. Copilot's claimed
  compile failures must be verified against actual fmt/clippy/test results. Use
  `gh api` replies and GraphQL thread resolution after pushing a fix; avoid `gh`
  `-b @file` reply bodies that can post literal file references

## Backlog reconcile and PR hygiene

* `ship-session-memory.md` also recorded PR `#186`, a chore branch bundling the
  064-to-066 ID-namespace collision reconcile and 052-S closure move. It staged
  only the intended concerns via explicit pathspecs, verified cached diffs, and
  fixed a real Copilot finding by removing a spurious `011-D related_to` link from
  `066-F.md`
* Drift discipline across PRs: `.cursor/mcp.json`, `.github/copilot-instructions.md`,
  `.backlogit/memories.json`, `.backlogit/telemetry.jsonl`, `.claude/`, and
  `docs/design-docs/.gitkeep` were never staged. Backlogit sync can self-heal
  collision-branch archives by stripping `archived_from` and re-injecting stale
  links; avoid backlog mutations on branches where they redirty out-of-scope
  files

## Autonomous usage measurement and NotReady hint run

* `orchestrator-usage-measurement-and-f1-fix-memory.md` completed AFK shipments
  `075-S` and `076-S` under operator-granted merge authority. PRs `#213`, `#214`,
  `#215`, and `#216` all merged, leaving 0 open PRs and 0 active/queued
  non-blocked items at `main` `409bd53`
* `075-S` / `073-F` closed the EMISSION-to-MEASUREMENT gap. Files included
  `src/models/metrics.rs`, `src/tools/read.rs`, `src/tools/mod.rs`,
  `docs/design-docs/engram-usage-telemetry-consumption-contract.md`, and
  decision `018`. `MetricsSummary` gained scalar counts
  `unique_tools_exercised` and `distinct_correlation_ids`; heavy
  per-correlation data stays off the summary and is surfaced only by
  `get_token_savings_report`. Review fixed two high-confidence P1 findings by
  excluding dead `session_count` and confining `by_correlation_id` to the report
  tool
* `076-S` / `074-F` reworded `DaemonError::NotReady` in `src/errors/mod.rs`.
  Decision: prefer precise text over lock-probing because `poll_until_ready`
  lacks workspace path, endpoint-to-path reversal is not cross-platform, and
  `DaemonLock::acquire` can delete lock and PID files as part of stale-PID
  recovery. Wire code `8006` / `DaemonNotReady` stayed unchanged
* Confirmed CI feature set is `--no-default-features --features
  cozo-backend,embeddings`; never use `--all-features` because it breaks on
  `otlp-export/observability.rs`. `main` is ruleset-protected, so closure commits
  must go through branch plus PR and merge commits only. Known flakes: Windows
  daemon TTL CozoDB lock and Ubuntu `integration_markdown_indexing::t030_003`
* Backlogit gotchas: `--status done` task creation archives without
  `archived_from`/`archived_status`; `shipment ship` normalizes manifest items but
  requires the shipment to be claimed first; avoid reflexive `backlogit sync` when
  hand-edited archived markdown would resurrect stale data. Remaining deferred
  work: DAX tree-sitter stash `F7E89921`, blocked `025-S` / `041-F` / `041.001-T`,
  and blocked `033.005-T`

## Consolidated originals

These verbose originals were consolidated here and moved to `docs/archive/memory/`:

* `docs/memory/2026-06-12/tmdl-expression-slice-memory.md`
* `docs/memory/2026-06-12/tmdl-parser-crate-start-memory.md`
* `docs/memory/2026-06-15/ship-050-S-post-merge-closure-memory.md`
* `docs/memory/2026-06-30/065-F-staging-session-memory.md`
* `docs/memory/2026-07-01/ship-052-S-verify-cli-rereview-session-memory.md`
* `docs/memory/2026-07-01/ship-052-S-verify-cli-session-memory.md`
* `docs/memory/2026-07-01/ship-session-memory.md`
* `docs/memory/2026-07-05/orchestrator-usage-measurement-and-f1-fix-memory.md`
