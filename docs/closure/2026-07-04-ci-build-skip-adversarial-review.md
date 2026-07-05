---
title: "Adversarial multi-model review — CI build-skip (paths-ignore), shipment 071-S"
type: closure
date: 2026-07-04
slug: ci-build-skip-adversarial-review
subject_commit: 2798d0906d0eba98c47da10f4ca1ab7bcc3c13d5
subject_branch: 071-ci-build-skip
scope: .github/workflows/ci.yml
reviewers: 3
review_models:
  - reviewer-a: gemini-3.1-pro-preview (Tier 3)
  - reviewer-b: gpt-5.4 (Tier 2)
  - reviewer-c: claude-opus-4.8 (Tier 1 skeptic)
verdict: APPROVE-WITH-FIXES
gate_blocking: true
---

# Adversarial Review — CI build-skip on non-code PRs (shipment 071-S)

## Verdict: **APPROVE-WITH-FIXES**

The change is **safe on its central risk axis** (the `build` check is *not* a
required status check — independently re-verified — so skipped runs cannot wedge
merges), **but it contains one confirmed under-run defect** that must be fixed
before merge: the blanket `**/*.md` ignore pattern silently skips CI on changes to
two live test-input fixtures (`tests/fixtures/verify/{conformant,malformed}.md`).

- **Gate-blocking finding:** 1 (P1 / MAJOR / **HIGH consensus** — all 3 reviewers).
- **Not blocking but must be acknowledged:** plan-invariant contradiction, unrun DoD.
- **Positively verified safe (HIGH consensus):** required-check invariant, security
  posture, merge/branch-protection behavior, release/deploy decoupling.

**One required change:** narrow the markdown ignore so `tests/**/*.md` still
triggers CI (see Remediation R1). This is a workflow-only edit with negligible
cost to the CI-minutes savings.

---

## Subject under review

Commit `2798d09` ("ci: skip Rust build on doc/backlog-only changes (071-S)"),
branch `071-ci-build-skip`, **not pushed**. Diff = **+24 lines, `ci.yml` only**
(`git show 2798d09 --stat`). Adds an identical `paths-ignore` block to *both* the
`on.push` (branches: main) and `on.pull_request` triggers:

```yaml
paths-ignore:
  - '.backlogit/**'
  - 'docs/**'
  - '**/*.md'
  - '.autoharness/**'
```

Everything else is byte-identical: single `build` job
(checkout → toolchain → cache → fmt → clippy → test → audit); feature set
`--no-default-features --features cozo-backend,embeddings --all-targets`; action
SHA pins unchanged; `permissions: contents: read` unchanged; large in-file comment
documenting all-match semantics + the required-check contingency; **no** PR-title
`if:` guard.

## Method

Primary evidence was gathered authoritatively (git `show`/`grep`/`ls-files`,
`gh api` on rulesets/rules/branch-protection), then three reviewer agents were
dispatched in parallel across **three different model vendors** (Google / OpenAI /
Anthropic) to maximize blind-spot diversity. Each received the same evidence bundle
and ruleset, was told to independently re-verify the crux claims, and returned
structured JSON findings only. Reviewer C (Claude) additionally re-ran the `gh api`
calls and `git grep`; Reviewer B (GPT) additionally ran `cargo test` against the
verify integration target to confirm the fixtures are live.

---

## Consensus findings (HIGH confidence — flagged by ALL 3 reviewers)

### C1 — `**/*.md` under-runs live test fixtures  ·  P1 · MAJOR · HIGH
**File:** `.github/workflows/ci.yml` lines **24** (push) and **30** (pull_request)
— the `- '**/*.md'` entry in both `paths-ignore` blocks.

**Issue.** `**/*.md` matches `tests/fixtures/verify/conformant.md` and
`tests/fixtures/verify/malformed.md`, which are **real on-disk test inputs**, not
docs. They are consumed by `tests/integration/cli_verify_test.rs`:

- `cli_verify_test.rs:67` runs the `verify` CLI against
  `tests/fixtures/verify/conformant.md` and asserts exit code `0`.
- `cli_verify_test.rs:74` runs it against `tests/fixtures/verify/malformed.md` and
  asserts exit code `1`.
- `cli_verify_test.rs:82` re-checks conformant via a backslash path.

These are the **only** `.md` files under `tests/` (`git ls-files "tests/**/*.md"`),
and both are tracked. A PR/push that edits **only** a verify fixture — e.g.
strengthening the malformed case, or changing `conformant.md` such that it no longer
verifies clean — is a `.md`-only change, so `paths-ignore` **skips CI entirely**.
The broken assertion merges with no build/test signal; the subsequent merge-commit
push to `main` is *also* `.md`-only and *also* skipped, so `main` never runs the
suite and sits with a latent test breakage until an unrelated later code PR reruns
the tests and fails with confusing attribution.

This directly contradicts the plan's stated invariant *"deliberately tight — bias
to over-run, never under-run"* and the commit message's claim that non-ignored
files re-arm the workflow. The `**/*.md` rationale in the plan
(`README, CHANGELOG, crate READMEs, .github/instructions/*.md`) never accounted for
`tests/**/*.md` fixtures.

**Blast radius / probability.** Narrow — exactly two files today, plus any future
`tests/**/*.md` fixture. Probability is *low-to-moderate*: verify-logic changes
touch `.rs` (which re-arms CI), but isolated fixture edits do happen (red-phase
tests, tightening a fixture). Eventually caught by the next code PR, but with a
window of latent breakage on `main` and misattributed failure.

**Reviewer agreement:** Gemini (MAJOR, ci.yml:23), GPT (MAJOR, ci.yml:24; also ran
`cargo test` confirming 3 passing cases drive the fixtures), Claude (MAJOR,
ci.yml:30). Unanimous.

**Fix.** See Remediation **R1**.

---

## Positive verifications (HIGH confidence — all 3 reviewers confirmed SAFE)

These are not defects; they are the adversarial confirmation that the change's
central hypotheses hold. Recorded so the consensus signal is preserved.

### V1 — Required-status-check invariant: VERIFIED SAFE (refutes the hypothetical P0)
Independently re-verified against the live repo:

- `gh api repos/softwaresalt/agent-engram/branches/main/protection` → **404**
  "Branch not protected" (classic protection absent).
- `gh api .../rulesets` → one active ruleset id **12812291** "PR-Required".
- `gh api .../rulesets/12812291` rules = `deletion`, `non_fast_forward`,
  `pull_request` (1 approval + code-owner + last-push approval + thread resolution),
  `copilot_code_review`, `update`. **No `required_status_checks` rule.**
- `gh api .../rules/branches/main` → same five rule types, no status-check rule.

The decision doc's central claim — *`build` is NOT a required status check* — is
**correct**. A skipped run therefore cannot leave a required check "Expected —
Waiting for status to be reported" and cannot block merges (which are review-gated,
and admin/bypass-merged given `require_code_owner_review: true` with **no CODEOWNERS
file**). The hypothetical P0 ("doc-only PRs hang forever") **does not apply to this
repo today.** The in-file contingency comment (companion always-green `build` job if
the check is ever promoted to required) is the correct future mitigation.

### V2 — Security posture: NO REGRESSION
`permissions: contents: read` unchanged; action SHA pins unchanged
(`actions/checkout@34e1148`, `dtolnay/rust-toolchain@29eef33`,
`Swatinem/rust-cache@e18b497`); trigger remains `pull_request` (**not** the
dangerous `pull_request_target`); `paths-ignore` only *narrows* existing triggers
and introduces no injection surface; the rejected PR-title `if:` guard was **not**
added. Feature flags and step sequence byte-identical.

### V3 — Merge / push-to-main / release decoupling: SAFE
No status check is required, so a missing `CI / build` check does not block PR merge
or the merge-commit push to `main`. `release.yml` triggers **only** on
`v[0-9]*.[0-9]*.[0-9]*` tags with its own quality gate — it is unaffected by
push/PR path filtering; there is no release/deploy coupling to the skipped `build`.
(Status-badge staleness on doc-only pushes is cosmetic; see U2.)

---

## Majority findings (MEDIUM confidence — flagged by >½ of reviewers)

### M1 — Plan invariant contradicted by the ignore list  ·  P2 · MINOR · MEDIUM
The plan (`docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md`) asserts
*"bias to over-run, never under-run"* and its `**/*.md` rationale row omits test
fixtures. Finding C1 is a concrete counter-example: the one `.md` class that is a
real test input is under-run. Explicitly broken out by Claude; acknowledged inside
the under-run findings of Gemini and GPT ("contradicting the plan's `bias to
over-run` claim"). **Fix:** amend the plan's ignore-list rationale to acknowledge
`tests/fixtures/**/*.md` as a test input and record the narrowed-glob decision
(couples with R1).

---

## Unique findings (LOW confidence — flagged by exactly one reviewer)

### U1 — DoD empirical verification outstanding  ·  P3 · MINOR · LOW  *(Claude)*
The commit is unpushed, so task `071.001-T`'s DoD empirical steps (observe a
doc-only PR actually skip, a code PR actually run, a mixed PR run) are asserted by
inference, not observed on GitHub. **Fix:** before merge, push a scratch doc-only
change and a `.rs` change to confirm skip-vs-run and record the run URLs in the task
closure. (Do this *after* R1 so the verification exercises the corrected filter,
including a fixture-only edit that must now re-arm CI.)

### U2 — Status-badge staleness on skipped push-to-main  ·  P3 · MINOR · LOW  *(aggregator)*
A doc-only push to `main` that skips CI produces no new run, so any README CI badge
reflects the last executed run rather than the latest commit. Cosmetic only — no
gate or merge impact. **Fix:** none required; note in the plan if badge freshness is
ever a concern.

---

## Aggregator note on the proposed fixes (adversarial-diversity signal)

Reviewer A (Gemini) proposed the fix `- '!tests/**/*.md'` inside `paths-ignore`.
**Treat this with caution:** GitHub Actions does not permit combining `paths` and
`paths-ignore` on the same event, and `!` negation is documented for the positive
`paths` filter — its behavior inside `paths-ignore` is not a reliable, documented
contract. Reviewers B and C independently proposed the **safe, unambiguous** fix:
replace the blanket `**/*.md` with scoped, non-executable markdown roots that
deliberately exclude `tests/**`. **Adopt the B/C approach (R1).** This is a concrete
example of why the multi-model protocol matters: one model's remediation syntax was
semantically unreliable; two others converged on the correct one.

---

## Remediation plan (ordered by confidence × severity)

| # | Finding | Confidence × Severity | Score | Action class |
|---|---------|-----------------------|-------|--------------|
| R1 | C1 — `**/*.md` under-runs verify fixtures | HIGH(3) × MAJOR(3) | **9** | `gated_auto` — confirm scoped glob, then apply |
| R2 | M1 — plan invariant contradicted | MED(2) × MINOR(2) | 4 | `advisory` (doc edit alongside R1) |
| R3 | U1 — DoD empirical verification unrun | LOW(1) × MINOR(2) | 2 | `advisory` (process step pre-merge) |
| R4 | U2 — badge staleness | LOW(1) × MINOR(2) | 2 | `advisory` (no action required) |

### R1 (required before merge) — concrete edit
Drop the blanket `**/*.md` from **both** `paths-ignore` blocks and enumerate only
the non-executable markdown roots. `docs/**` already covers documentation markdown,
so the replacement set is small:

```yaml
paths-ignore:
  - '.backlogit/**'
  - 'docs/**'
  - '.autoharness/**'
  - '*.md'                # root-level: README.md, CHANGELOG.md, AGENTS.md
  - '.github/**/*.md'     # instructions / prompts / agents (non-build)
  - 'scripts/**/*.md'     # scripts/metrics/README.md
```

This keeps `tests/**/*.md` (and any other non-doc markdown) **out** of the ignore
set, so fixture edits re-arm the full `fmt → clippy → test → audit` sequence.
Verified cost: the only markdown the blanket added over this scoped set is root
`README/CHANGELOG/AGENTS`, `.github/**` instructions, and `scripts/metrics/README.md`
— all still ignored — so the CI-minutes savings for doc/backlog closure PRs is
preserved. (Optionally also `crates/**/*.md` for future crate READMEs; none exist
today, and adding it is only safe while no crate `.md` is an `include_str!` input —
none is today.)

> Do **not** use the `- '!tests/**/*.md'` negation form; see the aggregator note.

---

## Bug/issue queue entry (P1 — gate-blocking)

```yaml
type: bug
title: "under-run: `**/*.md` paths-ignore skips CI on tests/fixtures/verify/*.md"
description: >
  The `**/*.md` entry added to both paths-ignore blocks in .github/workflows/ci.yml
  (commit 2798d09, branch 071-ci-build-skip) matches the live on-disk test fixtures
  tests/fixtures/verify/conformant.md and tests/fixtures/verify/malformed.md, which
  are consumed by tests/integration/cli_verify_test.rs (lines 67, 74, 82; asserts
  verify CLI exit codes 0/1). A PR or push that edits only a verify fixture is a
  .md-only change, so CI is skipped, letting a broken test input merge with no
  build/test signal and leave main's CI stale. Contradicts the plan's stated
  "bias to over-run, never under-run" invariant.
file: ".github/workflows/ci.yml"
line: 24            # also line 30 (pull_request block)
severity: "MAJOR"
confidence: "HIGH"
fix: >
  Replace the blanket '**/*.md' in both paths-ignore blocks with scoped
  non-executable markdown roots ('*.md', '.github/**/*.md', 'scripts/**/*.md';
  'docs/**' already present) so tests/**/*.md still triggers CI. Do not use
  '!tests/**/*.md' negation inside paths-ignore.
linked_review: "docs/closure/2026-07-04-ci-build-skip-adversarial-review.md"
```

Suggested command:

```bash
backlogit add --type bug \
  --title "under-run: **/*.md paths-ignore skips CI on tests/fixtures/verify/*.md"
```

---

## Bottom line

- **BLOCK?** No.
- **APPROVE?** No — one required fix.
- **APPROVE-WITH-FIXES.** Apply **R1** (narrow the markdown ignore so
  `tests/**/*.md` re-arms CI), update the plan rationale (**R2/M1**), and run the
  DoD skip-vs-run verification (**R3/U1**) — ideally including a fixture-only edit to
  prove the corrected filter re-arms CI. The required-check safety, security posture,
  and merge/release behavior are all independently verified sound.
