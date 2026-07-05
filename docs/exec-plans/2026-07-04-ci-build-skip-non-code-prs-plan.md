---
title: "Skip the CI Rust build on doc/backlog-only PRs (paths-ignore) — Plan"
type: plan
date: 2026-07-04
slug: ci-build-skip-non-code-prs
status: reviewed
stash_ref: FC881353
umbrella_feature: 071-F
shipment: 071-S
review_artifact: 071.001-R
source_tasks:
  - 071.001-T
related_decisions:
  - docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md
scope: .github/workflows/ci.yml
---

## Decision Summary

Stash `FC881353` observed that every PR this session — including pure
backlog/docs/planning closure PRs (067-S..070-S closures, spikes, unblocks) —
triggered a full ~3-3.5 min Rust build on `ubuntu-latest`, burning Actions
minutes on non-code changes. The grounding spike
(`docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md`) resolved the
crux: **`build` is NOT a required status check.** The `main` ruleset
`PR-Required` (id 12812291) enforces `deletion`, `non_fast_forward`,
`pull_request` (1 approval + code-owner + last-push approval + thread
resolution), `copilot_code_review`, and `update` — **no `required_status_checks`
rule**. The observed `BLOCKED`/`REVIEW_REQUIRED` is review-driven; merges are
admin-merged. PR #200 (a backlog-only PR) ran a 3m14s build for nothing.

Because no status check is required, **a workflow-level `paths-ignore` is the
correct, minimal, and safe mechanism**: skipping the CI run on doc/backlog-only
PRs cannot leave a required check pending and cannot block merges. Full clippy +
test coverage is preserved verbatim for any PR that touches code, because
`paths-ignore` only skips a run when **every** changed file matches an ignored
path — a single code file re-arms the whole workflow.

## Grounded current state (`.github/workflows/ci.yml` @ a3c2c81)

```yaml
on:
  push:
    branches: [ main ]
  pull_request:

permissions:
  contents: read

jobs:
  build:                 # <-- the check name observed in statusCheckRollup is "build"
    runs-on: ubuntu-latest
    steps:
      - Checkout (persist-credentials: false)
      - Install toolchain (clippy, rustfmt)     # dtolnay/rust-toolchain @sha
      - Cache cargo                              # Swatinem/rust-cache @sha
      - fmt:    cargo fmt --all -- --check
      - clippy: cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic
      - test:   cargo test  --no-default-features --features cozo-backend,embeddings --all-targets
      - audit:  cargo install cargo-audit --locked && cargo audit   # continue-on-error: true
```

- Single job `build`; single workflow `CI`.
- Triggers: `push` to `main` and all `pull_request` events. Neither has any
  `paths`/`paths-ignore` today, so *every* push/PR runs the full build.
- `release.yml` triggers only on `v*` tags → **unaffected by PR/push path
  filtering; out of scope.**

## Chosen mechanism (what Ship implements)

Add a `paths-ignore` list to **both** the `push` and `pull_request` triggers of
`ci.yml`, enumerating doc/backlog-only (non-code) paths. Do **not** change any
step, feature flag, or the job name. Do **not** touch `release.yml`.

Target shape:

```yaml
on:
  push:
    branches:
      - main
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '**/*.md'
      - '.autoharness/**'
  pull_request:
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '**/*.md'
      - '.autoharness/**'
```

### Path-set rationale (deliberately tight — bias to over-run, never under-run)

| Pattern | Why ignore | What it does NOT catch (still runs CI) |
|---|---|---|
| `.backlogit/**` | Backlog markdown/JSONL; queue↔archive moves. Source of the 067-S..070-S closure-PR waste. | — |
| `docs/**` | decisions / exec-plans / memory / guides. All spike & planning PRs. | — |
| `**/*.md` | README, CHANGELOG, crate READMEs, `.github/instructions/*.md` anywhere. | — |
| `.autoharness/**` | Harness registry/staging state (non-code), touched by staging PRs. | — |

**Intentionally NOT ignored (so CI still runs):** `**/*.rs`, `Cargo.toml`,
`Cargo.lock`, `*.toml` (incl. `rust-toolchain.toml`, `release.toml`),
`.github/workflows/**` (validate CI config changes), `scripts/**`,
`examples/**`, `crates/**` source, `src/**`. Any of these in a PR re-arms the
full build. This is the safe default: a mixed doc+code PR always runs.

### paths-ignore semantics we are relying on (documented, not assumed)

- A workflow is skipped **only when *all* files in the push/PR match a
  `paths-ignore` pattern**. One non-matching (code) file runs the whole
  workflow. → Coverage cannot be silently weakened on code PRs.
- `paths-ignore` and `paths` are mutually exclusive on the same event; we use
  only `paths-ignore`.
- On a skipped run, GitHub creates **no** check run for `CI / build`. Because
  `build` is **not required** (spike finding), the PR is not blocked — merges
  are gated by review only.

## Rejected alternative — PR-title `if:` conditional

The stash floated `if: !startsWith(github.event.pull_request.title, 'chore:')`
etc. **Rejected as the mechanism:**

1. **Fragile / bypassable** — depends on conventional-commit discipline; a
   mistitled or squash-renamed PR silently runs (or skips) the wrong way.
2. **No safer on required checks** — a job skipped via `if:` *also* reports no
   check; if `build` were ever required it would hang identically to
   `paths-ignore`, but with worse determinism.
3. **`paths-ignore` strictly dominates** — it decides on actual changed-file
   content, which is exactly the signal we want ("are there code changes?").

`paths-ignore` alone is chosen. No title guard is added.

## Future-coupling guardrail (carry into the ci.yml change as a comment)

This design is **coupled to the ruleset having no required status check.** If
`build` (or any CI job) is ever promoted to a **required status check** in the
`PR-Required` ruleset, `paths-ignore` will make doc-only PRs hang on *"Expected
— Waiting for status to be reported"* and block non-admin merges. The task adds
a short comment in `ci.yml` pointing at the spike and recording the contingency
below. **Not implemented now.**

Contingency pattern (for that future only — companion always-passing job with
the identical required check name):

```yaml
# Only if `build` becomes a REQUIRED status check:
  build:
    if: <changed files include code>            # real build
    ...
  build-skip:
    if: <changed files are doc/backlog-only>    # always-green stand-in
    runs-on: ubuntu-latest
    steps:
      - run: echo "No code changes — skipping build (required-check stand-in)."
    # NOTE: must publish the SAME check name the ruleset requires.
```

## Task Decomposition & Width

| Task | Concern | Width | Est. |
|---|---|---|---|
| 071.001-T | Add `paths-ignore` (doc/backlog-only) to `ci.yml` `push` + `pull_request` triggers; add the coupling comment; verify skip-vs-run behavior. | Single file — `.github/workflows/ci.yml` (CI/workflow concern only) | < 2h |

One task. This is a single-file, single-concern CI change — **no** width mixing
with Rust/schema/CLI work. Over-decomposition would be wrong; verification is
part of this task's DoD.

## Step 5.5 Scope Guard

**IN scope**
- `.github/workflows/ci.yml`: add `paths-ignore` to the `push` and
  `pull_request` triggers with the four doc/backlog-only patterns above.
- A brief in-file comment recording the required-check coupling + spike pointer.
- Required-check safety: verified safe because `build` is not required (no
  ruleset change is made or needed).

**OUT of scope (requires a new plan / new items)**
- Any broader CI redesign (matrix, split jobs, caching strategy, moving fmt/
  clippy/test/audit around).
- `release.yml` changes (tag-triggered; unaffected).
- Weakening or altering code-PR coverage (fmt/clippy/test/audit stay identical).
- Any change to the `PR-Required` ruleset (e.g. adding/removing required status
  checks). If `build` is later made required, the companion-job contingency is a
  **new** work item.
- PR-title-based conditionals (rejected above).

## Verification Plan (how Ship proves it — part of 071.001-T DoD)

1. **Doc-only PR** — branch touching only e.g. `docs/tmp-verify.md` (or a
   `.backlogit/**` change): confirm **no** `CI / build` run is created (Actions
   tab shows no run for the head SHA; PR "Checks" shows no build check), and the
   PR is not blocked by a pending build check (only the review gate remains).
2. **Code PR** — branch touching a `.rs` file: confirm CI runs and executes the
   **full unchanged** sequence fmt → clippy → test → audit with the same
   `--no-default-features --features cozo-backend,embeddings --all-targets`
   flags.
3. **Mixed PR** — doc + code in the same PR: confirm CI **runs** (a single code
   file defeats `paths-ignore`), proving coverage cannot be dropped on code.
4. **Required-check invariant** — re-confirm
   `gh api repos/softwaresalt/agent-engram/rules/branches/main` still contains
   **no** `required_status_checks` rule, i.e. the skip does not rely on and does
   not create a pending required check.
5. **YAML validity** — `ci.yml` parses; action SHAs unchanged (ci-security
   pinning intact); `permissions: contents: read` unchanged.

## Risk & Blast Radius

**Low, but touches CI gating — handle with the verification plan.** Single
workflow file; additive trigger filter; no step/flag/job-name change; no action
re-pin; `release.yml` untouched. The one real hazard (path filter hiding a code
change from CI) is structurally prevented by `paths-ignore` all-match semantics
and explicitly probed by verification steps 2–3. The one future hazard
(`build` becoming required) is documented with an in-file comment + contingency.
No `plan-harden` pass required: not multi-family, not a schema/CLI-distribution
change; it is a bounded single-file CI trigger edit.

## Provenance

- Stash: `FC881353` (kind task, priority medium).
- Spike / decision:
  `docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md`
  (**`build` is not a required status check**).
- Grounded file: `.github/workflows/ci.yml` @ `a3c2c81`.
- Conventions: `.github/instructions/workflows.instructions.md`,
  `.github/instructions/ci-security.instructions.md` (action SHA pinning +
  least-privilege permissions preserved).
- Review artifact: `071.001-R`. Shipment: `071-S` (queued).
