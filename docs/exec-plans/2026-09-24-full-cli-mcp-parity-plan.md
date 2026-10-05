---
title: "Full CLI parity for every user-facing MCP tool"
description: "Implementation plan closing every tool-level and parameter-level MCP-to-CLI gap and retiring the MCP-only allowlist"
source: "docs/decisions/2026-09-24-cli-parity-all-user-facing-tools-deliberation.md"
source_stash_ids:
  - "E06BABAD"
deliberation_id: "035-D"
related_deliberation_id: "036-D"
supersedes: "090.004-T"
revision: 3
status: "review-failed-circuit-open"
execution_ordering: "path-s-after-142-s (stale: 142-S still active, planned to be abandoned at H3 per Revision 18; Stage reading pending operator confirmation: re-gated to after 142-F closes, CP-FINAL)"
---

> **Status (2026-10-04, PR #410 review): ordering gate needs re-sequencing.** "142-S ships first" will not be met:
> 142-S has not shipped and is still active, and `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` Revision 18
> plans to abandon it at H3 (PS-5, after H1, conditional on the probe) and move its scope to one-task `142-F` slots.
> Stage reading, for operator confirmation: Path S is re-gated per Revision 18 to
> "after `142-F` closes (CP-FINAL)". This plan is still `review-failed-circuit-open` and not harvested; any further review
> needs explicit operator authorization.

## Source and Approval

* **Source:** `docs/decisions/2026-09-24-cli-parity-all-user-facing-tools-deliberation.md`
  (backlog deliberation `035-D`, stash `E06BABAD`). On 2026-09-24 the operator
  approved full parity as a product requirement, via Option A: one covering
  feature. The deliberation artifact records that decision and points
  `promoted_to` at this plan.
* **Ordering decision (final, operator, 2026-09-24 ~16:38): Path S.** The
  operator said "CLI parity cannot exist prior to ship; we need defer that to
  another shipment". 142-S ships first. Parity is a separate, later shipment
  built from this plan and existing stash `E06BABAD`. The earlier interim
  directive ("plan CLI parity before 142-S continues") is superseded. See
  [Execution Ordering](#execution-ordering-operator-gated).
* **Relationship to 036-D:** the harness-policy decision `036-D` (Option 2,
  durable) is recorded separately. This feature does not depend on it, because
  every unit below has a real executable red harness, including the docs and
  CI-config units.

## Problem Frame

### Tool-level gaps

Three user-facing MCP tools declare `IPC_AND_MCP` instead of `IPC_CLI_MCP` in
`src/tools/capabilities.rs`, so they have no CLI command:

| MCP tool | Capability | read_server_available | Build |
|---|---|---|---|
| `get_retrieval_eval_report` | Read | true | default |
| `query_changes` | Read | true | `git-graph` feature |
| `index_git_history` | Write | false | `git-graph` feature |

### Parameter-level gaps

Two parameter-level gaps exist. The Stage audit found them and the
Agent-Native Parity Reviewer confirmed them independently: all 21 default tool
schemas and both `git-graph` schemas were compared against the clap variants
and parameter builders.

* **`impact_analysis.powerbi_node_id`.** `engram impact <SYMBOL>` has a
  required positional. The handler in `src/tools/read.rs:786-808` works like
  this:
  * a non-blank `powerbi_node_id` wins over `symbol_name`
  * if neither is given, the handler returns `InvalidParams`
* **`flush_state.force`.** `engram flush` sends no params. The handler ignores
  `force` today.

### Guards that currently encode the gaps

* In `tests/contract/lint_dax_cli_parity_test.rs`:
  * `MCP_WITHOUT_CLI_ALLOWLIST` and `mcp_gap_allowlist_matches_documented_gap_rows`
  * the "MCP-only"/"daemon-only" acceptance branch of
    `cli_help_and_mcp_catalog_reference_canonical_doc`
* In `tests/contract/tool_descriptor_registry_test.rs`:
  `retrieval_eval_report_is_declared_mcp_only`, which asserts no CLI surface
  (at roughly lines 337-352).
* In `src/shim/tools_catalog.rs`: the `mcp_only_desc!` macro (roughly lines
  61-68) and the "MCP-only daemon surface" description text.
* In `src/tools/capabilities.rs`: the "No approved requirement adds a CLI
  surface" comment.
* In `docs/cli-mcp-parity.md`: the gap rows and the "MCP tools without CLI
  commands" section.
* In `docs/mcp-tool-reference.md`: the MCP-only wording at line 60.

### Guard semantics this plan corrects

`cli_tool_catalog_parity_test.rs` (F23) checks only that every method the CLI
invokes is declared `Cli`. `the_cli_surface_equals_the_declared_cli_descriptors`
compares the registry with itself. Nothing catches a tool that is declared
`Cli` but has no command.

The F23 scraper also reads `src/cli/**/*.rs` as plain text and ignores
`#[cfg]`. And the `lint_dax` help-resolution check runs `engram <cmd> --help`
for every mapped doc row, so a feature-gated command fails in the default
build.

### Planning-artifact sweep

Stage checked every non-archived execution plan and every queue item for text
that declares a user-facing tool MCP-only. None was found:

* `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md` item 10
  already carries the requirement amendment.
* The `lint_dax` mentions in `docs/exec-plans/2026-07-13-dax-intelligence-plan.md`
  are a historical review record of a gap that has since been closed.
* The 142.058-T text is the amendment itself.

Stage also swept `README.md`, `AGENTS.md`, and the top-level `docs/*.md`
files. The only hits were `docs/cli-mcp-parity.md` and
`docs/mcp-tool-reference.md`, and U9 fixes both. H-DOCS therefore cannot stay
red because of a file this plan does not cover.

## Requirements Trace

| Requirement (035-D / E06BABAD) | Units |
|---|---|
| Every user-facing MCP descriptor declares CLI | U1, U3 |
| A CLI command exists for each MCP-only tool | U2, U4, U5 |
| Every MCP parameter has a CLI equivalent | U6, U7 |
| A drift guard fails if an MCP-only user-facing tool or parameter is added | Harness H-PARAM, H-F23R, H-DOCS; U8 makes the `git-graph` lane gate CI |
| No doc, descriptor comment, or catalog description says MCP-only | U1, U3 (source); U9 (docs); H-DOCS token guard |
| No plan says MCP-only | Stage sweep above (clean); H-DOCS guards operator docs from now on |
| Internal `_health` and `_shutdown` are excluded | By construction: the guards iterate only descriptors that declare `StdioMcp`, and these two do not |
| 090.004-T is superseded | Stage archives it at harvest |

## Harness Specification (Ship Step 2 harness-architect)

The harness-architect writes and registers these as red harnesses before any
implementation, as P-002/P-004 require. Registering the `[[test]]` targets in
`Cargo.toml` is part of the harness output, not a separate task (per the
compound lesson on deferring granularity detail to the harness-architect).

### P-004 red-set rule

* **New harness files contain only test functions that fail at the red
  commit.** This satisfies P-004's "every test function" rule.
* **Guard hardening added to pre-existing targets is not in the red set.**
  This covers the H-F23R reverse check, the H-REG invariant, and the
  no-regression assertions. Those additions pass at the red commit, and the
  harness manifest says so explicitly.
* **Every unit has at least one red function in the manifest.** Its failure
  is recorded with exit codes and `FAILED` lines, not the tail of the output.

### Lanes

* **Default lane:** `cargo dev-test`.
* **`git-graph` lane:**
  `cargo test --no-default-features --features cozo-backend,embeddings,git-graph --test <target>`.
* **At red time**, run the `git-graph` lane over the existing targets that
  this feature touches. Record failures that already exist there separately
  from the listed red markers, and never use `continue-on-error`.
* **Coverage oracle:** add the new targets to the test-coverage-oracle
  manifest (`scripts/test-coverage-oracle`, completeness mode).

### Shared support: `tests/helpers/cli_argv.rs` (new, used through `tests/helpers/mod.rs`)

This module holds:

* `MCP_TOOL_CLI_ARGV`, which maps each tool to its argv path, for example
  `get_retrieval_eval_report → ["report", "retrieval-eval"]`
* `GIT_GRAPH_TOOLS`, the cfg-gated names
* the rename table

These are the single test-side source of truth, shared by:

* H-PARAM
* H-F23R (the `GIT_GRAPH_TOOLS` exemption)
* H-DOCS (checks that the doc rows agree with this map)
* F54's CLI dispatch, if needed. See
  [Execution Ordering](#execution-ordering-operator-gated).

### H-PARAM: `tests/contract/cli_mcp_parameter_parity_test.rs` (new)

* **`every_mcp_tool_has_cli_parameter_parity`** is table-driven over
  `surface_names(StdioMcp)`. It asserts completeness of `MCP_TOOL_CLI_ARGV`,
  that each descriptor declares `Cli`, and that every `inputSchema` property
  appears in the argv's own rendered `--help`.
  * Global flags from `src/cli/flags.rs` are excluded: `--id`, `--timeout`,
    `--workspace`, `--format`, `--json`.
  * Red until U7 (default lane) and U5 (`git-graph` lane).
* **Per-gap functions** for attribution:
  * default lane: `retrieval_eval_report_*` (U1/U2), `impact_powerbi_node_id`
    (U6), `flush_force` (U7)
  * `git-graph` lane, under `#[cfg(feature = "git-graph")]`: `query_changes_*`
    (U3/U4), `index_git_history_*` (U3/U5)
* **Matching rules:**
  * default `snake_case → --kebab-case`
  * positionals are rendered `<UPPER>` or `[UPPER]`, with default
    `snake_case → UPPER`
  * the rename table: `symbol_name→SYMBOL`, `scope_to_symbol→--scope-to`,
    `file_path→--file`, `name_prefix→--prefix`, `node_type→--type`,
    `model_path→MODEL`, `branch_name→--branch`, `compare_to→--compare`,
    `path→PATH`, `query→QUERY`
* Match only rendered tokens, because of the clap `long=`/`name=` pitfall.
* **Proxy limit (stated honestly):** this checks that every parameter is
  **exposed**. The per-unit harnesses (H-RETR, H-GIT, H-PARAM2) check that
  new parameters are **forwarded**. Existing mapped tools are already covered
  by the CLI suites (`cli_envelope`, `lint_dax`). Checking enum values and
  defaults is the P2 follow-up stash.
* **Red markers:**
  * default lane: `get_retrieval_eval_report` (U1/U2), `impact_analysis.powerbi_node_id`
    (U6), `flush_state.force` (U7)
  * `git-graph` lane: `query_changes` (U3/U4), `index_git_history` (U3/U5)
* Any extra red marker means the audit missed a gap. Ship must halt and send
  the item back to Stage to re-plan with operator sign-off. Do not stash it
  or defer it, because a permanently red guard would make the final gate
  impossible to pass.

### H-REG: `tests/contract/tool_descriptor_registry_test.rs` (existing)

* Invert `retrieval_eval_report_is_declared_mcp_only` into a test asserting
  the tool is declared on `Cli`. This is red for U1.
* Add a registry-level invariant: every descriptor that declares `StdioMcp`
  also declares `Cli`. This is red for U1 in the default lane and for U3 in
  the `git-graph` lane.

### H-ORACLE: `tests/fixtures/mcp_tool_catalog.expected.json` (existing fixture, `contract_mcp_catalog_oracle`)

* Update the expected descriptions of the three tools to their post-parity
  text. Red for U1 (default lane) and U3 (`git-graph` lane).
* Remove the `_description_macros.mcp_only_desc` entry. Red for U3.

### H-F23R: `tests/contract/cli_tool_catalog_parity_test.rs` (existing; hardening, not in the red set)

* **Reverse check:** every name in `surface_names(Cli)` must appear among the
  scraped `run_tool*` literals. Descriptors whose surfaces are exactly `[Cli]`
  are excluded (today only `doctor --smoke`), because the private `CLI_ONLY`
  constant cannot be named from tests. Scraping skips `#[cfg(test)]`/`mod tests`
  blocks.
* **cfg-awareness:** when `cfg!(not(feature = "git-graph"))`, exempt
  `GIT_GRAPH_TOOLS` from the forward "invoked ⊆ declared" check.
* **Red windows:** green at the red commit. It turns pending-red only when U1
  or U3 declares `Cli` before the command exists, and it is green again at U2
  and at U4/U5.
* **Cleanup:** rename the self-comparing
  `the_cli_surface_equals_the_declared_cli_descriptors` to reflect what it
  actually checks.

### H-RETR: `tests/contract/cli_retrieval_eval_report_test.rs` (new, U2)

* The `result` field of the `--json` envelope from
  `engram report retrieval-eval` equals the
  `tools::dispatch("get_retrieval_eval_report")` result, for both a seeded
  report and the empty report. The comparison ignores the envelope `id`. This
  is red.
* Exit code is 0 on success. The command does no threshold evaluation, unlike
  `engram eval`, which exits 3 when a threshold is breached.

### H-GIT: `tests/contract/cli_git_graph_test.rs` (new, `required-features = ["git-graph"]`, U4/U5)

* `query-changes` flag pass-through. This is red.
* `index-git-history` checks:
  * `--depth` and `--force` pass-through (red)
  * `--direct` is rejected (red)
  * a read-server target is refused with 16_001 and no side effects
    (no-regression)

### H-PARAM2: `tests/contract/cli_param_gaps_test.rs` (new, U6/U7)

These are end-to-end tests against a temporary-workspace daemon, following
the `cli_eval_test` pattern. At red, clap rejects the unknown argument with
exit 2.

* `engram impact --powerbi-node-id <ID>` reaches dispatch: it is not a clap
  exit 2, and the response reflects the Power BI selector. This is red.
* `engram impact` with both selectors reaches dispatch, and the handler's
  `powerbi_node_id` precedence is observable. This is red.
* `engram flush --force` reaches dispatch and exits 0. This is red.
* The byte-compatibility check for `engram impact <SYMBOL>` is a no-regression
  assertion. It lives in the existing CLI suite, not in this red file.

### H-CI: `tests/contract/ci_git_graph_lane_test.rs` (new, U8)

Parses `.github/workflows/ci.yml` and asserts two things. Both are red.

* A step runs `cargo test` with the `git-graph` feature set for:
  * `contract_cli_git_graph`
  * `contract_cli_mcp_parameter_parity`
  * `contract_cli_tool_catalog_parity`
  * `contract_tool_descriptor_registry`
  * `contract_mcp_catalog_oracle`
  * `contract_lint_dax_cli_parity`
* A clippy step runs with the `git-graph` feature set.

### H-DOCS: `tests/contract/lint_dax_cli_parity_test.rs` (existing, U9)

* Delete `MCP_WITHOUT_CLI_ALLOWLIST` and its gap-row test. Every catalog or
  dispatch tool row must be mapped (no `-` CLI cell).
* Delete the "MCP-only"/"daemon-only" acceptance branch.
* Add a machine-readable `feature: git-graph` marker in the notes cell. The
  help-resolution and "every real CLI command documented" checks honour that
  marker under `cfg!`.
* Every mapped doc row's CLI command must equal the `MCP_TOOL_CLI_ARGV` entry,
  so the two maps cannot drift apart.
* **CI coverage:** `ci.yml` `paths-ignore` skips docs-only PRs. For those PRs
  the gate is the local `cargo dev-test` (Step 4.3); any PR that touches
  code runs H-DOCS in CI.
* Add a token guard, with the token built from fragments. The guard fails if
  the MCP-only token appears in any of these:
  * catalog descriptions
  * `README.md` or `AGENTS.md`
  * top-level `docs/*.md`
  * `src/tools/capabilities.rs` or `src/shim/tools_catalog.rs`

  Historical directories are exempt: `docs/decisions/`, `memory/`,
  `exec-plans/`, `archive/`, `closure/`, `research/`, and `compound/`. The
  guard catches any future operator doc, including `docs/troubleshooting.md`,
  whichever order 142-S F55 lands in.
* This harness is red until U9. It is also red while U1/U3 catalog text
  changes are pending.

## Implementation Units

Ship's harnesses are listed per unit. The "Files" list is the implementation
surface only; harness files are Step 2 output.

### U1: Declare the CLI surface for `get_retrieval_eval_report` (code)

* **Change:**
  * switch the descriptor to `IPC_CLI_MCP` and keep `read_server_available: true`
  * replace the stale comment with a pointer to `engram report retrieval-eval`
  * rewrite the catalog description: drop the MCP-only text, reference the
    CLI surface `engram report retrieval-eval` and the doc URL
  * keep the default build free of warnings under `-Dwarnings`. Two items now
    have only cfg-gated users, so gate both with `#[cfg(feature = "git-graph")]`:
    * the `mcp_only_desc!` macro
    * the `IPC_AND_MCP` constant
* **Files:** `src/tools/capabilities.rs` and `src/shim/tools_catalog.rs`.
* **Harness:** H-REG and H-ORACLE (default lane). Both are green at U1.
* **Size / complexity:** S / low.

### U2: Add `engram report retrieval-eval` (code)

* **Change:**
  * add `ReportCommand::RetrievalEval` (`#[command(name = "retrieval-eval")]`)
    and a handler `run_retrieval_eval_report` that calls
    `run_tool("get_retrieval_eval_report", None, …)`
  * update the stale "(token-savings, eval, retry-metrics)" and
    "3 report children" doc text
* **Why this name:** it is distinct from `report eval` (`get_evaluation_report`)
  and from `eval`, which runs an evaluation. This settles 035-D Q1.
* **Files:** `src/bin/engram.rs` and `src/cli/commands/report.rs`.
* **Harness:** H-RETR, H-F23R reverse check (default lane), and the H-PARAM
  test for this tool. All are green at U2.
* **Size / complexity:** S / low.

### U3: Declare the CLI surface for the `git-graph` tools (code)

* **Change:**
  * switch the cfg-gated `query_changes` and `index_git_history` descriptors
    to `IPC_CLI_MCP`, keeping `read_server_available` as it is
  * rewrite both cfg-gated catalog descriptions to reference
    `engram query-changes` and `engram index-git-history`
  * **delete** `mcp_only_desc!` and the `IPC_AND_MCP` constant, because
    nothing uses them any more
* **Files:** `src/tools/capabilities.rs` and `src/shim/tools_catalog.rs`.
* **Harness:** H-REG and H-ORACLE in the `git-graph` lane. Both are green at
  U3.
* **`harness_cmd`:** the `git-graph` lane command.
* **Size / complexity:** S / low.

### U4: Add `engram query-changes` under `git-graph` (code)

* **Change:**
  * add a `#[cfg(feature = "git-graph")]` `QueryChanges` variant with its
    matching cfg-gated match arm in `main`
  * flags: `--file` (`#[arg(long = "file")]`) → `file_path`, `--symbol`,
    `--since`, `--until`, and `--limit` (u32). Pass `since`/`until` through
    unchanged so the handler stays the only validator.
  * add a cfg-gated handler in `search.rs` that uses `run_tool`
* **Files:** `src/bin/engram.rs` and `src/cli/commands/search.rs`.
* **Harness:** H-GIT (query cases), plus H-F23R reverse check and H-PARAM for
  this tool in the `git-graph` lane.
* **`harness_cmd`:** the `git-graph` lane command.
* **Size / complexity:** S / medium.

### U5: Add `engram index-git-history` under `git-graph` (code)

* **Change:**
  * add a `#[cfg(feature = "git-graph")]` `IndexGitHistory` variant with a
    cfg-gated match arm, taking `--depth` (u32) and `--force`
  * the handler in `indexing.rs` is **IPC only**: there is **no `--direct`**,
    because in-process execution would bypass the daemon's read-server refusal
  * it uses `run_tool_timed(…, INDEXING_TIMEOUT_SECS)`, as `engram index` and
    `engram eval` do. This is plain output; no progress extension is added.
* **Files:** `src/bin/engram.rs` and `src/cli/commands/indexing.rs`.
* **Harness:** H-GIT (index cases), plus H-F23R and H-PARAM for this tool in
  the `git-graph` lane.
* **`harness_cmd`:** the `git-graph` lane command.
* **Size / complexity:** S / medium.

### U6: Add `engram impact --powerbi-node-id` (code)

* **Change:**
  * `symbol` becomes `Option<String>`, and
    `--powerbi-node-id <ID>` (`value_name = "ID"`) is added
  * both belong to
    `ArgGroup::new("selector").args(["symbol", "powerbi_node_id"]).required(true).multiple(true)`
  * `run_impact` takes two `Option`s and leaves out any key that is absent
  * the handler keeps deciding precedence
  * the missing-selector clap error text changes, which I3 allows. Argv and
    params stay byte-compatible.
* **Files:** `src/bin/engram.rs` and `src/cli/commands/search.rs`.
* **Harness:** H-PARAM2 (impact cases) and the H-PARAM test for `impact_analysis`.
* **Size / complexity:** S / medium.

### U7: Add `engram flush --force` (code)

* **Change:** add a `--force` flag that forwards `{"force": true}`. To make
  pass-through testable, factor out a small `flush_params(force) -> Option<Value>`.
* **Files:** `src/bin/engram.rs` and `src/cli/commands/lifecycle.rs`.
* **Harness:** H-PARAM2 (flush case) and the H-PARAM test for `flush_state`.
* **Size / complexity:** XS / low.

### U8: Make the `git-graph` parity lane gate CI (config)

* **Change:** add a `.github/workflows/ci.yml` job step that runs
  `cargo test --no-default-features --features cozo-backend,embeddings,git-graph`
  on the six targets named in H-CI, plus a clippy step with the same feature
  set. It reuses the existing toolchain and cache steps, with pinned action
  SHAs unchanged.
* **Files:** `.github/workflows/ci.yml`.
* **Harness:** H-CI (green at U8).
* **Size / complexity:** XS / low.

### U9: Rewrite the parity docs to full parity (docs)

* **Change:**
  * in `docs/cli-mcp-parity.md`:
    * map the three tools, marking the `git-graph` rows `feature: git-graph`
    * update the `impact_analysis` row for `--powerbi-node-id`
    * update the `flush_state` row for `--force`
    * delete "MCP tools without CLI commands"
    * reword "Drift guard expectations" so it forbids gaps without using the
      literal token
  * in `docs/mcp-tool-reference.md`: remove the MCP-only wording at line 60
* **Files:** `docs/cli-mcp-parity.md` and `docs/mcp-tool-reference.md`.
* **Harness:** H-DOCS, which is a real executable red test that parses these
  docs. So this docs unit qualifies for `harness-ready` under the **current**
  P-002/P-004.
* **Size / complexity:** S / low.

## Dependency Graph

```text
U1 -> U2 -> U3 -> U4 -> U5 -> U6 -> U7 -> U8 -> U9
```

The chain is linear because every unit shares files with its neighbours:

* U1 and U3 edit `capabilities.rs` and `tools_catalog.rs`.
* U2, U4, U5, U6, and U7 edit `src/bin/engram.rs`.
* U8 needs the targets that U2–U7 turn green.
* U9 is last, because H-DOCS needs every catalog description and CLI command
  to exist first.

A single branch also executes sequentially under P-016. There are no cycles.

## Pending-Harness Contract for Ship Step 4.3

After Step 2, every red-set harness is red. At each unit's Step 4.3, Ship runs
`cargo dev-test`. For U3–U5, Ship also runs the `git-graph` lane command.

The only failures allowed are the still-pending harness targets of later
units in this feature. Ship's harness manifest lists them. A failure in any
other target is a regression and must be fixed inside the unit.

**Recorded deviation.** Step 4.3 literally says to return to build-feature
if any gate fails. That rule cannot hold for any multi-task feature once all
Step 2 harnesses exist. Two simpler alternatives were considered and
rejected:

* one mega-task, which breaks the Task Granularity rule
* writing each harness just in time, which contradicts P-002's "all
  harness-ready before implementation"

A Ship-template follow-up stash asks for the pending-harness rule to be made
explicit.

Expected pending-red targets after each unit closes:

| After | Default lane still red | `git-graph` lane still red |
|---|---|---|
| U1 | H-PARAM (table, retrieval, impact, flush); H-RETR; H-F23R reverse (pending, introduced by U1); H-PARAM2; H-CI; H-DOCS | as default, plus H-REG, H-ORACLE, H-PARAM (git), H-GIT |
| U2 | H-PARAM (table, impact, flush); H-PARAM2; H-CI; H-DOCS | as default, plus H-REG, H-ORACLE, H-PARAM (git), H-GIT |
| U3 | as U2 | as U2, plus H-F23R reverse (introduced by U3); H-REG and H-ORACLE now green |
| U4 | as U2 | H-PARAM (index_git_history); H-GIT (index); H-F23R reverse |
| U5 | as U2 | none beyond the default lane |
| U6 | H-PARAM (table, flush); H-PARAM2 (flush); H-CI; H-DOCS | same |
| U7 | H-CI; H-DOCS | same |
| U8 | H-DOCS | same |
| U9 | none | none |

**Final gate at U9:**

* `cargo dev-test`, `cargo ci` (all features), `cargo lint`, and
  `cargo audit`
* the U8 `git-graph` lane, run locally
* `backlogit docs lint --path docs`

Check exit codes and grep for `FAILED`; do not rely on the output tail.

## Execution Ordering (operator-gated)

> **DECIDED 2026-09-24 ~16:38 (operator, final): Path S.** 142-S ships
> first; parity is claimed as a later, separate shipment. Path P is not
> selected and not authorized. The Path P analysis below is kept as the
> record of the rejected alternative only. Nothing in this plan gates
> 142-S: parity review, harvest, and shipment assembly are not required
> before 142-S resumes.

### Planning

This plan has NOT passed the review gate (3 consecutive FAILs; circuit open;
see [Plan Review — Attempt 3](#plan-review--attempt-3-revision-3)). It is not
harvested. Under Path S that is not on 142-S's critical path.

### (Rejected alternative, Path P) Implementing before 142-S continues is technically feasible

* No remaining 142-S task owns a parity file:
  * F50 owns `crates/engram-indexer/src/preflight.rs`
  * F51, F52 and F53 own the launchers and launcher tests
  * F54 owns `tests/contract/read_server_cli_mcp_parity_test.rs`
  * F55 owns `docs/troubleshooting.md`
* The topology predecessors (137-S, 140-S, 141-S) are archived.
* As of 2026-09-24:
  * 142-S has no open PR
  * its branch is local only (not pushed)
  * there is a single worktree

### Policy blocks it without operator and Ship action

1. **P-001.** 142-S is `active`, and all six 142-F tasks are `active`. Ship
   cannot claim a second release unit until 142-S merges and closes, unless
   the operator explicitly grants `skip_policy: P-001` for this preemption.
2. **A shipment cannot be paused.** backlogit 1.8.0 supports only three
   shipment transitions: `queued → active`, `active → shipped`, and
   `active → abandoned`. Ship's claim verification also requires the claimed
   shipment to be the **sole** active shipment. So Path P means **abandoning
   142-S (a terminal state) and later re-shipping its unfinished items in a
   successor shipment.** Only Ship can do this, and only with operator
   authorization. Stage cannot.

   The Ship-owned Path P sequence:
   1. Confirm there is no open 142-S PR.
   2. Commit the WIP harness changes on the 142-S branch. Do not delete the
      branch.
   3. Move the six 142-F tasks from `active` back to `queued`, keeping their
      harness labels and notes.
   4. Transition 142-S `active → abandoned`, with a closure note naming the
      successor.
   5. Switch the single worktree to a fresh branch from `main` (P-016).
   6. Claim the parity shipment.

   After parity merges, Stage assembles a **successor shipment** for exactly
   the same unfinished 142-F items. That creates a new manifest; it does not
   edit 142-S. Ship then resumes on the preserved branch, rebased onto
   `main`.
3. **Cross-shipment effects.**
   * **The F54 harness line.**
     `assert!(!descriptor.surfaces.contains(&ToolSurface::Cli))` for
     `get_retrieval_eval_report` must go in either path. It conflicts with
     F54's acceptance criterion 6, which Stage already amended on 2026-09-24
     with no scope change; see the 035-D deliberation. Ship already owes this
     fix inside F54's own file.
   * **F55 is unchanged.** H-DOCS enforces the operator's rule on
     `docs/troubleshooting.md` automatically.
   * **Path S.** F54 has already merged, and its matrix exercises every
     declared surface. Once U1 and U3 flip descriptors, F54 exercises them on
     CLI too. Because F54 is no longer an active 142-S file by then, U1–U5
     own any change it needs. The Step 2 harness-architect adds
     `contract_read_server_cli_mcp_parity` to the red set of U1 (default lane)
     and U3 (`git-graph` lane). That target resolves CLI argv through the
     shared `tests/helpers/cli_argv.rs`.
   * **Path P.** F54 is implemented later, in the successor shipment. Ship is
     advised to resolve F54's CLI argv from `tests/helpers/cli_argv.rs`. This
     is guidance only: F54's acceptance criteria already require the
     generated matrix to cover CLI surfaces as they are declared.

### The two paths (decided: Path S)

* **Path S, sequential — SELECTED by the operator.** This keeps 035-D's
  original "after 142-S" ordering. 142-S resumes once three things are true:
  * the 036-D Option 2 harness is installed
  * the F54 harness line is fixed
  * F55 is re-labeled

  Parity is staged next (review attempt 4 with operator authorization,
  harvest, shipment) and claimed only after 142-S is shipped.
* **Path P, parity first — NOT selected.** An interim Stage recommendation
  favoured it; the operator rejected it. It would have required
  `skip_policy: P-001` and authorization for Ship to abandon 142-S. Neither
  is granted.
### No dependency edge between the shipments

* Adding one to 142-S would mutate an active manifest.
* Adding "blocked by 142-S" to the parity shipment would silently encode
  Path S.

P-001 enforces the gate at claim time.

## Decisions and Rationale

* **D1: `engram report retrieval-eval`.** It avoids collisions with
  `report eval` and `eval`.
* **D2: `git-graph` commands compile only under the feature.** This matches
  the cfg-gated descriptors and catalog entries, with cfg-aware guards and the
  U8 CI lane. (Q2)
* **D3: Parameter coverage plus result equivalence for the new reads.**
  Result equivalence is checked for `get_retrieval_eval_report`; the
  `query_changes` harness checks pass-through. F54's matrix adds
  default-lane result parity when it lands, but this feature does not depend
  on F54. (Q3)
* **D4: `read_server_available` and `CapabilityClass` are unchanged.** (Q4)
* **D5: Independent of stash D1FF9E31.** (Q5)
* **D6: Guards are harness output, and each unit has its own red assertion.**
  This replaces the deliberation's "invert last" idea. The pending-harness
  contract keeps Step 4.3 attributable.
* **D7: `flush --force` mirrors the MCP contract as it is today.** Whether the
  handler should honour `force` is a follow-up MCP-contract decision.
* **D8: No CLI definition move.** Moving the clap tree from `src/bin` into the
  library, so tests can walk `Command` rather than parse help text, was
  considered and deferred as a follow-up. Help-token parsing with explicit
  rules is enough for coverage, and the move would widen scope.

## Constitution Check

| Principle | Mapping | Status |
|---|---|---|
| I. Safety-First Rust | No `unsafe`; handlers return typed errors through `run_tool` | Compliant |
| II. Test-First (non-negotiable) | Step 2 harness per unit. Under the P-004 red-set rule, new harness files contain only functions that fail at the red commit; hardening added to existing targets is labeled as such | Compliant |
| III. Workspace Isolation | No new filesystem paths; `index-git-history` stays IPC-only (no `--direct`) | Compliant |
| IV. CLI Workspace Containment | New commands read or write only through the daemon in the bound workspace | Compliant |
| V. Observability | Reuses the runner's tracing; no new spans needed | Compliant |
| VI. Single Responsibility | Each unit covers one tool or one parameter, and one domain | Compliant |
| VII. Destructive Command Approval | No destructive command added | N/A |
| VIII. Safety Modes | No elevated-risk mode involved | N/A |
| IX. Git-Friendly Persistence | No persistence change | N/A |
| X. Context Efficiency | Guards derive from registries; docs are a checked copy | Compliant |
| XI. Merge Commit History | Unaffected; Ship's PR flow applies | Compliant |
| Task Granularity (non-negotiable) | Every unit changes at most 2 implementation files (harness files are Step 2 output) and one domain; the U2/U4–U7 `engram.rs` edits are one variant or one flag each | Compliant |
| Quality Gates | Step 4.3 runs under the pending-harness contract; U3–U5 also run the `git-graph` lane; the final gate adds `cargo ci`, `cargo lint` and `cargo audit` | Recorded deviation (see the pending-harness contract); Ship-template follow-up stashed |

**Recorded tension:** Step 4.3's `cargo dev-test` sees pending-red harness
targets from later units. This is the normal state after Step 2 for any
multi-task feature. The plan makes the allowed set explicit, so a real
regression can still be told apart from a pending harness.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Parsing `--help` text is brittle | Match only rendered tokens, accept both `<X>` and `[X]` positionals, and use the explicit rename table. D8 records the follow-up. |
| The `git-graph` lane never gates CI | U8, with H-CI enforcing it |
| Scripts break when `impact` gets an optional positional | Argv and params stay byte-compatible (no-regression test); `ArgGroup` with `multiple(true)` |
| Self-referential token false positive | The parity doc rewords its rule without the literal token; the guard is scoped and built from fragments |
| An extra gap appears at the red phase | Halt and re-plan through Stage with operator sign-off |

## Plan Hardening Signals

* **Public API, schema, or contract change: present.** New CLI commands, the
  optional `impact` positional, descriptor surface flips, and a CI workflow
  step. MCP schemas are unchanged.
* **Security, auth, or permission: present (low).** A Write method is exposed
  on the CLI. It is IPC-only and the read-server refusal is kept.
* **Migration or destructive action: absent.**
* **External integration or operator checkpoint: present.** The P-001
  ordering decision (resolved: Path S). The Path P preemption is not selected.
* **High runtime or rollback risk: absent.** The changes are additive and
  roll back with a revert.

Requires plan hardening: yes

## Runtime Verification and Closure

| Unit | Runtime surface | Runtime proof | Closure |
|---|---|---|---|
| U2 | CLI (read) | `engram report retrieval-eval --json` against a live daemon and the read server matches the MCP result | parity doc row (U9) |
| U4 | CLI (read, `git-graph`) | `engram query-changes --limit 1 --json` on a `git-graph` build | parity doc row |
| U5 | CLI (write, `git-graph`) | `engram index-git-history --depth 5` on the indexer; the read server refuses with 16_001 | parity doc row |
| U6 | CLI | `engram impact <SYMBOL>` and `engram impact --powerbi-node-id <ID>` on a Power BI fixture | parity doc row |
| U7 | CLI | `engram flush --force` exits 0 | follow-up stash for handler semantics |
| U8 | CI | the parity-lane job appears and passes on the PR | the H-CI guard enforces it |
| U9 | docs | `backlogit docs lint --path docs`; H-DOCS green | the guard enforces it permanently |

## Plan Hardening

**Required:** yes. The triggers are:

* a public CLI contract change
* a Write method on a new surface
* a CI workflow change
* an operator checkpoint for ordering

**Sources consulted:**

* `.github/policies/workflow-policies.md`: P-001, P-002, P-004, P-010, P-016
* `.github/instructions/constitution.instructions.md`
* `.github/agents/_ship.agent.md`: Step 4.2 `harness_cmd`, Step 4.3 gates
* `.cargo/config.toml`: `dev-test`, `ci` (all features), `lint`
* `.github/workflows/ci.yml`: no `git-graph` lane today
* `docs/compound/best-practices/clap-long-vs-name-attribute-2026-05-07.md`
* `docs/compound/independence-guard-fixture-prose-false-positive-2026-08-22.md`
* `docs/compound/workspace-status-code-graph-cfg-gate-false-premise-2026-07-07.md`
* `docs/compound/capability-rewrite-must-convert-every-consumer-2026-08-21.md`
* `docs/compound/test-failures/truncated-test-output-review-hid-call-site-regression-2026-09-12.md`
* `docs/compound/workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-defer-to-harness-architect-2026-07-25.md`
* `docs/compound/workflow-issues/mutually-exclusive-features-no-default-features-2026-04-20.md`

### Protected Invariants

* **I1.** No MCP `inputSchema` changes.
* **I2.** `read_server_available` and `CapabilityClass` are unchanged. The
  read-server refusal of `index_git_history` (16_001, no side effects) holds,
  and there is no in-process `--direct` path.
* **I3.** `engram impact <SYMBOL>` argv, params, and exit codes stay the
  same. Only the clap usage text for a missing selector may change.
* **I4.** `_health` and `_shutdown` never gain a CLI surface. This holds by
  construction, because they declare no `StdioMcp`.
* **I5.** This feature does not modify the 142-S manifest.
  * **Earlier Stage edit, disclosed:** on 2026-09-24, before this plan, Stage
    amended the planning text of 142.058-T (acceptance criterion 6) and
    142.058.002-ST. That was a scope-neutral correction and did not change
    the manifest.
  * **Path S selected:** 142-S is never abandoned or preempted for parity.
    (Under the rejected Path P only Ship, with operator authorization, could
    have abandoned it.)
* **I6.** The default-lane build never compiles `git-graph` commands, and
  the default-lane guards stay green, because they are cfg-aware.

### Risky Actions

```text
ProposedAction A1: Flip three descriptors to IPC_CLI_MCP (U1, U3)
  ActionRisk: medium. Approval: plan approval only. Expected: H-REG green at the unit; the command follows at the next unit.
  Rollback: revert the unit (additive).
ProposedAction A2: Make the engram impact positional optional (U6)
  ActionRisk: medium. Approval: none; the no-regression byte-compatibility test must pass. Rollback: revert U6.
ProposedAction A3: New Write CLI command index-git-history (U5)
  ActionRisk: medium. Approval: none; IPC-only; --direct rejected; refusal test. Rollback: revert U3-U5.
ProposedAction A4: New CI workflow step (U8)
  ActionRisk: low. Approval: none; pinned SHAs unchanged. Rollback: revert U8.
ProposedAction A5: Preempt the active release unit (Path P only) -- NOT SELECTED (operator chose Path S, 2026-09-24)
  ActionRisk: high (process; the shipment transition is terminal).
  Approval: REQUIRED. The operator grants skip_policy P-001 AND authorizes abandoning 142-S.
  Ship alone runs the sequence: no open PR; WIP committed on the preserved branch; the six
  tasks move active->queued; 142-S active->abandoned; fresh branch from main.
  Recovery: after parity merges, Stage assembles a successor shipment for the same unfinished
  142-F items, and Ship resumes on the preserved, rebased branch. The abandoned 142-S record
  stays as history.
```

### Added Verification

* **Environment prechecks:**
  * `cargo check --all-targets --features git-graph` compiles before red is
    declared
  * `git-graph` is additive to the default features: use
    `--no-default-features --features cozo-backend,embeddings,git-graph`, as
    `ci.yml` does
* **Red fidelity:** the H-PARAM red set must equal the list in the harness
  specification exactly. On any mismatch, halt and re-plan.
* **Blocked path:** if U6's Power BI fixture cannot be indexed in tests, the
  harness asserts only what reaches dispatch, and the runtime check becomes a
  closure step.
* **Final gate:** as listed in the
  [pending-harness contract](#pending-harness-contract-for-ship-step-43).

### Operational Closure

* **Monitoring:** the inverted H-DOCS, H-PARAM, H-REG, and H-F23R guards,
  plus the U8 CI lane.
* **Rollback trigger:** any regression in I1–I6.
* **Owners:** Ship owns execution. The operator decided Path S.
* **Validation window:** the parity PR CI, including the `git-graph` lane.

### Unresolved Operator Decisions

These block parity implementation only; they never block 142-S.

1. **Path P or Path S — RESOLVED: Path S** (operator, 2026-09-24 ~16:38).
2. **Review re-entry — OPEN.** Attempt 4 requires explicit operator
   authorization (circuit open after attempt 3). Not before 142-S ships.
3. **Historical-directory exemption.** Confirm that the directories listed in
   H-DOCS are exempt from the token guard as historical records. This plan
   assumes they are.

<!-- plan-review-attempt: 1 -->

## Plan Review — Attempt 1 (revision 1)

**Gate: FAIL.** Reviewed 2026-09-24 by seven personas: Constitution, Rust,
Scope Boundary, Learnings, Architecture, and Agent-Native Parity, with
Security folded into the Parity review.

Plan hardening was required, and revision 1 included it.

### P1 findings and how revision 2 resolves them

| # | Source | Finding | Resolution |
|---|---|---|---|
| 1 | Constitution | Constitution Check section missing | Added a [Constitution Check](#constitution-check) section |
| 2 | Constitution | U01 (config) had no red harness; placeholder stubs | U01 removed; target registration is harness-architect Step 2 output (compound lesson, 2026-07-25) |
| 3 | Constitution | U02 exceeded the 2-hour rule and its harness role was unclear | U02 removed as a task; its guards became the H-PARAM, H-REG, H-F23R, and H-DOCS harness specs |
| 4 | Constitution, Rust | Documented `git-graph` commands fail `--help` resolution in the default lane | H-DOCS adds a `feature: git-graph` marker with cfg-aware checks |
| 5 | Rust | `tool_descriptor_registry_test.rs:338` MCP-only assertion was not covered | H-REG inverts it (U1) |
| 6 | Rust | The F23 scraper ignores `#[cfg]`, so the default lane breaks | H-F23R adds a cfg-aware exemption |
| 7 | Rust | `mcp_only_desc!` becomes unused or dead, and carries the token | U1 cfg-gates it; U3 deletes it |
| 8 | Learnings | The `git-graph` lane never gates CI | New U8 (CI config) with harness H-CI |
| 9 | Learnings | Some harness cases pass on unchanged code | Each unit must have at least one observed red assertion; the rest are labeled no-regression |
| 10 | Architecture | F23 is not a true equality | H-F23R adds the reverse check; the Problem Frame is corrected |
| 11 | Architecture | The parameter guard used the docs as its mapping source | Mapping now comes from the test-local `MCP_TOOL_CLI_ARGV`, enforced complete against `surface_names(StdioMcp)` |
| 12 | Scope | The "no plan says MCP-only" requirement was not enforced | Stage swept the plans and backlog (clean); H-DOCS guards operator docs, README, AGENTS, and source text |
| 13 | Scope, Constitution | The source approval trail was not recorded | 035-D artifact updated to decided with `promoted_to` (done before harvest) |

### P2 findings

**Resolved in revision 2:**

* Transient-red table accuracy: replaced by the pending-harness contract.
* U07 routing and timeout: IPC-only, no `--direct`, uses `INDEXING_TIMEOUT_SECS`.
* U08 `ArgGroup` semantics: handler precedence is kept.
* Extra-gap path: now halts and re-plans.
* F55 scope drift: F55 is unchanged, and the guard covers it.
* Path P now requires that no 142-S PR is open.
* The clap `long = "file"` pitfall is addressed.

**Recorded as follow-up stash entries:**

* CLI help fidelity for enum values and defaults (`search --region` text,
  `limit` default, `PossibleValues`), with an optional move of the clap tree
  into the library (D8).
* The `flush_state` handler ignores `force`.

### P3 findings

These were applied inline:

* the U2 exit-code contract
* stale report doc text
* the U4 `--limit` type and since/until pass-through
* the cfg-gated match arms
* the self-referential token wording
* full-output failure checks
* the `--no-default-features` feature mix

<!-- plan-review-attempt: 2 -->

## Plan Review — Attempt 2 (revision 2)

**Gate: FAIL.** The attempt-1 P1s were verified resolved, except for the
035-D queue record. New P1s were raised; revision 3 resolves them as follows.

| # | Source | Finding | Revision 3 resolution |
|---|---|---|---|
| 1 | Constitution | The `git-graph` red and green checks are invisible to `cargo dev-test` | U3–U5 use the `git-graph` lane as their `harness_cmd`, and Step 4.3 adds that lane; the pending table has a `git-graph` column |
| 2 | Constitution | Path P "park" is impossible, because backlogit shipments cannot pause and Ship requires a sole active shipment | Path P is rewritten: operator-authorized abandonment by Ship, tasks moved back to `queued`, and a Stage-assembled successor shipment |
| 3 | Rust | `IPC_AND_MCP` becomes dead code under `-Dwarnings` | U1 cfg-gates it; U3 deletes it |
| 4 | Rust | The `mcp_tool_catalog.expected.json` oracle fixture was not covered | New H-ORACLE harness, red for U1 and U3 |
| 5 | Scope | The 035-D queue record was stale and implied "after 142-S" | `backlogit update 035-D` records Chosen Direction, Open Questions, and Notes; the 036-D record is updated too |
| 6 | Architecture | Under Path S, F54 exercises CLI on newly declared methods | Shared `tests/helpers/cli_argv.rs`; under Path S, F54's target joins the U1/U3 red set; under Path P it is guidance only |
| 7 | Learnings | The lint_dax cfg branch is not in the CI lane | H-CI and U8 cover six targets plus a `git-graph` clippy step |

**P2 and P3 findings applied inline:**

* P-004 red-set rule: new harness files are red-only, and hardening added to
  existing targets is labeled
* the pending-harness deviation is recorded with the rejected alternatives,
  and a Ship-template follow-up is stashed
* the README, AGENTS, and top-level docs sweep is recorded
* `cargo audit` is added to the final gate
* H-F23R red windows are corrected
* the `CLI_ONLY` exclusion now uses a surfaces-exactly-`[Cli]` rule
* scraping skips `mod tests`
* global flags are excluded from H-PARAM
* `value_name = "ID"`
* H-RETR compares the `result` field
* H-PARAM2 is observed end-to-end (clap exit 2 at red)
* doc rows must agree with the shared argv map
* the `paths-ignore` docs gate is stated honestly
* the earlier 142.058-T amendment is disclosed in I5
* "Planning: done" is corrected
* the deliberation's open questions are marked superseded
* red-time triage of pre-existing `git-graph` failures
* coverage-oracle manifest entries

**Left for build time:** the harness-architect settles any remaining
per-test red-ordering detail, per the 2026-07-25 compound lesson.

<!-- plan-review-attempt: 3 -->

## Plan Review — Attempt 3 (revision 3)

**Gate: FAIL. Circuit OPEN.** Three consecutive FAILs. This plan is NOT
approved and NOT harvested. Transcribed 2026-09-24 from the Stage checkpoint
memory `docs/memory/2026-09-24-stage-cli-parity-plan-checkpoint.md`; no new
review was run.

* **P1 (Rust):** `tests/fixtures/mcp_tool_catalog.expected.json` has no
  `query_changes` / `index_git_history` entries and
  `mcp_catalog_oracle_test.rs` is not cfg-aware, so the `git-graph` lane is
  already red for name-set/drift and U8 would make that red lane gate CI.
* **P2/P3:** the active→queued task move needs explicit authorization (moot
  under Path S); precheck task reassignment after abandonment (moot under
  Path S); name the P-004 `git-graph`-lane substitution as a deviation; run
  the `git-graph` lane U3–U9; add an F54 pending-red row and F54 target to
  H-CI for Path S; stale 035-D Options/status text (ordering corrected
  2026-09-24); F55 changes by label only; H-CI must reject
  `continue-on-error`.
* Constitution, Architecture, and Scope passed.

### Proposed revision-4 fixes (UNREVIEWED; do not treat as approved)

Add both `git-graph` entries to the oracle fixture with
`"feature":"git-graph"`, filtered by `load_expected` when
`!cfg!(feature = "git-graph")`, red for U3; record the current lane failure
as pre-existing; relabel the `_description_macros` removal as hygiene; apply
the Path S-specific P2/P3 items above; drop Path P material.

### Escalation payload (P-013.6)

* **Threshold:** plan-review, 3 consecutive FAILs (attempts 1–3).
* **Failure summary:** each revision resolved prior P1s but review surfaced
  new P1s in harness/CI coverage of the `git-graph` lane.
* **Refs:** this plan (attempt markers 1–3); 035-D; stash `E06BABAD`;
  memory `docs/memory/2026-09-24-stage-cli-parity-plan-checkpoint.md` and
  `docs/memory/2026-09-24-stage-path-s-handoff.md`.
* **Resolved escalation route:** `gpt-6-sol` / `openai` (fresh config;
  differs from the Stage route, so not ESCALATION_DEGRADED).
* **Disposition:** halted for operator/async review. Stage does not re-run
  plan-review. Attempt 4 requires explicit operator authorization and, per
  Path S, is not needed before 142-S ships.
