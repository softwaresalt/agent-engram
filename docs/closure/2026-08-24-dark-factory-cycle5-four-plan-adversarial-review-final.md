---
title: "Dark factory cycle 5 four-plan adversarial review final"
type: adversarial-review
doc_type: closure
source: "custom Adversarial Review agent bounded remediation rerun"
date: 2026-08-24
status: complete-pass-with-advisories
reviewers: 3
scope: final-bounded-remediation
---

## Result

The final bounded remediation review passes. Exactly three independently
routed reviewers completed every required domain against the exact current
content of the two changed plans. No reviewer read the prior rerun or another
reviewer's findings before returning.

Both prior MEDIUM findings are closed by unanimous review:

* `M-01` is closed 3/3
* `M-02` is closed 3/3

No HIGH or MEDIUM finding remains. Three LOW findings are preserved as
advisories. No P0 or P1 finding blocks the gate.

## Frozen review scope

The independent review covered only:

* `docs/exec-plans/2026-08-24-7b15b447-daemon-key-engram-caproot-plan.md`
* `docs/exec-plans/2026-08-24-1c2a3cb3-windows-caproot-object-identity-plan.md`

The prior rerun at
`docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-rerun.md`
was withheld from all reviewers. It was read by the parent only after all three
independent reviews returned, solely to reconcile `M-01`, `M-02`, and the
unchanged four-plan boundary state.

No plan, backlog item, source file, test, configuration, decision, prior closure
file, PR, shipment, or other workspace artifact was modified.

## Dispatch and independence evidence

Exactly three named reviewers were dispatched simultaneously. Checked-in
frontmatter plus the named dispatch slot is the routing evidence.

| Slot | Checked-in reviewer | Provider and family | Tier | Return |
|---|---|---|---|---|
| Reviewer A | Concurrency Reviewer | `openai/gpt-5.4-mini` | 1 | Complete |
| Reviewer B | Rust Engineer | `anthropic/claude-sonnet-4.6` | 2 | Complete |
| Reviewer C | Security Sentinel | `anthropic/claude-opus-4.6` | 3 | Complete |

Each reviewer attested `independent: true`, named only the two frozen plan
files in `files_reviewed`, and returned a complete coverage matrix. The
fail-closed condition did not trigger.

## Required domain coverage

| Reviewer | Security | Architecture | Concurrency/TOCTOU | Rust/API | Width | Constitution | TDD | Platform | Rollback/monitoring | Dependencies |
|---|---|---|---|---|---|---|---|---|---|---|
| Concurrency Reviewer | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete |
| Rust Engineer | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete |
| Security Sentinel | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete | Complete |

## Prior remediation closure

### M-01: branch-specific `.engram` authority

**Verdict: CLOSED (3/3).**

The current `7B15B447` plan now:

* Distinguishes the existing-child and cold-start branches
* Enumerates the presence probe, UUID read, PID read, cold-start create/open,
  and publish/read-back interactions
* Requires one retained child open after capability-relative creation on each
  mutually exclusive branch
* Forbids every branch helper from reopening `.engram` or deriving a second
  child authority
* Requires deterministic RED coverage for both probe-to-UUID and
  probe-to-PID substitution windows

This satisfies the prior rerun's `M-01` remediation without leaving a helper
reopen escape path.

### M-02: pinned Windows public trait/type bridge

**Verdict: CLOSED (3/3).**

The current `1C2A3CB3` plan now:

* Requires a Windows-gated compile-and-behavior assertion against the real
  value returned by `Dir::dir_metadata()`
* Invokes public `cap_fs_ext::MetadataExt::{dev, ino}` through the exact pinned
  4.0.2 trait/type chain
* Makes compilation of that public route a prerequisite to GREEN
* Fails the release unit closed if the public bridge does not compile
* Forbids `_WindowsByHandle`, private traits, raw handles, `unsafe`,
  panic-based production fallback, and dependency upgrades

This satisfies the prior rerun's `M-02` remediation. No private or unsafe
fallback remains available to the implementation.

## Consensus normalization

Confidence is based on independent reviewer agreement:

* HIGH: 3/3
* MEDIUM: 2/3
* LOW: 1/3

HIGH P0/P1 findings block. MEDIUM findings require a fix or explicit deferral.
LOW findings are advisory.

| Confidence | Count | Gate effect |
|---|---:|---|
| HIGH | 0 | None |
| MEDIUM | 0 | None |
| LOW | 3 | Advisory |

## Remediation queue

### LOW confidence

| ID | Plan | Severity | Domain | Finding | Disposition |
|---|---|---|---|---|---|
| L-01 | `7B15B447` | P2 | Dependencies | Reviewer B requested an explicit statement that `1CB366DB` begins only after U4 closure, rather than after U3 implementation. | Advisory. The plan already states that `1CB366DB` depends on completion of the release unit, and the release unit includes U4. Preserve full completion as the prerequisite. |
| L-02 | `1C2A3CB3` | P2 | Dependencies | Reviewer B requested that the plan name `5DF94427` explicitly when preserving separate release units. | Advisory. The plan states that it has no dependency on the other composition plans, and the prior rerun independently establishes both release boundaries. Keep them separate. |
| L-03 | `1C2A3CB3` | P3 | Platform verification | Reviewer B requested a concrete authoritative Windows runner label for the NTFS gate. | Advisory. U3 already requires recording the test volume filesystem and makes NTFS the required gate. Record the actual runner and filesystem in closure without narrowing the plan to one hosted image. |

### HIGH and MEDIUM confidence

None.

### P0/P1 backlog-ready entries

None.

## Four-plan boundary reconciliation

The prior rerun was consulted only after independent review completion. The
final boundary state is:

| Constraint | Final verdict |
|---|---|
| `7B15B447` precedes `1CB366DB` | Reconfirmed |
| `1CB366DB` retains its prior pass | Reconfirmed; unchanged and still gated by `7B15B447` completion |
| `1C2A3CB3` and `5DF94427` remain separate | Reconfirmed |
| `5DF94427` retains its prior pass | Reconfirmed; unchanged |
| `49000348` remains environment-blocked | Reconfirmed; non-executable |

The executable ordering remains:

```text
7B15B447 -> 1CB366DB

1C2A3CB3  (separate release boundary)
5DF94427  (separate release boundary)
49000348  (environment-blocked; non-executable)
```

## Final gate

**PASS WITH LOW ADVISORIES.**

The final review is valid: exactly three configured reviewers returned
independently, all required domains were complete, `M-01` and `M-02` closed
3/3, and no HIGH P0/P1 or MEDIUM finding remains. The two remediated plans may
advance under the dependency and release-boundary constraints above.
