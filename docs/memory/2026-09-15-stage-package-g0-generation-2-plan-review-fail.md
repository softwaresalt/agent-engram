---
type: session-memory
date: 2026-09-15
agent: stage
session: stage-g0-generation-2-2026-09-15
branch: chore/stage-g0-generation-2-planning
base_commit: f9425943ec6bdb6431389b25d1f2057f51755d3c
program_item: 027-D
package: G0
generation: 2
outcome: PLAN_REVIEW_FAIL — no harvest, no shipment, program HALTED
---

# Stage session memory — Package G0 generation 2

## Outcome

**FAIL at plan-review attempt 1.** No harvest, no backlog hierarchy, no
shipment, no PR. Generation 2's plan-review counter stands at **1**.

The program is **HALTED** per the lock's *If generation 2 also fails* clause.
Reopening requires fresh explicit operator authorization recorded as a lock
amendment.

## Preflight (fresh invocation, not a resumption)

| Gate | Result |
|---|---|
| Tool availability (P-012) | `DEGRADED_MODE` — MCP tools not directly invocable; **CLI fallback** used for every operation, per the registry's declared `cli_command` entries. `backlogit 1.10.1`. Never fell back to ad hoc `grep`/`cat` for backlog state |
| Index sync | `INDEX_SYNC_OK` (1376 artifacts) |
| Checkpoint enumeration | 25 checkpoints, **unfiltered**. Anomaly-first scan: 0 malformed, 0 quarantined (`needs_quarantine=0`, `quarantined=0`), 0 empty `agent`/`status`/`phase`. **0 active `stage`-owned** → ZERO-CANDIDATE NORMAL STARTUP. No restore, no resume, no prune, no resolve |
| Hook events | CLI surface does not expose `--consumer`; registry declares no `cli_command` for poll/ack. Skipped gracefully, session not failed |
| Preserved local state | `.backlogit/stash.jsonl` LF/CRLF drift left unstaged; `checkpoint-20260914-045836.json` left untracked (Stage-owned, `resolved`, not a candidate) |

Activation independently re-verified: `027-D` was `queued` on main; the nine
dependency edges matched the locked DAG exactly; `028-D`–`034-D` all `blocked`.

## Scope honoured

Touched only generation-2 artifacts plus two authorized backlog mutations.
**Did not** touch G1, G2, packages B/C/A/D/E, PR #396, `143.*`, the evidence
branch, any generation-1 artifact, or stash `4EF24729` (verified still
`active`). **No source, test, or configuration file was modified.** No PR
created or merged.

## Artifacts produced

| Path | Status |
|---|---|
| `docs/decisions/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-deliberation.md` | `accepted` — settles minimal API contract D1–D9, residuals Res-1..Res-5 |
| `docs/exec-plans/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-plan.md` | `blocked` — 11 units, integrated hardening H0–H7, attempt counter 1 |
| `docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md` | Terminal record — findings, 10 SETTLED items, escalation routes |

## What happened

The deliberation settled the contract before planning, as the lock requires:
identity = normalized canonicalized absolute path in an opaque
`CanonicalIdentity` (no `Debug`, private field, sole private constructor),
stored privately in `ResolvedFile`; **containment demoted to resolve-time input
validation; identity equality made the sole read-time authority** (the direct
correction of gen-1 finding A). `impl-plan` produced 11 units; hardening
signals (public contract + security) triggered `plan-harden` per P-006.

A six-persona / six-model adversarial panel returned **5 FAIL / 1 PASS** with
**2 P0 and 15 P1** (10 architecture-level).

## Why the gate failed

**Generation 2 reproduced the generation-1 failure pattern on its first
attempt** — *"the plan asserted a guarantee its mechanism did not deliver"* —
and, as with attempt 3, it did so **inside the residual-disclosure section
written to be honest**.

* **A1 (4-persona consensus).** Res-1 claimed the compare-to-read race was
  scoped to the leaf component and that "the ancestor chain is covered". False:
  the content read re-traverses the whole path, so any ancestor can be
  redirected in that window. U7 would have shipped this wording into
  `read_corpus` rustdoc as G1/G2's ground truth.
* **A2.** Canonical *pathnames* are names, not object identities — a mountpoint
  swap leaves the path unchanged; a symlink loop yields a canonicalize error,
  not `IdentityMismatch`.
* **A3.** The hermetic `FakeFs` test cannot be the primary proof of S1: the
  fake's behaviour is defined by the same plan, so it proves plumbing, not
  `std::fs::canonicalize` semantics. The only real-mechanism test was secondary
  and skippable.
* **A4 (verified).** H4/P8 claimed `cargo lint` enforces rustdoc `# Errors` /
  `# Panics`, but `src/lib.rs:12-13` **allows** both lints — an unearned claim
  *inside the hardening section*, compounding A1.
* **P0-2 (verified).** H5's diff allow-list made the package undeclarable:
  `src/services/mod.rs` must gain `pub mod testfs;` for U1 to compile, which H5
  forbids.
* **P0-1.** `read_corpus`'s return type was never specified, yet it is on the
  `pub` surface G1 may not redesign — the implementer would have silently frozen
  G1's contract.

**The correction budget never opened.** Program lock advancement contract item 8:
architecture-level P0/P1 findings block the package immediately. No correction
round was attempted; doing so would itself have been a violation.

## The substantive open question (operator decision)

A1+A2+A3 converge on something Stage cannot decide, because it concerns the
**operator-fixed invariant itself**: a stored-value-compared-for-equality
mechanism with no retained handle or anchor **cannot fully reject ancestor
substitution at read time**. It rejects substitutions completed *before* the
comparison (D4's core is sound) but not those inside the compare-to-read window,
and it cannot see mountpoint substitution at all.

Three routes, detailed in the review record's Escalation section: **(1)** narrow
S1/Res-1 honestly and accept less; **(2)** amend the lock to adopt Option C
(retained anchor + identity equality) — Stage's recommendation, though Stage
made no decision; **(3)** halt G0 permanently. Routes 2 and 3 require a recorded
lock amendment.

## Backlog mutations (both under Stage's own authority)

1. **`027-D` body rewritten** — discharges stash `F937D77C` (P-021 C5 duplicate
   scan: **CLEAN**, exactly one entry; P-021 C6 late-identifier reconciliation:
   **triggered** by three `N/A` source refs, result **no late identifier found
   and none expected** — PR #398 was a bounded no-shipment PR-only Ship
   operation, so the `N/A`s stand as a truthful terminal record; non-blocking).
   The stale "BLOCKED pending publication" opening is replaced by the satisfied
   gate plus dated historical context, and the consumed-operation outcome is
   recorded.
2. **`027-D` moved `queued` → `blocked`.** The authorized operation was consumed
   and failed; leaving it `queued` would falsely signal that another Stage
   operation is authorized, and the lock explicitly prohibits silent re-attempt.
3. **Stash `F937D77C` archived** (archive, never destructive removal, per
   `stash-archive-vs-remove-traceability`). `4EF24729` verified still `active`.

Downstream `028-D`–`034-D` remain `blocked`; all nine dependency edges
unchanged.

## Next steps

Blocked on the operator. Stage stops here and takes no further action. A future
authorized generation should start from the review record's **10 SETTLED items**
— notably that the oracle-manifest avoidance claim is independently verified
correct, that the containment demotion is security-sound, and that the disclosure
mechanism holds — rather than re-deriving them.
