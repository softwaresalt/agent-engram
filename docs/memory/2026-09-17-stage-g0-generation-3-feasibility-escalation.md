---
type: session-memory
date: 2026-09-17
agent: stage
session: stage-g0-generation-3-2026-09-17
branch: chore/stage-g0-generation-3-planning
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 3
attempt: 0
outcome: FEASIBILITY ESCALATION — deliberation did not promote to impl-plan; no plan, no plan-review, no harvest, no shipment; 027-D returned to blocked pending operator determination
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
authority_amendments: ["Amendment 1 + Correction 1 + Correction 2", "Amendment 2 (2026-09-17)"]
base_commit: 332ff03a4be1987082b8356f62691b55d96cf49a
merged_pr: 400
plan_review_attempt: 0
harvested: false
shipment_assembled: false
pull_request_created: false
engram_status: ENGRAM_DEGRADED
---

## What this session did

Executed one full authorized Stage package operation on G0 (`027-D`) generation 3,
activated by the operator after PR #400 merged to `main` at `332ff03a`.

The operation terminated at the **deliberation** stage on the lock's
per-target-triple **feasibility-escalation** path. This is **not** a
`PLAN_REVIEW_FAIL`: no implementation plan was produced and the generation-3
plan-review attempt counter stands at **zero**.

## Sequence

1. Startup: tool gate, config schema validation (Stage route `claude-opus-5` /
   `anthropic` / `high`; nested escalation route distinct, so not degraded),
   backlog index sync, checkpoint enumeration.
2. Checkpoint scan: **25 valid records, 0 active, 0 quarantined, 0 anomalous** →
   zero-candidate normal startup. No recovery performed.
3. Verified Amendment 1 (+ Corrections 1 and 2) and Amendment 2 present on this
   branch via the #400 merge.
4. Recorded the operator-authorized `027-D` `blocked` → `queued` transition and
   claimed it.
5. Deliberated. Enumerated supported target triples from the release and
   verify-release workflows, audited the locked dependency graph against
   vendored source, and determined feasibility per triple per fixed property.
6. Subjected the determination to independent multi-persona, multi-model
   adversarial verification **before** finalizing.
7. Corrected the grounds, wrote the terminal record, returned `027-D` to
   `blocked`.

## The determination

| Triple | Outcome |
|---|---|
| `x86_64-unknown-linux-gnu` | **ESCALATED — R9 unresolved.** A, B, C delivered and A2 closed by `rustix::fs::openat2` with `BENEATH \| NO_MAGICLINKS \| NO_SYMLINKS \| NO_XDEV`. R9 has **partial final-state detection only** (`-EAGAIN`); the revert-before-completion window is **undetected**, so R9's detectable arm is unmet and the feasibility clause fires. Fail closed. |
| `x86_64-pc-windows-msvc` | **ESCALATED — Property B only.** No safe from-handle high-resolution identity accessor; the one that exists is low-res and panics instead of failing closed (R4). R9 is preventable in safe Rust. |
| `aarch64-apple-darwin` | **ESCALATED — R9 infeasible** (with A2). No prevention route, only partial window-bounded `kqueue` detection. **Escalation grounds are R9 and A2**, on independent evidence. A and C are separately **`CANDIDATE — UNVERIFIED`**: they would rest on `O_NOFOLLOW_ANY`, which locked `rustix 1.1.3` does not name — safe flag access, flag semantics, and an on-target probe all outstanding, and **C's containment mechanism unidentified**. B's accessor is settled; the episode binding its object is not. Not "R9 and A2 only". |

**No supported target is contract-feasible.** Escalation fires on all three. R9
is the failing property on two of the three — Linux and macOS — and Property B
on the third; the determination stays strictly per-triple because the two R9
grounds differ.

## Why it did not proceed to impl-plan

The lock requires the minimal API contract to be settled **inside the
deliberation, before `impl-plan` begins**, and requires escalation plus
fail-closed where a fixed property cannot be delivered on any single supported
target. R9 is unsettled for **Linux and macOS** and Property B for **Windows**,
so the precondition for entering `impl-plan` is unmet on every target.
Proceeding would have meant asserting a guarantee the mechanism does not
deliver — the root cause recorded for every prior G0 failure.

## Verification caught real defects in my own reasoning

Worth carrying forward: the first draft of the deliberation contained a **P0
over-claim** (Linux R9 recorded as "Delivered" when `-EAGAIN` is final-state
detection sharing the same out-and-back blind spot used to disqualify macOS) and
a **verified-false claim** (Windows `share_mode` described as unreachable through
`cap-std`, when `cap_fs_ext::OpenOptionsExt::share_mode` is publicly re-exported).
Two candidate mechanisms were also missed — macOS `O_NOFOLLOW_ANY` (later
established in cycle 2 as an **unverified candidate**, not a mechanism available
under the locked safe API) and `kqueue`/`EVFILT_VNODE`.
All were corrected before finalization. The escalation decision
itself survived both reviews unchanged.

**Lesson:** the over-claim pattern that killed three prior generations reappeared
in a document written specifically to avoid it, and was caught only by
independent adversarial verification. Any future generation should verify its
feasibility claims against vendored source and adversarial review before
promoting, not after.

## PR #401 review cycle 1 — the same over-claim, one step down

Four Copilot threads on PR #401 (one each against `027-D`, the closure record,
the deliberation, and this memory) raised **one** blocking finding: **Linux was
still labelled *feasible*** while the same text disclosed an **undetected**
revert-before-completion R9 window. **VALID and blocking.** The first correction
downgraded Linux R9 from "atomic" to "detected at final state", then rested
feasibility on R9's "detectable → fail closed" arm — but an **undetected**
in-scope relocation class means that arm is **unmet**, and the lock routes that
case to the feasibility clause, which covers atomicity. The residual argument
was a **risk-window-magnitude** comparison (Linux's window is narrower than
macOS's), and the lock provides no narrow-window arm.

Corrected in all four mirrors: Linux is **ESCALATED / R9 UNRESOLVED, fail
closed**; the escalation set is recomputed from two targets to **all three**;
the magnitude argument is **withdrawn** as contract-satisfying; settled item 1 is
narrowed to A/B/C/A2; R1b precondition 5 now fails on Linux as well as macOS; and
a **fifth** operator determination (Linux R9) is added. Linux's A/B/C/A2 evidence
is preserved intact — only the overall feasibility label is withdrawn.

**Lesson, sharpened:** correcting an over-claim's *wording* is not the same as
correcting its *conclusion*. The downgrade from "atomic" to "detected" was
accurate, and the feasibility verdict that survived it was not.

## PR #401 review cycle 2 — the same pattern, third target

Four further Copilot threads on PR #401 (one each against `027-D`, the closure
record, the deliberation, and this memory) raised **one** blocking finding:
**macOS A and C were recorded as *delivered* via `O_NOFOLLOW_ANY`** — and the
macOS gap narrowed to "**R9 and A2 only**" — while the same text disclosed that
the locked `rustix 1.1.3` does **not name** the flag, leaving safe access to it
**unverified**. **VALID and blocking.** A conclusion cannot be stronger than the
caveat printed next to it, and an unverified mechanism discharges no fixed
property. Three antecedents were unestablished: **safe flag access** (the raw
constant correct and the locked safe API propagating it unaltered under
`#![forbid(unsafe_code)]` — note the risk is *kernel-side*, since `bitflags`'
`from_bits_retain` retains unknown bits by construction), **flag semantics**
(that XNU delivers A's one-episode zero-name binding), and a **positive
on-target planted-symlink probe**. The probe matters most: Darwin's `open(2)`
ignores unrecognised `O_` bits, so a wrong or absent constant yields a
**successful, symlink-*following*** open that is indistinguishable in-band from
a correct one — no error to fail closed on, and invisible to source inspection.

**C turned out weaker than A.** `O_NOFOLLOW_ANY` refuses symlinks only and
supplies **no boundary containment**; the lexical no-`..` pre-check the draft
leaned on enters the episode **as a name**, which Amendment 2 excludes from
satisfying C by construction. **No containment mechanism was identified in the
locked graph, and the space beyond it was not surveyed** — phrased that way
because the document's own standard treats an unqualified negative over an
unsurveyed space as the same defect class as an unqualified positive. On the
evidence gathered C is **unsupplied**, not merely unverified — and the document
had already said, twenty lines away, that per-component symlink refusal is
necessary but not sufficient for C.

Corrected in all four mirrors: macOS **A and C** are **`CANDIDATE —
UNVERIFIED`** under the locked safe API and **not inheritable as evidence**;
macOS **B** is split so that the `fstat`-on-fd **accessor is settled** and
object-derived while the **episode** binding the object it reads is not (the
Property B primitive audit's "B is satisfiable on both Unix targets" is amended
to match); **R1b precondition 4**, which the lock scopes to **in-episode
containment**, now fails on **macOS C** (Windows Part B and macOS Part A fall
under the feasibility clause instead); the phrase "the macOS gap is R9 and A2
only" is
**withdrawn** wherever it appeared; settled item 7 now settles only that the
question is **open**; the `rustix` flag attribution is corrected
(`OFlags::RESOLVE_BENEATH` is FreeBSD-gated; Linux's beneath bit is
`ResolveFlags::BENEATH`, an `openat2` resolve bit); and the cycle-1 disposition
of the `O_NOFOLLOW_ANY` finding is marked **superseded**.

**A binding scope definition was added** for "gap"/"only" formulations, because
cycle 2's new *escalation grounds* vocabulary made the Linux and Windows
phrasing read as a whole-surface clearance. Defined, not withdrawn, with the
lock governing on conflict: "the *T* gap is *P* only" means solely that among
the **fixed properties of the amended input** — A, B, in-episode containment,
and R9 — the blocking ground on *T* is *P*. It **does not adjudicate A2**: the
summary table records A2 unclosed on **both** Windows and macOS while macOS
reads "(R9, A2)" and Windows does not, an asymmetry that **pre-dates cycle 2 and
is deliberately left unresolved** — flagged for a later generation, with no
position taken and no verdict changed. The residual ledger governs the rest, and
it governs uniformly (none declared closed by this deliberation; A2 closed
*only* on Linux) — no target's surface is settled, which is why all three
escalate. No target determination changed.

**The separation is the point.** macOS **stays ESCALATED on R9 and A2**, which
are unmet on evidence **independent** of the flag question. Verifying
`O_NOFOLLOW_ANY` would **not** clear the escalation, and the escalation firing
does **not** excuse recording A/C as settled. The unverified A/C candidate is a
**separate, additional** unsettled item — not a further escalation ground.

**No counts changed:** escalation still fires on all three targets, R9 is still
the failing property on two of three, five operator determinations still stand,
twelve residuals still carried, none closed.

**Lesson, third time:** the over-claim does not only reappear across
generations, it reappears across *targets within one document* — and survives
the very review cycle that corrects it elsewhere. Cycle 1 fixed Linux while
leaving the structurally identical macOS claim standing two sections away. Any
future correction of an over-claim must sweep **every** target and **every**
property for the same pattern, not just the one the reviewer pointed at.

## Authoritative state

* `027-D` → **`blocked`**. The 2026-09-17 operator-authorized `blocked` →
  `queued` transition is **consumed**.
* `028-D`–`034-D` **unchanged**, all `blocked`. No downstream package touched.
* Combined Package G, generation-1, and generation-2 circuits remain
  **OPEN/triggered**.
* No harvest, no backlog hierarchy, no shipment, no PR.
* No source, test, template, or configuration file modified.
* Deferred stash entries `1674E8DE` and `2A9C802B` not inspected, classified,
  triaged, harvested, or archived. Stash `4EF24729` untouched.
* Queued shipments `140-S`, `141-S`, `142-S` untouched.
* Pre-existing `.backlogit/stash.jsonl` line-ending dirt and untracked
  `.backlogit/checkpoints/checkpoint-20260914-045836.json` preserved unmodified.

## Next operator action

**Five** determinations are requested in the terminal record: the **Linux R9**
disposition, the macOS disposition, the Windows Part B authorization, the
Windows R9 authorization, and whether this escalation counts against the renewed
generation-3 failure bound. Any resumption requires a new recorded amendment
where a fixed property or the supported-target set changes, and in every case a
fresh explicit transition of `027-D` to `queued`.

## Artifacts

* `docs/decisions/2026-09-17-package-g0-generation-3-contained-test-filesystem-seam-deliberation.md`
* `docs/closure/2026-09-17-package-g0-generation-3-feasibility-escalation-record.md`
* this memory file
