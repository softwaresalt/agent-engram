---
title: "Package G0 generation 3 — contained test-filesystem seam"
type: decision
doc_type: decision
date: 2026-09-17
agent: stage
decision_status: escalated-feasibility
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 3
attempt: 0
depth: deep
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
authority_amendments:
  - "Amendment 1 (2026-09-15), as corrected by Correction 1 (2026-09-16) and Correction 2 (2026-09-16)"
  - "Amendment 2 (2026-09-17) — in-episode workspace containment, ratified"
branch: chore/stage-g0-generation-3-planning
base_commit: 332ff03a4be1987082b8356f62691b55d96cf49a
supersedes: none
inherits_as_evidence:
  - "docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md"
  - "docs/decisions/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-deliberation.md"
  - "docs/exec-plans/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-plan.md"
promote_to: none
promotion_blocked_by: per-target-triple-feasibility-escalation
---

## Status and authority

This is a **new deliberation** opening Package G0 **generation 3**, a separate
operator-authorized work unit created under Amendment 1 of the program lock and
activated by the operator's explicit transition of `027-D` from `blocked` to
`queued` after PR #400 merged to `main` at `332ff03a`.

It is **not** a correction of any generation-1 or generation-2 artifact, and it
resets no circuit. The combined Package G, generation-1, and generation-2
circuits all remain **OPEN/triggered**. No generation-1 or generation-2
deliberation, plan, hardening section, or review record is edited by this
document. It carries a **fresh attempt counter starting at zero**.

**Outcome of this deliberation: the per-target-triple feasibility clause FIRES.**
The deliberation does **not** promote to `impl-plan`. The reasoning is recorded
in full below, and the terminal record is
`docs/closure/2026-09-17-package-g0-generation-3-feasibility-escalation-record.md`.

## The fixed input this deliberation had to realize

Three properties, all fixed, none sufficient alone. Restated from the lock as
amended; **the lock governs on any conflict**.

| Ref | Fixed property |
|---|---|
| **A** | **Same-object binding.** Each corpus read resolves its target by name in exactly **one resolution episode** producing one filesystem object. Both the read-time identity verification and **every byte** of content are performed against **that same already-obtained object**. After the verification point the read performs **zero** name-accepting filesystem operations across the transitive call graph. Verification succeeds before any content is released; on mismatch the read fails closed with no pathname-based fallback. |
| **B** | **Opaque per-file object-derived identity equality.** Read-time identity must equal the opaque stored authorized identity, compared for equality and never reconstructed, parsed, or re-derived. The identity must denote the **object**, not a name for it: both stored and read-time values are obtained **from the bound object**, with derivation symmetry. A canonicalized path string is a name and fails B regardless of type. |
| **C** | **In-episode workspace containment** (Amendment 2). The **same single resolution episode** that produces the bound object must itself enforce workspace containment, refusing to traverse outside the boundary **including the final component as resolved**, and **failing closed**. A separately-performed check whose result enters the episode **as a name** does not satisfy C. Binding a boundary reference is **not** the enforcement; the episode's own refusal to traverse outside it is. |

Episode counting is a **resolution count, not an atomicity claim** — the
distinction that makes **R9** decisive below.

## Supported target triples

"Supported platform" means **target triple**, not `cfg` family. Enumerated from
`.github/workflows/release.yml` and `.github/workflows/verify-release-assets.yml`:

| Target triple | Build/verify matrix |
|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` |
| `x86_64-pc-windows-msvc` | `windows-latest` |
| `aarch64-apple-darwin` | `macos-latest` |

`x86_64-apple-darwin` is **intentionally omitted** from the release matrix and is
therefore not a supported target for this analysis.

## Binding workspace constraint

`src/lib.rs` declares `#![forbid(unsafe_code)]` at crate root. Every candidate
primitive below is assessed **as reachable from safe Rust in this crate**. A
primitive that exists in the platform ABI but is reachable only through local
FFI is **not** available to this crate without an architectural change that this
deliberation has no authority to make unilaterally.

## Candidate primitive space, verified against vendored sources

Verified against the locked dependency graph, not against documentation alone.

| Crate | Version | Status | Relevance |
|---|---|---|---|
| `cap-std` / `cap-fs-ext` | `=4.0.2` | direct dependency | Capability-relative `Dir`, `open_dir_nofollow`, `FollowSymlinks::No` |
| `rustix` (`fs`) | `=1.1.3` | direct, `cfg(unix)` | `openat2` + `ResolveFlags`, `fstat` |
| `same-file` | `1.0.6` | transitive | `Handle::from_file`, opaque `Eq` |
| `file-id` | `0.2.3` | transitive | `FileId`, **path-based accessors only** |

**Existing in-tree prior art.** `src/shim/mod.rs` already performs
capability-relative no-follow directory traversal under `forbid(unsafe_code)`
(`open_or_create_subdir_nofollow` at `src/shim/mod.rs:462`,
`no_follow_diagnostics_dir` at `src/shim/mod.rs:486`). It establishes
the mechanism family is viable in this crate. It is **not** conformant with the
generation-3 fixed input — it seeds the walk from a name-based
`Path::canonicalize()` (R6/R8) and walks per component (R9) — and is cited as
**feasibility evidence only**, not as a design to inherit.

### Property B primitive audit

* **Unix targets.** `rustix::fs::fstat(fd)` and `std::os::unix::fs::MetadataExt`
  on `File::metadata()` yield `st_dev`/`st_ino` **from the open descriptor**.
  Object-derived, safe, stable. **B is satisfiable on both Unix targets.**
* **`file-id 0.2.3` is disqualified.** Its entire public accessor surface —
  `get_file_id`, `get_low_res_file_id`, `get_high_res_file_id` — takes
  `path: impl AsRef<Path>`. There is **no from-handle constructor**. A
  path-taking identity accessor fails Part B's provenance requirement outright,
  and calling it after verification would additionally violate Part A's
  zero-name-accepting-operations predicate.
* **`same-file 1.0.6` is disqualified on Windows.** `Handle::from_file` is
  correctly from-handle and `Handle` is opaque with derived `Eq`, which fits B's
  shape. Its Windows implementation, however, keys on
  `BY_HANDLE_FILE_INFORMATION`'s `nFileIndex{Low,High}`. The crate's own source
  comments state that these indices "are not always guaranteed to be unique"
  and that on ReFS uniqueness requires the distinct `FILE_ID_INFO` syscall,
  concluding that where the code is erroneous "two files will be reported as
  equivalent when they are in fact distinct." That is a **silent false-equal**,
  and R4 requires the mechanism to **fail closed where identity cannot be
  trusted**. `same-file` cannot detect the condition and does not fail closed.
  Its `PartialEq` additionally short-circuits on pointer identity and returns
  `false` when either key is absent, so its equality relation is not a pure
  object-identity relation.
* **A safe from-handle Windows accessor does exist, and it is still not
  sufficient.** `cap_fs_ext::MetadataExt::{dev, ino}` over
  `cap_primitives::fs::Metadata::from_file(&File)` is object-derived, safe, and
  already locked. It is nevertheless **disqualified on the same two grounds as
  `same-file`**: it is backed by `winapi_util::file::information`, i.e. the same
  **low-resolution** `BY_HANDLE_FILE_INFORMATION` keying with the ReFS
  non-uniqueness exposure, and it **panics** (`.expect(...)`) when the fields are
  absent rather than **failing closed**, contrary to **R4**.
* **Windows high-resolution identity** requires
  `GetFileInformationByHandleEx(FileIdInfo)` → 64-bit volume serial + 128-bit
  file ID. The Win32 primitive **exists**, and `file-id 0.2.3` calls it — but
  only behind its path-taking public functions; its handle-taking
  `get_file_info_ex` is **private** and `unsafe`. **No crate in the locked
  dependency graph exposes `FILE_ID_INFO` from a handle in safe Rust.** Reaching
  it requires either a new vetted dependency or local FFI, and local FFI is
  barred by `#![forbid(unsafe_code)]`.

## Per-target feasibility determination

### `x86_64-unknown-linux-gnu` — FEASIBLE, with R9 detected rather than atomic

`rustix::fs::openat2` is present in the locked `rustix 1.1.3` with
`ResolveFlags::{BENEATH, IN_ROOT, NO_MAGICLINKS, NO_SYMLINKS, NO_XDEV}`,
exposed as a **safe** function returning `OwnedFd`. It satisfies all three
properties in a single in-kernel resolution:

* **A** — one syscall, one resolution episode, one `OwnedFd`; identity and every
  content byte are taken from that descriptor.
* **B** — `fstat` on that descriptor yields `(st_dev, st_ino)` from the object.
* **C** — `RESOLVE_BENEATH` enforces containment **inside the kernel's own path
  walk**, which is the enforcement itself rather than a precheck whose result is
  carried in as a name. `RESOLVE_NO_MAGICLINKS` refuses synthetic-name
  redirection and `RESOLVE_NO_XDEV` refuses mountpoint traversal, which is the
  only place in this matrix where **A2** is additionally closed.
* **R9 — DETECTED AT FINAL STATE, NOT ATOMIC.** This is stated precisely
  because overstating it is the defect that terminated every prior attempt.
  There is **no kernel-wide rename-serialisation across a path walk**: `namei`
  takes per-dentry locks component by component (none at all in RCU-walk mode),
  and `rename_lock` is a seqlock consulted *inside* the terminal check, not held
  across the walk. What `RESOLVE_BENEATH` actually provides is a two-part
  taxonomy: `..`-escape, absolute jumps, magic links, and mount crossings are
  **refused during** the walk with `-EXDEV` (genuinely *prevented*), while a
  racing rename of an already-traversed intermediate directory is caught by the
  **terminal** `path_is_under()` check returning `-EAGAIN` (*detected at final
  state*). A rename that is **reverted before the walk completes** is
  **undetected** — structurally the same out-and-back blind spot used below to
  disqualify the macOS `..`-walk. Linux therefore qualifies under R9's
  **"detectable → fail closed"** arm, **not** under its "delivered/atomic" arm,
  and a residual revert-before-completion window is **disclosed, not closed**.

Failing closed on `-EAGAIN` rather than retrying (which the man page suggests) is
a **deliberate deviation** taken for security posture; its cost is that benign
concurrent renames produce spurious read failures.

Runtime availability is **not** implied by the triple: `openat2` requires kernel
≥ 5.6 and returns `ENOSYS` otherwise — and seccomp-filtered container runtimes
commonly return **`EPERM`** instead. The conformant disposition is to **fail
closed on any error** — `ENOSYS`, `EPERM`, `EXDEV`, `EAGAIN`, `ELOOP` — and take
**no fallback**. A per-component fallback would be precisely the prohibited
substitution of a weaker mechanism described as satisfying A and C. Because the
rule is "no fallback", the unenumerated-errno case is also safe; what narrows is
the **availability** envelope, not the guarantee.

> **`cap-std` is not a substitute here.** `cap-std` selects `openat2` when
> available and silently falls back to a per-component walk when it is not, and
> its public API exposes **no way to observe which mechanism ran**. A plan
> asserting kernel-enforced containment through `cap-std` would be asserting a
> guarantee it cannot establish — the exact over-claim pattern that terminated
> generations 1, 2, and 3-attempt-1. `rustix::fs::openat2` is called directly
> **because** its failure mode is observable.

### `x86_64-pc-windows-msvc` — BLOCKED ON PART B ONLY

Windows has **no** beneath-resolution primitive equivalent to `RESOLVE_BENEATH`.
The available shape is a per-component capability walk relative to a retained
directory handle, refusing reparse points at each component.

**R9 is preventable on Windows today, in safe Rust, with the locked dependency
set.** `cap_primitives::fs::OpenOptionsExt::share_mode` — publicly re-exported
as `cap_fs_ext::OpenOptionsExt`, from the direct dependency `cap-fs-ext 4.0.2` —
gives the caller control of `dwShareMode` on capability-relative opens, and its
own doc comment states that "to prevent race conditions on Windows, handles for
directories must be opened without `FILE_SHARE_DELETE`." Omitting
`FILE_SHARE_DELETE` on every intermediate directory handle causes any concurrent
rename or delete of those directories to fail with `ERROR_SHARING_VIOLATION`,
pinning the chain incrementally as the walk descends. Three qualifications are
recorded rather than assumed:

1. **Prevention is bounded to the held subtree.** Renaming an ancestor *above*
   the boundary relocates the whole subtree while every handle follows the
   object and every verdict stays true. That is **R6** boundary drift, and R6 is
   undecided — so this prevention is **conditional on R6**, not unconditional.
2. An attacker already holding a `DELETE`-access handle makes our open fail —
   fail-closed and benign, but an **availability** cost.
3. The interoperability cost scales with traversal depth × corpus breadth, and
   directory handles additionally require `FILE_FLAG_BACKUP_SEMANTICS`.

**The single blocking gap is Property B.** No safe from-handle **high-resolution**
identity accessor exists in the locked graph. The only safe from-handle accessor,
`cap_fs_ext::MetadataExt::{dev, ino}`, is low-resolution and **panics instead of
failing closed**, violating **R4**. Closing this requires a **new vetted
dependency** exposing `FILE_ID_INFO` from a handle, or an **isolated `unsafe`
boundary** — a change to the crate's declared safety posture. Stage may decide
neither unilaterally.

### `aarch64-apple-darwin` — INFEASIBLE FOR R9; ESCALATION TRIGGER

macOS provides **no** beneath-resolution primitive. There is no `openat2`;
`RESOLVE_BENEATH` is a Linux flag and `O_RESOLVE_BENEATH` is FreeBSD-only
(confirmed in `rustix`'s own `cfg(target_os = "freebsd")` gate).

**The traversal shape is better than a bare per-component walk, and this is
recorded so the operator's determination is not distorted.** macOS 11.0+
provides `O_NOFOLLOW_ANY`, under which a single `openat()` resolves a full
multi-component relative path in one in-kernel `namei` and fails if **any**
component — not merely the last — is a symlink. Because Apple Silicon has a hard
floor of macOS 11.0, it is **unconditionally available** on
`aarch64-apple-darwin`, with none of the runtime-availability caveat that
`openat2` carries on Linux. Combined with a lexical no-`..` constraint on a
caller-constructed relative path, it yields a genuine **single-episode contained
traversal**, satisfying A, B, and C's predicate.

One caveat is recorded rather than glossed: **`rustix 1.1.3` does not name
`O_NOFOLLOW_ANY`** — verified against its vendored `OFlags` definitions, which
name `RESOLVE_BENEATH` for FreeBSD and Linux but no Darwin no-follow-any flag.
Reaching it from safe Rust would rely on `OFlags::from_bits_retain` with an
unnamed raw constant, which is safe but unnamed-by-the-wrapper and would itself
need verification. It is therefore a **credible candidate**, not a settled
mechanism.

**What still fails is R9, and only R9 (with A2).** XNU's `namei` has no
`LOOKUP_IS_SCOPED` equivalent and no terminal `path_is_under()` re-check, so
mid-walk relocation of an already-traversed directory is **neither refused nor
detected**. Three detection routes were considered and each is rejected on the
merits rather than by absence:

* **`..`-walk revalidation** from the produced object using handle-relative
  `openat` is permitted by Correction 2's handle-based carve-out, but it is
  **detection with a re-entry race**: an attacker who relocates a directory out
  of the boundary and back before revalidation leaves the check true while the
  object was produced outside.
* **`rustix::fs::getpath` (`fcntl(F_GETPATH)`, Apple-gated, safe, already
  available)** returns an fd-derived path. It carries the **same re-entry race**
  and additionally yields a **name**, which Part B forbids as an identity basis.
* **`kqueue` + `EVFILT_VNODE`** with `NOTE_RENAME | NOTE_DELETE | NOTE_REVOKE`
  registered on the **already-held** intermediate descriptors is handle-based,
  consumes no name, is not a separate raceable pre-check, and — being
  event-based rather than state-based — does catch precisely the out-and-back
  sequence that defeats `..`-walk revalidation. It nevertheless leaves a
  residual window between `openat(component_i)` and `kevent()` registration for
  that descriptor, and says nothing about renames above the boundary. It is
  therefore **partial and window-bounded**, not reliable detection.
* **No rename-prevention route exists.** macOS has no mandatory share-mode
  equivalent, so the Windows `FILE_SHARE_DELETE` prevention has no macOS
  analogue.

R9's disposition is explicit: *must fail closed where the relocation is
detectable; where it is not — and by R9's own construction every per-step verdict
is true, so it generally is not — the deliberation **must escalate** under the
feasibility clause, which covers atomicity.*

**On `aarch64-apple-darwin` the relocation is neither preventable nor reliably
detectable.** The feasibility clause therefore fires. The disposition is stated
as *"partial, window-bounded, therefore not reliable"* and explicitly **not** as
*"no mechanism exists"* — an unqualified negative over an unsurveyed space is the
same defect class as an unqualified positive, which is why Correction 2 had to
split R1 in the first place.

## Feasibility determination summary

| Property | `x86_64-unknown-linux-gnu` | `x86_64-pc-windows-msvc` | `aarch64-apple-darwin` |
|---|---|---|---|
| A — same-object binding | **Delivered** (`openat2`) | Delivered (handle-relative walk) | Delivered (`O_NOFOLLOW_ANY` candidate) |
| B — object-derived identity | **Delivered** (`fstat` on fd) | **BLOCKED** — only safe from-handle accessor is low-resolution and panics instead of failing closed (R4) | **Delivered** (`fstat` on fd) |
| C — in-episode containment | **Delivered** (`RESOLVE_BENEATH`, `-EXDEV` during walk) | Predicate delivered; enforcement per-component | Predicate delivered (single-episode under `O_NOFOLLOW_ANY`) |
| R9 — atomicity | **Detected at final state, NOT atomic** — `-EAGAIN` fail-closed; revert-before-completion window undetected | **Preventable** via `FILE_SHARE_DELETE` omission (safe, locked); conditional on R6 | **INFEASIBLE — not preventable, only partial window-bounded detection** |
| A2 — mountpoint | Closed (`RESOLVE_NO_XDEV`) | Not closed | Not closed |

**Escalation fires on `x86_64-pc-windows-msvc` (Property B) and on
`aarch64-apple-darwin` (R9).** Feasibility therefore **differs across supported
targets**, which is the precondition the lock and the operator's standing
instruction both route to the escalation/fail-closed path.

**The Linux/macOS R9 difference is one of degree, not of kind**, and is recorded
as such: both end in a final-state containment check with the same out-and-back
blind spot. What differs is window magnitude — an in-kernel sub-microsecond
terminal check inside the syscall boundary versus a userspace multi-syscall
revalidation trivially winnable by a spinning attacker — and that the Linux check
precedes fd release. That is a defensible **risk-magnitude** argument and is not
presented as a categorical guarantee.

## What this deliberation settles

Recorded so a later authorized generation does not re-derive it.

1. **`rustix::fs::openat2` with `BENEATH | NO_MAGICLINKS | NO_SYMLINKS | NO_XDEV`
   is the conformant Linux primitive**, verified present and safe-API in the
   locked `rustix 1.1.3`. Fail closed on **any** error — `ENOSYS`, `EPERM`,
   `EXDEV`, `EAGAIN`, `ELOOP` — with **no fallback**. **R9 on Linux is
   `DETECTED (in-episode, final-state), NOT ATOMIC`**, with a disclosed
   revert-before-completion window. This qualifier is part of the settled item:
   a later generation inheriting item 1 inherits the qualifier, not an
   atomicity guarantee.
2. **`cap-std` must not be the vehicle for the containment claim**, because its
   mechanism selection is unobservable through its public API. Verified:
   `cap-primitives` tries `open_beneath` with only
   `BENEATH | NO_MAGICLINKS`, retries `EAGAIN` up to four times rather than
   failing closed, and falls through to a manual per-component walk on `ENOSYS`
   or `EPERM`, exposing no mechanism indicator to callers.
3. **`file-id 0.2.3` is disqualified** — every public accessor is path-taking
   and its handle-taking `get_file_info_ex` is private and `unsafe`; its
   `new_inode`/`new_low_res`/`new_high_res` constructors additionally
   reconstruct identity from raw integers, which is a parsing surface Part B
   forbids.
4. **`same-file 1.0.6` is disqualified on Windows** — low-resolution
   `nFileIndex` keying that the crate itself documents as non-unique on ReFS,
   with a silent false-equal failure mode and no fail-closed path, contrary
   to R4. Its `Key` contains `{volume, index}` only: the size-based mitigation
   its own comment describes is **not implemented**.
5. **Windows Part B requires `FILE_ID_INFO`**, which no crate in the locked
   graph exposes from a handle in safe Rust. The one safe from-handle accessor
   that does exist — `cap_fs_ext::MetadataExt::{dev, ino}` — is
   **low-resolution and panics rather than failing closed**, so it fails R4 for
   the same reason `same-file` does.
6. **R9 has no macOS *prevention* route** — no beneath-resolution primitive and
   no rename-prevention mechanism — and only **partial, window-bounded**
   detection via `kqueue`/`EVFILT_VNODE`. `..`-walk revalidation and
   `rustix::fs::getpath` were considered and rejected for a shared re-entry
   race, the latter additionally for yielding a name.
7. **macOS traversal shape is not the limiting factor.** `O_NOFOLLOW_ANY`
   (macOS 11+, unconditional on Apple Silicon) gives a single-episode contained
   traversal; it is **not named by `rustix 1.1.3`** and so remains a candidate
   requiring verification, not a settled mechanism. The macOS gap is **R9 and
   A2 only**.
8. **Windows R9 prevention is reachable today in safe Rust** via
   `cap_fs_ext::OpenOptionsExt::share_mode` omitting `FILE_SHARE_DELETE` on
   intermediate directory handles — conditional on R6, bounded to the held
   subtree, and carrying a real interoperability cost.
9. **The identity type shape that satisfies B** is an opaque newtype over a
   target-specific object key, constructed **only** from an open handle, with no
   public accessor and no parsing surface, compared solely by `Eq`. This is
   settled as a *shape*; no concrete Windows key can be named until (5)
   resolves.
10. **The `forbid(unsafe_code)` constraint is load-bearing**, not incidental: it
    is the binding reason the remaining Windows gap cannot be closed inside this
    crate.

## What remains undecided

Everything the lock left undecided remains undecided, and this deliberation adds
no decision it lacks authority to make: identity storage, the concrete Windows
identity primitive, R6 boundary establishment and revalidation, whether any
additional containment check is retained alongside in-episode enforcement, and
the crate/module location of the seam.

The **minimal API contract is therefore NOT settled**, because it cannot be
settled for two of three supported targets without operator determination. Under
the lock's requirement that the minimal API contract be settled inside the
deliberation **before `impl-plan` begins**, this deliberation **does not promote
to `impl-plan`**.

## Residual ledger carried forward

All **twelve** residuals are carried unmodified and none is declared closed by
this deliberation: **R1a, R1b, R2, R3, R4, R5, R6, R7, R8, R9, R10, A2**, plus
the **R2 × R5 × R7 composition**, plus **R1b's five prevention preconditions**
(R6 sound; R8 decidable; A2 inapplicable; a delivering primitive exists on the
target; R9 atomicity delivered, detectable, or escalated).

**R1b precondition 1 (R6 resolved soundly) is undecided on *every* target,
including Linux, and precondition 2 (R8 decidable, per-target hazard set
enumerated) is not discharged for any target** — the Linux R8 hazard set is not
enumerated, and `RESOLVE_NO_XDEV` refuses bind-mount traversal but does not
address overlayfs, which is a single mount and lands in R4 identity-trust.
**Precondition 4** additionally fails on Windows Part B and **precondition 5** on
macOS R9. Because R1b's preconditions are necessary, **R1b prevention is not
established on any target.**

**R10 is explicitly not discharged.** No plan exists, so no claim about the
decidability of the zero-name-accepting-operations count has been made. Any
future plan must still state *how* it makes that count decidable rather than
asserting an unestablishable zero. Relatedly, the sequencing of any `..`-walk
revalidation relative to the verification point is **not settled here**:
`openat(fd, "..")` is handle-relative but does accept the literal name `".."`,
so it is only unambiguously admissible when sequenced **before** verification,
where Part A's zero-count does not yet bind.

## Generation-2 settled items, as inherited

Items 1–5, 7, and 10 are inherited as evidence and are not re-litigated.
Items **6** and **8** are inherited **as narrowed by Correction 2**. Item **9**
is inherited **as narrowed by Correction 1** — a retained capability is required
**alongside** identity equality and never as a substitute for it. Inheritance
does not convert any settled item into an unexamined assumption; each was
re-checked against the amended fixed input before use, and item 6's withdrawn
conclusion is **not** relied upon anywhere above.

## Independent verification of this deliberation

Because the escalation determination is itself the deliverable, its technical
claims were put through **independent multi-persona, multi-model adversarial
verification before this document was finalized** — a Rust reviewer verifying
every crate and primitive claim against vendored source, and a security reviewer
auditing the escalation reasoning **in both directions** (is it over-claimed, and
is it under-claimed).

Both reviewers independently confirmed **the escalation decision is correct**.
Both also found defects in the **grounds**, which are corrected above rather than
carried:

| Finding | Severity | Disposition |
|---|---|---|
| Linux R9 recorded as "Delivered" — `-EAGAIN` is final-state **detection**, not atomicity, and shares the same out-and-back blind spot used to disqualify macOS | P0 | **Corrected.** Downgraded to `DETECTED, NOT ATOMIC` with the revert-before-completion window disclosed, and carried into settled item 1 |
| "The kernel's own rename-serialisation" — no such serialisation exists across a path walk | P1 | **Corrected.** False premise removed |
| Windows `share_mode` claimed unreachable through `cap-std` — it is publicly re-exported as `cap_fs_ext::OpenOptionsExt::share_mode` | P1 (false claim) | **Corrected.** Windows R9 reclassified from infeasible to authorization-gated |
| Windows Part B reason overstated as "no safe from-handle accessor" | P1 | **Corrected.** Reason restated as low-resolution + panics instead of failing closed |
| macOS `O_NOFOLLOW_ANY` (macOS 11+, unconditional on Apple Silicon) missed | P1 | **Corrected.** macOS gap narrowed to R9 and A2; flagged as unnamed by `rustix 1.1.3` |
| macOS `kqueue`/`EVFILT_VNODE` handle-based detection missed; "no way to detect" was an unqualified negative | P1 | **Corrected.** Recorded as partial and window-bounded, not absent |
| R1b preconditions 1 and 2 fail on **all** targets, not only the two escalating ones | P2 | **Corrected** in the residual ledger |
| `EPERM` missing from the fail-closed errno set (seccomp runtimes) | P2 | **Corrected** |
| `..`-walk admissibility under Part A's zero-name predicate asserted, not established | P2 | **Corrected.** Sequencing constraint now stated explicitly |
| `same-file`'s documented size-based mitigation is not implemented | P2 | **Incorporated** — strengthens the disqualification |

The corrected claims were re-verified directly against vendored source before
incorporation: `cap_fs_ext` re-exports `OpenOptionsExt` (`lib.rs:37`) and
`share_mode` is declared and implemented in `cap-primitives`
(`fs/open_options.rs:313`, `:428`); `rustix 1.1.3` names `RESOLVE_BENEATH` for
FreeBSD and Linux but **no** Darwin no-follow-any flag; and the in-tree prior art
is present at the line numbers cited above.

This verification did **not** run against an implementation plan, consumed **no**
plan-review attempt, and opened **no** correction budget.

## Operator determination requested

The escalation requires an explicit operator determination on four points. Stage
has made none of them.

1. **`aarch64-apple-darwin`** — accept a hard fail-closed arm on a supported
   release target, or remove macOS from G0's supported set, or accept
   `kqueue`-based partial, window-bounded R9 detection as sufficient. Note that
   the macOS gap is **R9 and A2 only**: the traversal shape itself is
   deliverable via `O_NOFOLLOW_ANY`.
2. **Windows Part B** — authorize a new vetted dependency exposing
   `FILE_ID_INFO` from a handle, or authorize an isolated `unsafe` boundary
   outside `#![forbid(unsafe_code)]`, or accept fail-closed on Windows. This is
   the **sole** blocking Windows gap.
3. **Windows R9** — this is **not** a feasibility question but an authorization
   one. Prevention is reachable today via `cap_fs_ext::OpenOptionsExt::share_mode`.
   The ask is to authorize a **deliberate, user-visible,
   interoperability-blocking mechanism applied to every intermediate directory
   of every corpus traversal** — materially more than "accept R6's disclosed
   side effect", which concerns one long-lived boundary handle.
4. **The failure bound** — the lock records that whether a feasibility
   escalation counts against the renewed generation-3 failure bound is **not
   decided** and requires explicit operator determination. This deliberation
   reached escalation **without** a plan-review failure, and takes no position
   on whether the bound is consumed.

Until these are determined, `027-D` returns to **`blocked`** and G0 halts.
