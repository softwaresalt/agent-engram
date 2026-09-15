---
title: "Package G0 generation 2 — plan-review record (attempt 1)"
type: closure
doc_type: closure
date: 2026-09-15
agent: stage
verdict: FAIL
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 2
plan_review_attempt: 1
plan_reviewed: docs/exec-plans/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-plan.md
source_document: docs/decisions/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-deliberation.md
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
branch: chore/stage-g0-generation-2-planning
harvested: false
shipment_assembled: false
---

## Verdict

**FAIL.** The plan does not reach `PLAN_REVIEW_PASS`. Under the program lock's
Program advancement contract item 4, `PLAN_REVIEW_PASS` requires **zero
unresolved P0 and zero unresolved P1**. This review closed with **2 P0** and
**15 P1** findings, of which **10 are architecture-level**.

**No harvest. No backlog hierarchy. No shipment. No PR.**

## Correction budget: NOT OPENED

Program lock, Program advancement contract item 8:

> **One mechanical correction and confirmation round per package, maximum.**
> Architecture-level P0/P1 findings block the package immediately; the
> correction budget does not open for them.

This review produced ten architecture-level P1 findings and one architecture-level
P0. The correction budget therefore **never opened**, and Stage performed **no**
correction round. Correcting and re-reviewing within this session would itself
be a policy violation.

## Review method

Multi-model adversarial panel, per compound learning
`workflow-issues/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md`.
Seven seats were planned; six were convened (the adjudicator seat was not needed
because the outcome was not close — see Consensus below).

| Persona | Model | Verdict |
|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | **FAIL** |
| Correctness Reviewer | `gpt-5.6-terra` | **FAIL** |
| Security Reviewer | `claude-sonnet-5` | **FAIL** |
| Constitution Reviewer | `grok-4.6` | **FAIL** |
| Maintainability Reviewer | `claude-opus-4.7` | **FAIL** |
| Scope Boundary Auditor | `gemini-3.8-flash` | **PASS** |

**5 FAIL / 1 PASS.**

## The terminal finding

**A1 — Res-1 is an over-claim. Four-persona, four-model consensus.**

The plan's Res-1 states the compare-to-read race is scoped to *"the leaf
component only. The ancestor chain is covered, because any ancestor redirect
changes the derived identity."*

This is **false**. `canonical_identity()` and the subsequent content read are two
independent, unsynchronized filesystem operations. The read re-walks the path
from scratch, so **any** ancestor component can be redirected in the window
between the identity comparison returning `equal` and the `open()` that follows
— exactly as undetectably as a leaf swap. The ancestor chain is covered only for
redirects completed **before** the read-time comparison, not for redirects inside
the compare-to-read window, which is the only window the residual is about.

Independently reported by Rust Reviewer (P1 ARCHITECTURE), Correctness Reviewer
(P1 ARCHITECTURE, twice, from two different angles), and Security Reviewer
(P2 ARCHITECTURE, twice).

**Why this is terminal.** The generation-1 memory records that every G0 failure
had one root cause: *"the plan asserted a guarantee its mechanism did not
deliver"*, and that attempt 3 died on *"a newly-introduced over-claim inside the
very hardening section written to disclose residuals honestly."* Generation 2
reproduced that pattern exactly — in the residual-disclosure section, in the
first review round. The Security Reviewer noted the aggravating factor: U7's
acceptance criterion directs this residual into `read_corpus` rustdoc "verbatim
in meaning", so the inaccurate scope would have **shipped as the documented
guarantee that G1 and G2 read as ground truth**.

## Findings

### P0

| ID | Persona | Class | Finding |
|---|---|---|---|
| **P0-1** | Maintainability | ARCHITECTURE | `read_corpus`'s return type is never specified — only its parameters and its failure branch. It is on the `pub` surface G1 is **forbidden to redesign**, so whatever the implementer picks at U7 silently becomes the frozen G1 contract. |
| **P0-2** | Constitution | MECHANICAL | Hardening gate **H5**'s diff allow-list makes the package **undeclarable**. `src/services/mod.rs` enumerates every child module, so U1 cannot compile without adding `pub mod testfs;` — but H5 forbids touching any file outside the three listed. Follow H5 → orphan module, no build. Declare the module → H5/INV-6 fail. **Independently verified against `src/services/mod.rs`.** |

### P1 — architecture-level (block immediately; no correction budget)

| ID | Persona(s) | Finding |
|---|---|---|
| **A1** | Rust, Correctness ×2, Security ×2 | **Res-1 over-claim** — the compare-to-read window spans the entire resolved path, not the leaf. *Terminal finding; see above.* |
| **A2** | Correctness | Canonical **pathname** equality is not object identity. A mount/bind-mount swapped at the same mountpoint leaves the canonical path unchanged while traversal reads a different tree. A symlink loop (`ws/a -> ws/z`, `ws/z -> ws/a`) yields a canonicalization **error**, not `IdentityMismatch`. D4's "any ancestor redirect changes the identity" is therefore too strong. |
| **A3** | Correctness | **U9 cannot be the primary proof of S1.** `FakeFs::canonical_identity` is specified *by this same plan*, so the hermetic test proves only that the fake behaves as written — not that `std::fs::canonicalize` rejects real symlinks, Windows junctions, or reparse points. The only test exercising the real mechanism (U10) is explicitly secondary and skippable. |
| **A4** | Constitution, Rust | **H4/P8's clippy gate is unearned.** `src/lib.rs:12-13` carries `#![allow(clippy::missing_errors_doc)]` and `#![allow(clippy::missing_panics_doc)]`, so `cargo lint` will **not** enforce the rustdoc sections the plan claims it enforces — including the gen-1 M-2 closure. **Independently verified.** An unearned claim *inside the hardening section*, compounding A1. |
| **A5** | Maintainability | `FakeFs` and the fault-injection surface are unconditionally `pub`, not gated behind `#[cfg(test)]` or a `test-support` feature — test scaffolding reachable from production paths and every downstream crate consumer. |
| **A6** | Maintainability | `ResolvedCorpus` exposes **no enumeration accessor**. G1 must either force a read to enumerate (coupling enumeration to the read-time authority) or petition to reopen G0 — which the lock forbids without a new authorization. |
| **A7** | Maintainability | P6 duplicates platform-prefix normalization into the seam with **no drift-detection test** against `src/db/workspace.rs::normalize_canonical`. Drift produces spurious `IdentityMismatch` on Windows verbatim inputs. |
| **A8** | Maintainability | Placing a **test** seam under `src/services/` purely to avoid a coverage-manifest edit is tooling driving semantics; the plan treats the location as final rather than as known-temporary. |
| **A9** | Rust | Non-UTF-8 corpus entries have **no defined lossless representation**: `read_dir` yields `OsString` and identity holds `PathBuf`, but `ResolvedFile::rel_path` is a `String`. `to_string_lossy` would break reconstruction and can alias. The plan's "non-UTF-8 covered" risk row is unearned. |
| **A10** | Maintainability | `FsOp` is `pub` for fault-injection keying, inviting G1 to couple assertion logic to G0's operation taxonomy. *(reported P2; grouped here as surface-contract scope)* |

### P1 — mechanical

| ID | Persona | Finding |
|---|---|---|
| **M1** | Correctness, Constitution | U9's red→green story is **impossible as ordered**: U9 depends on U7, so the comparison already exists when U9 is written. A test cannot be observed in the specified behavioral red state before it exists. |
| **M2** | Constitution | U3 (ten trait methods + `RealFs`) and U4 (constructor, builders, link resolution, second ten-method impl) each far exceed the `<5 functions` granularity heuristic. D8's "each within the 2-hour rule" has no mechanism. |
| **M3** | Constitution | U11 is **11 test scenarios** against a `<4` cap. |
| **M4** | Constitution | **No `Constitution Check` section**, which the workspace constitution requires of every implementation plan. |
| **M5** | Constitution | U1 demands 14 `FsError` variants and 10 `FsOp` discriminants but names only the 2 new ones; the inherited 13 + 9 (and U11's ten call sites) live only on the gen-1 evidence branch. As written U1 is research, not 2-hour work. |
| **M6** | Rust | `RealFs` is absent from D7's exhaustive `pub` list, yet U10 is an external `tests/` target that exercises it — inaccessible as specified. |

### P2 and advisories

Recorded but not adjudicated, because the package is blocked on P0/P1: U2's
four-scenario cap (Constitution P2); D8's bundling claim vs the U6/U7 split
(Constitution P2); U8's missing `compile_fail` red/green exemption (Constitution
P2); U8 unit-count inflation (Maintainability P2); H1's over-broad `contains`
token ban (Rust/Constitution advisory); `assert_eq!` requiring `Debug` on a
deliberately non-`Debug` type (Rust advisory); ceremony/duplication in the
closure sections (Maintainability P2); Requirements-trace gaps for S3, S5, D1,
D6, U4, U5 (Correctness P2, Scope advisories 1 and 4); U10's tests+config width
touch (Scope advisory 2); U11 table-driven framing (Scope advisory 3).

## What this review SETTLES — inheritable if G0 is ever restaged

Recorded so a future authorized generation does not re-litigate them.

1. **The oracle-manifest avoidance claim is CORRECT and independently verified.**
   The Scope Boundary Auditor verified against `.cargo/test-coverage-manifest.toml`
   and both `scripts/test-coverage-oracle.{ps1,sh}` that (a) `src/services/` is a
   declared surface, (b) its `targets` glob includes `unit_*`, (c) adding a
   `unit_*` target cannot fail `completeness` mode, and (d) the module-completeness
   check inspects only **immediate children** of `src/`, so a nested module adds no
   top-level surface. **Adding `unit_testfs_identity` requires zero manifest edits
   and leaks no G2 scope.** (Note: this settles the *tooling* claim only; A8
   separately challenges the *placement* on maintainability grounds.)
2. **No scope leakage into G1, G2, B, C, A, D, or E.** No unit performs assertion
   evaluation, registry seeding, real-root binding, or CI/oracle registration.
   Scope Boundary Auditor: PASS.
3. **No prohibited artifact was touched.** PR #396, `143.*`, the evidence branch,
   every generation-1 artifact, and stash `4EF24729` are unmodified. Verified by
   the Scope Boundary Auditor and by the session's own diff.
4. **The declared Rust derive chains are valid.** `PathBuf` satisfies
   `Clone + PartialEq + Eq`; fieldless `FsOp` supports `Ord` and `(FsOp, PathBuf)`
   is a valid `BTreeMap` key; `IdentityMismatch { rel_path: String }` preserves
   `Debug + Clone + PartialEq + Eq + thiserror::Error`; `FsError` does **not**
   contain `CanonicalIdentity`. (Rust Reviewer, advisory.)
5. **The trait is object-safe and the interior-mutability choice is sound.**
   `canonical_identity(&self, &Path) -> Result<CanonicalIdentity, FsError>` and
   `read_dir(&self, ..) -> Result<Vec<OsString>, FsError>` are object-safe; no
   declared method is generic or returns `Self`. `RefCell` makes `FakeFs`
   non-`Sync`, but neither `TestFs` nor `&dyn TestFs` declares `Send`/`Sync`.
   (Rust Reviewer, advisory.)
6. **The resolve-time-only containment demotion is SECURITY-SOUND.** Identity
   equality at read time is strictly stronger than containment: the stored value
   was already proven in-workspace at resolve time, so requiring exact equality
   transitively re-affirms containment without re-checking it. Nothing of
   security value is lost by removing containment from the read path.
   (Security Reviewer, Q3 — no finding.)
7. **The disclosure mechanism is sound and honestly scoped.** A private field
   with no accessor means no code outside `identity.rs` can reach the `PathBuf`
   to print it — so omitting the `Debug` derive converts a would-be silent leak
   into a **compile error**. (Security Reviewer, Q1 — no finding.) Note
   Maintainability's competing P1 recommendation: a manual `Debug` rendering a
   constant opaque token (the `secrecy::Secret` pattern) preserves non-disclosure
   equally while removing the transitive boilerplate tax. Both reviewers agree the
   invariant holds; they disagree on ergonomics.
8. **Using a symlink-following primitive as the identity basis is a reasoned,
   disclosed trade-off**, not a regression of settled item 9 ("no code path
   implicitly follows a symlink"), which governs metadata typing during
   enumeration. (Security Reviewer, Q2 — no finding.)
9. **Option B (retained capability handle / `CapRoot` shape) is correctly
   rejected** because it cannot satisfy the fixed "stored, compared-by-equality"
   invariant. (Security Reviewer, Q2.) **However**, A1/A2/A3 collectively indicate
   that the *fixed invariant itself* may be unable to deliver the root-cause
   rejection at read time without a handle or anchor — see Escalation below.
10. **The threat-model calibration is correct**: test-only infrastructure, no
    production reachability, and deferring Option C as unnecessary machinery for
    a non-production oracle is appropriate. (Security Reviewer, Q5.)

## Escalation to the operator

Program lock, § *If generation 2 also fails*:

> **If G0 generation 2 fails its plan-review gate, the program halts.** No
> generation 3 may be created, and no downstream package may be re-sequenced
> around G0, without a further explicit operator authorization recorded as an
> amendment to this document. Stage escalates to the operator and stops. Silent
> re-attempt is prohibited.

**The program is HALTED.** Stage stops here.

### The decision the operator must make

Findings A1, A2, and A3 converge on one substantive question that Stage has **no
authority to answer**, because it concerns the operator-fixed invariant itself:

> Can a **stored-value-compared-for-equality** identity mechanism — with no
> retained handle or anchor — actually reject ancestor substitution **at read
> time**, given that the content read re-traverses the whole path afterward?

The panel's consensus is **no, not completely**:

* It rejects substitutions completed **before** the comparison (A1 confirms this
  much *is* delivered — the core of D4 is sound).
* It cannot reject substitutions inside the **compare-to-read window** (A1).
* It cannot detect mountpoint substitution at all, because canonical *pathnames*
  are names, not object identities (A2).

Option C (retained root anchor + per-file identity equality), which this
deliberation deferred as "not minimal", is the mechanism the panel repeatedly
gestured at. Adopting it would require amending the lock's "Deliberately
undecided" framing, because it changes what the *minimal* contract is.

**Three routes, for the operator to choose between:**

| Route | What it means | Requires |
|---|---|---|
| **1. Narrow the claim** | Keep Option A exactly as designed, but scope S1 and Res-1 honestly to "substitutions completed before read-time identity derivation". The package then delivers less than the root-cause evidence demands | New authorization; no lock amendment |
| **2. Amend to Option C** | Adopt retained anchor + identity equality so the read cannot re-traverse mutable ancestors | New authorization **and** a recorded amendment to the lock's G0 fixed input |
| **3. Halt G0 permanently** | Re-sequence or abandon the program | A recorded amendment |

Stage recommends the operator treat **Route 2** as most likely to satisfy the
root-cause evidence, but makes **no** decision and takes **no** further action.

## Artifacts

| Path | Status |
|---|---|
| `docs/decisions/2026-09-15-...-deliberation.md` | Preserved, `accepted` (the deliberation gate passed; the **plan** gate failed) |
| `docs/exec-plans/2026-09-15-...-plan.md` | `plan_status: blocked`, attempt counter 1 |
| This record | Terminal artifact for generation 2 attempt 1 |

Generation-1 artifacts, PR #396, `143.*`, the evidence branch, and stash
`4EF24729` were read-only throughout and are unmodified.
