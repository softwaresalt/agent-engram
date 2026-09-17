---
title: "Package G0 generation 3 — per-target-triple feasibility escalation record"
type: closure
doc_type: closure
date: 2026-09-17
agent: stage
verdict: ESCALATED
verdict_class: feasibility-escalation
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 3
attempt: 0
plan_review_attempt: 0
plan_reviewed: none
source_document: docs/decisions/2026-09-17-package-g0-generation-3-contained-test-filesystem-seam-deliberation.md
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
authority_amendments:
  - "Amendment 1 (2026-09-15), corrected by Correction 1 and Correction 2 (2026-09-16)"
  - "Amendment 2 (2026-09-17) — in-episode workspace containment, ratified"
branch: chore/stage-g0-generation-3-planning
base_commit: 332ff03a4be1987082b8356f62691b55d96cf49a
impl_plan_created: false
plan_hardening_performed: false
plan_review_performed: false
harvested: false
shipment_assembled: false
pull_request_created: false
operator_resolution: pending
---

## Verdict

**ESCALATED — per-target-triple feasibility clause.**

This is **not** a `PLAN_REVIEW_FAIL`. No implementation plan was produced, no
plan-review was run, and the generation-3 plan-review attempt counter stands at
**zero**. The generation-3 Stage operation terminated at the **deliberation**
stage, on the path the lock prescribes for exactly this outcome.

**No harvest. No backlog hierarchy. No shipment. No pull request.**

## Why the operation terminated here

The lock requires the **minimal API contract to be settled inside the
deliberation, before `impl-plan` begins**, and requires that where **no
available primitive can deliver a fixed property on any single supported
target**, the deliberation **escalate to the operator and fail closed for that
target**.

Feasibility **differs across the three supported target triples**:

| Target triple | Determination |
|---|---|
| `x86_64-unknown-linux-gnu` | **Feasible.** A, B, and C delivered by `rustix::fs::openat2` with `RESOLVE_BENEATH \| NO_MAGICLINKS \| NO_SYMLINKS \| NO_XDEV`, verified present and safe-API in the locked `rustix 1.1.3`. **R9 is detected at final state, not atomic** — `-EAGAIN` from the terminal `path_is_under()` check, with a disclosed revert-before-completion window. |
| `x86_64-pc-windows-msvc` | **Blocked on Property B only.** No safe from-handle **high-resolution** identity accessor exists in the locked graph; the one that does exist is low-resolution and panics rather than failing closed (R4). R9 is **preventable today in safe Rust** via `cap_fs_ext::OpenOptionsExt::share_mode`, so R9 is an authorization question, not a feasibility one. |
| `aarch64-apple-darwin` | **Infeasible for R9.** No beneath-resolution primitive and no rename-prevention mechanism; only partial, window-bounded detection via `kqueue`/`EVFILT_VNODE`. The traversal shape itself is deliverable via `O_NOFOLLOW_ANY`, so the gap is **R9 and A2 only**. |

Because Property B cannot be settled for Windows and R9 cannot be settled for
macOS, the lock's precondition for entering `impl-plan` — a settled minimal API
contract — is unmet. Proceeding would have required asserting a guarantee the
mechanism does not deliver, the single root cause recorded for every prior G0
failure.

## What was verified rather than assumed

Each disqualification is grounded in vendored source in the locked dependency
graph, not in documentation:

* **`file-id 0.2.3`** exposes `get_file_id`, `get_low_res_file_id`, and
  `get_high_res_file_id`, **all taking `impl AsRef<Path>`**. Its handle-taking
  `get_file_info_ex` is **private and `unsafe`**. Fails Part B provenance.
* **`same-file 1.0.6`** keys Windows identity on
  `BY_HANDLE_FILE_INFORMATION`'s `nFileIndex{Low,High}`. The crate's own source
  comments record that these are not guaranteed unique, that ReFS requires
  `FILE_ID_INFO`, and that the failure mode reports **distinct files as
  equivalent**. Its `Key` holds `{volume, index}` only — the size-based
  mitigation its comment describes is **not implemented**. Silent false-equal
  with no fail-closed path, contrary to **R4**.
* **`cap-std 4.0.2` / `cap-primitives 4.0.2`** try `open_beneath` with only
  `BENEATH | NO_MAGICLINKS`, **retry `EAGAIN` up to four times** rather than
  failing closed, and fall through to a manual per-component walk on `ENOSYS` or
  `EPERM`, exposing **no** mechanism indicator on the public API. It cannot
  carry a containment guarantee.
* **`cap_fs_ext::OpenOptionsExt::share_mode` IS publicly available** (re-exported
  at `cap-fs-ext/src/lib.rs:37`; declared and implemented at
  `cap-primitives/src/fs/open_options.rs:313` and `:428`), making Windows R9
  prevention reachable in safe Rust with the current lock.
* **`cap_fs_ext::MetadataExt::{dev, ino}`** is a safe from-handle Windows
  identity accessor, but is low-resolution and **panics** rather than failing
  closed — disqualified on R4 for the same reason as `same-file`.
* **`rustix 1.1.3`** exposes `openat2` as a safe function with the full
  `ResolveFlags` set, Linux-gated only. It names `RESOLVE_BENEATH` for FreeBSD
  and Linux but **no Darwin no-follow-any flag**, so macOS `O_NOFOLLOW_ANY`
  remains an unverified candidate rather than a settled mechanism.
* **`src/lib.rs:10`** declares `#![forbid(unsafe_code)]`, the binding reason the
  remaining Windows gap cannot be closed inside this crate.

## Residual ledger

All **twelve** residuals (**R1a, R1b, R2–R10, A2**), the **R2 × R5 × R7
composition**, and **R1b's five prevention preconditions** are carried forward
unmodified. **None is declared closed.**

**R1b preconditions 1 and 2 fail on *every* target** (R6 undecided everywhere;
the R8 per-target hazard set enumerated nowhere), **precondition 4 fails on
Windows Part B**, and **precondition 5 fails on macOS R9**. R1b prevention is
therefore **not established on any target**. **R10 is not discharged**; no plan
exists, so no decidability claim was made.

## Independent verification

The escalation determination was subjected to independent multi-persona,
multi-model adversarial verification before this record was written. Both
reviewers independently confirmed the **escalation decision is correct**, and
both found defects in the supporting grounds — including one **P0 over-claim**
(Linux R9 recorded as atomic rather than detected) and one **false claim**
(Windows `share_mode` described as unreachable). All findings were corrected in
the deliberation before finalization; the full finding table and dispositions are
recorded there under *Independent verification of this deliberation*.

That verification ran against a **deliberation, not a plan**. It consumed **no**
plan-review attempt and opened **no** correction budget.

## Correction budget

**NOT OPENED, and not applicable.** The correction budget attaches to
plan-review findings. No plan-review occurred. This record consumes no
correction round.

## Failure-bound position

The lock records, twice, that **whether a feasibility escalation counts against
the renewed generation-3 failure bound is not decided and requires an explicit
operator determination**. It also records that escalation may be the **expected**
rather than exceptional outcome where no atomic beneath-resolution primitive
exists on a supported target.

This operation reached escalation **without** a plan-review failure. Stage takes
**no position** on whether the renewed bound is consumed. That determination is
reserved to the operator.

## Authoritative state after this record

* `027-D` is returned to **`blocked`**. The operator-authorized `blocked` →
  `queued` transition of 2026-09-17 was **consumed** by this generation-3
  operation.
* `028-D` through `034-D` are **unchanged** and remain `blocked`. No downstream
  package was planned, deliberated, harvested, or re-sequenced.
* The combined Package G, generation-1, and generation-2 circuits remain
  **OPEN/triggered**. This record resets, reopens, and clears none of them.
* No source, test, template, or configuration file was modified.

## Operator determination requested

1. **`aarch64-apple-darwin`** — accept a hard fail-closed arm on a supported
   release target, remove macOS from G0's supported set, or accept
   `kqueue`-based partial, window-bounded R9 detection as sufficient. The macOS
   gap is **R9 and A2 only**; the traversal shape is deliverable.
2. **Windows Part B** — authorize a vetted dependency exposing `FILE_ID_INFO`
   from a handle, authorize an isolated `unsafe` boundary outside
   `#![forbid(unsafe_code)]`, or accept fail-closed on Windows. This is the
   **sole** blocking Windows gap.
3. **Windows R9** — an authorization question, not a feasibility one. Authorize
   applying `FILE_SHARE_DELETE` omission to **every intermediate directory of
   every corpus traversal**, which is materially more than accepting R6's
   disclosed single-boundary-handle side effect, or accept fail-closed.
4. **Failure bound** — determine whether this escalation counts against the
   renewed generation-3 bound.

Any resumption of G0 requires a **new recorded amendment** to the program lock
if the operator's determination changes a fixed property or the supported-target
set, and in every case requires a **fresh explicit transition of `027-D` to
`queued`**.

## Artifacts

| Artifact | Path | State |
|---|---|---|
| Generation-3 deliberation | `docs/decisions/2026-09-17-package-g0-generation-3-contained-test-filesystem-seam-deliberation.md` | escalated-feasibility |
| This terminal record | `docs/closure/2026-09-17-package-g0-generation-3-feasibility-escalation-record.md` | terminal |
| Session memory | `docs/memory/2026-09-17-stage-g0-generation-3-feasibility-escalation.md` | recorded |

No generation-1 or generation-2 artifact was edited. Stash `4EF24729`, PR #396,
all `143.*` artifacts, and the evidence branch are untouched.
