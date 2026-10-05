---
title: "CLI parity for every user-facing MCP tool"
description: "Deliberation on closing every MCP-only user-facing tool gap so the engram CLI matches MCP at tool and parameter level"
topic: "Operator requirement 2026-09-24: all user-facing tools callable via CLI with full MCP parity; no plan or policy may declare a user-facing tool MCP-only"
depth: "standard"
decision_status: "decided"
promoted_to: "docs/exec-plans/2026-09-24-full-cli-mcp-parity-plan.md"
source_stash_ids:
  - "E06BABAD"
linked_artifacts:
  - ".backlogit/queue/035-D.md"
  - ".backlogit/queue/142.058-T.md"
  - ".backlogit/queue/142.058.002-ST.md"
  - ".backlogit/queue/090.004-T.md"
  - "docs/cli-mcp-parity.md"
  - "docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md"
tags:
  - "cli-parity"
  - "mcp"
  - "operator-requirement"
---

## Problem Frame

On 2026-09-24 the operator stated the product intent explicitly. Every
user-facing Engram tool must be callable from the `engram` CLI with full
functional parity to its MCP tool. No plan, policy, descriptor comment, catalog
description, or operator doc may say a user-facing tool is intentionally
MCP-only.

Out of scope for this requirement:

* Internal IPC methods: `_health` and `_shutdown`.
* Local or internal CLI commands: `shim`, `daemon`, `install`, `update`,
  `reinstall`, `uninstall`, `manifest`, `verify`, `migrate-down`, and
  `doctor --smoke`. The requirement runs from MCP to CLI, not the reverse.

Success means all three of these hold:

* Every user-facing MCP descriptor declares a CLI surface.
* Every MCP parameter has a CLI equivalent.
* The drift guard fails if an MCP-only user-facing tool is ever added again.

## Research Findings

* **This is not new intent.** Feature 090-F (source stash 30F372C8, archived)
  already required "every agent-facing MCP tool has an equivalent engram
  subcommand". 090-F shipped the audit (090.001-T), the mapping document, and
  the drift guard. Its gap-closing task, 090.004-T, stayed `blocked` under the
  archived feature and was never executed. The audit it was waiting for now
  exists as `docs/cli-mcp-parity.md`.
* **Tool-level gaps.** Each of these descriptors in `src/tools/capabilities.rs`
  declares `IPC_AND_MCP`, which has no CLI surface:
  * `get_retrieval_eval_report`: in the default catalog. Its comment says "No
    approved requirement adds a CLI surface", which the operator requirement
    now supersedes. Its catalog description says "MCP-only daemon surface".
  * `query_changes`: gated behind the `git-graph` feature.
  * `index_git_history`: gated behind the `git-graph` feature.
* **Parameter-level gaps.** `impact_analysis` is documented as having the
  `powerbi_node_id` selector on MCP only. No full parameter-by-parameter audit
  of the other 17 mapped tools has been recorded, so there may be more gaps.
* **Guards that currently enforce the gaps.** `MCP_WITHOUT_CLI_ALLOWLIST` in
  `tests/contract/lint_dax_cli_parity_test.rs` lists the three tools above.
  `docs/cli-mcp-parity.md` contains gap rows and a "MCP tools without CLI
  commands" section. The contract tests require an "MCP-only" rationale for any
  tool that has no CLI command.
* **Where active shipment 142-S touches this.** In F54 (142.058-T), acceptance
  criterion 6 and the F54 harness both encoded "`get_retrieval_eval_report` on
  MCP only". Stage has corrected that planning text so it keeps F54's scope and
  drops the gap assumption; see the Decision section. The old plan
  (`docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`, item 10)
  kept the tool MCP-only "unless a separate CLI requirement is approved". The
  plan now carries an amendment note recording that the requirement exists.
* **Lessons from `docs/compound/`:**
  * clap `long=` vs `name=` pitfalls (042-F).
  * `git-graph` cfg-gating: the CI integration lane runs without `git-graph`.
  * Token-scanning guards should build their tokens from fragments.
  * A parity test that already passes on unchanged code is not a RED harness.

## Options Evaluated

### Option A: One covering feature for full parity, sequenced after 142-S (recommended)

Synthesize a new top-level feature. Candidate decomposition, to be finalized by
`impl-plan`:

1. Parameter-parity audit, planned as a spike or characterization task. It
   compares every MCP `inputSchema` property with the CLI flags and produces a
   confirmed gap list.
2. A CLI read accessor for `get_retrieval_eval_report`, plus a descriptor
   change to `IPC_CLI_MCP` and an updated catalog description. The command name
   is still open; see the questions below.
3. `git-graph`-gated CLI subcommands for `query_changes` and
   `index_git_history`, with cfg-aware tests.
4. A `--powerbi-node-id` flag for `engram impact`, plus any other parameter
   gaps the audit confirms.
5. Invert the drift guard: remove `MCP_WITHOUT_CLI_ALLOWLIST` and assert that
   every user-facing MCP descriptor declares CLI.
6. Rewrite `docs/cli-mcp-parity.md` to remove every "MCP-only gap" statement.

Each unit follows the 2-hour rule with one domain per task (code, tests, or
docs).

* **Pros:** closes the requirement completely, has a single auditable owner,
  and makes the matrix generated from F54 descriptors cover the new CLI
  surfaces automatically.
* **Cons:** medium effort, roughly 6–9 tasks. It has to wait for 142-S.
* **Effort:** medium.

### Option B: Close only `get_retrieval_eval_report` now

* **Pros:** smallest diff.
* **Cons:** breaks the explicit "full parity" intent. The `git-graph` and
  parameter gaps stay open, and the allowlist survives.
* **Effort:** low.

### Option C: Fold the CLI work into active 142-S

* **Cons:** the 142-S manifest cannot change while it is active, and the scope
  expansion is not approved. It would also mix the preflight/launcher release
  with CLI feature work.
* **Status:** not allowed.

### Option D: Revive 090.004-T under archived 090-F

* **Pros:** reuses existing provenance.
* **Cons:** the parent feature is archived. A new release unit would need a
  covering feature anyway, and the task's text is stale.
* **Status:** rejected. Record 090.004-T as superseded instead.

## Trade-off Comparison

| Criterion | A: covering feature | B: single tool | C: fold into 142-S | D: revive 090.004-T |
|---|---|---|---|---|
| Meets operator intent | Full | Partial | Full, but at the wrong time | Full |
| Policy compliance | Yes | Yes | No (active manifest, unapproved scope) | Weak (archived parent) |
| Risk | Moderate (touches the CLI surface) | Low | High | Moderate |
| Sequencing | After 142-S | After 142-S | Concurrent | After 142-S |

## Decision

**DECIDED 2026-09-24 (operator): Option A — full parity is an approved product requirement.** Open questions Q1–Q5 are resolved in the plan's Decisions (D1–D5).

**ORDERING DECIDED 2026-09-24 ~16:38 (operator, final): Path S, sequential.** Operator: "CLI parity cannot exist prior to ship; we need defer that to another shipment." 142-S ships FIRST; CLI parity is a separate, later shipment built from this deliberation, existing stash `E06BABAD`, and the draft plan. Path P (parity first via `skip_policy: P-001` and abandoning 142-S) is NOT selected and is not authorized. No new intake is created for parity. An earlier interim note on this page (and in the plan) that made ordering "operator-gated" with Stage recommending Path P is superseded by this decision. The plan has not passed review (3 consecutive FAILs; circuit open), so parity is not harvested; see the plan's Execution Ordering and Plan Review sections.

**Original recommendation text (retained for provenance):** Option A.

* **Sequencing:** the new feature depends on 142-S reaching `shipped`. That
  keeps P-001 to one release unit and lets the new CLI surfaces land against
  the three-surface topology after F46–F49. It also means F54's generated
  matrix is already in place to cover them.
* **090.004-T:** archive it as superseded when this feature is harvested.

Stage actions already taken, without expanding 142-S scope:

* Corrected F54 (142.058-T) acceptance criterion 6.
* Corrected the F54 subtask 142.058.002-ST.
* Added the plan amendment note.
* Commented on 142.058-T, 142.059-T, and 090.004-T.

F54 still adds no CLI command. Ship has been asked to remove the one harness
line that hard-codes the absence of a CLI surface and to re-verify the red
phase.

## Rejected Alternatives

* **B:** partial parity violates the stated intent.
* **C:** P-001 and active-manifest immutability, plus unapproved scope.
* **D:** the parent is archived and the scope is stale.

## Unresolved Questions

> **Superseded 2026-09-24:** Q1–Q5 are resolved in plan decisions D1–D5. The "invert the guard last" mitigation below is replaced by plan D6: guards are harness output, with a pending-harness contract.

1. **CLI name for `get_retrieval_eval_report`.** Candidates:
   * `engram report retrieval-eval`, which matches the pattern of the other
     read reports.
   * `engram eval --latest`.
2. **Descriptor scope for the `git-graph` subcommands.** Should they be
   compiled only under the `git-graph` feature, matching the catalog? The
   recommendation is yes, with cfg-aware drift guards.
3. **Should the parity audit also check output-shape equivalence** (JSON-RPC
   envelope parity), or only parameter coverage?
4. **Read-server availability.** The new CLI read accessor should inherit
   `read_server_available: true`. Confirm this against the F19/F24 rules after
   142-S.
5. **Interaction with stash D1FF9E31** (the CLI-first process model research).
   The recommendation is to keep them independent.

## Risks and Mitigations

* **Risk:** the drift-guard inversion breaks CI before every gap is closed.
  **Mitigation:** order the guard inversion last, after every CLI surface task
  is done.
* **Risk:** `git-graph` tests are skipped in the default CI lane.
  **Mitigation:** add cfg-aware tests and an explicit `--features git-graph`
  verification step.
* **Risk:** the F54 harness still contains the CLI-absence assertion.
  **Mitigation:** Ship removes it inside F54's owned test file before
  implementation. It is not a new scope item.
