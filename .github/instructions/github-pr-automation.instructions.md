---
description: "GitHub-specific PR automation: Copilot Review polling, comment resolution, and CI check monitoring"
applyTo: '**'
---

# GitHub PR Automation Instructions

These instructions define the GitHub-specific automation protocols for
pull request review and CI check monitoring. They extend the general
`pull-request.instructions.md` and `ci-security.instructions.md` with
concrete GitHub API operations, polling cadences, and comment lifecycle
management.

## Scope

These instructions apply when the target workspace is hosted on GitHub
and agents interact with pull requests via GitHub MCP tools or the `gh`
CLI. Agents MUST follow these protocols for all PR review polling,
comment addressing, and CI status monitoring.

---

## Part 1: Copilot Review Automation

### 1.1 Request Copilot Review

After creating or updating a PR, request a Copilot review:

```text
Tool: mcp_github_request_copilot_review
  owner: softwaresalt
  repo:  agent-engram
  pullNumber: <pr_number>
```

If the MCP tool is unavailable, fall back to the CLI:

```bash
gh pr edit <pr_number> --add-reviewer "copilot"
```

### 1.2 Poll for Review Completion

Copilot review typically completes within 2–5 minutes. Use a
back-off polling strategy:

| Attempt | Wait before poll | Cumulative wait |
|---------|-----------------|-----------------|
| 1       | 2 minutes       | 2 min           |
| 2       | 2 minutes       | 4 min           |
| 3       | 3 minutes       | 7 min           |
| 4       | 3 minutes       | 10 min          |
| 5       | 5 minutes       | 15 min          |

**Poll mechanism** — use the MCP tool to read review comments:

```text
Tool: mcp_github_pull_request_read
  owner: softwaresalt
  repo:  agent-engram
  pullNumber: <pr_number>
```

Inspect the returned reviews and review comments. Copilot review
comments are identified by the author login `copilot-pull-request-reviewer[bot]`
or similar bot author association.

**Completion signal**: Treat any Copilot-authored review with `state != PENDING`
as complete, including `COMMENTED`, `CHANGES_REQUESTED`, and `APPROVED`.
Review comments attached to a non-`PENDING` review also count as completion.

**Merge-gate invariant — `commit_id == current HEAD` (load-bearing)**: A review
merely *existing* is sufficient to consider a *review cycle* complete, but it is
**NOT** sufficient to **merge**. Copilot review is asynchronous and is
re-triggered by every push; each push re-adds Copilot to `requested_reviewers`
and the fresh review takes ~4–5 minutes to post. In the window after the final
push but before that review lands, the PR transiently shows **0 unresolved
threads** and **`mergeable_state == clean`** even though the review for the
current HEAD has not posted yet. Before merging you MUST therefore confirm a
Copilot review exists whose `commit_id` equals the current HEAD sha:

```bash
# HEAD sha the merge would land
gh api repos/<owner>/<repo>/pulls/<n> --jq '.head.sha'
# Copilot review commit_ids (must include the HEAD sha above). Use a PREFIX
# match: the REST `/reviews` endpoint returns the login
# `copilot-pull-request-reviewer[bot]`, while the GraphQL/`gh pr view`
# surface normalizes it to `copilot-pull-request-reviewer` (no `[bot]`
# suffix). An exact `==` match against either form silently drops the review
# and the load-bearing gate can never be satisfied.
#
# `--paginate` is REQUIRED: `gh api` returns only the first 30 reviews by
# default, and the list is ordered oldest-first, so the newest review (the one
# at HEAD) is LAST. After a multi-cycle review loop (>30 total reviews) the HEAD
# review spills onto page 2+ and an un-paginated call silently reports a stale
# commit_id, falsely failing the gate. See
# `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md`.
gh api --paginate repos/<owner>/<repo>/pulls/<n>/reviews \
  --jq '.[]|select(.user.login|startswith("copilot-pull-request-reviewer"))|{state,commit_id}'
```

This binds the merge to a review of the exact commit being merged. Re-check the
full 4-point merge gate — (1) a Copilot review with `commit_id == HEAD`, (2)
Copilot removed from `requested_reviewers`, (3) 0 unresolved review threads, (4)
`mergeable_state == clean` — after **every** push, including a one-line review-nit
fix, because any push resets the review clock. Rationale and post-mortem:
`docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`
(PRs #239/#240 were merged in this exact gap).

**Timeout**: If no Copilot review for the current HEAD appears after 15
minutes (5 poll attempts), do **NOT** merge. The current-HEAD invariant above
is non-negotiable; this timeout must not become a bypass that recreates the
exact post-push race it exists to prevent. Halt merge automation, log a
warning, and escalate to the operator (note in the PR that automated review
did not land for the current HEAD). A human may then decide whether to merge
manually — the timeout suspends automation, it does not authorize an
unreviewed-HEAD merge.

### 1.3 Categorize Review Comments

For each Copilot review comment, classify it:

| Category | Criteria | Action |
|----------|----------|--------|
| **Valid** | Comment identifies a real issue confirmed by local analysis | Fix the code |
| **Partial** | Comment is partially correct or overly broad | Fix the valid part, reply with explanation |
| **Invalid** | Comment is a false positive or stylistic disagreement | Decline with rationale |
| **Informational** | Comment is a suggestion, not a defect | Acknowledge, apply if low-risk |

### 1.4 Address and Fix Comments

For each comment requiring a fix:

1. **Understand context**: Read the file and surrounding code referenced
   by the comment's `path` and `line`/`start_line` fields.
2. **Apply the fix**: Make the minimal targeted change that resolves the
   issue without introducing scope creep.
3. **Verify locally**: Run the relevant quality gate
   (`cargo fmt --all -- --check`, `cargo clippy -- -D warnings -D clippy::pedantic`, `cargo dev-test`)
   to confirm the fix doesn't break anything.
4. **Commit**: Use a `fix:` conventional commit referencing the comment
   (e.g., `fix: address copilot review — null check on user input`).

### 1.5 Reply to Addressed Comments

After fixing each comment, reply to the review thread:

```text
Tool: mcp_github_add_reply_to_pull_request_comment
  owner: softwaresalt
  repo:  agent-engram
  pullNumber: <pr_number>
  commentId: <comment_id>
  body: "Fixed in <commit_sha>. <brief description of the fix>"
```

For declined comments, reply with the rationale:

```text
body: "Declined — <rationale>. The current implementation <explanation>."
```

For partial comments:

```text
body: "Partially addressed in <commit_sha>. Applied: <what was fixed>.
Not applied: <what was declined and why>."
```

### 1.6 Resolve Review Threads

After replying to a comment, resolve the review thread using the
GitHub GraphQL API. There is no REST endpoint or MCP tool for thread
resolution — use `gh api graphql`:

```bash
gh api graphql -f query='
  mutation ResolveThread($threadId: ID!) {
    resolveReviewThread(input: { threadId: $threadId }) {
      thread { isResolved }
    }
  }
' -f threadId="<thread_node_id>"
```

**Obtaining the thread node ID**: When reading PR review comments via
MCP or REST, each review comment includes a `node_id` field (the
GraphQL global ID). For threaded review comments, query the thread:

```bash
gh api graphql -f query='
  query GetThreads($owner: String!, $repo: String!, $pr: Int!) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $pr) {
        reviewThreads(first: 100) {
          nodes {
            id
            isResolved
            comments(first: 1) {
              nodes { body path line }
            }
          }
        }
      }
    }
  }
' -f owner="softwaresalt" -f repo="agent-engram" -F pr=<pr_number>
```

Match threads to addressed comments by `path` and `line`, then resolve
each thread using its `id`.

**Rules**:

* Only resolve threads for comments that have been fixed or explicitly
  declined with a rationale reply.
* Never resolve threads without first posting a reply explaining the
  resolution.
* Never resolve threads authored by human reviewers — only bot-authored
  threads (Copilot, linters, etc.) may be auto-resolved.

### 1.7 Push Fixes and Re-request Review

After all addressable comments are handled:

1. Push the fix commits to the branch.
2. Re-request Copilot review if new code was pushed:

   ```text
   Tool: mcp_github_request_copilot_review
     owner: softwaresalt
     repo:  agent-engram
     pullNumber: <pr_number>
   ```

3. Poll again per Section 1.2 to verify the new review is clean.

### 1.8 Stop Conditions for Review Cycles

| Counter | Limit | Action |
|---------|-------|--------|
| Review-fix-push cycles | 3 | Accept remaining comments as backlog follow-ups |
| Same comment re-raised after fix | 2 | Escalate to operator — likely a fundamental disagreement |

### 1.9 Pre-Merge Review Readiness Verification (Defense in Depth)

This gate is a **NON-NEGOTIABLE** pre-merge verification that runs
independently of shadow review. Even if the local review gate in the ship
workflow reported success earlier, this step re-checks from scratch that the PR
still reflects a current-HEAD local review result before any merge is presented
as ready or executed.

This gate applies to **all pull requests** created or merged by the Ship agent:
feature PRs, chore PRs, and post-merge closure PRs. There is no exception for
"small" or "hygiene" PRs. Every merge requires a fresh local review readiness
record covering the current HEAD. Shadow review is optional and advisory by default.

In dark mode, this gate is still local-review-first: unresolved local P0/P1
findings block merge, `READY_WITH_FOLLOWUPS` is allowed only when follow-up item
IDs or explicit residual-risk notes are recorded, and advisory shadow-review
comments are surfaced as follow-ups unless elevated by policy or operator.

#### 1.9.1 Readiness Query

Run a single GraphQL query to fetch PR head SHA, PR body, review decision,
shadow-review requests/reviews, and review threads:

```bash
gh api graphql -f query='
  query PRReviewReadiness($owner: String!, $repo: String!, $pr: Int!, $threadCursor: String) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $pr) {
        headRefOid
        body
        reviewDecision
        reviewRequests(first: 100) {
          nodes {
            requestedReviewer {
              __typename
              ... on Bot  { login }
              ... on User { login }
              ... on Team { name  }
            }
          }
        }
        reviews(last: 50) {
          nodes {
            author { login }
            state
            submittedAt
            commit { oid }
          }
        }
        reviewThreads(first: 100, after: $threadCursor) {
          nodes {
            id
            isResolved
            comments(first: 1) {
              nodes { author { login } body path line }
            }
          }
          pageInfo { hasNextPage endCursor }
        }
      }
    }
  }
' -f owner="softwaresalt" -f repo="agent-engram" -F pr=<pr_number>
```

Omit `threadCursor` entirely on the first request — passing an empty string
supplies an invalid Relay cursor and the query can fail before pagination
starts. Declare it as `$threadCursor: String` (nullable, no default) so the
initial call resolves it to `null`.

If `pageInfo.hasNextPage` is true, re-run the query with
`-f threadCursor="{endCursor}"` and merge the `reviewThreads.nodes`
results. Repeat until `hasNextPage` is false. **Do not skip
pagination** — a hard gate that misses operator-visible review data is unsafe. If
pagination cannot complete (API error, rate limit), fail closed and
halt rather than declaring readiness.

#### 1.9.2 Local Readiness Record

The PR description or other operator-visible readiness summary MUST contain a
local review block for the current HEAD. Use this format or an equivalent
machine-readable variant:

```markdown
## Local Review Readiness

- Reviewed HEAD: `<sha>`
- Outcome: `READY` | `READY_WITH_FOLLOWUPS` | `BLOCKED`
- Blocking findings: `P0=0, P1=0`
- Full local build: `<command and successful result>` | `not applicable — <docs/backlog-only rationale>`
- Follow-ups: `none` | `<item ids or residual-risk notes>`
- Shadow review: `not requested` | `requested` | `clean` | `comments pending`
```

If the workspace uses a different readiness artifact, the PR body must still
contain the reviewed HEAD SHA, the outcome, successful full local build evidence
(or explicit non-applicability), and the follow-up handling summary so the merge
gate can confirm the current PR state without relying on hidden local state.

#### 1.9.3 Advisory Bot Identity

The Copilot review bot appears under different login strings depending
on the API surface:

| API context | Login string |
|-------------|-------------|
| GraphQL `Bot.login` (reviews, reviewRequests) | `copilot-pull-request-reviewer` (no `[bot]` suffix) |
| REST `review.user.login` | `copilot-pull-request-reviewer[bot]` |
| REST timeline `requested_reviewer.login` | `Copilot` (display form) |

When matching in GraphQL responses, use `copilot-pull-request-reviewer`
(without `[bot]`). When matching in REST responses, use
`copilot-pull-request-reviewer[bot]`. For review thread comments
returned via GraphQL, the `author.login` field uses the no-suffix form.

#### 1.9.4 Gate Checks

Evaluate five checks in order. All five must pass for merge readiness.

**Check 1 — Local review coverage (record covers current HEAD)**:

1. Record `headRefOid` from the query response.
2. Parse the PR `body` for the `## Local Review Readiness` block (or the
   equivalent repository-approved marker).
3. Extract `Reviewed HEAD`.
4. If the block is missing or `Reviewed HEAD` does not match `headRefOid`, the
   local review is stale or absent. Halt and require the caller to rerun local review
   for the current HEAD.

**Check 2 — Local readiness outcome (no unresolved blocking findings)**:

1. Extract `Outcome` and `Blocking findings` from the local readiness block.
2. If `Outcome` is `BLOCKED`, **GATE FAILS**. Halt and report the blocking local review.
3. If `Blocking findings` reports any unresolved P0 or P1 findings, **GATE FAILS** even
   if the outcome string is malformed or overly optimistic.
4. If `Outcome` is `READY` or `READY_WITH_FOLLOWUPS` and blocking findings are clear,
   proceed to Check 3.

**Check 3 — Follow-up handling is explicit**:

1. If `Outcome` is `READY`, the `Follow-ups` field may be `none`.
2. If `Outcome` is `READY_WITH_FOLLOWUPS`, the `Follow-ups` field must list
   follow-up item IDs, queued backlog work, or explicit residual-risk notes.
3. If the field is missing or empty for `READY_WITH_FOLLOWUPS`, **GATE FAILS**.
4. Otherwise, proceed to Check 4.

**Check 4 — Full local build evidence for code-changing PRs**:

1. If the PR adds, removes, or changes source code, the readiness block must list
   the full local build command and a successful result.
2. If the PR is documentation-only or backlog-only, the readiness block may state
   `Full local build: not applicable` with a short rationale.
3. If build applicability is ambiguous, required evidence is missing, or the
   recorded full local build result failed, **GATE FAILS**.
4. Otherwise, proceed to Check 5.

**Check 5 — Copilot-review completion & thread resolution (P-018, fail-closed)**:

1. Determine `copilot_review.enforcement` from `.autoharness/workspace-profile.yaml`
   (`auto` | `required` | `disabled`, default `auto`) and `copilot_review.max_wait_seconds`
   (integer ≥ 0, default `0`).
2. Run the deterministic gate:
   `autoharness gate copilot-review <pr_number> --repo softwaresalt/agent-engram --enforcement <mode> [--max-wait <max_wait_seconds>]`.
3. Interpret the verdict / exit code:
   * `SATISFIED` or `NOT_APPLICABLE` (exit 0) — Copilot review is complete for the
     current HEAD with no open Copilot threads, or Copilot is not in play. Proceed.
   * `WAITING_FOR_REVIEW`, `UNRESOLVED_THREADS`, `REVIEW_TIMEOUT`,
     `DETECTION_AMBIGUOUS`, or `VERIFY_FAILED` (non-zero) — **GATE FAILS.** Halt.
     `--admin` does not bypass this. Wait for review completion, resolve every
     Copilot thread, then re-run. `REVIEW_TIMEOUT` still blocks; only an explicit,
     audited `autoharness gate copilot-review ... --force` (logged under
     `.autoharness/gates/`) may override, and only with operator authority.
4. Only when the gate returns a PASS verdict (or an audited `--force` is recorded)
   does the readiness gate reach **GATE PASSES**. The PR is ready for merge presentation.

**Human review threads (Check 5 precedence)**: Human review threads are surfaced
in the merge-readiness summary but do not block this local-readiness gate by
default. **Copilot-authored threads are not advisory here**: an existing
Copilot review is itself an engagement signal, so Check 5 (P-018) takes precedence
and every unresolved Copilot-authored thread BLOCKS the merge — the legacy
"advisory shadow-review does not block" rule never applies once Copilot is engaged.
However, if the repository has branch
protection rules requiring conversation resolution, approved reviews,
or if a human reviewer submitted a `CHANGES_REQUESTED` review, those
constraints may independently block the merge at the GitHub level. The
`reviewDecision` field from the query reflects the overall PR review
decision (`APPROVED`, `CHANGES_REQUESTED`, `REVIEW_REQUIRED`, or null)
and should be reported in the merge-readiness summary.

#### 1.9.5 Terminal States

| Condition | Action |
|-----------|--------|
| Local review block missing from PR body | **Halt.** Report that readiness evidence is absent. |
| Local review block references the wrong HEAD SHA | **Halt.** Report stale review and current HEAD SHA to operator. |
| Local readiness outcome is `BLOCKED` or blocking findings remain | **Halt.** List blocking findings. Do not proceed to merge. |
| `READY_WITH_FOLLOWUPS` omits follow-up handling | **Halt.** Report missing follow-up IDs or residual-risk notes. |
| Code-changing PR omits successful full local build evidence | **Halt.** Run the full local build successfully or explain non-applicability only for documentation-only/backlog-only work. |
| Copilot review enabled but incomplete for HEAD, or Copilot threads unresolved (`autoharness gate copilot-review` returns non-zero) | **Halt (P-018).** Wait for Copilot review completion and resolve every Copilot thread. `--admin` does not bypass. `REVIEW_TIMEOUT` blocks; only an audited `--force` overrides. |
| Shadow review unavailable or still pending, Copilot not engaged (gate returns `NOT_APPLICABLE`) | **Warning.** Note in PR summary. Shadow review remains advisory when Copilot is not in play. |
| All 5 checks pass | **Ready.** Present PR for merge approval. |

Shadow-review timeout does not fail this gate when Copilot is not engaged; the
required dependency is local review coverage for the current HEAD. If the operator
wants shadow review to become merge-blocking for a specific PR, that escalation
must be explicit. When Copilot review IS engaged, the §1.9.4 Check 5 copilot-review
gate is an additional fail-closed dependency (P-018).

#### 1.9.6 Dark-Mode Merge Authorization and Admin Fallback

When `DARK_MODE_ACTIVE` is present under P-017, the activation record may satisfy
the P-014 operator approval signal only when all of these are true:

1. The PR is inside the recorded dark-mode `scope`.
2. `merge_approval_pre_authorized` is `true`.
3. The §1.9 local readiness gate passed for the current `headRefOid`.
4. Required CI/checks are green or explicitly marked non-applicable.
5. P-009 merge-commit-only and P-016 worktree topology checks passed.
6. The §1.9.4 Check 5 copilot-review gate returned a PASS verdict for the current
   `headRefOid` (or an audited `--force` override is recorded). A
   `COPILOT_REVIEW_BLOCK` is never satisfied by the activation record.

If any condition is false or ambiguous, fail closed and wait for an explicit
operator approval signal.

Before any admin fallback, attempt the normal merge path first and classify the
result:

| State | Meaning | Dark-mode action |
|---|---|---|
| `NORMAL_MERGE_READY` | Normal merge can proceed with merge commit strategy | Merge normally; record `DARK_MODE_MERGE_AUTHORIZED` when approval came from the activation record |
| `REVIEW_REQUIRED_BLOCK` | Branch protection rejected merge for required review approval | Admin fallback may be attempted only if `admin_fallback_pre_authorized` is `true` |
| `CONVERSATION_RESOLUTION_BLOCK` | Branch protection requires unresolved conversations to be resolved | Admin fallback may be attempted only if explicitly covered by `admin_fallback_pre_authorized`; otherwise halt |
| `CHECKS_BLOCK` | Required checks are failed, pending, missing, or not explicitly non-applicable | Halt; admin fallback is forbidden |
| `MERGE_STRATEGY_BLOCK` | Merge commit strategy is unavailable or squash/rebase is selected | Halt under P-009; admin fallback is forbidden |
| `MISSING_ADMIN_RIGHTS` | Admin fallback was authorized but credentials lack bypass rights | Halt with an operator-visible reason |
| `UNKNOWN_MERGE_BLOCK` | The merge rejection cannot be classified confidently | Halt; do not guess or bypass |
| `COPILOT_REVIEW_BLOCK` | The P-018 copilot-review gate returned a BLOCK verdict (review incomplete for HEAD, unresolved Copilot threads, timeout, or unverifiable) | Halt; admin fallback may **never** bypass this. Resolve via review completion + thread resolution, not `--admin`. |

Admin fallback cannot bypass stale local readiness, unresolved local P0/P1
findings, failed required CI/checks, a P-018 `COPILOT_REVIEW_BLOCK`, P-009, P-016,
secrets-safety concerns, or scope mismatch. Every normal merge attempt and admin
fallback attempt must be recorded in the PR readiness/merge summary with the state,
decision, command/API used, and result.

### 1.10 Post-Merge Closure PR Shadow Review Surveillance

When the Ship agent creates a dedicated post-merge closure branch and PR:

1. Optionally request Copilot shadow review per §1.1 immediately after PR creation.
2. Poll per §1.2 back-off cadence.
3. Apply the full §1.3–§1.7 fix cycle for any comments raised.
4. Run §1.9 local readiness gate before presenting the post-merge closure PR for merge.
5. Obtain explicit operator approval before merging the post-merge closure PR.

Post-merge closure PRs are not exempt from the P-014 gate. The operator must
approve each merge individually — approval for the main PR does not carry over
to the post-merge closure PR.

---

## Part 2: CI Check Monitoring

### 2.1 Wait for CI Checks to Start

After pushing commits or creating a PR, CI checks may take 10–30
seconds to initialize. Wait at least 30 seconds before the first
status poll.

### 2.2 Poll CI Check Status

Use the MCP tool to read check run status:

```text
Tool: mcp_github_pull_request_read
  owner: softwaresalt
  repo:  agent-engram
  pullNumber: <pr_number>
```

Alternatively, use the `gh` CLI for more granular check-run data:

```bash
gh pr checks <pr_number> --watch --fail-fast
```

Or query check runs directly:

```bash
gh api repos/softwaresalt/agent-engram/commits/<head_sha>/check-runs \
  --jq '.check_runs[] | {name, status, conclusion}'
```

### 2.3 Polling Cadence for CI

| Attempt | Wait before poll | Cumulative wait |
|---------|-----------------|-----------------|
| 1       | 30 seconds      | 30 sec          |
| 2       | 1 minute        | 1.5 min         |
| 3       | 2 minutes       | 3.5 min         |
| 4       | 2 minutes       | 5.5 min         |
| 5       | 3 minutes       | 8.5 min         |
| 6       | 3 minutes       | 11.5 min        |
| 7       | 5 minutes       | 16.5 min        |
| 8+      | 5 minutes       | +5 min each     |

**Timeout**: If checks have not completed after 30 minutes, halt
polling and report to the operator. Do not wait indefinitely.

### 2.4 Interpret Check Results

Parse check run results into actionable categories:

| Conclusion | Meaning | Action |
|------------|---------|--------|
| `success` | Check passed | No action needed |
| `failure` | Check failed with actionable errors | Invoke fix-ci protocol |
| `cancelled` | Check was cancelled (often by a newer push) | Re-trigger if needed |
| `timed_out` | Check exceeded its time limit | Investigate resource issues, re-trigger once |
| `action_required` | Check needs manual intervention (e.g., security review) | Report to operator |
| `skipped` | Check was skipped by condition | Verify the skip was expected |
| `neutral` | Informational check | Log and continue |
| `stale` | Check is outdated (superseded by newer commit) | Ignore, newer checks are authoritative |

### 2.5 Extract Failure Details

When a check fails, extract the failure details for diagnosis:

```bash
gh api repos/softwaresalt/agent-engram/check-runs/<check_run_id>/annotations \
  --jq '.[] | {path, start_line, end_line, annotation_level, message}'
```

Check annotations provide file paths, line numbers, and error messages
that map directly to code locations — use these for targeted fixes.

For checks without annotations, retrieve the log output:

```bash
gh run view <run_id> --log-failed
```

### 2.6 Fix-Push-Poll Loop

After diagnosing and fixing CI failures:

1. Run the failing checks locally first (per fix-ci skill protocol).
2. Commit and push the fix.
3. Wait for CI to re-trigger (Section 2.1 timing).
4. Poll for new check results (Section 2.3 cadence).
5. Repeat until all checks pass or circuit breaker triggers.

### 2.7 CI Circuit Breakers

| Counter | Limit | Action |
|---------|-------|--------|
| Fix-push-poll iterations | 5 | Halt, leave PR for manual intervention |
| Same check fails 3 times consecutively | 3 | Halt that check's fix loop, report systematic failure |
| Total CI wait time | 30 minutes per cycle | Halt polling, report timeout |

---

## Part 3: Combined Review + CI Workflow

When both Copilot Review and CI checks are active on the same PR, follow
this sequencing:

1. **Push code** → triggers both CI and Copilot Review.
2. **Poll CI status** (Section 2.2) — CI results usually arrive first.
3. **Poll Copilot Review** (Section 1.2) — review typically takes 2–5 min.
4. **Fix CI failures first** — CI failures are typically more mechanical
   and faster to resolve.
5. **Address review comments** — may overlap with CI fixes. If a review
   comment targets the same code as a CI failure, fix once and reference
   both in the commit message.
6. **Push combined fixes** → re-triggers both CI and review.
7. **Resolve addressed threads** (Section 1.6) — only after fixes are
   pushed and replies posted.
8. **Final verification poll** — confirm both CI green and review clean.

### Interaction with fix-ci Skill

When the pr-lifecycle skill delegates to fix-ci, the fix-ci skill
SHOULD follow the CI polling protocol in Part 2 of this document
rather than ad-hoc polling. The review comment handling in fix-ci
Step 3/Step 6 SHOULD follow Part 1 of this document for GitHub-hosted
repositories.

### Interaction with pr-lifecycle Skill

The pr-lifecycle skill's Step 3 (handle review feedback) SHOULD follow
Part 1 of this document for the complete Copilot Review workflow on
GitHub-hosted repositories. The pr-lifecycle skill's Step 4 (handle CI
failures) SHOULD reference Part 2 for GitHub-specific polling and
failure extraction.

The pr-lifecycle skill's **merge step** MUST enforce the `commit_id == current
HEAD` merge-gate invariant defined in Section 1.2: do not merge until a Copilot
review whose `commit_id` equals the HEAD sha has landed, Copilot has been removed
from `requested_reviewers`, there are 0 unresolved threads, and
`mergeable_state == clean` — re-checked after the final push. A transient
`mergeable_state == clean` immediately after a push is NOT a merge signal. See
`docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`.

---

## Environment Detection

These instructions apply when the repository is hosted on GitHub.
Agents detect this via:

* Git remote URL containing `github.com`
* Presence of GitHub repository metadata or tooling under `.github/`

For GitHub-hosted repositories:

* Part 1 (PR review polling, Copilot Review handling, and comment
  lifecycle management) applies whenever agents interact with pull
  requests via GitHub MCP tools or the `gh` CLI.
* Part 2 (CI polling and check monitoring) applies when the workspace
  CI platform is GitHub Actions. Agents MAY detect that via:
  * Presence of `.github/workflows/` directory
  * `GitHub Actions` resolving to `GitHub Actions`

When the repository is not on GitHub, these instructions do not apply.
Fall back to the generic CI and PR protocols in `ci-security.instructions.md`
and `pull-request.instructions.md`.

Generated by autoharness | Template: github-pr-automation.instructions.md.tmpl
