---
title: "A feature-cfg gate can encode a false premise that a trivially-passing test cements"
description: "get_workspace_status returned all-zero code_graph counts in every shipped default build because the DB read was gated behind #[cfg(feature = git-graph)] on the false premise that the code-graph indexer needs git-graph. The guarding test (s072) bound an EMPTY workspace and asserted code_files==0, so it passed trivially while cementing the bug; the only positive test was #[ignore]d on the CI platform. Meanwhile get_workspace_statistics read the same counts ungated and was correct — a same-data divergence between two tools."
problem_type: "shipped_default_bug_hidden_by_feature_gate_and_trivial_test"
category: "observability-correctness"
component: "src/tools/lifecycle.rs get_workspace_status / feature flags / test matrix"
root_cause: "a #[cfg(feature = X)] gate encoded an assumption about a subsystem that was false (git-graph gates git *history* tooling, not the code-graph indexer); the negative test exercised a trivial input (empty workspace) so it passed for the wrong reason; the positive test that would have caught it was ignored on the CI OS"
resolution_type: "remove_false_gate_and_add_positive_default_feature_test"
date: "2026-07-07"
shipment: "078-F"
---
# A feature-cfg gate can encode a false premise that a trivially-passing test cements

## Problem

`get_workspace_status` gated its code-graph counts behind
`#[cfg(feature = "git-graph")]`, with the `#[cfg(not(...))]` arm returning
`CodeGraphStats::default()` (all zeros). The shipped binary uses default features
(`embeddings, cozo-backend`, **no** `git-graph`), so **every end user** saw
`code_graph: {0,0,0,0,0}` regardless of how much was indexed. The sibling tool
`get_workspace_statistics` read the *same* counts via `CodeGraphQueries::count_*`
with no gate and was correct (271 files / 2332 symbols) — a silent divergence
between two tools reporting the same data.

Separately, `get_daemon_status.memory_bytes` used `sysinfo::System::used_memory()`
(whole-machine RAM, ~22 GB) instead of process RSS (~1.2 GB) — the same class of
"wrong scope" reporting bug. `get_health_report` used the correct
`process(pid).memory()` pattern, so again two tools diverged on the same concept.

## Why the tests didn't catch it

1. **The negative test passed for the wrong reason.** `s072` bound an *empty* temp
   workspace and asserted `code_files == 0` "without git-graph feature." With no
   source files the count is zero regardless of the gate, so the test cemented the
   buggy premise without ever exercising a populated graph.
2. **The positive test was ignored on the CI platform.** The only test asserting
   non-zero `code_graph` from `get_workspace_status`
   (`graph_vector_rehydration_test`) is `#[cfg_attr(any(windows, linux), ignore)]`
   for an unrelated CozoDB SQLITE_BUSY restart issue — so it never ran on CI (Linux)
   or on Windows dev machines.
3. **Feature-matrix blind spot.** CI's unit lane is `test --lib` (no integration
   tests); CI's integration lane runs `--no-default-features --features
   cozo-backend,embeddings` (git-graph OFF, so the gated block compiles out); the
   only lane where the gated code compiles is `--all-features` (`cargo ci`), which
   the GitHub Actions workflow does not use.

## Lessons

- **Treat a `#[cfg(feature = X)]` around a data read as a claim that must be true in
  every build, and test the shipped default feature set explicitly.** A bug that
  only manifests without an optional feature is invisible to any lane that enables
  that feature.
- **When two tools expose the same underlying data, assert they agree.** The fix's
  regression test cross-checks `get_workspace_status` against
  `get_workspace_statistics`; that invariant would have caught the divergence.
- **A test that asserts `== 0` / an empty result is suspect.** Confirm it fails when
  the code is wrong: `s072` could never have failed because its input produced zero
  legitimately. Prefer positive fixtures (index a real file, assert `> 0`).
- **"Wrong scope" reporting bugs cluster.** System-vs-process memory and
  feature-gated-vs-ungated counts were both "one tool right, its sibling wrong."
  When you find one, grep for the same concept elsewhere (`used_memory` vs
  `process(pid).memory()`; gated vs ungated `count_*`).
- **Verify observability fixes in the *running* daemon.** Rebuild + reinstall +
  restart the workspace daemon and read the tool back: `daemon-status.memory_bytes`
  must track process RSS, `workspace-status.code_graph` must match `stats`.

## Fix

Remove the false gate; read the counts unconditionally (mirroring
`get_workspace_statistics`), warn on DB-connect failure instead of a silent zero.
Extract a shared `services::process_memory::current_process_memory_bytes()` used by
`get_daemon_status`, `get_health_report`, and the legacy handler. Replace the
trivial `s072` with a positive, default-feature test that indexes a real fixture
and asserts non-zero counts equal to `get_workspace_statistics`.
