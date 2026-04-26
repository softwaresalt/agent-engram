---
title: Autoharness tune PR memory
date: 2026-04-26
branch: chore/autoharness-tune-2026-04-26
pr: 30
status: blocked
---
## Completed work

* Applied the autoharness v1.3.2 tune-up on `chore/autoharness-tune-2026-04-26`
* Committed and pushed:
  * `d2e4665` - harness tune-up across Stage, Ship, PR lifecycle, workspace discovery, and manifest/report artifacts
  * `4191fbe` - tracked pre-existing workspace/startup state plus backup snapshots
* Opened draft PR [#30](https://github.com/softwaresalt/agent-engram/pull/30)
* Confirmed `cargo fmt --all -- --check` passes
* Confirmed `cargo clippy -- -D warnings -D clippy::pedantic` passes

## Blocked condition

`cargo test` fails in an unrelated integration test:

```text
integration_graph_vector_rehydration::daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted
```

The failure reproduces when rerunning the single test. The restarted daemon reports
`code_graph.edges = 0` after rehydration, which violates the test expectation that
at least two edges are restored.

## Scope check

This branch does not modify Rust source files or test files. The diff is limited to:

* `.github/agents/`
* `.github/skills/`
* `.github/instructions/`
* `.autoharness/`
* `agent-engram.code-workspace`
* `start.ps1`

## PR state

* PR URL: <https://github.com/softwaresalt/agent-engram/pull/30>
* Draft: yes
* Base branch: `main`
* Head branch: `chore/autoharness-tune-2026-04-26`
* CI: rerunning after review-fix commit `bc9df10`
* Copilot review request via `gh pr edit --add-reviewer copilot` was unavailable in this environment

## Review follow-up

* Addressed 7 Copilot review comments in commit `bc9df10`
* Fixed `start.ps1` to:
  * default `COPILOT_HOME` to the workspace-local `.copilot` directory
  * inject generated agents into `.github/local-agents/`
  * preserve CLI argument passthrough with `@args`
* Clarified `.github/instructions/architecture-doc.instructions.md` so `docs/research/` has a single unambiguous purpose statement
* Rewrote `.github/agents/ship.agent.md` source-artifact cleanup guidance to use only supported backlog operations and existing `references` links
* Redacted machine-specific absolute paths from `.autoharness/tuning-reports/2026-04-26-tuning-report.md`
* Updated `.autoharness/harness-manifest.yaml` so the TUNE-017 note matches the current branch strategy
* Replied to each Copilot comment after push and resolved all 7 review threads via `gh api graphql`
* Identified and fixed 4 follow-up Copilot comments after a later review pass

## Next step

Address the 4 open Copilot follow-up threads on `start.ps1` and this memory file,
then push the fixes, reply on each thread, resolve them with `gh api graphql`,
and move PR #30 to the merge approval gate.
