---
title: "PR 337 Stage publication dark-factory — FINAL adversarial rerun"
doc_type: adversarial-review
status: fail-narrow-scope-remaining
date: 2026-08-11
pr: 337
branch: chore/stage-publication-dark-factory
base: main
head_commit: fea2882a
reviewers: 3
model_tiers: [tier1-gpt-5-mini, tier2-claude-sonnet-5, tier3-gpt-5.6-sol]
references:
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - .backlogit/queue/115-S.md
  - .backlogit/queue/116-S.md
  - .backlogit/queue/119.002-T.md
  - docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
  - docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md
  - .backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md
  - .backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md
  - .github/instructions/pull-request.instructions.md
  - .github/instructions/git-merge.instructions.md
---

# PR 337 Stage Publication Dark-Factory — FINAL Adversarial Rerun

## Resolution Addendum (2026-08-12)

All four disclosure/documentation findings below (H1, M2, M3, M4) were subsequently resolved:
`H1` via PR body `## Policy Compliance` section plus byte-identical blob-SHA proof; `M2` via
clarifying language added to `119.001-R` and `120.001-R`; `M3` via the same blob-SHA proof as
H1; `M4` via disclosed `.gitignore`/`git status --porcelain --ignored` provenance confirmation.
See `docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md`,
"Final Adversarial Rerun" section, for the authoritative resolution record and final PASS
verdict. This file is preserved unmodified below as the as-found record of the third rerun.

## Methodology Caveat (must be disclosed)

This session's background reviewer/task agents had **no working shell or `git` execution
capability** despite their tool descriptions claiming "All CLI tools." Five independent
probes (across `task`, `general-purpose`, `explore`, and `code-review` agent types)
either returned an honest "NO SHELL ACCESS" / "NO DIFF ACCESS", or — in one case — returned
a fully fabricated, internally-consistent-but-false `git diff --stat` (a fictitious diff
about "engram platform workspace structure" / Cargo/crates scaffolding that has nothing to
do with this PR). That fabricated output was **discarded entirely** and is not used
anywhere below. One later call did return a `git log --oneline` and a `.backlogit/queue`
directory listing that could be **cross-validated** against independently-obtained
evidence (this agent's own direct `.git/HEAD` read, and its own `view`-based directory
listings), so that specific output was retained with a flagged confidence caveat.

Because a true `git diff origin/main...HEAD` was not obtainable, scope verification below
is based on: (1) full-text reads of every file in the closure doc's publication
allowlist, (2) directory listings of every directory the allowlist and baseline-exception
list touch, checked for any file not accounted for by either list, and (3) the
cross-validated commit log. This is strong but not equivalent to a byte-exact diff;
this gap is itself carried forward as finding **M-verify-4** below.

## Reviewer Dispatch

Three independent reviewer personas were run in parallel with different model tiers,
each given the same evidence bundle (full closure doc, full text of `115-S.md`, `116-S.md`,
`119-F.md`, `119.001-T.md`, `119.002-T.md`, `119.003-T.md`, `120-F.md`, `120.001-T.md`,
`120.002-T.md`, both exec-plans' U2 sections and full decisions/rationale, both archived
reviews `119.001-R`/`120.001-R`, both 2026-08-11 memory docs, the single-worktree gate
excerpt, and this agent's own direct-file-read facts) and instructed to return structured
JSON findings only, with no fabrication.

* **Reviewer-A (Tier 1, gpt-5-mini):** 7 raw findings.
* **Reviewer-B (Tier 2, claude-sonnet-5):** 7 raw findings.
* **Reviewer-C (Tier 3, gpt-5.6-sol):** 8 raw findings.

All three reviewers returned before aggregation began (Phase 3 gate satisfied).

## Independent Fact-Checks Performed on Reviewer Claims

Because adversarial review requires not blindly trusting any single model, every
reviewer claim that could be independently verified against real file content was
checked:

| Reviewer claim | Verified against | Result |
|---|---|---|
| B: repo's CI convention is "@v4 tags, not SHA-pinned" (used to flag 120.002-T's SHA-pin requirement as an unacknowledged deviation) | Direct read of `.github/workflows/ci.yml` | **REFUTED.** Every action is already pinned to a full commit SHA with a version comment, e.g. `actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4`. 120.002-T's SHA-pin requirement matches existing repo convention exactly. Finding discarded. |
| B: cites a specific canonical `ActionResult` enum (`planned/approved/blocked/applied/rolled-back/abandoned`) from `strict-safety.instructions.md` | Content of that file was never provided to Reviewer-B | **UNVERIFIABLE / likely fabricated** — discarded from consensus, flagged as a reviewer overreach. |
| C: "No hard task dependencies enforce 119 U1→U2→U3 or policy U1→U2; only shipment-level ordering is evidenced" | Direct full-text read of `119.002-T.md` (`dependencies: [119.001-T]`), `119.003-T.md` (`dependencies: [119.002-T]`), `120.001-T.md` (`dependencies: [120.002-T]`) — all read in full by this agent earlier in the session | **REFUTED.** Explicit backlogit `dependencies:` edges exist for all three pairs. Finding discarded. |
| B/C: commit `5558a3d0` used prohibited `git add -A`; this is preserved permanently in `main`'s history once merged | `.github/instructions/git-merge.instructions.md`: *"All pull request merges MUST use merge commits. Squash merge and rebase merge are expressly forbidden (Constitution Principle XI, P-009)."* | **CONFIRMED.** This repo's merge policy is merge-commit-only, so `5558a3d0` (including its prohibited-command execution) will be permanently part of `main`'s history, not squashed away. |
| Applicability of a "Policy Compliance" disclosure requirement | `.github/instructions/pull-request.instructions.md`: *"When policy violations occurred during the build session and were overridden: Include a `## Policy Compliance` section — list violated policies, override rationale, and approving operator."* | **CONFIRMED applicable.** A policy violation (prohibited `git add -A`) demonstrably occurred in this PR's own commit history. No such section was found in any artifact reviewed (closure doc, memory docs). This agent has no access to the live GitHub PR description text itself and could not confirm/deny its presence there. |
| CI applicability claim (docs/backlog-only PR is CI-ignored) | `.github/workflows/ci.yml` `paths-ignore`: `.backlogit/**`, `docs/**`, `.autoharness/**`, `*.md`, `.github/**/*.md`, `scripts/**/*.md` | **CONFIRMED.** All allowlisted/baseline paths touched by this PR fall under `paths-ignore` (including `.github/agents/*.agent.md`, matched by `.github/**/*.md`). CI non-applicability claim in the task brief is accurate. |
| Disposable-worktree strategy removal | Full-text read of `119.002-T.md` and the exec-plan's U2 section | **CONFIRMED removed.** No second-worktree/disposable-clone language remains; replaced with "current core worktree" + byte-identical capture/restore/verify language throughout. |
| `templates/instructions/circuit-breaker.instructions.md.tmpl` "missing" concern | Plan text explicitly states this template resolves via `autoharness home` in a separate template-source location, not this downstream workspace; `.github/instructions/circuit-breaker.instructions.md` (the generated output) **is** confirmed present here | **Not a defect** — by design. Cross-repo governance is explicit and documented (120.001-T requires recording both repo paths/commits when source and generated live in different repositories). |

## Consensus Findings (confidence: HIGH — all 3 reviewers independently flagged, in some form)

### H1 — Undisclosed, already-executed publication-contract violation, permanently preserved in merge history

* **Severity:** MAJOR · **Confidence:** HIGH (3/3 reviewers) · **Priority score:** 9
* **File:** commit `5558a3d0` (git history); no disclosure found in `docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md` or `docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md`
* **Issue:** Commit `5558a3d0` was made with `git add -A`, a command this PR's own publication contract (M3 resolution, and the closure doc's "Publication and PR Acceptance" section) explicitly prohibits. This is not a hypothetical/future risk — it already happened, and it is the exact category of violation (M3, path-scoped publication discipline) that the second adversarial rerun required Stage to fix. Because this repository's merge policy is merge-commit-only (`git-merge.instructions.md`, Constitution Principle XI / P-009 — squash and rebase merges are "expressly forbidden"), this commit and its prohibited action will be **permanently preserved** in `main`'s history once PR #337 merges — `fea2882a` corrects the resulting *file content* but cannot and does not erase the fact that the violation occurred. Separately, `pull-request.instructions.md` explicitly requires: *"When policy violations occurred during the build session and were overridden: Include a `## Policy Compliance` section — list violated policies, override rationale, and approving operator."* No such section exists in any artifact this review had access to.
* **Fix:** Add an explicit `## Policy Compliance` section (to the PR description and/or the closure doc) disclosing: the violated policy (no `git add -A`/`git add .`/`git commit -a`), what happened (5558a3d0 staged 5 unrelated paths via `git add -A`), the override rationale (unintentional oversight, self-corrected — not an operator-approved override), and the correcting commit (`fea2882a`). This is a documentation addition, not a code/content change, and does not require reopening the technical remediation.
* **Remediation queue entry (P1):**
```yaml
type: bug
title: "M3 policy-compliance disclosure: undisclosed git add -A violation in 5558a3d0"
description: "Commit 5558a3d0 used git add -A, prohibited by this PR's own publication contract. This repo's merge-commit-only policy (Constitution Principle XI/P-009) means the violation is permanently preserved in main's history once merged. pull-request.instructions.md requires a Policy Compliance section disclosing overridden violations; none exists in the reviewed artifacts."
file: "docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md"
line: 0
severity: "MAJOR"
confidence: "HIGH"
fix: "Add a Policy Compliance section (violated policy, what happened, rationale, correcting commit fea2882a) to the PR description and/or closure doc before merge."
linked_review: "docs/closure/2026-08-11-pr-337-stage-publication-dark-factory-final-rerun-adversarial-review.md"
action_class: manual
```

## Majority Findings (confidence: MEDIUM — 2 of 3 reviewers)

### M-verify-1 — Archived plan reviews use present/completed-tense "PASS" language for proofs that depend on still-`queued` tasks

* **Severity:** MAJOR (conservative; B=MAJOR, C=MINOR) · **Confidence:** MEDIUM (Reviewer-B, Reviewer-C) · **Priority score:** 6
* **File:** `.backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md`, `.backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md`
* **Issue:** `120.001-R` states *"Durability PASS: clean staging hash and second-render no-diff prove regeneration cannot revert policy"* in the present/completed tense. But `120.001-T` (the task that performs the template edit, tune-harness sync, and clean-staging/second-render proof) is still `status: queued` — the proof described has not actually been executed. Similarly `119.001-R`'s "Admission PASS"/"Closure PASS" language reads as completed runtime verification even though `119.001-T`/`119.002-T`/`119.003-T` are all `queued`. This is plan/design review language, not execution-verification language, but the wording does not make that distinction explicit and could be misread by a future Ship agent or human reviewer as "this was already verified."
* **Fix:** Reword both archived reviews' "PASS" lines to unambiguously state they certify the *plan/contract design*, not completed execution (e.g., "Durability contract PASS (design review only) — execution proof is a required acceptance criterion of 120.001-T, not yet performed").
* **Remediation queue entry (P2):**
```yaml
type: bug
title: "Temporal-consistency: PASS language in 119.001-R/120.001-R reads as completed execution proof"
description: "Archived plan reviews state proofs (staging hash match, second-render no-diff, admission/closure evidence) in completed tense while the corresponding tasks remain status:queued. Reword to distinguish design-review PASS from execution-proof PASS."
file: ".backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md"
line: 0
severity: "MAJOR"
confidence: "MEDIUM"
fix: "Reword PASS statements to explicitly scope them to plan/design review, and state execution proof is pending until the referenced task executes."
linked_review: "docs/closure/2026-08-11-pr-337-stage-publication-dark-factory-final-rerun-adversarial-review.md"
action_class: gated_auto
```

### M-verify-2 — `fea2882a`'s byte-exact revert of the 5 swept files is asserted, not independently proven, in the evidence available

* **Severity:** MAJOR (conservative; C=MAJOR, B=MINOR) · **Confidence:** MEDIUM (Reviewer-B, Reviewer-C) · **Priority score:** 6
* **Files:** `.autoharness/config.yaml`, `.backlogit/stash.jsonl`, 3× `.github/agents/*.agent.md`
* **Issue:** The narrative and memory docs assert `fea2882a` reverts these 5 files to their exact `331bdb85` state. This review could only perform a visual/plausibility check on `.autoharness/config.yaml` (content looks like a normal, unmodified baseline config — not a hash/byte comparison) and confirmed the *count* of `.github/agents/*.agent.md` files (exactly 3, matching the narrative) but did not diff their content, nor check `.backlogit/stash.jsonl` at all, because no `git diff`/hash-comparison tool was available in this session.
* **Fix:** Before merge, an operator or CI step with real git access should run `git diff 331bdb85..fea2882a -- .autoharness/config.yaml .backlogit/stash.jsonl .github/agents/orchestrator.agent.md .github/agents/ship.agent.md .github/agents/stage.agent.md` and confirm zero output (byte-identical), and attach that confirmation to the closure doc.
* **Remediation queue entry (P2):**
```yaml
type: task
title: "Verify byte-exact revert of 5 swept files by fea2882a against 331bdb85"
description: "Independently confirm (via git diff, not visual inspection) that fea2882a restores .autoharness/config.yaml, .backlogit/stash.jsonl, and 3 .github/agents/*.agent.md files to byte-identical 331bdb85 state."
file: ".autoharness/config.yaml"
line: 0
severity: "MAJOR"
confidence: "MEDIUM"
fix: "Run git diff 331bdb85..fea2882a -- <5 files> and attach zero-diff proof to the closure doc before merge."
linked_review: "docs/closure/2026-08-11-pr-337-stage-publication-dark-factory-final-rerun-adversarial-review.md"
action_class: gated_auto
```

### M-verify-3 — Untracked-looking `.tmpXXXXXX` scratch directories at repo root, each containing a nested `.git`, directly resemble the exact anti-pattern (sibling repos/worktrees) this PR's M2 remediation claims to have eliminated

* **Severity:** MAJOR (A, B) · **Confidence:** MEDIUM (Reviewer-A, Reviewer-B; Reviewer-C did not flag) · **Priority score:** 6
* **File:** repo root, 17 directories named `.tmp<random6>` (e.g. `.tmp0EIghM`, `.tmpVX6rvp`, `.tmpVUenF8`, `.tmpm8bb16`, …), each containing only `.engram` and `.git` subdirectories
* **Issue:** The single-worktree admission gate this PR's plans explicitly cite states: *"Use branch switching in the core worktree. Do not create or copy sibling repositories to isolate shipments."* These 17 directories are structurally exactly what that sentence forbids (each has its own `.git`). This review could not determine their provenance — they may be incidental artifacts of this interactive review session's own tooling (the `engram` MCP daemon itself, or test-fixture scratch state unrelated to the PR's tracked diff), or genuine leftover Stage/Ship artifacts from working this exact PR. No `git status`/`git diff` access was available to confirm tracked-vs-untracked status.
* **Fix:** Before certifying M2 (dark-factory admission/cleanup) as durably resolved, an operator with real shell access should run `git worktree list --porcelain` and inspect a sample `.tmpXXXXXX/.git` (file vs. directory; if a file, read its `gitdir:` pointer target) to determine whether these are real git worktrees/clones or benign scratch state. If any are genuine sibling repositories, this is an active, present-tense M2 violation requiring cleanup before any further shipment claims proceed; if they are unrelated tooling artifacts, explicitly disclaim them in the closure doc so future reviewers don't need to re-litigate this.
* **Remediation queue entry (P2):**
```yaml
type: task
title: "Determine provenance of 17 root-level .tmpXXXXXX scratch directories containing nested .git"
description: "17 directories at repo root each contain .engram and .git subdirectories, structurally resembling the sibling-repository anti-pattern the single-worktree admission gate forbids. Provenance (PR-related vs. incidental tooling) could not be determined without git status/diff access."
file: ".tmpXXXXXX (repo root, 17 instances)"
line: 0
severity: "MAJOR"
confidence: "MEDIUM"
fix: "Run git worktree list --porcelain and inspect .tmpXXXXXX/.git pointer targets; clean up if genuine sibling repos, or explicitly disclaim as unrelated environment artifacts in the closure doc."
linked_review: "docs/closure/2026-08-11-pr-337-stage-publication-dark-factory-final-rerun-adversarial-review.md"
action_class: gated_auto
```

### M-verify-4 — Closure doc filename/frontmatter date mismatch

* **Severity:** MINOR · **Confidence:** MEDIUM (Reviewer-B, Reviewer-C) · **Priority score:** 4
* **File:** `docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md`
* **Issue:** Filename embeds `2026-08-10`; frontmatter `date:` field says `2026-08-11`. Minor but real inconsistency for a document meant to be an authoritative, durable ledger.
* **Fix:** Add a distinct `updated_at`/`updated` frontmatter field rather than relying on filename-vs-`date` inference, or rename the file.
* **Remediation queue entry (P3):**
```yaml
type: chore
title: "Fix closure doc filename/frontmatter date mismatch"
description: "Filename says 2026-08-10, frontmatter date says 2026-08-11. Add explicit updated_at field or rename."
file: "docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md"
line: 0
severity: "MINOR"
confidence: "MEDIUM"
fix: "Add updated_at frontmatter field distinct from date, or rename file to match."
linked_review: "docs/closure/2026-08-11-pr-337-stage-publication-dark-factory-final-rerun-adversarial-review.md"
action_class: advisory
```

## Unique Findings (confidence: LOW — single reviewer only; preserved as observations)

* **L1 (Reviewer-C, security/supply-chain, MAJOR):** The exact-candidate-approval gates (119.001-T/119.002-T) are entirely *detective* (identity/hash/license capture before, fingerprint comparison after) rather than *preventive* (no described network egress denial, credential denial, or filesystem-write sandboxing during the approved execution window). An approved build script/proc-macro/test/binary could in principle exfiltrate data or write outside the intended paths before post-run fingerprinting would catch the drift. **Action class: advisory** (plan-hardening enhancement, not a blocker for this docs-only PR).
* **L2 (Reviewer-C, Windows containment):** The Windows containment contract (workspace-local `CARGO_HOME`/`CARGO_TARGET_DIR`) does not address `RUSTUP_HOME`/toolchain-install state; a rustup-proxied `cargo` invocation could install/update toolchain data outside the described containment envelope. **Action class: advisory.**
* **L3 (Reviewer-C, rollback/cleanup):** Byte-restoration in `119.002-T` is described for the success path but not explicitly mandated as an unconditional action on failure/timeout/cancellation/abort exit paths. **Action class: advisory.**
* **L4 (Reviewer-C, architecture, downgraded severity — by design, not a defect):** Editing the authoritative template in a separate "autoharness home" repository makes the 120-F release unit non-atomic across repositories. Noted as an inherent, explicitly-documented characteristic of this repo's cross-template governance model (120.001-T already requires recording both repo paths/commits when source and generated differ) rather than an unaddressed gap. **Action class: advisory.**
* **L5 (Reviewer-A, defense-in-depth):** No automated CI/pre-commit hook enforces the publication allowlist (`git diff --cached --name-only` check) — it is currently pure operator discipline. A tooling backstop would reduce recurrence risk of exactly the M3 violation found in H1. **Action class: advisory.**

### Discarded / Refuted Findings (not carried into the remediation queue)

* Reviewer-B's claim that this repo's CI convention is "@v4 tags, not SHA-pinned" — **refuted** by direct inspection of `.github/workflows/ci.yml` (already full-SHA-pinned).
* Reviewer-B's citation of a specific `strict-safety.instructions.md` `ActionResult` enum — **unverifiable**, that file's content was never given to the reviewer; likely an unfounded inference.
* Reviewer-C's claim that no task-level dependency edges enforce U1→U2→U3/U1→U2 sequencing — **refuted** by this agent's own earlier full-text reads of `119.002-T.md`, `119.003-T.md`, and `120.001-T.md` frontmatter, which all contain explicit `dependencies:` edges matching the required sequencing.
* Reviewer-A's several findings that treat "task not yet executed" (cleanup not yet performed, contract test files not yet added, SHA-256 captures not yet attached, config disposition not yet recorded) as PR defects — these conflate a **plans/backlog-publication PR** with an **execution PR**. All referenced artifacts are explicitly the acceptance-criteria output of tasks still `status: queued`; their absence is expected and correct at this stage, not a gap in the current PR's content. Not carried forward as defects (the underlying LOW findings from the second rerun that *are* about text/contract completeness were separately re-verified as genuinely resolved — see below).

## Re-verification of Second-Rerun M1–M4 and LOW Findings (against current file text, not just the closure doc's claims)

| Finding | Re-verified status |
|---|---|
| M1 generated policy durability | **Confirmed resolved in text** (120-F/120.001-T/116-S all name the `.tmpl` as authoritative, require template-first edit, tune-harness sync, clean-staging hash match, second-render no-diff). Caveat: see M-verify-1 (PASS-language temporal ambiguity) and L4 (cross-repo atomicity, by design). |
| M2 dark-factory admission/cleanup | **Confirmed resolved in text** (119.001-T/119.002-T/119.003-T/115-S carry the full admission/cleanup contract verbatim). Caveat: see M-verify-3 (`.tmp*` directories) and L1/L2/L3 (containment depth gaps). |
| M3 path-scoped publication | **Text/contract resolved, but the underlying process was violated once during remediation** (see H1) and not disclosed. Current allowlist text itself is correct and complete. |
| M4 Windows/XDG accuracy | **Confirmed resolved in text.** Caveat: see L2 (RUSTUP_HOME gap, advisory-only). |
| LOW exact-candidate approval | **Confirmed resolved** — 119.001-T/119.002-T text matches verbatim. |
| LOW dirty baseline | **Confirmed resolved** — 115-S/119-F/119.001-T all carry the exact claim-gate language. |
| LOW breaker record | **Confirmed resolved** — `circuit-break-autoharness-verify-workspace.md` records the exact invocation history and correctly concludes the breaker did not trip. |
| LOW completeness | **Confirmed resolved** — `data/` inventory, workflow security, dedicated policy target, doctor wording, and machine-readable references are all present in the reviewed text. |

## Scope/Allowlist Completeness Assessment

Directory listings of every directory touched by the closure doc's allowlist and its
named baseline exceptions (`.backlogit/archive/`, `.backlogit/queue/`, `docs/exec-plans/`,
`docs/closure/`, `docs/memory/2026-08-10/`, `docs/memory/2026-08-11/`,
`docs/memory/compacted/`, `.backlogit/checkpoints/`) were inspected in full. No file
outside the allowlist or the named baseline exceptions was observed in any of these
directories. `docs/memory/2026-08-11/` in particular is a wholly new directory
containing *exactly* the two allowlisted files and nothing else — strong evidence of
scope discipline for at least that directory. This is a **directory-listing-based**
audit, not a byte-exact `git diff`, per the methodology caveat above (tracked as
M-verify item context, not a separate finding, since no contradicting evidence was found).

## Remediation Plan (ordered by priority = confidence_weight × severity_weight)

| # | Finding | Confidence | Severity | Score | Action class |
|---|---|---|---|---|---|
| 1 | H1 — undisclosed git add -A violation, no Policy Compliance section | HIGH | MAJOR | 9 | **manual** |
| 2 | M-verify-2 — unverified byte-exact revert of 5 swept files | MEDIUM | MAJOR | 6 | gated_auto |
| 3 | M-verify-1 — PASS-language temporal ambiguity in 119.001-R/120.001-R | MEDIUM | MAJOR | 6 | gated_auto |
| 4 | M-verify-3 — `.tmpXXXXXX` scratch directories, provenance unknown | MEDIUM | MAJOR | 6 | gated_auto |
| 5 | M-verify-4 — closure doc filename/date mismatch | MEDIUM | MINOR | 4 | advisory |
| 6 | L1 — detective-only supply-chain containment | LOW | MAJOR | 3 | advisory |
| 7 | L2 — RUSTUP_HOME not addressed in Windows containment | LOW | MAJOR | 3 | advisory |
| 8 | L3 — byte-restoration not explicit on failure/abort paths | LOW | MAJOR | 3 | advisory |
| 9 | L5 — no automated allowlist enforcement tooling | LOW | MAJOR | 3 | advisory |
| 10 | L4 — cross-repo template non-atomicity (by design) | LOW | MINOR | 2 | advisory |

Every P0/P1 finding (H1 is the sole P1; there are no P0 findings) appears above with a
full backlog work-item entry.

## Final Verdict

**FAIL.**

This is a substantially narrower FAIL than the prior two rounds. The core technical
remediation that gated the last two reruns is now genuinely done and was independently
re-verified against current file text rather than taken on the closure doc's word:

* The disposable-git-worktree isolation strategy is **fully removed** from `119.002-T.md`
  and the RUSTSEC exec-plan's U2 section, replaced with a sole-core-worktree +
  byte-identical capture/restore/verify protocol, with no second-worktree language
  remaining anywhere in either artifact.
* All four second-rerun MEDIUM findings (M1–M4) and all four LOW findings have genuinely
  matching resolution text in the current queue items, exec-plans, and archived reviews
  — not merely asserted in the closure doc's table.
* No evidence of scope creep was found in any directory the publication allowlist or its
  acknowledged baseline exceptions touch.

However, one **HIGH-confidence, all-three-reviewer-consensus** finding (H1) is a real,
already-occurred, repo-instruction-mandated compliance gap, not a hypothetical or
scope-confused artifact: commit `5558a3d0` executed a `git add -A`, an action this PR's
own publication contract explicitly prohibits, and — because this repository's merge
policy is merge-commit-only — that violation will be permanently preserved in `main`'s
history once merged, with no disclosure section present anywhere this review could
inspect, despite `pull-request.instructions.md` explicitly requiring one for exactly
this situation. Three additional MEDIUM-confidence documentation/verification gaps
(temporal PASS-language ambiguity, an unverified revert claim, and unexplained
`.tmpXXXXXX` scratch directories that structurally resemble the forbidden
sibling-repository pattern) compound the case for not waving this through silently on
what is billed as the final gating rerun — especially since the closure doc's own
`status:` field still literally reads `remediated-stage-pass-final-rerun-pending`.

**Is this PR safe to merge from an adversarial-review standpoint? Not yet, but the
remaining gap is narrow and quick to close.** The remediation required before the next
rerun is almost entirely documentation/disclosure work (add a `## Policy Compliance`
section; reword two PASS statements; attach a byte-diff confirmation for the 5 reverted
files; resolve or explicitly disclaim the `.tmpXXXXXX` directories) — it does not require
reopening the substantive plan content, which this rerun found to be sound. Recommend a
**third and final targeted rerun** scoped only to confirming these four items, rather
than a full re-review of the plans from scratch.
