---
title: "PA-5 deliberation: read-handler conversions needed by F54"
date: 2026-09-27
feature: 142-F
stash_sources:
  - 86F93068
  - 4628001C
related_tasks:
  - 142.058-T (F54, parity test)
  - 142.065-T (PRE-4b)
  - 142.066-T (PRE-4)
status: decided
decided_on: 2026-10-04
decision_record: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md Revision 18, section 1.1 OD-5 (verbatim) and readings R-1, R-2; planning and harvest in PA5-P (section 7.6)"
author: Stage
---

> **Status (2026-10-04, PR #410 review): decided.** The operator answered OD-5 on 2026-10-04 12:32 -07:00: "Q1: Yes;
> Q2: Yes; Q3: Yes, although that would seem to contradict the decision on Q1; PA-5 tasks can go under 142-F." Option
> PA5-C is selected; the decomposition plan Revision 18 records the answer verbatim (section 1.1) with readings R-1 and
> R-2, and Stage plans and harvests the PA-5 tasks in PA5-P (section 7.6) as Slot-19.k under `142-F`. The "awaiting
> operator decision" wording below describes the state on 2026-09-27.

# PA-5 deliberation: read-handler conversions needed by F54

This record covers the deliberation step only. It has no implementation plan,
no plan review, and no harvest. Nothing in the backlog, the stash, or the source
was changed.

## Problem frame

### What stash `86F93068` says

In read-server mode, only `get_workspace_statistics` is meant to read the
admitted (pinned) generation's database. That happens once PRE-4b
(`142.065-T`) and PRE-4 (`142.066-T`) land. Every other generation-backed read
handler still reads the managed binding: `snapshot_dispatch_context`,
`ReadRequestContext::from_managed_state`, or `connect_db(data_dir)`. Request
entry also dispatches those handlers with no pinned context, so their responses
carry no `provenance.generation_id`.

Checked against source on 2026-09-27:

* `GENERATION_PINNED_READS` does not exist in `src/` yet. `142.066-T` (queued)
  introduces it in `src/daemon/request_entry.rs` as
  `&["get_workspace_statistics"]`.
* `tools::dispatch_with_read_context` (`src/tools/mod.rs`) already attaches
  provenance generically when it is given a context that has a generation.
  However, every dispatch arm calls a two-argument handler, so the context is
  only a label. PRE-4b fixes this for `get_workspace_statistics` alone.
* `get_workspace_status` (`src/tools/lifecycle.rs:1115`) opens
  `connect_db(&snapshot.data_dir, &snapshot.branch)` for its `code_graph`
  block. This is stash `4628001C`. It swallows count errors with
  `unwrap_or(0)`.

### Which handlers F54 actually exercises

`tests/contract/read_server_cli_mcp_parity_test.rs` has five tests. Two of them
depend on PA-5:

| F54 test | Handlers it needs converted |
|---|---|
| `cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read` | `get_workspace_status` only (CLI and MCP must both be accepted, carry `provenance.generation_id == EXPECTED_GENERATION`, and be equal apart from `connection_count`). |
| `generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior` | Every descriptor where `generation_read()` is true: `Read` + `read_server_available` + `DaemonHandler`. Each must be accepted **and** carry the expected generation on **every** declared surface. |
| `generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces`, `control_descriptors_are_refused_without_side_effects`, `unknown_ipc_methods_are_refused_without_side_effects` | None. |

The matrix test covers these rows (from `src/tools/capabilities.rs`):

| Group | Handlers | Data they read today |
|---|---|---|
| G1 status | `get_workspace_status`, `get_daemon_status` | Managed binding. The status reads the code-graph DB through `connect_db`. The daemon status reads process facts and a health summary, not generation data. |
| G2 code-graph | `query_memory`, `map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_graph`, plus `query_changes` under `git-graph` | Managed `connect_db`. |
| G3 reports | `get_health_report`, `get_branch_metrics`, `get_token_savings_report`, `get_evaluation_report`, `get_mutable_script_retry_metrics`, `get_retrieval_eval_report` | `.engram/metrics/` files and the retrieval-eval report, not the generation DB. The manifest inventory seals only `engram.db`. |
| G4 lint | `lint_dax` | Workspace source files through the managed binding. |
| Done by PRE-4b/PRE-4 | `get_workspace_statistics` | Generation DB. |

That is 15 rows to act on in the default build, and 16 with `git-graph`.

### Two findings that change the scope

1. **The equivalence test alone needs one conversion, but F54 as a whole needs
   all 15.** Converting only `get_workspace_status` can turn the equivalence
   test GREEN. The matrix test stays RED until every generation-read row is
   accepted with provenance.
2. **Converting handlers is not enough for the matrix test.** F54's fixture
   generation database contains only `:create probe_row {id => val}`
   (`publish_fixture_generation`, lines 476-522). The handlers that do not
   hide errors fail against it. `list_symbols` and `impact_analysis` return
   `CodeGraphError::SymbolNotFound` (7004) for `f54_fixture_root`
   (`read.rs:424`, `read.rs:795`), and the others hit missing-relation query
   errors. PRE-4 already expects this for the `stats` row. For the G2 rows to
   be accepted, the fixture must publish a real, indexed generation of its own
   `src/lib.rs`, using the PRE-1/PRE-2 build-and-publish path. That fixture
   change is test work inside `142.058-T`, not handler work.
   `get_workspace_status` does not need the fixture change, because its counts
   fall back to `unwrap_or(0)`.

## Options

### PA5-A: Convert only what the equivalence test pins, and defer the rest

* **Scope:** Convert `get_workspace_status` so its `code_graph` block reads the
  admitted generation DB. Take an `Option<&Arc<ReadRequestContext>>` using the
  PRE-4b two-argument-wrapper pattern, and add the method to
  `GENERATION_PINNED_READS`. Record the other 14 rows as a new follow-up stash
  entry.
* **Tasks:** 1 (S, medium).
* **Pros:** Smallest change. It also resolves `4628001C`, and it reuses the
  PRE-4b seam unchanged.
* **Cons:** F54 (`142.058-T`) still cannot close GREEN, because the matrix test
  stays RED. Its acceptance criteria would have to be split, or it would stay
  active or blocked for a long time. The S4 release (see the glossary) would
  ship with a known RED contract test, or without F54.

### PA5-B: Convert every generation-backed read

* **Scope:** Give every G1-G4 handler an admitted-context variant, add each one
  to `GENERATION_PINNED_READS`, and change the F54 fixture to publish a real
  indexed generation.
* **Tasks:** about 9 or 10.
* **Pros:** F54 goes fully GREEN with no change to its expectations.
* **Cons:** G3 and G4 do not read generation data. The metrics files, the eval
  report, and the DAX sources are not in the sealed inventory. PRE-4b also
  records that a generation context has `root_path() == None`, so a converted
  metrics read would either find nothing or quietly fall back to the managed
  workspace. PRE-4b and D5-D forbid a managed-data fallback, so the only legal
  version of PA5-B puts a generation label on data that did not come from the
  generation. `get_daemon_status` has the same problem, because it reports
  daemon facts. This option tells F54 what it expects, but not the truth.

### PA5-C (recommended): Convert by data class, and make F54's expectation honest for non-generation reads

* **Scope:**
  1. **G1 status and G2 code-graph** (`get_workspace_status`, `query_memory`,
     `map_code`, `list_symbols`, `unified_search`, `impact_analysis`,
     `query_graph`, and `query_changes` when `git-graph` is on): real
     conversions that follow the PRE-4b pattern. Each keeps a byte-identical
     two-argument managed wrapper, adds a `_with_context` variant, uses the
     shared `queries_from_read_context` seam (generation DB when
     `generation()` is `Some`, and never `connect_db` on a generation
     context), updates its dispatch arm, and joins `GENERATION_PINNED_READS`
     in the same change.
  2. **Non-generation reads** (`get_daemon_status`, G3 reports, and G4
     `lint_dax`): no data-path conversion. Add a descriptor field to F19
     (working name `generation_backed: bool`) and extend F54's
     `expected_outcome` with a new outcome (working name
     `DaemonScopedRead`): accepted on every declared surface, with **no**
     generation provenance, and no side effects. They stay reachable on a read
     server, which is what `read_server_available` already promises.
  3. **The F54 fixture** publishes a real indexed generation, so the G2 rows
     can be accepted for real data rather than for an empty database.
* **Tasks:** about 7, listed below.
* **Pros:** F54 goes fully GREEN, and every provenance claim is true. The
  follow-up surface is smaller than in PA5-B. The new descriptor field also
  gives a later change one place to derive `GENERATION_PINNED_READS` from, if
  wanted.
* **Cons:** It changes the F19 descriptor type and F54's expectation table, so
  it touches the contract. It needs your OK (Q1), because an unreviewed change
  to a RED contract test's expectations looks like weakening the test.

### Options considered and rejected

* **Handlers tolerate missing relations** (treat query errors as empty
  results) so the probe_row fixture passes. Rejected: it hides real read
  errors, and it contradicts the typed-error contract.
* **Managed-data fallback when the generation read fails.** Rejected: forbidden
  by D5-D and by the PRE-4b "never add a managed-data fallback" rule.

## Recommendation

Choose **PA5-C**. It converts only the reads that really come from the
generation database, which are `get_workspace_status` and the six (or seven)
code-graph reads. The daemon-status, report, metrics, and lint reads stay
available on a read server and do not claim a generation they never read.
F54's fixture is upgraded to a real indexed generation, so the converted reads
are tested on real data. F54 can then close fully GREEN with truthful
provenance.

### Provisional task outline (for the later impl-plan step, not harvested)

| # | Provisional task | Files (production, then test) | Size / complexity | Depends on |
|---|---|---|---|---|
| PA5-T1 | Pinned `get_workspace_status`: the `code_graph` block reads the admitted generation, and the method joins `GENERATION_PINNED_READS` (consumes `4628001C`) | `src/tools/lifecycle.rs`, `src/tools/mod.rs`, `src/daemon/request_entry.rs` (one-line list edit); harness in a new integration test | S / medium | `142.065-T`, `142.066-T` |
| PA5-T2 | Pinned `query_memory` and `list_symbols` | `src/tools/read.rs`, `src/tools/mod.rs`, `src/daemon/request_entry.rs` (list edit) | S / medium | PA5-T1 |
| PA5-T3 | Pinned `map_code` and `impact_analysis` | same three | S / medium | PA5-T2 |
| PA5-T4 | Pinned `query_graph` and `unified_search` (the unified search also touches embedding status, so it may need to split) | same three | M / medium | PA5-T3 |
| PA5-T5 | Pinned `query_changes` (`git-graph` only) | same three | S / low | PA5-T4 |
| PA5-T6 | F19 `generation_backed` field, and F54 `DaemonScopedRead` outcome for G3, G4, and `get_daemon_status` | `src/tools/capabilities.rs`; `tests/contract/read_server_cli_mcp_parity_test.rs` | S / low | `142.066-T`; Q1 approval |
| PA5-T7 | F54 fixture publishes a real indexed generation of its `src/lib.rs` | `tests/contract/read_server_cli_mcp_parity_test.rs` only | S / medium | PRE-1/PRE-2 (`142.061-T`/`142.062-T` range); Q2 approval |

* **Count:** 7 tasks, or 6 if PA5-T5 is folded into PA5-T4. Each is within the
  2-hour rule. Every conversion task touches fewer than 5 functions and has 3
  or fewer test scenarios, following the PRE-4b shape (pinned read, the data
  is real, no stray databases).
* **Ordering:** PA5-T1 to PA5-T5 run in sequence. They all edit `read.rs` or
  `lifecycle.rs` together with `mod.rs` and the `GENERATION_PINNED_READS`
  list, so parallel branches would conflict, and P-016 allows only one active
  implementation branch anyway. PA5-T6 and PA5-T7 are independent of the
  conversions. F54 (`142.058-T`) goes GREEN only after all 7 tasks.
* **Invariants that every PA-5 task carries forward from PRE-4b and PRE-4:**
  the two-argument managed path stays byte-identical, and `connect_db` is
  unreachable on a generation context. There is no managed fallback, and
  `src/db/` is untouched. Each method joins `GENERATION_PINNED_READS` in the
  same change that converts it. Managed mode stays byte-identical, and
  `_health` never consults the gate.
* **Release placement:** all PA-5 tasks belong in S4 (the release with the
  parity test, docs, and PA-5 work), after S1's `142.065-T` and `142.066-T`.
* **F54 interaction during the S1-S3 releases:** F54's equivalence case stays
  RED until PA5-T1, as PRE-4 already records. The quiescence evidence from
  PRE-4 revision 5 (a pinned read against an unchanged manifest leaks no
  activation side effects) now also covers `get_workspace_status`. That
  matters because F54's `workspace_binding_fingerprint` calls
  `get_workspace_status` inside the refusal windows. PA5-T1 must re-assert that
  evidence for `status`.

## Disposition for `4628001C`

* **Consumed by PA5-T1** under every option (A, B, or C), because all three
  convert `get_workspace_status`.
* **Resolution:** in read-server mode, the `code_graph` block reads counts from
  the admitted generation's database. `connect_db` runs only on the managed
  (`None` context) path. This removes the tension with `142.052-T`'s rule that
  a read server does no hydration.
* **Unchanged:** managed-mode behavior, including the tolerant `unwrap_or(0)`
  counts.
* **Stays valid:** `142.063-T`'s PRE-3F characterization note about
  `connect_db` running at HEAD. It records behavior before PA-5.
* **Stash action:** none now. The entry stays active until PA-5 is harvested.
  It is then archived with a forward reference to PA5-T1. Its reconciliation
  record (PR #407; review thread N/A) stands.

## Open questions for the operator

1. **Q1 (decides PA5-B versus PA5-C).** Reads that do not come from the
   generation are `get_daemon_status`, the five metrics and report reads,
   `get_retrieval_eval_report`, and `lint_dax`. Should they be labeled with
   generation provenance (PA5-B), or marked "not generation-backed" in F19 so
   F54 expects them to be accepted with no provenance (PA5-C, recommended)?
2. **Q2.** May `142.058-T`'s F54 fixture be changed to publish a real indexed
   generation of its own `src/lib.rs`, replacing the `probe_row`-only
   database? Without this change, the code-graph rows cannot be accepted under
   any option.
3. **Q3.** For a pinned `get_workspace_status`, which fields come from the
   generation? The recommendation is that only the `code_graph` counts change
   source. `path`, `branch`, `db_path`, `stale_files`, and `scan_status` keep
   reporting the managed binding, so F54's binding fingerprint stays stable.
   The alternative is to report the runtime copy's `db_path`.
4. **Q4 (only if you pick PA5-A).** Should F54 (`142.058-T`) be split into
   "equivalence GREEN now" and "matrix GREEN later", or stay active until the
   follow-up lands?

## Next Stage step once you answer

After your answers: run impl-plan on this record, then plan-harden (the
dispatch changes call for careful mode), then plan-review. The harvest goes
under `142-F`, and the tasks go into the S4 release.
