---
title: "167-S pinned workspace status runtime verification"
date: 2026-10-31
shipment_id: "167-S"
feature_id: "142-F"
task_id: "142.080-T"
verdict: "PASS_WITH_FOLLOW_UP"
releasability: "READY_WITH_CONDITIONS"
follow_up_stash: ["D74F9442"]
---

## Scope and prechecks

The changed surface is `get_workspace_status` over dispatch and read-server
request admission. Command adapters exercise the CLI, API and background
activation fixtures declared in `.autoharness/workspace-profile.yaml`.
No browser or external manual checkpoint applies.

The worktree-local build uses `CARGO_TARGET_DIR=target/validation` because an
existing daemon owns the default-directory executable. No process was stopped.
`cargo build --all-targets` passed. The built CLI reports
`engram 0.3.0-rc.1+g5f1d8023-dirty`, identifying the pre-commit build.

## Validator evidence

| Surface | Command or fixture | Expected and observed result |
|---|---|---|
| CLI | `target/validation/debug/engram.exe --version` | Version printed; exit 0 |
| CLI/API | Built CLI with `--workspace . daemon-status` and `workspace-status` | Bound read-server PID 43836 responds; exit 0 |
| API | `cargo test --test integration_generation_pinned_workspace_status -- --nocapture` | Three scenarios pass, three consecutive runs |
| API | Same target after fixture repair, including direct binary execution from `tmp/167-fixture-check` | All three pass with no pre-existing `tmp` child |
| Background activation/API | `cargo test --test integration_read_server_generation_wiring -- --nocapture` | Thirteen scenarios pass in three consecutive runs with identical outcome maps |
| API isolation | `cargo test --test integration_lifecycle_read_generation_pin --test contract_read_path_pinning_enforcement` | Three lifecycle and four enforcement checks pass |

The bound daemon is an existing process, not the rebuilt server. Its status
probe proves reachability and binding only. Its yellow unindexed-session
health and absent active generation are not represented as a successful
activation of this implementation. Behavioral evidence comes from the test
fixtures, including pinned counts, managed binding preservation, absent-root
observability and no unintended database creation. No live sync was requested
against the operator's read-server workspace.

## Quality and review

Pedantic clippy and formatting passed. The full
`cargo dev-test --no-fail-fast` run completed with three failing targets:

* Library cancellation/indexing test: missing retained prepass snapshot
* Shim pre-initialize concurrency test: expected probe response not observed
* Daemon startup-order test: 30-second timeout

Each failing test passed on its first exact isolated rerun, without source
changes. The raw full-suite exit was 101; it is not reported as an unqualified
zero-failure run. The task's target and generation-wiring cases remained green.

The final full suite with `RUST_TEST_THREADS=1` subsequently exited 0:
278 test binaries, 2,604 passed, zero failed and 20 ignored.
No test was excluded. Final formatting, pedantic clippy, all-targets build
and coverage completeness oracle also passed.

Three-persona review identified a fixture-parent assumption, reproduced from
a clean working directory and repaired without weakening assertions.
Post-remediation consensus: no remaining P0/P1/P2/P3 in-scope findings.
The reviewed production blobs are unchanged in isolated IC
`5beae02c670d8410c5f5f0e125f3641185183428`; the fixture repair is `4c063863`.

## Releasability handoff

* Verdict: `PASS_WITH_FOLLOW_UP`; no missing runtime prerequisite
* Invariants: generation data supplies only code-graph counts; managed binding
  metadata remains stable; only status reads receive the no-generation
  observability fallback; no managed database opens on generation reads
* Healthy signals: pinned target and generation wiring stay green; status
  retains binding fields and reports generation provenance when admitted
* Failure signals: incorrect generation counts, binding drift, unexpected
  database creation, persistent startup or handshake failures
* Monitoring: repository maintainer observes the next `main` CI cycle and
  downstream PA5 consumer runs, using `docs/log-observation-guide.md`
* Rollback trigger: a reproducible status/admission regression after one
  isolated confirmation
* Rollback: a reviewed revert branch affecting only this task's source/test
  changes; preserve backlog archival and closure evidence
* Owner: repository maintainer `softwaresalt`
* Validation window: the first 30 minutes after merge, including the next
  `main` CI cycle; downstream PA5 consumer validation remains a later check
* Release path: merge-only slice; no separate tag, publish or deployment
* Residual risk: `D74F9442`, eight pre-existing unchanged-`Cargo.lock`
  advisories accepted by explicit Operator/Stage P-021 C3 disposition

ProposedAction: merge the bounded pinned-status conversion after current-HEAD
review and hosted checks. ActionRisk: moderate lifecycle/dispatch change.
ActionResult: local fixture and final full-suite verification passed.
Delivery is halted before PR creation because the operator-authored routing
commit `9f10775d` added `.autoharness/config.yaml` outside shipment scope.
Hosted gates and merges remain unperformed. Merge is pre-authorized only
within 167-S; admin fallback is not authorized. This runtime record is not a
post-merge closure artifact and does not satisfy P-020 closure.

## Confirmed resume

The preceding halt is historical. On `2026-10-31`, the operator explicitly
confirmed recovery of `checkpoint-20261031-054021.json` and authorized
non-rewriting relocation of the routing update into the Step 6 closure PR.
Restore, bounded pruning, resume and official checkpoint resolution succeeded.
The feature branch now restores the configuration to its `origin/main` blob;
the original operator commit remains in history.

The resumed pinned-status sanity check passed 3/3. Rust and Cargo blobs are
unchanged since `4c063863`; configuration and lockfile net diffs are empty.
The lifecycle gate passed. Delivery resumes at Step 5, subject to current-HEAD
review and hosted gates. `D74F9442` remains the accepted baseline follow-up.
Both scoped merges are pre-authorized; admin fallback remains prohibited.
