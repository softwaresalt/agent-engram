---
title: "Dark factory cycle 5 four-plan adversarial review rerun"
type: adversarial-review
doc_type: closure
source: "custom Adversarial Review agent three-model consensus rerun"
date: 2026-08-24
status: complete-with-required-remediation
commit: 72600a33284148c6a13ef807812fd0e7e06d883a
reviewers: 3
---

# Dark factory cycle 5 four-plan adversarial review rerun

## Result

The fresh review produced no HIGH-confidence finding and no P0 or P1
finding. Two MEDIUM-confidence P2 findings require the proposed fixes below
before their release boundaries enter implementation. Three LOW-confidence
findings remain advisory.

The consensus gate is **pass with required MEDIUM remediation**. No
HIGH-confidence P0/P1 gate blocker exists. Reviewer A's raw `block` label is
normalized to a LOW P2 advisory because Reviewers B and C explicitly accepted
the documented ReFS residual. Confidence is determined by reviewer agreement,
not by an individual reviewer's overall label.

## Scope and provenance

The parent established exact HEAD as
`72600a33284148c6a13ef807812fd0e7e06d883a`. The worktree `.git` file resolves
to the allowed worktree administration directory, and its `HEAD` file binds
the worktree to `refs/heads/stage/dark-factory-cycle2-20260824-1540`.

The review covered:

* `docs/exec-plans/2026-08-24-7b15b447-daemon-key-engram-caproot-plan.md`
* `docs/exec-plans/2026-08-24-1cb366db-bind-proof-composition-plan.md`
* `docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md`
* `docs/exec-plans/2026-08-24-5df94427-workspace-id-parent-fsync-plan.md`
* `docs/decisions/2026-08-24-workspace-authority-followups-deliberation.md`
* `docs/decisions/2026-08-24-windows-caproot-object-identity-spike.md`
* `docs/decisions/2026-08-24-workspace-id-parent-fsync-decision.md`
* The cited environment blocker and adversarial-review blocker decisions
* `AGENTS.md`, the constitution, strict-safety instructions, and
  adversarial-review instructions
* Current source and pinned dependency surfaces only for feasibility

No prior review report was supplied to a reviewer. Each reviewer attested that
it read neither a prior report nor another reviewer's findings.

## Dispatch evidence

Exactly three named reviewers were invoked simultaneously in one dispatch.
Repository frontmatter plus the named dispatch slot is the canonical routing
evidence. Runtime model self-introspection is not exposed and is therefore
recorded as unavailable, not as dispatch failure.

| Slot | Checked-in reviewer | Provider and family | Tier | Return |
|---|---|---|---|---|
| Reviewer A | Concurrency Reviewer | `openai/gpt-5.4-mini` | 1 | Complete |
| Reviewer B | Rust Engineer | `anthropic/claude-sonnet-4.6` | 2 | Complete |
| Reviewer C | Security Sentinel | `anthropic/claude-opus-4.6` | 3 | Complete |

All three returned findings and covered every required domain for every plan:
security, architecture, concurrency/TOCTOU, Rust safety/API feasibility,
scope/width, constitution, TDD RED/GREEN sufficiency, platform verification,
rollback/monitoring, and dependencies. No fail-closed dispatch condition
occurred.

## Boundary and dependency verdicts

| Constraint | Verdict | Evidence |
|---|---|---|
| `7B15B447` before `1CB366DB` | Confirmed | All three reviewers confirmed the frontmatter, dependency graph, and retained-child prerequisite. |
| `1C2A3CB3` remains separate | Confirmed | It changes Windows object-identity proof and has no dependency on the bind/daemon composition release units. |
| `5DF94427` remains separate | Confirmed | It changes Unix parent-directory durability and is explicitly excluded from the bind/daemon shipment width. |
| `49000348` environment-blocked and non-executable | Confirmed | The cited cloud-placeholder decision requires a real cloud-backed repository environment and remains independently blocked. |
| PR #362 | Untouched | No reviewer or parent action read or mutated the PR workflow. |
| Shipment 125-S | Untouched | No reviewer or parent action read or mutated shipment state. |

The executable ordering remains:

```text
7B15B447 -> 1CB366DB

1C2A3CB3  (separate release boundary)
5DF94427  (separate release boundary)
49000348  (environment-blocked; non-executable)
```

## Consensus method

Findings were semantically normalized within plan/release boundaries rather
than matched only by wording:

* HIGH: 3/3 reviewers
* MEDIUM: 2/3 reviewers
* LOW: 1/3 reviewers
* Severity: P0 through P3, taking the most conservative reviewer severity
* LOW findings: advisory regardless of the originating action label

Queue order uses confidence first, then severity, then dependency-safe release
order.

## Consensus-weighted remediation queue

### MEDIUM confidence

| ID | Plan | Severity | Domain | Action | Finding | Proposed fix |
|---|---|---|---|---|---|---|
| M-01 | `7B15B447` | P2 | Concurrency/TOCTOU | `gated_auto` | Reviewers B and C independently found that U3's acceptance language does not fully enumerate branch-specific `.engram` interactions. B distinguished the mutually exclusive probe-to-UUID and probe-to-PID windows; C identified cold-start `create_dir_all` as another relevant interaction. | Before implementation, make U3's acceptance check enumerate presence probe, UUID read, PID read, cold-start create, and publish. Require one retained child after creation/open and prohibit branch helpers from reopening `.engram`. The create operation may remain relative to the retained workspace root, but the resulting child must be opened once and threaded through all later operations. |
| M-02 | `1C2A3CB3` | P2 | Rust safety/API feasibility | `gated_auto` | Reviewers B and C independently identified that the exact public trait/type chain from `Dir::dir_metadata()` to `cap_fs_ext::MetadataExt::{dev, ino}` needs compile proof against pinned 4.0.2. | Add a Windows-gated compile-and-behavior assertion to U1/U2 using the real `dir_metadata()` return type and the public `cap_fs_ext::MetadataExt` import. Keep the existing pin, avoid raw handles and private traits, and fail the release unit if the public call does not compile on Windows. Do not use `unwrap`, `expect`, or `panic!` in production code. |

Both MEDIUM findings have actionable fixes and may not be silently deferred.

### LOW confidence

| ID | Plan | Severity | Domain | Action | Finding | Disposition |
|---|---|---|---|---|---|---|
| L-01 | `1C2A3CB3` | P2 | Platform/security | `advisory` | Reviewer A requested a runtime NTFS-only fail-closed gate because ReFS may expose a 128-bit identifier through a 64-bit `ino()` projection. | Advisory. Reviewers B and C accepted the plan's narrower NTFS verification gate and explicit ReFS residual. Do not claim ReFS closure. Promote to a separate release unit if ReFS support becomes required. |
| L-02 | `5DF94427` | P3 | Rust API feasibility | `advisory` | Reviewer B requested confirmation of the exact safe `Dir::reopen_dir`/`into_std_file` signatures in pinned cap-std 4.0.2. | Advisory implementation check. Confirm the safe public API before GREEN work; do not substitute unsafe, raw handles, or ambient-path reopen. |
| L-03 | `7B15B447` | P3 | Security | `advisory` | Reviewer C noted that cold-start `create_dir_all` is an additional `.engram` touch not named in the problem frame. | Folded into M-01's acceptance fix. The operation is capability-relative and is not independently exploitable on the reviewed evidence. |

### HIGH confidence

None.

### P0/P1 backlog-ready entries

None. No P0 or P1 finding was returned.

## Per-plan release verdicts

| Plan | Verdict | Conditions |
|---|---|---|
| `7B15B447` | Proceed after remediation | Apply M-01 to implementation acceptance criteria; retain dependency precedence over `1CB366DB`. |
| `1CB366DB` | Proceed after `7B15B447` | No independent consensus finding; preserve the hard prerequisite. |
| `1C2A3CB3` | Proceed after remediation | Apply M-02; verify on NTFS and preserve the ReFS residual. |
| `5DF94427` | Proceed with advisory | Confirm the pinned safe reopen API before GREEN implementation. |
| `49000348` | Blocked | Environment gate remains unresolved and non-executable. |

## Raw reviewer evidence

The following material preserves each reviewer's routing attestation,
independence attestation, domain coverage, and unnormalized finding objects.
No reviewer saw this assembly or another reviewer's evidence.

### Reviewer A raw evidence

**Routing:** Concurrency Reviewer; checked-in
`.github/agents/subagents/concurrency-reviewer.agent.md`;
`openai/gpt-5.4-mini`; Tier 1; named slot Reviewer A; runtime
self-introspection `not exposed`.

**Independence:** `prior_reports_read=false`;
`other_reviewers_seen=false`.

| Plan | Sec | Arch | Conc/TOCTOU | Rust/API | Width | Const | TDD | Platform | Rollback | Deps |
|---|---|---|---|---|---|---|---|---|---|---|
| `7B15B447` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `1CB366DB` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Finding | Pass | Pass |
| `1C2A3CB3` | Finding | Pass | Pass | Pass | Pass | Pass | Pass | Finding | Pass | Pass |
| `5DF94427` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |

Reviewer A's plan evidence:

* `7B15B447`: one retained `.engram` capability closes the probe/PID/UUID
  mixed-directory window; deterministic hooks avoid timing tests
* `1CB366DB`: one `GitMetadata` proof closes three-call bind composition;
  `7B15B447` is the correct hard prerequisite
* `1C2A3CB3`: safe pinned APIs are feasible, but ReFS identity width remains
  unresolved in A's assessment
* `5DF94427`: duplicate-handle parent sync has explicit success/error ordering
  and remains Unix-only

Unnormalized finding:

```json
{
  "semantic_id": "windows-refs-object-identity-no-ntfs-gate",
  "plan": "1C2A3CB3",
  "release_boundary": "1C2A3CB3",
  "severity": "P2",
  "action_class": "manual",
  "domain": "platform_verification",
  "location": "docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md:24-38",
  "issue": "The Windows same-object proof is not closed on ReFS/other non-NTFS filesystems; the plan only records the caveat instead of making the implementation fail closed.",
  "evidence": "The spike says ReFS file IDs can be 128-bit and should not be claimed collision-free without evidence, while U3 records the filesystem/caveat but adds no NTFS-only runtime guard.",
  "proposed_fix": "Add an explicit runtime filesystem gate that fails closed outside NTFS, or split ReFS into a separate follow-up proof."
}
```

Raw overall label: `block`.

### Reviewer B raw evidence

**Routing:** Rust Engineer; checked-in
`.github/agents/subagents/rust-engineer.agent.md`;
`anthropic/claude-sonnet-4.6`; Tier 2; named slot Reviewer B; runtime
self-introspection `not exposed`.

**Independence:** `prior_reports_read=false`;
`other_reviewers_seen=false`.

| Plan | Sec | Arch | Conc/TOCTOU | Rust/API | Width | Const | TDD | Platform | Rollback | Deps |
|---|---|---|---|---|---|---|---|---|---|---|
| `7B15B447` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `1CB366DB` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `1C2A3CB3` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `5DF94427` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |

Reviewer B's plan evidence:

* `7B15B447`: source confirms separate `.engram` opens at the presence,
  identity, and PID helpers; `CapRoot::open_child_dir` supports the design
* `1CB366DB`: lifecycle currently composes three independently resolved values;
  a crate-private combined result is feasible
* `1C2A3CB3`: the Windows no-op is real and the public metadata extension is
  the intended safe path
* `5DF94427`: parent sync after cleanup and before either successful return
  gives the required error precedence without consuming retained authority

Unnormalized findings:

```json
[
  {
    "semantic_id": "capfsext-trait-impl-chain-verify",
    "plan": "1C2A3CB3",
    "release_boundary": "1C2A3CB3",
    "severity": "P3",
    "action_class": "advisory",
    "domain": "rust_safety_api_feasibility",
    "location": "docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md §U2; docs/decisions/2026-08-24-windows-caproot-object-identity-spike.md §Findings",
    "issue": "The spike states cap_fs_ext::MetadataExt implements dev()/ino() for 'cap_primitives::fs::Metadata on Windows.' cap_std::fs::Dir::dir_metadata() returns cap_std::fs::Metadata, which is a re-export of cap_primitives::fs::Metadata in practice but this equivalence is an implementation detail of the version pin, not a guaranteed public contract. If the re-export chain breaks or the trait is not blanket-implemented across the re-export boundary, `use cap_fs_ext::MetadataExt as _` applied to the dir_metadata() return type will fail to compile on Windows.",
    "evidence": "workspace.rs:421-429 shows Unix object_identity uses `use cap_std::fs::MetadataExt as _` on the dir_metadata() result. The plan proposes replacing that import with `use cap_fs_ext::MetadataExt as _` for cross-platform coverage. The spike does not show a compile-verified usage, only a trait-inspection finding. cap-fs-ext-4.0.2/src/metadata_ext.rs is cited but not readable in this review session.",
    "proposed_fix": "During U2 implementation, verify the exact import path compiles on Windows before marking GREEN: add a `#[cfg(windows)] let _: (u64, u64) = { use cap_fs_ext::MetadataExt as _; let m = self.dir.dir_metadata()?; (m.dev(), m.ino()) };` compile-only probe in the test module. If the trait is not in scope for the cap_std::fs::Metadata type, the alternative is to use cap_primitives::fs::Metadata directly via a transitive re-export; this must be confirmed against the exact 4.0.2 dependency tree without a version upgrade."
  },
  {
    "semantic_id": "dir-reopen-api-name-confirm",
    "plan": "5DF94427",
    "release_boundary": "5DF94427",
    "severity": "P3",
    "action_class": "advisory",
    "domain": "rust_safety_api_feasibility",
    "location": "docs/exec-plans/2026-08-24-5df94427-workspace-id-parent-fsync-plan.md §U2; docs/decisions/2026-08-24-workspace-id-parent-fsync-decision.md §Evidence",
    "issue": "The decision doc asserts 'cap-std 4.0.2 exposes safe Dir::reopen_dir(&dir)' and the plan depends on this for the duplicate-then-convert pattern. cap-std's Dir API naming for re-opening is version-specific; if the method is named differently (e.g., try_clone, open_dir, or is absent from the public Dir surface in 4.0.2), the U2 implementation will not compile without discovering this at build time.",
    "evidence": "The plan relies on this API without showing a compile-verified call site. The Cargo.toml view was truncated; the exact cap-std 4.0.2 feature set was not inspected. The decision doc cites 'Dir::reopen_dir(&dir)' and 'Dir::into_std_file()' as the safe route, but does not show a working invocation.",
    "proposed_fix": "At the start of U1 (before writing the RED test), add a compile-probe `#[cfg(unix)] { let _: cap_std::fs::Dir = engram_root.dir.try_clone().unwrap_or_else(|_| panic!()); }` or equivalent to confirm the reopen API name and signature before the implementation proceeds. If the method name differs, adjust accordingly within the safe public API surface without touching the 4.0.2 pin."
  },
  {
    "semantic_id": "7b15-engram-open-count-baseline",
    "plan": "7B15B447",
    "release_boundary": "7B15B447",
    "severity": "P2",
    "action_class": "advisory",
    "domain": "concurrency_toctou",
    "location": "src/db/workspace.rs:834-855 (daemon_key_for_workspace); workspace.rs:872 (read_pid_file_via); workspace.rs:892 (workspace_id_present_via); workspace.rs:756-758 (workspace_id_from_metadata)",
    "issue": "The plan's problem frame says '.engram' is opened 'separately in workspace_id_present_via, workspace_id_from_metadata, and read_pid_file_via.' In the current source, the primary TOCTOU window in daemon_key_for_workspace is: workspace_id_present_via opens and drops .engram (line 892), then on the True branch workspace_id_from_metadata opens .engram again (line 756-758). The read_pid_file_via branch (line 872) is only reached when workspace_id_present_via returns False, so the probe/PID window is sequential-not-concurrent only in that branch — not all three are open simultaneously. The plan's U1 and U2 correctly cover both windows, but the plan text slightly overstates the simultaneous-open scenario. This does not invalidate the fix but could cause confusion during U3 review if the refactor does not eliminate all three opens when implementation proceeds.",
    "evidence": "workspace.rs:834-855: the present_via call (line 834) is followed by either workspace_id_from_metadata (line 835) on the True branch or read_pid_file_via (line 838) on the False branch. The two helper calls are mutually exclusive in daemon_key_for_workspace but each independently opens .engram from root.",
    "proposed_fix": "In U3 implementation, explicitly document that the retained engram_root CapRoot must cover both branches: (a) probe → identity read (True branch) and (b) PID read (False branch). A code comment in daemon_key_for_workspace naming the one retained child and noting that both branches consume it will make the reviewer's verification of the 'exactly one retained child per branch' invariant unambiguous."
  }
]
```

Raw overall label: `approve-with-advisories`.

### Reviewer C raw evidence

**Routing:** Security Sentinel; checked-in
`.github/agents/subagents/security-sentinel.agent.md`;
`anthropic/claude-opus-4.6`; Tier 3; named slot Reviewer C; runtime
self-introspection `not exposed`.

**Independence:** `prior_reports_read=false`;
`other_reviewers_seen=false`.

| Plan | Sec | Arch | Conc/TOCTOU | Rust/API | Width | Const | TDD | Platform | Rollback | Deps |
|---|---|---|---|---|---|---|---|---|---|---|
| `7B15B447` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `1CB366DB` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| `1C2A3CB3` | Pass | Pass | Pass | Finding | Pass | Pass | Pass | Pass | Pass | Pass |
| `5DF94427` | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |

Reviewer C's plan evidence:

* `7B15B447`: current helper-level `.engram` reopens confirm the substitution
  window; one retained child closes it without public capability leakage
* `1CB366DB`: the three independent lifecycle calls are confirmed and one
  combined proof is architecturally narrow
* `1C2A3CB3`: the Windows no-op and exact dependency pin are confirmed; NTFS is
  the required gate and ReFS remains a declared residual
* `5DF94427`: both successful publication outcomes require parent sync before
  return, while unrelated publication errors retain precedence

Unnormalized findings:

```json
[
  {
    "semantic_id": "capfs-ext-metadata-type-compatibility",
    "plan": "1C2A3CB3",
    "release_boundary": "1C2A3CB3",
    "severity": "P2",
    "action_class": "advisory",
    "domain": "rust_safety_api_feasibility",
    "location": "docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md:U2",
    "issue": "Plan U2 specifies using cap_fs_ext::MetadataExt::{dev, ino} on dir_metadata() result, but the spike states the trait is implemented for cap_primitives::fs::Metadata while Dir::dir_metadata() returns cap_std::fs::Metadata. The plan does not document the type bridging path.",
    "evidence": "Current Unix object_identity (workspace.rs:421-429) uses cap_std::fs::MetadataExt, not cap_fs_ext::MetadataExt. The spike finding says 'this safe public trait implements dev() and ino() for cap_primitives::fs::Metadata on Windows'. cap_std::fs::Metadata wraps cap_primitives::fs::Metadata (verified by cap-std source structure), so the trait import should work via Deref or re-export, but this bridging is not stated in the plan.",
    "proposed_fix": "Defer rationale: cap_std::fs::Metadata is a newtype wrapper that Derefs to cap_primitives::fs::Metadata in cap-std 4.0.2, so the trait import will resolve. The implementor should verify the import compiles on Windows during U2 and document the type path in the commit message. No plan change required — this is an implementation-time verification item, not a design defect."
  },
  {
    "semantic_id": "daemon-key-engram-reopen-count-mismatch",
    "plan": "7B15B447",
    "release_boundary": "7B15B447",
    "severity": "P3",
    "action_class": "advisory",
    "domain": "security",
    "location": "docs/exec-plans/2026-08-24-7b15b447-daemon-key-engram-caproot-plan.md:Problem Frame",
    "issue": "Plan problem frame states .engram is opened in workspace_id_present_via, workspace_id_from_metadata, and read_pid_file_via. Code inspection shows workspace_id_present_via (line 892) and read_pid_file_via (line 872) both open .engram from root, and workspace_id_from_metadata (line 756-758) also opens .engram from root. However, the cold-start path (line 855) calls workspace_id_from_metadata which creates .engram via create_dir_all (line 735) and then opens it (line 756-758). The problem frame is accurate in identifying three independent opens, but does not mention that the cold-start create_dir_all (line 735) is a fourth ambient-relative .engram touch point that U3 must also thread through the retained child.",
    "evidence": "workspace_id_from_metadata line 735: root.dir.create_dir_all('.engram') — this is relative to the retained root and thus already capability-safe, but it is a fourth .engram interaction not enumerated in the problem frame.",
    "proposed_fix": "Defer rationale: create_dir_all is relative to root.dir (already retained), so it is not a substitution vector. The omission from the problem frame does not create a security gap. Implementor should confirm U3 does not inadvertently remove this create_dir_all or redirect it."
  }
]
```

Raw overall label: `approve-with-advisories`.

## Final gate statement

This rerun is valid: exactly three configured reviewers returned independently,
all required domains were covered, and no prohibited prior review or peer
finding was read. The four plans may advance only under the release verdicts
above. `7B15B447` remains before `1CB366DB`; `1C2A3CB3` and `5DF94427`
remain separate; `49000348` remains environment-blocked and non-executable.
PR #362 and shipment 125-S remain untouched.
