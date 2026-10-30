---
title: Copilot launcher preflight timeout repair
date: 2026-10-29
updated_at: 2026-10-29T21:05:49Z
status: superseded
---

Historical timeout-workaround evidence, superseded by the
[startup separation](../../memory/2026-10-30/copilot-startup-index-separation-memory.md).
The timeout and executable-override changes below were removed. The original
forced-build circuit remains open; it was not retried.

## Scope and evidence

* Work item: 143-F, blocked pending operator disposition
* Branch: `fix/copilot-preflight-budget`; one worktree
* Original launcher gave the supervisor 12000 ms and capped overrides at 30000 ms.
* Preflight builds a full generation, including embeddings, before readiness probes.
* Regression observed the old `12000` argument instead of the larger budget.
* A real five-minute launch also failed with `relay_deadline`, elapsed 299107 ms.
* The first 30-minute real launch also failed with `relay_deadline`, elapsed
  1799136 ms.
* Final launcher default is now 1800000 ms, with the existing 3000 ms relay/margin
  reservation. The maximum is 1800000 ms; explicit shorter budgets remain supported.

## Changes and checks

* Changed both launchers, their contract tests, the troubleshooting guide, and
  the operator glossary.
* Added optional `ENGRAM_EXE_PATH` to both launchers, preserving existing defaults.
* Optimized Cozo in development builds with a package-scoped Cargo profile.
* Ignored generated cascade-close probe directories and `.pytest_cache/` because
  inaccessible directories blocked Cargo's initial package fingerprint scan.
* Tried Cargo package exclusions first; ineffective, so reverted that manifest edit.
* No permissions, pre-existing processes, or generated Engram data were manually
  changed. Stopped only the daemon created by this session's initial CLI probe.
* Both binaries built with `cargo build --workspace --bins`.
* Built an optimized executable pair with
  `cargo build --workspace --bins --target-dir target\startup` because Windows
  prevented replacing in-use default-target executables. Did not terminate
  unidentified processes or change their permissions.
* Final launcher contract tests passed: 27 tests across `contract_start_launcher`,
  `contract_start_launcher_failure`, and `contract_start_sh_launcher`, using
  `target\validation`.
* Formatting and strict all-target Clippy passed. Formatting passed again after
  removing the temporary diagnostic source.
* Full `cargo dev-test` was not run; focused launcher tests covered the changes.
* `cargo audit` failed on eight pre-existing advisories in the unchanged lockfile.
  Dependencies were not changed as part of this startup repair. Captured the
  separate dependency-maintenance scope as stash 25A1BE9E, requiring deliberation.

## Runtime diagnostics and final outcome

* An optimized real preflight returned Build `cause=verifier` after 930870 ms.
* Direct full sync against that unpublished candidate succeeded in 634059 ms:
  113 parsed files, 334 hash-skipped files, no file errors.
* A temporary example reproduced the actual candidate builder on a current-thread
  runtime and exposed SQLite `database is locked (code 5)`. Read-only progress
  probes overlapped the writer and could cause contention; discontinued them.
* Removed the temporary `examples/startup_build_diagnostic.rs` source.
* Final actual launch used `target\startup\debug\engram.exe` without SQLite probes
  and still failed Build with `relay_deadline` after 1799091 ms.
* This was the third deadline failure. The circuit is open; do not retry startup.
* Evidence: [circuit-breaker checkpoint](../../memory/2026-10-29/circuit-break-copilot-preflight.md).
* No successful preflight or actual Copilot version output was observed.
* `.env.local` was not modified; ordinary startup does not select the isolated
  executable unless the operator supplies `ENGRAM_EXE_PATH`.
* No commits, pushes, PRs, or merge actions were performed.

## Safety and next steps

* Investigate-first mode; scoped launcher/build changes are moderate risk.
* No destructive action approved or performed.
* Await operator guidance; the repair is not complete.
* A separate full-build performance investigation, potentially using a release
  build, is a candidate next step, not an authorized fourth startup attempt.
* Preserve the uncommitted changes for review. Do not mark 143-F done or report
  startup readiness from the passing fixture tests.
