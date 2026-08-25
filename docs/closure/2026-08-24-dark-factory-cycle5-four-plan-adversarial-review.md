---
title: "Dark factory cycle 5 four-plan adversarial review"
doc_type: closure
source: "custom Adversarial Review agent failed-closed dispatch evidence"
date: 2026-08-24
status: failed-closed
review_commit: "72600a33284148c6a13ef807812fd0e7e06d883a"
---

# Dark factory cycle 5 four-plan adversarial review

> [!IMPORTANT]
> **HISTORICAL / SUPERSEDED.** Any queued-shipment, executable-handoff, old-roster, old-edge, or old reviewed-file statement below is source-head history only. It cannot authorize claim or implementation. Current authority: [PR #363 fail-closed planning authority](../decisions/2026-08-25-pr-363-fail-closed-planning-authority.md).

## Gate decision

**FAILED CLOSED. No adversarial consensus was assembled.**

Exactly five reviewers were dispatched simultaneously and independently in the
requested agent/model slots. All five returned, but none evidenced its observed
model identity. Reviewers A and B also could not evidence `git rev-parse HEAD`.
Reviewers C and D reported the requested commit as HEAD but explicitly reported
their model identity as unverifiable. Reviewer E likewise reported its model as
unverifiable despite returning `COMPLETE`.

The operator required failure closure when any requested identity could not be
evidenced. The findings below therefore remain unmerged reviewer observations.
They have no HIGH, MEDIUM, or LOW confidence classification and do not clear any
plan's adversarial-review gate.

No plan, backlog item, source, test, configuration, PR, or shipment was changed.
Plan `49000348` was not treated as executable. PR #362 and shipment `125-S` were
not inspected or touched.

## Authoritative evidence standard and requirement resolution

A reviewer response is consensus-eligible only when execution-system task or
dispatch-result metadata, or runtime metadata, binds that specific response to
its observed provider/model identity. The minimum durable receipt is a stable
response/task identifier plus the execution-system provider/model field bound
to the response. Checked-in routing configuration, requested model labels,
named slots, and reviewer self-assertion are insufficient.

The initial five outputs, the later three-response rerun, the bounded final
rerun, their embedded raw outputs, and the checked-in reviewer frontmatter were
examined during PR #363 remediation. No response/task IDs or execution-system
model fields are preserved for the rerun responses. All available identity
statements are configuration intent or reviewer self-report, and runtime model
identity is explicitly recorded as unavailable.

**Requirement-resolution decision:** authoritative execution binding does not
exist in the available record, and no explicit operator requirement change was
recorded. This initial standard remains authoritative. The configuration-only
rerun and final rerun are invalidated, have zero consensus-eligible reviewers,
and cannot clear the four-plan gate.

## Reviewed plans

* `7B15B447`:
  `docs/exec-plans/2026-08-24-7b15b447-daemon-key-engram-caproot-plan.md`
* `1CB366DB`:
  `docs/exec-plans/2026-08-24-1cb366db-bind-proof-composition-plan.md`
* `1C2A3CB3`:
  `docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md`
* `5DF94427`:
  `docs/exec-plans/2026-08-24-5df94427-workspace-id-parent-fsync-plan.md`

Review prompts required each reviewer to use
`git show 72600a33284148c6a13ef807812fd0e7e06d883a:<path>` for pinned reads,
cover cited decisions and deliberations, inspect current source only for
feasibility, and assess constitution and TDD sufficiency.

## Dispatch and identity evidence

| Slot | Requested agent | Requested model | Observed model | HEAD evidence | Raw result | Consensus eligible |
|---|---|---|---|---|---|---|
| A | Architecture Strategist | `gpt-5.4-mini` | `UNVERIFIABLE` | `UNVERIFIABLE` | `FAILED_CLOSED` | No |
| B | Concurrency Reviewer | `gpt-5.4-mini` | `UNVERIFIABLE` | `UNVERIFIABLE` | `FAILED_CLOSED` | No |
| C | Security Reviewer | `claude-sonnet-4.6` | `UNVERIFIABLE - model identity not exposed in this execution context; same-model multi-persona review is not multi-model consensus` | `72600a33284148c6a13ef807812fd0e7e06d883a` | `FAILED_CLOSED` | No |
| D | Rust Safety Reviewer | `claude-sonnet-4.6` | `UNVERIFIABLE - model identity not exposed to the agent execution context` | `72600a33284148c6a13ef807812fd0e7e06d883a` | `COMPLETE` | No |
| E | Security Sentinel | `claude-opus-4.6` | `UNVERIFIABLE` | `72600a33284148c6a13ef807812fd0e7e06d883a` | `COMPLETE` | No |

The table preserves the raw identity, model, HEAD, and result fields returned by
each reviewer. The task dispatch configuration proves which models were
requested, not which models actually executed.

### Reviewer A raw evidence

```json
{
  "slot": "A",
  "agent": "Architecture Strategist",
  "requested_model": "gpt-5.4-mini",
  "observed_model": "UNVERIFIABLE",
  "head": "UNVERIFIABLE",
  "result": "FAILED_CLOSED",
  "evidence": [
    "view C:/Source/GitHub/engram-stage-dark-factory-cycle2-20260824-1540/.git -> gitdir: C:/Source/GitHub/engram/.git/worktrees/engram-stage-dark-factory-cycle2-20260824-1540",
    "view C:/Source/GitHub/engram/.git/worktrees/engram-stage-dark-factory-cycle2-20260824-1540/HEAD -> Permission denied and could not request permission from user",
    "read plan docs, instructions, decision docs, compound learning, and source files via pinned file views"
  ]
}
```

### Reviewer B raw evidence

```json
{
  "slot": "B",
  "agent": "Concurrency Reviewer",
  "requested_model": "gpt-5.4-mini",
  "observed_model": "UNVERIFIABLE",
  "head": "UNVERIFIABLE",
  "result": "FAILED_CLOSED",
  "evidence": [
    "The worktree .git file points to C:/Source/GitHub/engram/.git/worktrees/engram-stage-dark-factory-cycle2-20260824-1540, which is outside the allowed tree for this review surface.",
    "Attempts to read C:/Source/GitHub/engram/.git/worktrees/engram-stage-dark-factory-cycle2-20260824-1540/HEAD and logs/HEAD were denied.",
    "Read-only review otherwise covered AGENTS.md, the constitution/strict-safety/adversarial-review instructions, the four plan docs, the cited deliberation/decision/spike/compound docs, and src/db/workspace.rs plus src/tools/lifecycle.rs."
  ]
}
```

### Reviewer C raw evidence

```json
{
  "slot": "C",
  "agent": "Security Reviewer",
  "requested_model": "claude-sonnet-4.6",
  "observed_model": "UNVERIFIABLE - model identity not exposed in this execution context; same-model multi-persona review is not multi-model consensus",
  "head": "72600a33284148c6a13ef807812fd0e7e06d883a",
  "result": "FAILED_CLOSED",
  "evidence": [
    "Model identity cannot be cryptographically verified; 'claude-sonnet-4.6' is a label but not a provably independent reviewer distinct from prior same-session persona analyses",
    "No subagent dispatch surface is available to independently verify reviewer identity separation",
    "Result is FAILED_CLOSED to prevent this review from being miscounted as consensus clearance"
  ]
}
```

Reviewer C also disclosed incomplete direct instruction coverage:
`.github/instructions/constitution.instructions.md` was not directly read, and
the adversarial-review rules were taken through `AGENTS.md` and plan sections.

### Reviewer D raw evidence

```json
{
  "slot": "D",
  "agent": "Rust Safety Reviewer",
  "requested_model": "claude-sonnet-4.6",
  "observed_model": "UNVERIFIABLE - model identity not exposed to the agent execution context",
  "head": "72600a33284148c6a13ef807812fd0e7e06d883a",
  "result": "COMPLETE",
  "evidence": [
    "Cargo.lock cap-std 4.0.2, cap-fs-ext 4.0.2 entries confirmed",
    "Read all four plans and their named decision documents",
    "Read src/db/workspace.rs, src/tools/lifecycle.rs, src/lib.rs, Cargo.toml, Cargo.lock, and AGENTS.md"
  ]
}
```

The `COMPLETE` result is ineligible because the same result explicitly says the
observed model is unverifiable.

### Reviewer E raw evidence

```json
{
  "slot": "E",
  "agent": "Security Sentinel",
  "requested_model": "claude-opus-4.6",
  "observed_model": "UNVERIFIABLE",
  "head": "72600a33284148c6a13ef807812fd0e7e06d883a",
  "result": "COMPLETE",
  "evidence": [
    "All four plans read and verified against source at HEAD",
    "Deliberation, spike, decision, and blocker documents read",
    "Source files src/db/workspace.rs and src/tools/lifecycle.rs verified for claim accuracy",
    "Cargo.toml cap-fs-ext pin verified at =4.0.2",
    "AGENTS.md constitution reviewed for forbid(unsafe_code) and TDD requirements"
  ]
}
```

The `COMPLETE` result is ineligible because the same result explicitly says the
observed model is unverifiable.

## Required claim checks

These are raw reviewer statements, not consensus classifications.

| Required check | Unmerged evidence |
|---|---|
| `7B15B447` precedes `1CB366DB` | A, B, C, D, and E reported this as supported by `depends_on_stash_id`, the dependency graph, and the deliberation |
| `1C2A3CB3` and `5DF94427` remain separate widths | A, B, C, D, and E reported the plans' dependency sections preserve separation |
| ReFS caveat is honest | A, B, C, D, and E reported that NTFS is the required gate and ReFS 128-bit truncation remains an explicit residual |
| Parent-fsync precedence is exact | A, B, C, D, and E reported the stated precedence as `Ok`/`AlreadyExists` success overridden by sync failure, with unrelated publication errors preserved |
| No unsafe is introduced | C, D, and E reported that the plans prohibit unsafe and raw-handle conversion |
| No ambient reopen is introduced | C and E marked this only partial because current `workspace_id_from_metadata` uses ambient `canonicalize()` for a display/error path; D separately questioned the exact safe `reopen_dir`/`into_std_file` API route |

Because reviewer identities are ineligible, even five matching raw statements
do not constitute HIGH confidence.

## Unmerged per-plan observations

The following register preserves substantive reviewer output without semantic
deduplication, confidence scoring, or remediation ordering.

### `7B15B447`

* Reviewer B, P2: final key-provenance assertions may not prove that exactly one
  retained `.engram` child is used; add an assertion that fails on a second
  child open
* Reviewer C, P1: current source contains the targeted independent child-open
  window; the plan's retained-child direction is appropriate
* Reviewer C, P2: name exact checkpoint placement so a moved hook cannot produce
  a false security claim
* Reviewer D, P1: current source confirms independent `.engram` child opens;
  retain one child through probe, UUID, PID, and publication
* Reviewer D, P3: describe the defect as handle-relative double-open TOCTOU, not
  an ambient reopen
* Reviewer E, P2: the problem frame overstates the count and nature of the
  reopen; current code retains the workspace root but opens the child more than
  once
* Reviewer E, P1: audit the existing ambient `canonicalize()` used to derive an
  error/display path and either remove it or prove it cannot affect authority

### `1CB366DB`

* Reviewer B, P2: the mixed-tuple RED may not prove that
  `resolve_git_metadata` is called exactly once; add a deterministic re-entry or
  call-count assertion
* Reviewer C, P1: current lifecycle source confirms three independent
  resolution calls
* Reviewer D, P1: current source confirms the plan premise and the hard
  dependency on `7B15B447`
* Reviewer E, P2: identify the lifecycle checkpoint/seam that makes the RED
  deterministic and prevents an implementation-budget overrun

### `1C2A3CB3`

* Reviewer C, P2: current Windows `prove_names_same_object` is a no-op; the
  direct different-object RED is suitable
* Reviewer C, P3: update the stale source comment claiming safe Windows identity
  requires unsafe when the implementation is eventually performed
* Reviewer D, P1: current source confirms the Windows no-op; verify
  `cap_fs_ext::MetadataExt` directly before implementation
* Reviewer D, P3: specify trait imports or fully qualified calls to avoid
  `MetadataExt` method ambiguity
* Reviewer E, P2: state the exact target `cfg` set for the cross-platform
  identity function rather than saying only "supported platforms"

### `5DF94427`

* Reviewer B, P2: error injection alone may not prove cleanup-before-sync and
  sync-before-readback; expose ordered cleanup, sync, and readback events
* Reviewer C, P3: current source confirms the parent directory is not synced
* Reviewer D, P1: the plan's exact safe API route is not proven; verify whether
  pinned 4.0.2 exposes the asserted `reopen_dir` and `into_std_file` operations
  without unsafe or dependency changes
* Reviewer D, P1: current source confirms both successful publication outcomes
  can return without a parent barrier
* Reviewer D, P3: gate the helper and every call site on Unix, and require the
  Windows test to observe zero parent-sync events
* Reviewer E, P2: explain that reopening `"."` from the retained directory
  capability cannot be redirected by ancestor substitution

## Unmerged cross-release observations

* Reviewer C, P2: all planned RED harnesses remain pre-implementation; preserve
  strict RED-before-GREEN execution
* Reviewer D, P2: reported a crate-level clippy-policy concern outside the
  declared plan surfaces; this was not normalized into the plan review
* Reviewers C, D, and E confirmed the current source defects that motivate the
  four plans, but premise confirmation is not itself a plan-remediation finding
* Reviewers A and B completed broad document/source coverage but are ineligible
  because neither model identity nor HEAD could be evidenced
* Reviewer C is additionally ineligible because required instruction files were
  not all read directly

## Remediation queue status

The requested confidence-by-severity remediation queue is **withheld**.
Producing HIGH, MEDIUM, or LOW entries would falsely represent an eligible
five-reviewer consensus.

The four plans remain blocked. A replacement run must provide verifiable
observed agent/model identity for every slot, exact-commit evidence for every
reviewer, and complete direct coverage of all required instruction files before
normalization can begin.
