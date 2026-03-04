---
name: fix-ci
description: "Usage: Fix CI. Detects CI pipeline failures on the current branch's PR, reproduces and fixes errors locally, runs all CI gates, then pushes and polls until the pipeline passes."
version: 1.0
input:
  properties:
    pr-number:
      type: integer
      description: "PR number to fix. Auto-detected from current branch if omitted."
    owner:
      type: string
      description: "Repository owner. Auto-detected from git remote if omitted."
    repo:
      type: string
      description: "Repository name. Auto-detected from git remote if omitted."
    max-iterations:
      type: integer
      description: "Maximum fix-push-poll cycles before halting (default: 3)."
    poll-interval:
      type: integer
      description: "Seconds between CI status polls (default: 30)."
    max-wait:
      type: integer
      description: "Maximum seconds to wait for CI checks per cycle (default: 600)."
  required: []
---

# Fix CI Skill

Automates the cycle of detecting CI pipeline failures on a GitHub PR, reproducing errors locally, applying fixes, running all CI gates to confirm the fix, then pushing and polling until the remote pipeline passes. The skill iterates through fix-push-poll cycles up to a configurable limit, halting only when all checks pass or the iteration cap is reached.

## Prerequisites

* The workspace is a Git repository with a remote configured on GitHub.
* The current branch has an open pull request (or `pr-number` is provided).
* The project compiles before starting (`cargo check` passes).
* GitHub MCP tools are available (`mcp_github_pull_request_read`).
* The `.github/copilot-instructions.md` coding standards are accessible.

## Quick Start

Invoke the skill from the current branch:

```text
Fix CI
```

To target a specific PR:

```text
Fix CI pr-number 42
```

The skill runs autonomously through all required steps, halting only when CI passes or the maximum iteration count is reached.

## Parameters Reference

| Parameter        | Required | Type    | Default | Description                                                   |
| ---------------- | -------- | ------- | ------- | ------------------------------------------------------------- |
| `pr-number`      | No       | integer | —       | PR number to fix. Auto-detected from current branch if omitted |
| `owner`          | No       | string  | —       | Repository owner. Auto-detected from git remote if omitted     |
| `repo`           | No       | string  | —       | Repository name. Auto-detected from git remote if omitted      |
| `max-iterations` | No       | integer | 3       | Maximum fix-push-poll cycles before halting                     |
| `poll-interval`  | No       | integer | 30      | Seconds between CI status polls                                |
| `max-wait`       | No       | integer | 600     | Maximum seconds to wait for CI checks per cycle                |

## Required Steps

### Step 1: Identify Target PR

Determine the current branch and locate the associated pull request.

1. Run `git branch --show-current` to get the active branch name.
2. Run `git remote get-url origin` to extract the repository owner and name from the remote URL. Parse the `owner/repo` segment from the URL (handles both HTTPS and SSH formats). Use these values when `owner` and `repo` are not provided as input.
3. If `pr-number` is provided, use it directly. Otherwise, search for an open PR matching the current branch using `mcp_github_search_pull_requests` with a query filtering by head branch and repository.
4. Use `mcp_github_pull_request_read` with method `get` to retrieve the PR details, including the head branch name and head SHA.
5. Report the PR number, branch, and head SHA before proceeding.

### Step 2: Check CI Status

Poll the PR's check run statuses to determine which checks need attention.

1. Use `mcp_github_pull_request_read` with method `get_status` to retrieve all check run statuses for the PR.
2. If all checks have passed, report success and stop — no fixes are needed.
3. If any checks are still *pending*, wait for the configured `poll-interval` (default 30 seconds) and re-poll. Continue polling until all checks have completed or `max-wait` is exceeded.
4. If `max-wait` is exceeded with checks still pending, report the pending checks and halt.
5. When checks have completed, identify which specific checks failed. The CI pipeline runs these checks in order: *fmt*, *clippy*, *test*, *audit*. Note that CI installs `cargo-audit` before running it.
6. Also use `mcp_github_pull_request_read` with method `get_comments` to check for CI bot failure summaries that may provide additional diagnostic context.
7. Report the list of failed checks before proceeding to local reproduction.

### Step 3: Reproduce Failures Locally

Run the failing CI checks locally in CI pipeline order to capture detailed error output.

* Run each check as a separate terminal command (one command per invocation — project rule).
* If a command produces output that may exceed the terminal buffer, redirect to a file using `Out-File` (e.g., `cargo test --all-targets 2>&1 | Out-File target\ci-fix-test.txt`).
* Use workspace-relative paths for any output files (e.g., `target\ci-fix-results.txt`).

The CI checks in pipeline order:

1. `cargo fmt --all -- --check` — format verification.
2. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — lint compliance.
3. `cargo test --all-targets` — test execution.
4. `cargo audit` — dependency vulnerability scan. If `cargo-audit` is not installed locally, install it first with `cargo install cargo-audit --locked`.

Run only the checks that failed remotely (and any earlier checks in the pipeline that gate them). Capture and parse the error output from each failing command to identify specific errors, file locations, and error codes.

### Step 4: Diagnose and Fix

Analyze each failing check's error output, apply targeted fixes, and verify the fix resolves the specific failure.

For each failing check, working in CI pipeline order:

1. Parse the error output to identify root causes — specific file paths, line numbers, error codes, and error messages.
2. Read the affected source files to understand the context around each error.
3. Apply the minimal fix that resolves the error while following the project's coding standards from `.github/copilot-instructions.md`.
4. After applying fixes for a specific check, re-run that check locally to verify the fix.
5. If the re-run reveals new failures introduced by the fix, diagnose and fix those as well.
6. Continue iterating on the specific check until it passes locally.
7. Move to the next failing check in pipeline order and repeat.

Common fix patterns by check type:

* *fmt*: Run `cargo fmt --all` to auto-fix formatting, then verify with `cargo fmt --all -- --check`.
* *clippy*: Address each warning individually — common issues include missing documentation, unused imports, needless borrows, and type complexity.
* *test*: Investigate test assertion failures, compilation errors in test code, and missing test fixtures. Fix the implementation rather than weakening the test, unless the test itself contains a bug.
* *audit*: Review the advisory details from `cargo audit` output. Update the affected dependency version in `Cargo.toml`, or add an advisory ignore if the vulnerability does not apply to the project's usage.

### Step 5: Local CI Gate

This is a **hard gate**. All checks pass locally before proceeding to push. Run all CI checks in pipeline order regardless of which ones originally failed, since fixes may have introduced regressions in previously passing checks.

1. Run `cargo fmt --all -- --check`. If violations are found, run `cargo fmt --all` to auto-fix, then re-run the check to confirm it passes.
2. Run `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`. Fix any warnings or errors, then re-run until the command exits cleanly.
3. Run `cargo test --all-targets`. Fix any failures, then re-run until all tests pass.
4. Run `cargo audit`. If advisories are found, update affected dependencies or add ignores, then re-run until clean.
5. If fixes applied in steps 2–4 cause an earlier check to fail, restart from step 1 and repeat the full cycle.
6. All four checks exit 0 before proceeding.
7. Report results: fmt exit code, clippy exit code, test counts and pass rate, audit exit code.

### Step 6: Stage, Commit, and Push

Stage all changes, compose a descriptive commit message, and push to the remote.

1. Run `git add -A` to stage all modified, created, and deleted files.
2. Compose a commit message following *Conventional Commits* format:
   * Subject: `fix(ci): resolve {check-names} failures`
   * Body: itemized list of fixes applied with brief descriptions of each change
   * Footer: `Refs: #{pr-number}`
3. Run `git commit` with the composed message.
4. Run `git push` to push the commit to the remote branch.
5. Report the commit hash before proceeding to remote polling.

### Step 7: Poll Remote CI

After pushing, poll the PR's check statuses until all checks complete, then decide whether to iterate or finish.

1. Wait for `poll-interval` seconds (default 30) after pushing to allow CI to start.
2. Use `mcp_github_pull_request_read` with method `get_status` to retrieve updated check run statuses.
3. If checks are still *pending*, wait for `poll-interval` seconds and re-poll. Continue until all checks complete or `max-wait` is exceeded.
4. If all checks pass, proceed to Step 8 with a success status.
5. If any checks fail, increment the iteration counter.
   * If the counter is below `max-iterations` (default 3), loop back to Step 3 to reproduce the new failures locally and begin another fix cycle.
   * If the counter has reached `max-iterations`, proceed to Step 8 with a failure status and the accumulated findings.

### Step 8: Completion Report

Summarize the outcome of the fix cycle.

Report the following:

* **Final status**: all checks passed, or maximum iterations reached with remaining failures.
* **Iterations completed**: how many fix-push-poll cycles ran.
* **Commits made**: list of commit hashes produced during the fix cycle.
* **Fixes applied**: summary of all changes made across all iterations.
* If checks are still failing after reaching `max-iterations`:
  * The specific checks that remain broken.
  * The latest error output from the failing checks.
  * Recommendations for manual review, including affected files and error patterns.

## Troubleshooting

### PR not found for current branch

Verify the current branch has been pushed to the remote and an open PR exists. If the branch was recently pushed, the PR may need to be created first. Provide `pr-number` explicitly to bypass auto-detection.

### CI checks stay pending beyond max-wait

The CI runner may be queued or slow. Increase `max-wait` and re-invoke the skill. Alternatively, check the GitHub Actions tab for the repository to see if runners are available.

### Fixes pass locally but fail in CI

Verify the local Rust toolchain matches CI. The CI pipeline uses `dtolnay/rust-toolchain@stable` — run `rustup show` locally to confirm the active toolchain. Check that `rust-toolchain.toml` (if present) matches the CI configuration in `.github/workflows/ci.yml`.

### Audit failures

The CI pipeline runs `cargo audit` for known vulnerabilities. This check cannot be fixed by code changes alone — it requires dependency updates. Run `cargo audit` locally to see the advisory details, then update the affected dependency in `Cargo.toml` or add an exemption if the advisory does not apply.

### max-iterations reached without resolution

Some failures may require architectural changes or upstream dependency fixes that fall outside the scope of automated repair. Review the accumulated error output in the completion report and consider manual intervention.

### Terminal output truncated

When a CI check produces extensive output, redirect to a file:

```text
cargo test --all-targets 2>&1 | Out-File target\ci-fix-test.txt
```

Then read the output file to review the full error details.

---

Proceed with the user's request following the Required Steps.
