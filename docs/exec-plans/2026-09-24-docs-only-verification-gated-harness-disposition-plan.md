---
title: "Verification-gated harness disposition for documentation-only tasks"
description: "Implementation plan for 036-D Option 2: a durable, template-backed harness-policy path for docs-only tasks, with code tasks kept test-first"
source: "docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md"
source_stash_ids:
  - "1AD161B8"
deliberation_id: "036-D"
revision: 3
status: "review-failed-circuit-open"
implementation_repo: "softwaresalt/autoharness (../autoharness, separate later session)"
adoption_repo: "softwaresalt/agent-engram (this workspace, operator-owned install)"
installed_by: "operator commit f6f3171f, landed on main through H0 (PR #409, merge 7984f896)"
---

> **Status (2026-10-04, PR #410 review): superseded by the operator's install.** This plan's review circuit stayed open,
> but the operator installed the docs-only route directly in this workspace (commit `f6f3171f`: P-002/P-004
> workflow-policies 1.25.0, `_ship.agent.md` Step 2, build-feature and harness-architect skills), and it reached `main`
> through H0 (PR #409, merge `7984f896`). Do not re-run review or harvest this plan for the engram install; the upstream
> autoharness template channel remains the operator's to drive.

## Source and Approval

* **Source:** `docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md`
  (backlog deliberation `036-D`, stash `1AD161B8`).
* **Operator decision (2026-09-24):** Option 2, a durable verification-gated
  disposition for documentation-only tasks. Option 1, a one-off waiver for
  142.059-T, is NOT authorized. This plan uses no waiver in any form.
* **Operator direction (this session):**
  * Support docs-only tasks permanently.
  * Keep every code task test-first.
  * If a policy blocks the obvious path, find a compliant one.
* **Ordering:** 142-S is active and ships first in this repo. CLI parity
  (stash `E06BABAD`, deliberation `035-D`) stays deferred to a later, separate
  shipment under Path S. This plan does not touch parity work.
* **Revision 3** answers the plan-review attempt 2 FAIL. Revision 2 answered
  attempt 1. Both review records are at the end of this file.

## Where the Work Runs (Determination)

1. **The four affected files are template-managed.**
   `.autoharness/harness-manifest.yaml` maps each one to an upstream template
   and records its checksum. A local-only edit is drift.
   * `.github/agents/_ship.agent.md`
   * `.github/policies/workflow-policies.md`
   * `.github/skills/harness-architect/SKILL.md`
   * `.github/skills/build-feature/SKILL.md`
2. **This repo has no template sources.** `git ls-files '*ship*.tmpl'
   '*workflow-policies*.tmpl'` returns nothing. The harness home is the
   autoharness 1.5.0 Python package in site-packages, and its CLI has no render
   command.
3. **The upstream templates have no docs-only path.** They live in
   `softwaresalt/autoharness` (`../autoharness`, read-only for this session).
   Nothing upstream tracks this amendment. Upstream has 21 shipments queued and
   0 active.
4. **An engram Ship chore would deadlock.** Under P-001 it would queue behind
   142-S. 142-S cannot finish until F55 (142.059-T) clears the gate, and Ship
   here has no template to edit.
5. **Auto-tune cannot run yet.** It is an elective agent. Orchestrator Step E1
   blocks electives while a shipment is `active` or the tree is dirty.
6. **An operator-owned persistent mechanism already exists.**
   * `.autoharness/config.yaml` declares that operator edits are preserved
     across re-install and tune.
   * Its `lifecycle_hooks.pre_task_completion.validation_gates` block declares
     operator-authored commands per path pattern. The only placeholders allowed
     are `{file_path}`, `{task_id}`, and `{result}`.
   * The installed 1.5.0 CLI already enforces those gates with
     `autoharness gate check --base <ref> [--head <ref>] --task <id> --json`.
   * Nothing in engram invokes it today. No script or workflow calls
     `gate check`, and the block is commented out.

**Result:**

* **Upstream (autoharness):** U1–U4 are template changes, implemented in a
  separate later session with its own Stage → Ship pipeline.
* **Verification commands:** each workspace's operator declares them in its
  own persistent config. For engram, that is part of A1.
* **Engram:** keeps this plan, stash `1AD161B8`, and `036-D` as the traceable
  intake, plus adoption actions A1–A3. None of them is a new engram shipment,
  and Stage harvests nothing here (D5).

## Problem Frame

Ship will not start work on F55 (142.059-T), and no existing label lets it:

* **Ship Step 2, item 4** halts unless every task carries `harness-ready`.
* **Ship Step 3, item 2** admits only `harness-ready` tasks.
* **P-002** makes `harness-ready` the only precondition.
* **P-004** requires a red phase from generated tests before that label, and
  only harness-architect may apply it.

F55 owns only `docs/troubleshooting.md` and has no test to generate. Its
`harness-not-applicable` label satisfies no gate, so 142-S is halted.

There is a secondary defect. Step 2, item 1 lists only `queued` tasks, but a
claimed shipment's tasks are `active`. Step 3, item 1 already derives an
executable set that covers both.

## Requirements Trace

| Requirement (036-D Amendment Specification) | Units |
|---|---|
| 1. Canonical label `harness-verification-gated` | U1, U3 |
| 2. Qualifications: harness-architect checks (a)–(c); Ship re-checks (a) and (c) and confirms the (b) gate mapping | U1, U2a, U3 |
| 3. P-002 accepts either label; code, test, and config tasks unchanged | U1 |
| 4. P-004 does not apply; manifest records verification evidence; no red-phase claim | U1, U3 |
| 5. Step 2 item 4 accepts either label; item 1 covers `queued` and `active` | U2a |
| 6. Step 3 item 2 admits the qualified label | U2a |
| 7. Step 4 completion gate is the verification; Step 4.3 still runs | U2b, U4 |
| 8. Anti-loophole: declared or actual non-prose path → P-002 violation, P-005, halt | U1, U2a, U2b, U3 |
| 9. Channel: upstream templates, then install here (deviation D10) | U1–U4, A1, A3 |
| 10. No 142-S manifest change; F55 re-harnessed after install | A2 |
| Operator: every code task stays test-first | U1 (code path unchanged); all units are test-first |

## Constitution Check

| Principle / rule | Compliance |
|---|---|
| I Safety-first Rust | Not applicable. The upstream work is Python contract tests and Markdown templates. No engram Rust changes. |
| II Test-first; Dev Workflow 1 (harness before code) | Every U-unit is test-first. The docs-only path covers only tasks that write no production code, and it is fenced by (a), (c), and the diff-scope guard. The feature 142-F still has red harnesses for all five code tasks. The `harness-ready` + P-004 path is unchanged. |
| III / IV Containment | Stage writes only in this workspace. The upstream intake is a command for the operator to run. Path normalization and `DOCS_ROOT` fail-closed rules are in the shared definitions. |
| V Observability | The manifest records gate names, the acceptance trace, the (c) evidence, the start SHA, and the tree snapshot. It records pass/fail and exit codes only, never raw output. |
| VI Single responsibility | harness-architect classifies and labels. Ship re-checks and enforces. build-feature executes. The operator authors the commands. |
| VII / VIII Destructive approval, safety modes | No destructive action. PA1–PA5 are classified. PA1 and PA4 run in Careful plus Freeze-scope mode, limited to the named paths. Commands come only from the operator-owned config (D11). |
| IX Git-friendly persistence | A1 is two isolated, revertible commits. Checksums are never hand-edited. |
| X Context efficiency | One plan artifact. The upstream packet points at it rather than copying it. |
| XI Merge-commit history | The A1 commits stay identifiable inside the 142-S merge. |
| Task granularity / width | Each U-unit is one template plus one named test file, with at most 3 scenarios. |
| Dev Workflow 2 (backlog-driven) | Stash `1AD161B8` and `036-D` carry engram tracking. A1 and A3 are recorded as operator checkpoints in the 036-D notes. The intake packet creates upstream tracking. |
| agent-engram / backlogit overlays | Existing backlog operations only. No new tracker. |
| **Justified deviations** | Dev Workflow 3 (branch per release unit) and Dev Workflow 1: A1 lands operator-owned harness and config commits on the 142-S feature branch, outside any shipment, with no harness of its own. The simpler alternatives are rejected below: a separate branch needs a checkout on a dirty single worktree, and an engram chore deadlocks. A1 is bounded by preconditions (D7, PA1). |

## Shared Definitions (written into U1; referenced by U2a, U2b, U3)

**Path normalization (applies first).**

* Normalize every owned path relative to the repository root.
* Reject `..` segments, absolute paths, and symlinks.
* On case-insensitive file systems, compare case-insensitively.
* Fail closed if `{{DOCS_ROOT}}` resolves to empty, `.`, or the repository
  root.

**Qualification (a): prose-surface allowlist.**

* Every owned path is a `.md` file under `{{DOCS_ROOT}}/`.
* It is NOT under any agent-consumed knowledge directory: `{{DOCS_COMPOUND}}`,
  `{{DOCS_PLANS}}`, `{{DOCS_DECISIONS}}`, `{{DOCS_MEMORY}}`, or
  `{{DOCS_CLOSURE}}`. All of these are existing upstream variables.
* It is NOT any `*.agent.md`, `*.instructions.md`, `*.prompt.md`, or
  `SKILL.md` file.
* It is NOT under the backlog directory.
* It is NOT a top-level file (`README.md`, `AGENTS.md`, `CLAUDE.md`,
  `CHANGELOG*`).
* Tasks that fail (a) use `harness-ready`.

**Qualification (b): operator-declared verification gates.** Checks are
never built from backlog text.

* Every owned path is matched by at least one gate in `.autoharness/config.yaml`
  → `lifecycle_hooks.pre_task_completion.validation_gates`.
* harness-architect records in the harness manifest:
  * the matched gate names (not command text)
  * `Verification Gates: MAPPED`
  * `Surface Check: DOCS-ONLY CONFIRMED`
  * an `Acceptance Trace` that maps at least one acceptance criterion to a
    gate
  * the task start SHA
  * a `git status --porcelain --untracked-files=all` snapshot
  * a hash of the matched-gate list
* Gate commands are written by the operator. They must be deterministic,
  exit non-zero on failure, and read only named paths. `autoharness gate check`
  enforces the closed placeholder vocabulary.
* If `lifecycle_hooks` is absent or no gate matches, (b) fails closed.

**Qualification (c): no executable reader of the surface.**

* harness-architect runs a fixed command defined by the template:

  ```text
  git grep -n -I -F -e <owned path> -e <basename> -e <stem> -- . ':!{{DOCS_ROOT}}' ':!<backlog dir>'
  ```

  It searches tracked files only, so `target/` is excluded.
* Any hit in a non-Markdown file disqualifies the task.
* The manifest records the command and the hit count as `Surface Check`
  evidence.
* Readers that walk the directory, glob, or build the name at runtime are out
  of scope for (c). The Step 4.3 full test suite backstops them (D12).

**Derived, not authored (P-010).** harness-architect never edits a task's
acceptance criteria or verification text.

* If the task's verification cannot be mapped to operator gates, it halts and
  reports.
* The operator may then add gates. That is operator-owned config.
* Otherwise the task returns to Stage.

## Implementation Units (autoharness repo)

Each unit edits one template. Its named contract test is generated first by
the upstream harness-architect, in the style of
`tests/test_harness_architect_p004_contract.py`. At least one scenario per
unit asserts the rendered output with variables resolved.

### U1: Policy template, P-002 and P-004

* **Files:** `templates/policies/workflow-policies.md.tmpl`,
  `tests/test_docs_verification_gated_policy_contract.py`.
* **Changes:**
  * P-002 precondition: `harness-ready`, OR `harness-verification-gated` with
    (a)–(c).
  * The postcondition fields for the new label.
  * Enforcement: the queue admits either label, qualified.
  * P-004 governs `harness-ready` only. A verification-gated manifest never
    records `Red Phase: CONFIRMED`.
  * Anti-loophole: a declared or actual non-prose path is a P-002 violation.
    It is recorded through P-005, and Ship halts.
  * The Shared Definitions.
  * A version-history row.
* **Scenarios (3):**
  1. P-002 names both labels and (a)–(c), including the fail-closed
     `DOCS_ROOT` rule.
  2. P-004's carve-out forbids the red-phase claim.
  3. Anti-loophole routes to P-005.
* **Posture:** test-first. **Size:** S. **Complexity:** low.

### U2a: Ship template, Step 2 and Step 3 gating

* **Files:** `templates/agents/_ship.agent.md.tmpl`,
  `tests/test_ship_verification_gated_queue_contract.py`.
* **Changes:**
  * Step 2, item 1: under a shipment, the Step 3 item 1 derived executable set
    **replaces** the list that items 2 and 4 read. This follows
    `2026-08-21-ship-executable-set-must-wire-into-actual-loop-variable.md`.
    Without a shipment, the list is `{{STATUS_QUEUED}}` plus
    `{{STATUS_ACTIVE}}`. No new placeholder.
  * Step 2, item 2: a task that already carries `harness-verification-gated`
    with its manifest fields is skipped. Every other unlabeled task goes to
    harness-architect for classification.
  * Step 2, item 4: after all sibling harnesses exist, Ship does all of the
    following for each verification-gated task:
    * re-runs (a)
    * re-runs the (c) command
    * confirms the matched-gate hash still matches the config

    On any failure, or on a missing field, it records P-005 and halts.
    `harness-not-applicable` still halts.
  * Step 3, item 2 admits both labels on the same terms.
* **Scenarios (3):**
  1. The derived set is the list that items 2 and 4 read, traced in rendered
     output.
  2. A qualified task is accepted. Unlabeled and `harness-not-applicable`
     tasks are rejected.
  3. The re-check halts on a non-prose path, a (c) hit, or a gate-hash
     mismatch.
* **Posture:** test-first. **Size:** S. **Complexity:** medium.

### U2b: Ship template, Step 4 execution and diff-scope guard

* **Files:** `templates/agents/_ship.agent.md.tmpl`,
  `tests/test_ship_verification_gated_execution_contract.py`.
* **Changes:**
  * Step 4.2, for a verification-gated task: the completion gate is
    `autoharness gate check --base <start SHA> --task <id> --json`, which must
    exit 0. It runs after the task's changes are committed, because `gate
    check` compares refs (`--head` defaults to `HEAD`) and does not read the
    working tree.
  * The JSON report must show at least one gate run on every changed owned
    file.
  * This runs in addition to Step 4.3, never instead of it.
  * No other command is executed for the task.
  * Diff-scope guard before completion. The changed set is:
    * the committed diff `<start SHA>..HEAD`, plus
    * the current `git status --porcelain --untracked-files=all` set, minus
      the recorded start snapshot.

    Any changed path that fails (a) is a P-002 violation, recorded through
    P-005, and Ship halts. This handles dirty files that existed before the
    task, and new untracked files.
* **Scenarios (3):**
  1. The gate command substitution, with Step 4.3 intact.
  2. The guard ignores a dirty file that existed before the task.
  3. The guard halts on a new untracked non-prose file.
* **Depends on:** U2a (same file, applied in sequence).
* **Posture:** test-first. **Size:** S. **Complexity:** medium.

### U3: harness-architect skill template, classification

* **Files:** `templates/skills/harness-architect/SKILL.md.tmpl`,
  `tests/test_harness_architect_verification_gated_contract.py`.
* **Changes:**
  * Step 3 classification, before scaffolding: apply path normalization, then
    (a), (b), and (c).
  * If all pass:
    * write the manifest fields
    * apply `harness-verification-gated`
    * remove `harness-not-applicable`
    * generate no test file
  * If any fails, use the `harness-ready` path. If no harness is possible,
    halt and report. Never fall back to a waiver.
  * The two labels are mutually exclusive.
* **Scenarios (3):**
  1. The qualification gate, including the fixed (c) command.
  2. The manifest fields, with no red-phase claim and no raw output.
  3. Mutual exclusion, and a halt when no gate maps.
* **Posture:** test-first. **Size:** S. **Complexity:** medium.

### U4: build-feature skill template, invocation

* **Files:** `templates/skills/build-feature/SKILL.md.tmpl`,
  `tests/test_build_feature_verification_gated_contract.py`.
* **Change:**
  * Invocation accepts either label.
  * For a verification-gated task, the loop edits only owned prose files.
  * The loop's gate is the U2b `gate check` call.
* **Scenarios (2):**
  1. Both labels are accepted.
  2. The completion gate is `gate check` exit 0.
* **Posture:** test-first. **Size:** XS. **Complexity:** low.

The label-vocabulary unit from revision 2 is deferred. No known workspace
enforces that list.

### Recommended upstream carry-forward (181-F / 187-S)

Upstream feature 181-F (shipment 187-S, queued, review-blocked) rewrites Ship
Step 2 activation. Upstream Stage should:

* sequence this chore first
* add a 181-F requirement that its resolver and result schemas carry the
  verification-gated classification, with a regression contract test

That is upstream's decision. Engram detects a loss at A3 (see A3).

## Adoption Actions (this repo, not harvested)

### A1: Operator install (operator-owned; PA1)

**Preconditions:**

* U1–U4 are merged to the autoharness default branch.
* Ship is halted at 142-S Step 2, with no task mid-build.
* The four target files and `.autoharness/config.yaml` show no changes in
  `git status --porcelain`. All five were clean at planning time.
* The operator has checked read-only whether 181-F records the carry-forward
  (`backlogit get 181-F --cwd ../autoharness`), and has noted the answer in
  036-D.
* Careful plus Freeze-scope mode is on, limited to those five paths.

**Customization inventory (recorded this session):**

* `workflow-policies.md`, `harness-architect/SKILL.md`, and
  `build-feature/SKILL.md` match their manifest checksums.
* `_ship.agent.md` does not match. The file hash is `66f6fcdc…`; the manifest
  has `182cf1ae…`. It carries two kinds of local edits:
  * Frontmatter changes from `a0ccdc27`: `tools`, `reasoning_effort`,
    `model_provider`, and `model_family`. These 4 lines are frontmatter only.
  * Body changes from `2c13e65b` and `5bb6f4ea`: 24 insertions and 26
    deletions, in hunks near lines 274, 305, 455, 613, 763, and 849.

  A1 preserves both.

**Method:**

1. In `../autoharness`, run `git diff <1.5.0 base>..<U-series merge>` for the
   four U templates.
2. Resolve any conditional or optional-pack blocks using this workspace's
   `capability_packs`.
3. Substitute this workspace's resolved variables from the manifest.
4. Apply the result to the four files.
5. Where a hunk's context has moved, or overlaps a customization, adapt only
   the context and never the content. Note each adaptation.

**Config (same session):**

* Uncomment `lifecycle_hooks`.
* Declare `pre_task_completion` validation gates whose patterns cover
  `docs/troubleshooting.md`. For example:
  * a docs-lint gate
  * a one-way code-subset gate that names every error-code source explicitly
  * an F54-matrix surfaces gate
* Every command must exit non-zero on failure. In PowerShell, that means an
  explicit `exit 1`.
* Every command reads only named paths.
* Leave `pre_execution` and `telemetry` untouched.
* Smoke test: run `autoharness gate check --base HEAD --no-count --json`. It
  must load the config without error.

**Proof:**

* `git diff` shows only hunks that match the rendered U-series hunks one for
  one, plus the config block.
* Each commit's `git diff --cached --name-only` lists exactly its intended
  paths.

**Commits** (isolated, on the 142-S branch):

1. `chore(agents): install verification-gated docs harness disposition`
   (the four files). The body records the upstream merge SHA.
2. `chore(settings): declare docs validation gates for verification-gated tasks`
   (config only).

Both commits are named in the 142-S PR body. A reviewer approves them
separately, and the 142-S closure record carries a residual note. The dirty
files of the operator and Ship stay unstaged.

**Do not:**

* hand-edit manifest checksums
* touch `.backlogit/config.yaml`

No workflow, script, or Ship gate in engram reads the manifest or runs
`verify-workspace`, so stale checksums do not fail 142-S.

### A2: Ship re-harnesses F55 inside 142-S (Ship-owned)

* Ship re-reads the installed files and re-runs Step 2.
* harness-architect classifies 142.059-T. It maps F55's existing verification
  items to the operator gates, runs (c), and applies
  `harness-verification-gated`.
* If any item has no gate, it halts. The operator may add a gate. Otherwise
  Stage re-deliberates. There is no waiver.
* This is existing 142-S Step 2 work, with no manifest change.
* Ship also owes the F54 harness fix recorded in 035-D. U2a's re-check runs
  after the F54 fix, so (c) is evaluated against the final sibling harnesses.

### A3: Post-142-S reconciliation tune (operator-invoked elective; PA4)

**Preconditions:**

* 142-S is merged and closed.
* No shipment is `active`.
* The tree is clean.
* The package is upgraded to include U1–U4.

**Expected tune behavior:** auto-tune's checksum scan flags managed files it
did not write, including the four A1 files. It proposes actions, and the
operator decides.

**Pass condition.** Classify each reported difference:

* **U-series hunks:** zero drift. If a later upstream release, such as 181-F,
  dropped them, stop and fix upstream.
* **Other upstream changes since 1.5.0:** reviewed one by one.
* **Recorded customizations** (the `_ship` frontmatter and body hunks): kept
  or dropped on purpose.

Tune, not a hand edit, writes the new checksums. Never refresh checksums just
to silence drift.

## Dependency Graph and Execution Sequence

```text
S0 operator: file upstream intake; order it next upstream (PA3)
  -> [autoharness] U1
  -> [autoharness] U3, U4        (after U1)
  -> [autoharness] U2a -> U2b    (after U1 and U3)
  -> [autoharness] single PR merged
  -> [engram] A1 operator install + config gates (PA1)
  -> [engram] A2 Ship: F54 harness fix, re-harness F55, resume, ship 142-S
  -> [engram] closure query recorded in 036-D and the 142-S closure record
  -> [engram] A3 operator elective tune (PA4)
  -> [engram] Stage: archive stash 1AD161B8, close 036-D
  -> [engram] parity (E06BABAD) per the existing Path S handoff
```

* **No cycles.** 142-S stays the first and only engram shipment. The upstream
  chore is another repo's release unit.
* **Bounded wait:** if 181-F is sequenced first upstream, the wait is bounded
  by the R4 timebox.

## Decisions and Rationale

* **D1:** harness-architect labels; Ship re-checks (P-002 role separation).
* **D2:** label only, with no new backlogit `harness_status` value. Ship gates
  on labels. (Resolves open question 1.)
* **D3:** fix upstream, not locally. (Resolves open question 2.)
* **D4:** Step 2 reuses the executable-set derivation. (Resolves open question
  3.)
* **D5:** no harvest in engram. An engram shipment could not be implemented,
  would deadlock behind 142-S, and would duplicate Ship's Step 2 work.
  Upstream Stage harvests U1–U4 there from this plan.
* **D6:** A1 waits for the upstream merge.
* **D7:** A1 is an operator action, not an elective run. Step E1 governs how
  the Orchestrator routes electives. A1 touches the same artifacts, so it is
  bounded by its preconditions and recorded in 036-D and the 142-S PR.
* **D8:** qualification (a) is an allowlist. It excludes agent-consumed
  knowledge directories, because their Markdown steers agents (a
  prompt-injection surface).
* **D9 (spec deviation, strengthening):** the acceptance trace and the
  start-state record go beyond spec point 2. They keep the operator's
  test-first intent visible without claiming a red phase. Revision 2's
  baseline exit codes are dropped, because `gate check` runs on modified files
  only and a baseline would be vacuous.
* **D10 (spec deviation):** spec point 9 said auto-tune regenerates the
  files. Step E1 blocks that before 142-S closes. A1 plus A3 replace it,
  covering four files (U3 and U4 are needed for D1) plus operator config.
* **D11 (spec refinement, security):** verification commands come only from
  the operator-owned, tune-preserved config, and only `autoharness gate check`
  executes them. The spec said the task's verification section "records"
  commands. This plan keeps that section as intent and never executes its
  text. That closes the path from untrusted backlog text to a shell command.
* **D12 (spec refinement):** spec 2(c) says "no executable test exercises the
  surface." This plan makes (c) a deterministic literal search across tracked
  non-Markdown files. That is broader than tests, and conservative. Dynamic
  readers, such as walks, globs, and names built at runtime, are left to the
  Step 4.3 full-suite backstop and recorded as residual risk.

## Rejected Alternatives

* **Waiver (036-D Option 1):** not authorized.
* **Operator fork (`preserved_artifacts`):** loses upstream fixes. Kept only
  as the R4 fallback.
* **A preserved instruction file overriding Step 2:** contradicts P-002, so it
  is a waiver in disguise.
* **Engram Ship chore, or `shipment return-blocked`:** leads to a deadlock or
  a manifest mutation, and Ship's role does not grant `return-blocked`.
* **Shell commands derived from task text:** untrusted input reaching
  execution (D11).
* **A new upstream check-runner CLI:** unnecessary, because the installed
  `gate check` already enforces operator-declared gates. It would also force a
  mid-shipment package upgrade before A2.
* **Editing the installed package data:** not durable, and outside this
  workspace.

## Risks and Caveats

* **R1: F55's verification may not map to gates.**
  Mitigation: the operator authors gates in A1. If they cannot, Ship halts and
  Stage re-deliberates.
* **R2: A1 render error.**
  Mitigation: the A1 method and proof, isolated commits, a separate reviewer
  approval, and A3.
* **R3: upstream collision with 181-F.**
  Mitigation: the carry-forward recommendation, the A1 precondition that
  checks it, and A3 detection.
* **R4: upstream queue depth.**
  Mitigation: the operator orders this chore next upstream (PA3). If it has not
  merged within 5 working days of intake, or its upstream plan-review circuit
  opens, the operator chooses between waiting and the fork fallback.
* **R5: loophole reuse.**
  Mitigation: the (a) allowlist, the U2a re-check, the U2b diff-scope guard,
  and the closure query.
* **R6: the 142-S PR carries harness and config commits.**
  Mitigation: isolated commits, a PR note, a separate reviewer approval, and a
  residual note.
* **R7: 142-S is abandoned.**
  Mitigation: the A1 commits can be cherry-picked, and A3 re-derives them.
* **R8: the lifecycle gates affect other tasks.**
  Mitigation: patterns are scoped to docs paths. Only U2b invokes
  `gate check`, and only for verification-gated tasks.
* **R9: dynamic readers slip past (c).**
  Mitigation: the Step 4.3 full `cargo dev-test` run.

## Plan Hardening Signals

* **Public API, schema, or contract change: present.** P-002 and P-004 are
  fleet-wide policy contracts.
* **Security, auth, permission, or compliance: present (low).** Operator
  config commands are executed only through `gate check`, and no backlog text
  is ever executed (D11).
* **Migration, destructive, or irreversible step: absent.** All changes are
  revertible text and config edits.
* **External integration, operator checkpoint, or external dependency:
  present.** The plan depends on another repo, and A1 and A3 are operator
  checkpoints.
* **High runtime, rollout, or rollback risk: present (moderate).** The plan
  changes gating fleet-wide and installs into a live shipment.

Requires plan hardening: yes

## Runtime Verification and Closure

* **Runtime surfaces:** agent and policy behavior only. No engram binary,
  CLI, MCP, or IPC surface changes.
* **U-series:** the upstream contract tests pass, and the full upstream suite
  stays green.
* **A1:** the proof and the `gate check` smoke test pass.
* **A2:**
  * Step 2 gates five tasks as `harness-ready` and one as
    `harness-verification-gated`.
  * F55's manifest carries the fields, with no red-phase claim.
  * `gate check` exits 0, with at least one gate on
    `docs/troubleshooting.md`.
  * The guard's changed set is exactly `docs/troubleshooting.md`.
* **Closure query:** run it before the 142-S closure record is written.

  ```powershell
  backlogit list --json | ConvertFrom-Json | Where-Object { $_.labels -contains 'harness-verification-gated' } | Select-Object id
  ```

  This form was checked this session with `harness-not-applicable`, and it
  returned `142.059-T`. Pass condition: every returned task's owned files meet
  (a). Record the result in the 142-S closure record and in the 036-D notes.
* **Validation window:** A1 through A3.

## Upstream Intake Packet (for the operator)

Stage cannot write outside this workspace. The operator, or a Stage session
started in `../autoharness`, runs:

```powershell
backlogit stash add "DOCS-ONLY VERIFICATION-GATED HARNESS DISPOSITION (agent-engram 036-D Option 2, operator-decided 2026-09-24; engram stash 1AD161B8; blocks engram shipment 142-S task 142.059-T). New label harness-verification-gated for tasks whose normalized owned paths are all .md under DOCS_ROOT excluding knowledge dirs (compound/plans/decisions/memory/closure), agent/instruction/prompt/SKILL files, backlog and top-level files; every owned path matched by an operator-declared lifecycle_hooks pre_task_completion validation gate in .autoharness/config.yaml (commands never derived from backlog text); fixed git grep evidence that no tracked non-Markdown file references the path. P-002 accepts it; P-004 unchanged for harness-ready and never claimed for the new label; anti-loophole P-005 halt. Ship Step 2 reuses the executable-set derivation as the list items 2/4 read and re-checks (a)/(c)/gate hash; Step 3 admits the label; Step 4.2 gate is 'autoharness gate check --base <start SHA> --task <id> --json' in addition to 4.3, plus a diff-scope guard over committed diff and untracked delta; harness-architect classifies and labels; build-feature accepts it. Code tasks stay test-first. Recommended: sequence before 181-F/187-S and carry the classification into 181-F. Source plan: agent-engram docs/exec-plans/2026-09-24-docs-only-verification-gated-harness-disposition-plan.md (units U1, U2a, U2b, U3, U4)." --kind chore --priority critical
```

## Plan Hardening

**Required:** yes. The plan changes a policy contract, depends on another
repo, has operator checkpoints, and carries moderate rollout risk.

**Consulted.**

Engram compound learnings (`docs/compound/`):

* `autoharness-optional-pack-content-not-gated-in-templates-2026-08-31.md`:
  fix the template upstream so the render matches.
* `workflow-issues/narrow-scope-expansions-require-stage-2026-08-18.md`: keep
  commits unmixed.
* `workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-defer-to-harness-architect-2026-07-25.md`:
  harness-architect owns ordering.
* `independence-guard-fixture-prose-false-positive-2026-08-22.md`: token scans
  misfire on prose, so (c) counts only non-Markdown hits.
* `test-failures/search-all-test-dirs-when-removing-error-codes-2026-05-09.md`:
  grep the whole tree, because module filters miss files.

Autoharness compound learnings (`docs/compound/`):

* `2026-08-15-checksum-drift-fix-correctly-surfaces-preexisting-self-hosted-customization.md`:
  never refresh checksums to silence drift.
* `2026-08-06-shipment-mixed-role-detection-report-only.md` and
  `2026-08-21-backlogit-1-10-shipment-claim-cascades-to-children.md`: a claim
  activates members. This is version-specific, so re-verify it on the
  installed backlogit.
* `2026-08-18-ship-role-boundary-copilot-findings-in-forbidden-artifacts.md`:
  leave tracked residual notes.
* `2026-08-21-ship-executable-set-must-wire-into-actual-loop-variable.md`: a
  derived set must replace the list that is actually read.

Engram harness files:

* Orchestrator Step E1
* P-001, P-002, P-004, P-005, and P-010
* the commit-scope list
* the harness manifest
* the constitution, including Development Workflow 1–3
* `.autoharness/config.yaml` `lifecycle_hooks`
* `autoharness gate check --help` (1.5.0)

Upstream: the four templates, the `src/autoharness/gates/` package, and 181-F
and 187-S.

**Protected invariants:**

* **I1:** Every code, test, config, script, and harness-file task still needs
  `harness-ready` and P-004.
* **I2:** There is no waiver.
* **I3:** Only Ship changes the 142-S manifest, statuses, and labels.
* **I4:** The dirty edits of the operator and Ship are preserved.
* **I5:** Only harness-architect labels, and it never edits task text.
* **I6:** The engram install matches a merged upstream template.
* **I7:** No backlog text is ever executed.

**Risky actions (strict-safety):**

| ProposedAction | ActionRisk | Approval | Precondition | Rollback |
|---|---|---|---|---|
| PA1: operator A1 commits (four harness files + config) on active 142-S branch | moderate | operator, explicit; reviewer approves both commits separately | A1 preconditions; Careful + Freeze-scope | `git revert` both; F55 halts again (no waiver) |
| PA2: upstream merge of P-002/P-004 change | high | upstream plan-review PASS + operator merge | upstream suite green | revert upstream PR; re-tune |
| PA3: operator orders the upstream chore next | low | operator | 0 upstream `active` (true today) | re-order |
| PA4: elective auto-tune (A3) | moderate | operator; Careful mode | A3 preconditions | restore `.autoharness/backups/`; revert |
| PA5: harness-architect labels 142.059-T | low | Ship role | A1 done | label removed on qualification failure |

**Blocked paths:**

* F55 does not qualify: Ship halts, and Stage re-deliberates. Any change to
  F55's text needs operator approval while 142-S is active.
* The upstream circuit opens: go to the R4 decision point.
* A3 shows U-series drift: stop and fix upstream.
* The config smoke test fails: revert the config commit, and do not re-run
  Step 2.

**Monitoring:**

* Step 2 label counts
* the `gate check` JSON report
* the closure query

**Owners:**

* Operator: A1, A3, and PA3
* Ship: A2
* Stage: archive `1AD161B8` and close `036-D`

**Validation window:** A1 through A3.

**Unresolved operator decisions (they block execution, not planning):**

1. Authorize the upstream intake and order it next upstream (PA3).
2. Authorize A1, including the config gates, after the upstream merge (PA1).

## Plan Review

### Attempt 1 (revision 1): FAIL

* **Personas:** Constitution Reviewer, Scope Boundary Auditor, Architecture
  Strategist, Rust Reviewer, and Learnings Researcher (medium confidence).
* **P1 findings** (all addressed in revision 2):
  * no Constitution Check
  * (a) admitted top-level and agent Markdown
  * P-010 Ship rewrite of task verification
  * missing Ship-side qualification re-check
  * 181-F coupling treated as a rebase note
  * no actual-diff enforcement
  * unsound A1/A3 proof method
  * (c) detection undefined
* **P2 and P3:** addressed in revision 2 (see the revision 2 notes carried
  into revision 3).

<!-- plan-review-attempt: 1 -->

### Attempt 2 (revision 2): FAIL

* **Personas:** Constitution Reviewer (PASS with conditions), Scope Boundary
  Auditor (PASS with advisories), Architecture Strategist (CONDITIONAL PASS,
  1 new P1), Rust Reviewer (PASS with P2s), Security Lens Reviewer
  (CONDITIONAL FAIL, 1 P1), and Learnings Researcher (high confidence, no P1).
* **P1: the diff-scope guard miscounted dirty and untracked files, and the
  start commit was unrecorded.** Revision 3 records the start SHA and the
  porcelain snapshot, and computes the changed set as the committed diff plus
  the untracked delta (U2b scenarios 2 and 3).
* **P1: command constraints were self-declared, with untrusted backlog text
  reaching the shell.** Revision 3 sources commands only from the
  operator-owned config and executes them only through the installed
  `autoharness gate check` (D11, I7). It records pass/fail only.
* **P2s addressed:**
  * Ship now re-runs (c) and checks the gate hash, not just (a).
  * `DOCS_ROOT` fails closed, and paths are normalized.
  * Principles for the deviations are named.
  * The target files must be clean (A1).
  * Tune behavior on mismatched checksums is defined (A3).
  * Conditional blocks are handled, and the `_ship` customizations are
    inventoried in the frontmatter and the body.
  * The 181-F carry-forward is checked at A1 and detected at A3.
  * The label-count claim is replaced by the `gate check` report and a closure
    query.
  * The (c) broadening is recorded, with an exact command (D12).
  * Agent-consumed knowledge dirs are excluded (D8).
  * Output is pass/fail only.
  * A1 records the merge SHA and gets a separate reviewer approval.
  * The executable-set wiring learning is cited and wired into U2a.
  * The error-code sources are explicit in the operator gates.
* **P3s addressed:**
  * safety modes
  * missing Constitution rows
  * pinned closure query and timing
  * "five templates" corrected to four
  * 181-F wording
  * U5 deferred
  * PowerShell `exit 1`
  * citation paths fixed

<!-- plan-review-attempt: 2 -->

### Attempt 3 (revision 3): FAIL (circuit open)

**Personas and verdicts:**

* Constitution Reviewer: PASS, 2 P2 findings.
* Scope Boundary Auditor: CONDITIONAL PASS, 2 P2 findings.
* Rust Reviewer: PASS, P3 findings only.
* Security Lens Reviewer: PASS with 3 P2 findings. Its attempt-2 P1 is
  resolved.
* Architecture Strategist: FAIL, 1 P1.
* Learnings Researcher: 1 P1. Confidence: medium.

**P1 findings (both block harvest):**

1. **The start SHA and snapshot are recorded too early.** Shared Definitions
   (b) has harness-architect record them at Step 2 classification. By Step
   4.2, the five sibling code tasks have committed `src/**` changes. The
   diff-scope guard and `gate check --base <start SHA>` would then see those
   files and halt with a false P-002.
   **Fix:** Ship records the start SHA and snapshot when it claims the task
   at Step 4.1. Add a U2b scenario in which a sibling commit lands between
   Step 2 and Step 4.
2. **R8 may be wrong about backlogit's completion broker.** The claim
   "nothing in engram invokes `gate check`" misses one path. `backlogit move
   --status done` runs "the configured pre-task-completion gate" and records
   `pre_task_completion_gate_passed` events. `backlogit shipment ship` fails
   closed without them (see
   `docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`).
   It is unverified whether that broker reads `.autoharness/config.yaml`
   `lifecycle_hooks`. `.backlogit/config.yaml` and `.backlogit/hooks.yaml`
   contain no gate declaration.
   **Fix:** before A1, find out which config the broker reads. If it is
   `lifecycle_hooks`, uncommenting that block gates the `done` transition for
   every 142-S task. Model that, and require F55's done-evidence before
   `shipment ship`.

**P2 findings (to fold into revision 4):**

* **`gate check` side effects:**
  * It writes `gate-state.json`.
  * On the third failure it writes a circuit-break checkpoint under
    `docs/memory/`, which fails (a).
  * Requeue semantics conflict with I3.
  * Advisory enforcement exits 0.

  **Fix:** require `enforcement: absolute`, require every `results[].passed`
  to be true, and treat gate-tool artifacts as a circuit halt, not a P-002
  violation.
* **Gate commands run with `shlex` and `shell=False`.** Use the form
  `pwsh -NoProfile -File <script> {file_path}`, or an equivalent without a
  shell. Add a safe-name check on `{file_path}`.
* **The A1 smoke test runs no gate.** Instead, assert that the gates are
  enabled and that their names match, and run each gate once against a known
  bad input. Record pass/fail only.
* **The start snapshot must include content hashes** (`git hash-object`) for
  files that were already dirty. Then re-check the gate hash just before
  `gate check`.
* **The (c) search excludes all of DOCS_ROOT and `.autoharness/`.** Exclude
  only `{{DOCS_ROOT}}/**/*.md`, add `-i`, and add `':!.autoharness/'`.
* **The operator must explicitly accept spec deviations D9–D12.** Add this as
  unresolved operator decision 3.
* **Reword A1 as a template install, not a Principle II deviation.** U1–U4
  contract tests cover its content.
* **Exclude `docs/research`** (`design_docs`, `product_specs`), or justify
  including it.

**P3 findings (advisory):**

* Name `cargo ci` (all features) as the R9 backstop, or record the gap.
* Say what happens to a failed committed gate run: fix forward or revert.
* Fold the path-normalization check and the "at least one gate on every
  changed owned file" check into existing scenarios.
* Remove or assign the "Step 2 label counts" monitor.
* Cite `2026-07-01-subprocess-validation-gating.md` (autoharness) and the
  gate-evidence learning above.

**Gate:** FAIL. P1 findings remain after the maximum 2 re-entry cycles.
Revision 4 is NOT written. The fixes above are recorded but unreviewed, and
no fourth review attempt is run.

<!-- plan-review-attempt: 3 -->

### P-013.6 Escalation Payload

```yaml
threshold: plan-review consecutive FAIL; attempt counter 3 (max 2 re-entry cycles)
failure_summary: >-
  Revisions 1-3 each cleared the prior P1s, but review surfaced new P1s each
  time. Remaining P1s: (1) per-task start SHA recorded at Step 2 instead of
  Ship Step 4.1; (2) unverified coupling between backlogit's completion broker
  and .autoharness lifecycle_hooks.
last_actions:
  - attempt 1 FAIL (8 P1) -> revision 2
  - attempt 2 FAIL (2 P1: diff guard, command trust) -> revision 3 (operator-config gates via autoharness gate check)
  - attempt 3 FAIL (2 P1 above)
artifact_refs:
  - docs/exec-plans/2026-09-24-docs-only-verification-gated-harness-disposition-plan.md (revision 3)
  - docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md
  - .backlogit/queue/036-D.md
  - stash 1AD161B8
telemetry_evidence: none (telemetry block disabled in .autoharness/config.yaml)
resumption_checkpoint: docs/memory/2026-09-24-stage-036d-option2-plan-circuit-open.md
resolved_escalation_route: {model_family: gpt-6-sol, model_provider: openai, reasoning_effort: xhigh}
same_route_guard: not triggered (Stage route claude-opus-5.5/anthropic/high differs)
handoff_status: ESCALATION_DEGRADED; no engram tool surface in this Stage session; operator halt
```

### Post-review research note (not a revision, not a review attempt)

This note was checked read-only after the circuit opened. The `backlogit move
--help` output says: "For task/subtask completions the pre-task-completion
gate broker may run autoharness gate check." Gate refusals exit with code 6,
7, or 8.

That confirms part of attempt-3 P1 #2. The broker is the same tool-enforced
completion path, and enabling `lifecycle_hooks` in engram's
`.autoharness/config.yaml` would apply it to every 142-S task's `done`
transition. Two points are still unverified:

* the broker's default base ref
* its result when no gate pattern matches a task's changed files

A revision 4 should probably use this broker, and the
`pre_task_completion_gate_passed` evidence it records for `shipment ship`, as
the completion gate. That would replace the separate Step 4.2 call in U2b.
