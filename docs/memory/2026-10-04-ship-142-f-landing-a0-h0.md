# Ship memory: 142-F landing steps A0, C1, A0R, T0, H2a, H0

Date: 2026-10-04. Plan: `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` (Revision 18, frozen), section 7.1.
Precondition: P0 PASSED (`docs/scratch/2026-10-04-142-f-p0-worktree-classification.md`, 118 paths: W1 72, W2 46).

Approvals recorded (not re-asked): PS-2 (A0/A0R), PS-13 (C1), PS-3 (T0, PA-7), PS-4 (H2a tag push only, PA-7,
careful mode, high risk), PS-12 (H0, operator OD-2 answer 2026-10-04).

## Results

* **A0** (parked branch, HEAD `f6f3171f`, before any switch): one worktree; no dirty path under `src/`, `tests/`,
  `crates/`, Cargo files or `build.rs`; 118 dirty paths. B0 = `cargo test --test contract_read_server_cli_mcp_parity
  -- --nocapture`: 1 passed, 4 failed (expected B0 status; no A0 mismatch). Built in `tmp/b0-target` because the
  running engram daemon locks `target/debug/engram.exe` (first attempt failed at build, no test ran).
* **CG-B**: `origin/main` = `32f6f9c7` extracted with `git archive | tar` into `tmp/cgb-main`, `cargo dev-test
  --no-fail-fast`: **RED**, 269 targets, 2495 passed, 2 failed, 20 ignored.
  `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary` fails reproducibly
  (known baseline, owner `142.060-T` / Slot-01). `integration_daemon_startup_order::run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`
  failed under load and passed on isolated rerun (flake). Consequence: later slots wait on Slot-01 (CG-B); A0-H0 not
  stopped.
* **C1**: `.backlogit/checkpoints/checkpoint-20260924-010034.json` copied to `tmp/w2-carry/`; SHA-256 both
  `697EBAC81E1595ADED44524F8DDD559C8FE655B072CA453465E4D31418849BF1`; restored to HEAD; path clean. H3 copies it back.
  Keep `tmp/w2-carry/` until H3.
* **A0R**: `0f2cca208827f09d9ac3e76b3db110391c6976c7` (parent `f6f3171f`), commits only
  `docs/memory/2026-10-04-ship-142-f-a0-b0.md` (B0 path; per-row map, full output, CG-B, C1 hash).
* **T0**: 7.3 re-derived from `main..A0R^` (11 commits), identical. Annotated tag `parked/142-s-split-0f2cca20`, tag
  object `bb82ad4ef0f9b167e0ab7f039bdfd39f99ab7311`, peeled `0f2cca20...`; all eleven parked SHAs reachable;
  `git cat-file -e <T0>:docs/memory/2026-10-04-ship-142-f-a0-b0.md` exit 0.
* **H2a**: no workflow tag trigger matches `parked/` (only `release.yml`, `v[0-9]*.[0-9]*.[0-9]*`); no pre-push hook.
  `git push origin refs/tags/parked/142-s-split-0f2cca20` (tag only). Remote
  `https://github.com/softwaresalt/agent-engram.git`: `bb82ad4e... refs/tags/parked/142-s-split-0f2cca20`,
  `0f2cca20... refs/tags/parked/142-s-split-0f2cca20^{}` - both equal local. Parked branch not on the remote.
* **H0**: `git fetch origin`; `git switch -c chore/harness-142-f-w3 origin/main` (dirty paths carried, no
  collision); `git cherry-pick -x f6f3171f` conflicted in `.autoharness/config.yaml` only (as predicted); the nine
  operator blobs taken; commit `87b23274c0cdf0a2dbd0c35660f5fc3cb2932522` changes exactly the nine 7.3 paths,
  operator author and message kept. Gates: `cargo fmt --all -- --check` pass; `cargo clippy --all-targets -- -D
  warnings -D clippy::pedantic` pass; YAML/JSONL parse check pass; no test run (harness/config/docs only).
  PR https://github.com/softwaresalt/agent-engram/pull/409 (`READY_WITH_FOLLOWUPS`), Copilot review requested.
  Not merged: operator merges with a merge commit (P-014, P-009).

## Residual risk / follow-up

* P3, operator-authored: the operator's `config.yaml` removes main's `model_routing.stage.escalation` and the empty flat
  `model_routing.escalation`; Stage escalation falls back to tier3, the same route as Stage (ESCALATION_DEGRADED).
  Noted in the PR body for the operator.
* All PR paths are in `ci.yml` `paths-ignore`; CI may not run on PR 409.

## State at end

* Branch: `chore/harness-142-f-w3` (H1 starts from fetched `origin/main` after H0 merges; no return to the parked
  branch is required). Parked branch tip `0f2cca20`, untouched after A0R.
* Dirty paths: 117 = P0's 118 minus the C1 checkpoint (backed up, clean). All 117 are in the P0 list.
* HALTs: none.

## H1 landing PR #410 (2026-10-04)

* PR: https://github.com/softwaresalt/agent-engram/pull/410; branch `chore/stage-142-f-planning`.
* Final pushed HEAD: `e43517892dd4c48827261bb6188e465ce6bd8e72`.
* Copilot review: `COMMENTED` on the final HEAD, no new findings; 3 original comments replied to and resolved. One W1 Stage-memory YAML issue fixed in the final commit; two Stage-owned acceptance-criteria findings deferred as `3D5F23F0` and `47DAEF9D` (P-021 C1; frozen plan Revision 18 unchanged).
* Local readiness: `READY_WITH_FOLLOWUPS`; P-018 gate: `SATISFIED`; unresolved review threads: 0.
* CI: no checks reported (docs/backlog-only paths ignored). Copilot noted partial review coverage (38/73 paths).
* PR is `MERGEABLE` / `CLEAN`; not merged. Awaiting the operator's P-014 merge decision; Stage deliberation remains needed for both deferred entries.

## H0 merged (2026-10-04)

* Operator approval (verbatim, 2026-10-04 16:55 -07:00): "PR 409: Merge approved".
* Merge gate re-verified immediately before merge: HEAD `87b23274c0cdf0a2dbd0c35660f5fc3cb2932522`; Copilot review
  `5408157235` (COMMENTED) on `commit_id` == HEAD; `requested_reviewers` empty; review threads 4 total / 0 unresolved;
  `mergeable_state` clean; `autoharness gate copilot-review 409 --enforcement auto` = SATISFIED (exit 0).
* P-009: repo allows merge commit only (squash/rebase disabled). `gh pr merge 409 --merge --match-head-commit 87b23274...`
  (no `--admin`). Merged 2026-10-04T23:56:21Z.
* Merge commit on `origin/main`: `7984f8965b66f2b0dfa4711c00b08a86e1e373a7` (parents `32f6f9c7`, `87b23274`).
* Blob check: `git diff --quiet f6f3171f origin/main -- <nine 7.3 paths>` exit 0 (equal).
* Branch `chore/harness-142-f-w3` not deleted (repo `delete_branch_on_merge: false`). Local worktree left on
  `chore/harness-142-f-w3`; dirty paths untouched (113 `git status --short` entries, same before and after merge);
  `tmp/w2-carry/` kept.
