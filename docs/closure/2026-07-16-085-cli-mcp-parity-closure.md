---
title: "Operational Closure — 085-S CLI<->MCP parity audit, mapping doc & drift guard"
doc_type: closure
source: "085-S shipment (feature 090-F; tasks 090.001-T, 090.002-T, 090.003-T)"
description: >-
  Post-merge closure for shipment 085-S. Records the audit-first decision, the canonical
  CLI<->MCP mapping document, the drift-guard contract test and its self-adversarial
  hardening, the canonical-URL and uninstall-doc corrections from Copilot review, the
  deferred functional-gap and routing-identity follow-ups, and the CI cozo-connect flake.
topic: "CLI<->MCP parity: canonical mapping doc + contract-enforced drift guard"
depth: "closure"
decision_status: "SHIPPED — merged to main as merge commit 945ece65 via PR #259"
author: ship
date: 2026-07-16
verdict: SHIPPED
pr: 259
merge_commit: 945ece650c59bc61ff7865fc0d739c0e96e26267
target_commit: 945ece650c59bc61ff7865fc0d739c0e96e26267
branch: feat/085-cli-mcp-parity
scope: "Audit CLI<->MCP surface parity, publish a canonical mapping doc, and enforce it with a contract test"
reviewers:
  - gpt-5.6-sol
  - gpt-5.6-terra
  - gemini-3.1-pro-preview
  - gemini-3.5-flash
  - copilot
linked_artifacts:
  - "085-S"
  - "090-F"
  - "090.001-T"
  - "090.002-T"
  - "090.003-T"
  - "090.004-T"
  - "090.005-T"
---

## Summary

engram exposes two command surfaces over the same daemon: the MCP tool catalog (agent-facing,
JSON-RPC) and the `engram` CLI (operator-facing, clap). Before this shipment there was no
authoritative record of which MCP tools map to which CLI commands, which surfaces are
intentionally single-sided, and no mechanism to catch silent drift when either surface changes.

Feature 090-F closes that gap with three deliverables: a **parity audit** that supersedes the
stale 2026-07-05 gap list, a **canonical mapping document** (`docs/cli-mcp-parity.md`) that is
the single source of truth for the CLI<->MCP correspondence, and a **drift-guard contract test**
(`tests/contract/lint_dax_cli_parity_test.rs`, target `contract_lint_dax_cli_parity`) that fails
CI whenever the real dispatch table, the documented mapping, or the two intentional-asymmetry
allowlists disagree.

This shipment is documentation plus one contract test: it changes no runtime behavior, no schema,
and no on-disk format. The functional work of *closing* any confirmed parity gaps is explicitly
out of scope and tracked as follow-on work (see Deferred follow-ups).

## Tasks shipped

* `090.001-T` — Parity audit + canonical MCP<->CLI mapping doc. Enumerates every MCP tool and
  every CLI command, classifies each as mapped, MCP-only, or local-only, and supersedes the stale
  2026-07-05 gap list.
* `090.002-T` — Parity drift-guard contract test. Introspects the real dispatch `match` in
  `src/tools/mod.rs`, runs `engram <cmd> --help` via `CARGO_BIN_EXE_engram`, and cross-checks the
  doc and both allowlists bidirectionally.
* `090.003-T` — Bidirectional doc parity: every catalog tool description references the mapping
  doc and the CLI surface, asserted by the contract test.

## Key decisions

### Audit-first — supersede the stale gap list

The plan gate `088.001-R` flagged (F5) that harvesting from the 2026-07-05 gap list would encode
stale assumptions. 090.001 therefore rebuilt the gap inventory from the live surfaces: the audit
correctly **excludes** `create_task` / `update_task` (documented in copilot-instructions but not
real MCP tools), identifies the genuine **MCP-only** tools (`get_retrieval_eval_report`,
`query_changes`, `index_git_history` — the last two feature-gated behind `git-graph`), and the
**local-only** CLI commands (shim, daemon, install, update, reinstall, uninstall, manifest,
verify, migrate-down) that operate on the local process/workspace and have no daemon-tool analog.

### The drift guard is an oracle, not a mirror

The contract test does not re-encode the mapping by hand; it derives it. `dispatch_tool_names()`
text-parses all dispatch arms from `src/tools/mod.rs` via `include_str!` (feature-independent),
and `tools_catalog::all_tools()` is the compiler-checked runtime catalog. Independent
adversarial code review (`gpt-5.6-sol`, run before Copilot) raised two P2s: the union check
`catalog.contains || dispatch.contains` could mask catalog<->dispatch drift, and the line-based
dispatch parser could silently under-count a rustfmt-wrapped multi-line `|` alternation. Both
were fixed pre-Copilot in `ee3f035` by adding `dispatch_table_is_superset_of_catalog`, which
asserts the structured catalog (21 tools) is a subset of the text-parsed dispatch names (23),
using the compiler-checked catalog as an oracle so any parser under-count or stale catalog entry
fails loudly. All current dispatch arms are single-line, so the parser is correct for today's
code; the oracle guards against future regressions.

### Canonical doc URL in code references

Copilot review (T3/T4/T7) noted that a repo-relative `docs/cli-mcp-parity.md` reference is not
resolvable for a globally-installed binary run against an arbitrary workspace. The CLI
`long_about` (`src/bin/engram.rs`) and both catalog macros (`cli_desc!` / `mcp_only_desc!` in
`src/shim/tools_catalog.rs`) now emit the canonical URL
`https://github.com/softwaresalt/agent-engram/blob/main/docs/cli-mcp-parity.md`, and the contract
test asserts that URL via a `DOC_URL` constant.

### Uninstall doc corrected to the destructive default

Copilot review (T1/T2) caught that the doc described `engram uninstall` as runtime-only cleanup.
The default (`keep_data=false`) deletes the entire `.engram/` directory
(`src/installer/mod.rs:692-698`); `--keep-data` preserves `config.toml` and removes only runtime
artifacts (`run/`, `logs/`, `.version`). The mapping doc row and summary bullet were corrected to
state the destructive default.

## Review resolution

* **Cross-model adversarial review (pre-PR):** `gpt-5.6-sol` (rust), `gemini-3.1-pro-preview`
  (security), `gpt-5.6-terra` (scope) — P1/P2/P3 findings fixed before the PR opened.
* **Independent adversarial code review (pre-Copilot):** `gpt-5.6-sol` raised two P2s on the
  drift guard; fixed in `ee3f035` (the superset oracle above).
* **Copilot — 5 review passes, all resolved:**
  * `2561d83a` — clean (0 threads).
  * `ee3f035` — 7 findings. T1/T2 (uninstall docs), T3/T4/T7 (canonical URL) fixed in `7d8db7d`.
    T5 (help-resolution does not verify routing identity) and T6 (feature-gated wrapped-arm
    parser under-count) deferred to `090.005-T` with rationale; the currently-known feature-gated
    tools are already enforced via `MCP_WITHOUT_CLI_ALLOWLIST` +
    `mcp_gap_allowlist_matches_documented_gap_rows`.
  * `7d8db7d` — 1 finding: a stray ESC (0x1B) control character in the `090.005-T` task file,
    introduced by a shell escape when the follow-up task was created. Fixed in `444feef`.
  * `444feef` — clean.
  * All threads were replied to and resolved via `resolveReviewThread` after each fix.

## Deferred follow-ups

* `090.004-T` (blocked) — Close functional MCP<->CLI parity gaps. Explicitly **not in the 085-S
  shipment**: the shipment delivered the audit, doc, and guard. Whether any confirmed asymmetry
  should be closed (e.g. adding a CLI analog for an MCP-only tool) is a product-scope decision to
  be re-harvested into a follow-on shipment from the 090.001 audit findings — not implemented from
  the stale list. Kept queued.
* `090.005-T` (queued) — CLI<->MCP parity: routing-identity + compiler-checked dispatch registry.
  Deepens the guard from existence+documentation parity to routing identity (T5) and a
  compiler-checked dispatch-name registry that includes feature-gated names (T6).

## CI note

The `build` job failed once at `7d8db7d` on
`real_path_via_run_retrieval_eval_dispatch_matches_ground_truth`
(`tests/integration/retrieval_eval_regression_test.rs`): the asserted resolved-edge count was 1
instead of the ground-truth 3. The root cause was an upstream `cozo::storage::sqlite` `connect_db`
panic on a tokio worker thread (`cozo-0.7.6/src/storage/sqlite.rs:49`) that degraded the graph
build for that run — an environmental DB-connection flake, not a code regression. The full 085-S
merge (`945ece65`) diff is the new mapping doc (`docs/cli-mcp-parity.md`), the modified contract
test (`tests/contract/lint_dax_cli_parity_test.rs`), CLI/catalog description strings
(`src/bin/engram.rs`, `src/shim/tools_catalog.rs`), and four backlog files (090.001-T–090.003-T
archived, 090.005-T added); none of it is retrieval-eval or DB code.
The job went green on the next run (`444feef`, a one-line backlog-file change), confirming the
flake. This is a candidate for a DB-connect hardening chore.

## Release observability

Feature 090-F changes no runtime surface, schema, or on-disk format, so the rollout carries no
runtime rollback risk. The observability posture is inverted from a normal feature: the shipped
artifact **is** the monitor.

### Healthy signals

* The `contract_lint_dax_cli_parity` contract test passes in CI. Because it derives the mapping
  from the real dispatch table, the CLI `--help` surface, and the catalog at test time, a green
  run is direct evidence that the documented parity matches reality.

### Failure signals

* `contract_lint_dax_cli_parity` fails — a CLI command or MCP tool was added, removed, or renamed
  without a corresponding update to `docs/cli-mcp-parity.md` or the allowlists. The failing
  assertion names the drifted surface.

### Monitoring method, baseline, threshold

* Method: the `contract_lint_dax_cli_parity` contract test in CI.
* Baseline: 11/11 parity assertions pass at merge (`945ece65`); catalog = 21 tools, dispatch = 23
  names (the 2 extra are the `git-graph` feature-gated tools, documented as gaps).
* Threshold to investigate: any failure of `contract_lint_dax_cli_parity`, or a change to the
  dispatch table or CLI command set that is not reflected in the mapping doc.

### Owner and observation window

* Owner: ship and repository maintainer.
* Duration: passive, with a coverage caveat. The guard runs whenever a PR changes a non-doc path
  (`src/tools/`, `src/shim/tools_catalog.rs`, `src/bin/engram.rs`, `tests/**`, `Cargo.toml`),
  which is exactly when the code surface it protects can drift. CI `paths-ignore` excludes
  `docs/**` for pull requests, so a PR that edits **only** `docs/cli-mcp-parity.md` skips the
  build and does not re-run the guard; a doc-only inconsistency would therefore surface on the
  next code-touching PR rather than immediately. This is acceptable because drift originates from
  code-surface changes, which always trigger CI. No timed window is required because there is no
  runtime rollout.
* Outcome (pre-release validation): local gates and CI green; 5 Copilot passes resolved; 4-point
  merge gate CLEAN at `444feef`.

### Rollback trigger and procedure

* Trigger: the mapping doc or drift guard is later found to encode an incorrect parity claim that
  blocks legitimate surface changes.
* Procedure: revert the merge with `git revert -m 1 945ece65`. Runtime blast radius is nil — the
  daemon and CLI behavior are unchanged either way; the revert removes the mapping doc, the
  contract-test changes, and the doc-reference strings in tool descriptions and CLI help. Note the
  revert also reverses the backlog metadata in the same commit: it moves `090.001-T`–`090.003-T`
  out of the archive back to the queue and removes `090.005-T`. That backlog state is inert
  (no runtime effect), but re-apply it manually after the revert if the archival should stand.

## Verdict

SHIPPED. Merged to main as merge commit `945ece65` via PR #259. Shipment 085-S and feature 090-F
are archived; tasks 090.001-T / 090.002-T / 090.003-T are done. Deferred follow-ups 090.004-T
(functional gap closure) and 090.005-T (routing-identity + dispatch registry) remain queued. Next
in queue: 086-S (092-F, writer-side workspace+config atomicity).
