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
  Object-derived, safe, stable. **B's accessor is settled on both Unix
  targets.** On `aarch64-apple-darwin` that settles the **accessor only**: the
  accessor is object-derived and requires no further verification, but the
  **episode that produces the object it is applied to** rests on the unverified
  `O_NOFOLLOW_ANY` candidate, so macOS B is **not independently settled
  end-to-end** (PR #401 review cycle 2).
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

### `x86_64-unknown-linux-gnu` — A, B, C, A2 DELIVERED; R9 UNRESOLVED → ESCALATION

`rustix::fs::openat2` is present in the locked `rustix 1.1.3` with
`ResolveFlags::{BENEATH, IN_ROOT, NO_MAGICLINKS, NO_SYMLINKS, NO_XDEV}`,
exposed as a **safe** function returning `OwnedFd`. It satisfies the three
lettered properties in a single in-kernel resolution — and does **not** satisfy
the fixed atomicity property **R9**:

* **A** — one syscall, one resolution episode, one `OwnedFd`; identity and every
  content byte are taken from that descriptor.
* **B** — `fstat` on that descriptor yields `(st_dev, st_ino)` from the object.
* **C** — `RESOLVE_BENEATH` enforces containment **inside the kernel's own path
  walk**, which is the enforcement itself rather than a precheck whose result is
  carried in as a name. `RESOLVE_NO_MAGICLINKS` refuses synthetic-name
  redirection and `RESOLVE_NO_XDEV` refuses mountpoint traversal, which is the
  only place in this matrix where **A2** is additionally closed.
* **R9 — UNRESOLVED; PARTIAL FINAL-STATE DETECTION ONLY; ESCALATES.** This is
  stated precisely because overstating it is the defect that terminated every
  prior attempt.
  There is **no kernel-wide rename-serialisation across a path walk**: `namei`
  takes per-dentry locks component by component (none at all in RCU-walk mode),
  and `rename_lock` is a seqlock consulted *inside* the terminal check, not held
  across the walk. What the **flag set** actually provides is a two-part
  taxonomy: `..`-escape, absolute jumps, and magic links are **refused during**
  the walk (`-EXDEV` for the escaping jumps, `-ELOOP` for magic links), and
  mount crossings are refused during the walk by `RESOLVE_NO_XDEV` rather than
  by `RESOLVE_BENEATH` — all genuinely *prevented* — while a racing rename of an
  already-traversed intermediate directory is caught by the
  **terminal** `path_is_under()` check returning `-EAGAIN` (*detected at final
  state*). A rename that is **reverted before the walk completes** is
  **undetected** — structurally the same out-and-back blind spot used below to
  disqualify the macOS `..`-walk. R9 admits exactly two arms: **fail closed
  where the relocation is detectable**, and otherwise **escalate under the
  feasibility clause, which covers atomicity**. The in-scope relocation class is
  **not wholly detectable on Linux**: in the revert-before-completion case every
  per-step verdict is true *and* the terminal `path_is_under()` check is true,
  which is exactly the condition R9 routes to escalation. Linux therefore
  satisfies **neither** the "delivered/atomic" arm **nor** the "detectable →
  fail closed" arm for the in-scope class, and **falls to R9's escalation arm**.
  The `-EAGAIN` behaviour is retained as **partial** detection that narrows but
  does not discharge R9; the revert-before-completion window is **undetected and
  not closed**.

**Determination for `x86_64-unknown-linux-gnu`: NOT CONTRACT-FEASIBLE — R9
UNRESOLVED, ESCALATED, FAIL CLOSED.** Properties **A**, **B**, and **C** are
delivered and **A2** is closed, and that evidence stands unchanged and
inheritable. But the amended input fixes **containment atomic with the
production of the bound object** — the property the ledger tracks as **R9** —
the per-target-triple feasibility clause covers **every** fixed property
**including atomicity**, and delivering A/B/C on a target does **not** discharge
R9 on it.
Linux joins Windows and macOS in the escalation set. *(Corrected in the PR #401
review cycle 1: this target was previously recorded as "FEASIBLE" on the
strength of the detectable arm, which the undetected revert-before-completion
window does not support.)*

Failing closed on `-EAGAIN` rather than retrying (which the man page suggests) is
a **deliberate deviation** taken for security posture; its cost is that benign
concurrent renames produce spurious read failures. It is a hardening measure
within an escalated target, **not** a discharge of R9.

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

### `x86_64-pc-windows-msvc` — ESCALATED; BLOCKED ON PART B ONLY

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

### `aarch64-apple-darwin` — ESCALATED; INFEASIBLE FOR R9

macOS provides **no** beneath-resolution primitive. There is no `openat2`;
`RESOLVE_BENEATH` is a Linux flag and `O_RESOLVE_BENEATH` is FreeBSD-only
(confirmed in `rustix`'s own `cfg(target_os = "freebsd")` gate).

**The traversal shape is *potentially* better than a bare per-component walk,
and this is recorded so the operator's determination is not distorted — in
either direction.** macOS 11.0+ provides `O_NOFOLLOW_ANY`, under which a single
`openat()` resolves a full multi-component relative path in one in-kernel
`namei` and fails if **any** component — not merely the last — is a symlink.
Because Apple Silicon has a hard floor of macOS 11.0, the *platform* flag is
unconditionally present on `aarch64-apple-darwin`, with none of the
runtime-availability caveat that `openat2` carries on Linux. That is the whole
of what the flag supplies: **symlink refusal on every component, and nothing
else.** It is **not** a beneath-resolution primitive and supplies **no**
boundary containment of its own, so it does **not** by itself yield the
single-episode contained traversal **C** requires. Recorded as a **conditional
candidate, not a delivery.**

One caveat is recorded rather than glossed, and it is **load-bearing rather than
cosmetic**: **`rustix 1.1.3` does not name `O_NOFOLLOW_ANY`** — verified against
its vendored flag definitions. `OFlags` names `RESOLVE_BENEATH` only under
`cfg(target_os = "freebsd")` (that is FreeBSD's `O_RESOLVE_BENEATH`), while
Linux's beneath bit is `ResolveFlags::BENEATH`, an `openat2` *resolve*-argument
bit rather than an `O_*` bit; **no Darwin no-follow-any flag is named under any
spelling, in `OFlags` or elsewhere in the `fs` surface**. Reaching it from safe
Rust would rely on
`OFlags::from_bits_retain` with an unnamed raw constant, which is safe but
unnamed-by-the-wrapper. **Three distinct things are therefore unverified, and
none is established anywhere in this deliberation:**

1. **Safe flag access** — that the raw constant is correct for this target and
   that the locked safe API propagates it **unaltered** to the underlying
   `openat()` under `#![forbid(unsafe_code)]`. Note precisely where the risk is
   **not**: `bitflags`' `from_bits_retain` retains unknown bits **by
   construction**, so the wrapper is not the fail-open path.
2. **Flag semantics, and the containment mechanism the flag does not supply** —
   that XNU's actual behaviour delivers **A**'s one-episode, zero-name binding,
   *and* that some mechanism delivers **C**'s requirement that the episode
   **itself** refuse to traverse outside the boundary, **including the final
   component as resolved**, and fail closed. Symlink refusal on every component
   is **necessary but not sufficient** for C. A lexical no-`..` constraint on a
   caller-constructed relative path does **not** close the gap: it is a
   **separately-performed check whose result enters the episode as a name**,
   which Amendment 2 excludes from satisfying C by construction. **No macOS
   in-episode containment mechanism was identified in the locked dependency
   graph, and the primitive space beyond that graph was not surveyed** — stated
   that way deliberately, since an unqualified negative over an unsurveyed space
   is the same defect class as an unqualified positive. On the evidence
   gathered, C's antecedent is therefore not merely unverified but
   **unsupplied**.
3. **Positive on-target behavioural confirmation** — because Darwin's `open(2)`
   does **not** reject unrecognised `O_` bits, an incorrect or absent constant
   yields a **successful, symlink-*following*** open that is **indistinguishable
   in-band** from a correct no-follow-any open. There is no error to fail closed
   on, so source and API inspection **cannot** detect this failure mode. Only a
   positive probe — an open through a **planted symlink** under the flag that
   **must** fail, executed on-target — can establish (1) and (2). No such probe
   has been run.

It is therefore an **UNVERIFIED CANDIDATE**, not a settled mechanism, and
**nothing in this deliberation rests on it.** **A and C on macOS are
`CANDIDATE — UNVERIFIED`**: not delivered, not settled, and **not inheritable as
evidence** by a later authorized generation. **C is the weaker of the two** — it
lacks an identified mechanism at all, not merely a verified one. Neither A nor C
individually, nor A/B/C as a package, may be stated as satisfied for macOS until
all three items above are discharged.

**R9 and A2 fail on grounds that are independent of that candidate, and keeping
the two separate is what the operator's determination turns on.** R9 and A2 are
unmet for the reasons given below, and would remain unmet even if
`O_NOFOLLOW_ANY` were fully verified tomorrow. The A/C candidate is a
**separate, additional** unsettled item — neither a substitute ground for the
R9/A2 escalation nor a further escalation trigger of its own. The earlier
reading that the macOS gap is "**R9 and A2 only**" is **withdrawn** (PR #401
review cycle 2); the escalation grounds are R9 and A2, but the macOS surface is
**not** otherwise settled. XNU's `namei` has no `LOOKUP_IS_SCOPED` equivalent
and no terminal `path_is_under()` re-check, so mid-walk relocation of an
already-traversed directory is **neither refused nor detected**. Three detection routes were considered and each is rejected on the
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
| A — same-object binding | **Delivered** (`openat2`) | Delivered (handle-relative walk) | **CANDIDATE — UNVERIFIED** (rests on `O_NOFOLLOW_ANY`, unnamed by locked `rustix 1.1.3`; safe access, semantics, and on-target probe all outstanding) |
| B — object-derived identity | **Delivered** (`fstat` on fd) | **BLOCKED** — only safe from-handle accessor is low-resolution and panics instead of failing closed (R4) | **Accessor settled** (`fstat` on fd, object-derived), but the episode binding the object it reads is the **unverified** A candidate — **not independently settled end-to-end** |
| C — in-episode containment | **Delivered** (`RESOLVE_BENEATH`, `-EXDEV` during walk) | Predicate delivered; enforcement per-component | **CANDIDATE — UNVERIFIED, and no mechanism identified** — `O_NOFOLLOW_ANY` refuses symlinks only and supplies no boundary containment; a lexical no-`..` pre-check enters the episode as a **name**, which Amendment 2 excludes. None found in the locked graph; space beyond it unsurveyed |
| R9 — atomicity | **UNRESOLVED → ESCALATED** — partial final-state detection only (`-EAGAIN` fail-closed); revert-before-completion window undetected, so the "detectable → fail closed" arm is unmet | **Preventable** via `FILE_SHARE_DELETE` omission (safe, locked); conditional on R6 | **INFEASIBLE — not preventable, only partial window-bounded detection** |
| A2 — mountpoint | Closed (`RESOLVE_NO_XDEV`) | Not closed | Not closed |
| **Overall contract feasibility** | **ESCALATED (R9)** | **ESCALATED (Property B)** | **ESCALATED (R9, A2)** — and A/C additionally **unverified candidates** |

**Reading the macOS column.** The escalation grounds for `aarch64-apple-darwin`
are **R9 and A2**, which are unmet on evidence independent of any
`O_NOFOLLOW_ANY` question. **A and C are separately `CANDIDATE — UNVERIFIED`**:
they are not additional escalation grounds, and they are equally not
deliverables. Verifying `O_NOFOLLOW_ANY` would *not* clear the escalation, and
the escalation firing on R9/A2 does *not* excuse recording A/C as settled. The
two must be read apart (PR #401 review cycle 2).

**Escalation fires on all three supported target triples** —
`x86_64-unknown-linux-gnu` on **R9**, `x86_64-pc-windows-msvc` on **Property B**,
and `aarch64-apple-darwin` on **R9** (with **A2**). **No supported target is
contract-feasible**, which is the precondition the lock and the operator's
standing instruction both route to the escalation/fail-closed path. The
escalation is therefore **universal across the supported set**, not a per-target
divergence. **R9 is the failing property on two of the three targets** — Linux
and macOS — and Property B on the third; the determination nonetheless remains
strictly **per-triple**, because the R9 grounds differ (partial in-kernel
final-state detection versus no prevention route and only partial
window-bounded detection) and delivering a property on one supported target does
not discharge it on another.

**The Linux/macOS R9 difference is one of degree, not of kind — and degree does
not satisfy the contract.** Both end in a final-state containment check with the
same out-and-back blind spot. The window magnitudes differ — an in-kernel
sub-microsecond terminal check inside the syscall boundary versus a userspace
multi-syscall revalidation trivially winnable by a spinning attacker — and on
Linux the check precedes fd release. **That is a risk-magnitude observation and
is expressly not a contract-satisfaction argument.** R9 admits no
narrow-window arm: either the in-scope relocation class is detectable, in which
case the read fails closed, or it is not, in which case the deliberation
escalates. Because the revert-before-completion case is undetected on **both**
targets, **both route to escalation**. Any earlier reading of this paragraph as
grounds for treating Linux as feasible is **withdrawn** (PR #401 review
cycle 1).

## What this deliberation settles

Recorded so a later authorized generation does not re-derive it.

1. **`rustix::fs::openat2` with `BENEATH | NO_MAGICLINKS | NO_SYMLINKS | NO_XDEV`
   is the conformant Linux primitive**, verified present and safe-API in the
   locked `rustix 1.1.3`. Fail closed on **any** error — `ENOSYS`, `EPERM`,
   `EXDEV`, `EAGAIN`, `ELOOP` — with **no fallback**. Item 1 settles this
   primitive for **A, B, C, and A2 only**. **R9 on Linux is `UNRESOLVED —
   PARTIAL FINAL-STATE DETECTION ONLY, ESCALATED, FAIL CLOSED`**, with an
   undetected revert-before-completion window. This qualifier is part of the
   settled item: a later generation inheriting item 1 inherits an **escalated,
   undischarged R9** — not an atomicity guarantee, and not a satisfied
   "detectable → fail closed" arm.
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
7. **macOS traversal shape is *unsettled*, and is not recorded as the limiting
   factor either way.** `O_NOFOLLOW_ANY` (macOS 11+, unconditional on Apple
   Silicon) supplies **symlink refusal on every component and nothing else** —
   it is not a beneath-resolution primitive and supplies **no** boundary
   containment. It is **not named by `rustix 1.1.3`**, leaving three things
   unverified: **safe flag access** under the locked API, **flag semantics plus
   the in-episode containment mechanism C needs and the flag does not supply**,
   and a **positive on-target planted-symlink probe** (necessary because Darwin
   `open(2)` ignores unrecognised `O_` bits, so a wrong or absent constant
   yields a **successful, symlink-*following*** open, indistinguishable in-band
   from a correct one, with no error to fail closed on). **A and C on macOS are
   therefore `CANDIDATE —
   UNVERIFIED`, not delivered and not inheritable as evidence**, with **C the
   weaker**: it has no identified mechanism at all. macOS **B's accessor** is
   settled; the episode producing the object it reads is not. The macOS
   **escalation grounds** are **R9 and A2**, which stand on independent
   evidence; that is *not* the same as saying the macOS gap is "R9 and A2 only",
   a formulation **withdrawn** in PR #401 review cycle 2. This item settles only
   that the question is **open**.
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
settled for **any of the three** supported targets without operator
determination. Under the lock's requirement that the minimal API contract be
settled inside the deliberation **before `impl-plan` begins**, this deliberation
**does not promote to `impl-plan`**.

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
**Precondition 4** (a primitive delivering **in-episode containment** exists on
the target) is containment-scoped by the lock, and following PR #401 review
cycle 2 it additionally fails on **`aarch64-apple-darwin` C** — the only
candidate primitive there, `O_NOFOLLOW_ANY`, refuses symlinks and supplies no
containment, so it is not a *delivering* containment primitive. **Windows Part B
and macOS Part A are not precondition-4 failures**; they fall under the
per-target-triple feasibility clause, which expressly covers every fixed
property including Part B. **Precondition 5**
fails on **both** `aarch64-apple-darwin` R9 **and** `x86_64-unknown-linux-gnu`
R9 — on each, R9 is reached only through the **escalation** arm, which discloses
the gap rather than delivering atomicity or establishing reliable detection.
Because R1b's preconditions are necessary, **R1b prevention is not established
on any target.**

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
| Linux R9 recorded as "Delivered" — `-EAGAIN` is final-state **detection**, not atomicity, and shares the same out-and-back blind spot used to disqualify macOS | P0 | **Corrected, then further corrected.** First downgraded to `DETECTED, NOT ATOMIC` with the revert-before-completion window disclosed; that downgrade was itself insufficient and was superseded in PR #401 review cycle 1 — see *PR #401 review cycle 1 correction* below |
| "The kernel's own rename-serialisation" — no such serialisation exists across a path walk | P1 | **Corrected.** False premise removed |
| Windows `share_mode` claimed unreachable through `cap-std` — it is publicly re-exported as `cap_fs_ext::OpenOptionsExt::share_mode` | P1 (false claim) | **Corrected.** Windows R9 reclassified from infeasible to authorization-gated |
| Windows Part B reason overstated as "no safe from-handle accessor" | P1 | **Corrected.** Reason restated as low-resolution + panics instead of failing closed |
| macOS `O_NOFOLLOW_ANY` (macOS 11+, unconditional on Apple Silicon) missed | P1 | **Corrected, then further corrected.** First recorded as narrowing the macOS gap to R9 and A2 while flagged as unnamed by `rustix 1.1.3`; that pairing was itself an over-claim and was superseded in PR #401 review cycle 2 — A and C are now `CANDIDATE — UNVERIFIED`. See *PR #401 review cycle 2 correction* below |
| macOS `kqueue`/`EVFILT_VNODE` handle-based detection missed; "no way to detect" was an unqualified negative | P1 | **Corrected.** Recorded as partial and window-bounded, not absent |
| R1b preconditions 1 and 2 fail on **all** targets, not only the two escalating ones | P2 | **Corrected** in the residual ledger |
| `EPERM` missing from the fail-closed errno set (seccomp runtimes) | P2 | **Corrected** |
| `..`-walk admissibility under Part A's zero-name predicate asserted, not established | P2 | **Corrected.** Sequencing constraint now stated explicitly |
| `same-file`'s documented size-based mitigation is not implemented | P2 | **Incorporated** — strengthens the disqualification |

The corrected claims were re-verified directly against vendored source before
incorporation: `cap_fs_ext` re-exports `OpenOptionsExt` (`lib.rs:37`) and
`share_mode` is declared and implemented in `cap-primitives`
(`fs/open_options.rs:313`, `:428`); `rustix 1.1.3` names `OFlags::RESOLVE_BENEATH`
under FreeBSD only, with Linux's beneath bit being `ResolveFlags::BENEATH` (an
`openat2` resolve-argument bit), and names **no** Darwin no-follow-any flag under
any spelling *(attribution corrected in PR #401 review cycle 2)*; and the in-tree
prior art is present at the line numbers cited above.

This verification did **not** run against an implementation plan, consumed **no**
plan-review attempt, and opened **no** correction budget.

### PR #401 review cycle 1 correction

Four Copilot review threads on PR #401 — against this document, the terminal
record, the session memory, and the `027-D` current-state paragraph — identified
**one** blocking defect, raised four times: **`x86_64-unknown-linux-gnu` was
still recorded as contract-*feasible*** on the strength of R9's "detectable →
fail closed" arm, while the same text disclosed an **undetected**
revert-before-completion window.

The finding is **VALID and blocking**. R9's disposition is binary: fail closed
where the in-scope relocation is detectable, escalate where it is not. An
undetected relocation class means the detectable arm is **unmet**, and the lock
routes that case to the feasibility clause, which covers atomicity. The earlier
reading substituted a **risk-window-magnitude** comparison — Linux's window is
narrower than macOS's — for contract satisfaction. The lock provides no
narrow-window arm, and this is the same over-claim pattern, one step further
down, that the original P0 finding named.

Corrections applied, in this document and in every mirror:

* Linux R9 is restated as **UNRESOLVED → ESCALATED, fail closed**; `-EAGAIN` is
  retained as **partial** detection that narrows but does not discharge R9.
* The Linux per-target determination is restated as **NOT CONTRACT-FEASIBLE**.
* The escalation set is recomputed from **two** targets to **all three**.
* The risk-magnitude paragraph is **expressly withdrawn** as a
  contract-satisfaction argument and retained only as a magnitude observation.
* Settled item 1 is narrowed to **A, B, C, and A2 only**.
* R1b **precondition 5** is recorded as failing on Linux as well as macOS.
* A **fifth** operator determination — the Linux R9 disposition — is added.

**A, B, C, and A2 evidence for Linux is preserved intact**: `openat2` still
delivers same-object binding, from-handle identity, in-kernel in-episode
containment, and mountpoint refusal, all verified against vendored source. What
is withdrawn is only the **overall feasibility label**. The terminal state is
unchanged — this remains a **feasibility escalation before planning**, the
plan-review attempt counter remains **zero**, the correction budget remains
**unopened**, `027-D` remains **blocked**, and no plan, harvest, or shipment
exists.

### PR #401 review cycle 2 correction

Four further Copilot review threads on PR #401 — against this document, the
terminal record, the session memory, and the `027-D` current-state paragraph —
identified **one** blocking defect, raised four times: **`aarch64-apple-darwin`
A and C were recorded as *delivered* via `O_NOFOLLOW_ANY`** (and the macOS gap
correspondingly narrowed to "R9 and A2 only"), while the *same* text disclosed
that `rustix 1.1.3` does not name the flag and that safe access to it is
**unverified**.

The finding is **VALID and blocking**. A conclusion may not be stronger than the
caveat printed beside it. An unverified mechanism cannot discharge a fixed
property: it establishes neither **safe flag access** under the locked API and
`#![forbid(unsafe_code)]`, nor **flag semantics** sufficient for A's one-episode
zero-name binding and C's own in-episode refusal to traverse outside the
boundary including the final component. This is the **same over-claim pattern**
the P0 finding and review cycle 1 both named — an unverified-but-plausible
mechanism written up in the vocabulary of a settled one — surfacing on a third
target.

Corrections applied, in this document and in every mirror:

* macOS **A and C** are restated as **`CANDIDATE — UNVERIFIED`** under the
  locked safe API. Neither individually nor as an A/B/C package may they be
  stated as settled or deliverable.
* macOS **B** is restated as *available but not independently settled*: the
  `fstat`-on-fd accessor is safe and present, but is exercisable only against an
  object bound by the unverified A candidate.
* The two unverified antecedents are named explicitly — **safe flag access** and
  **flag semantics** — so a later generation knows exactly what must be verified,
  together with a third: a **positive on-target planted-symlink probe**. The
  probe is not optional rigour. Darwin's `open(2)` does **not** reject
  unrecognised `O_` bits, so a wrong or absent constant yields a **successful,
  symlink-following** open that is **indistinguishable in-band** from a correct
  one — there is no error to fail closed on, and source inspection cannot see
  it. Correspondingly, the risk is recorded as **kernel-side, not wrapper-side**:
  `bitflags`' `from_bits_retain` retains unknown bits by construction.
* **C's mechanism is recorded as UNSUPPLIED, not merely unverified.**
  `O_NOFOLLOW_ANY` refuses symlinks only and supplies no boundary containment,
  and a lexical no-`..` pre-check on a caller-constructed path enters the
  episode **as a name**, which Amendment 2 excludes from satisfying C. No macOS
  in-episode containment mechanism has been identified.
* macOS **B** is split correctly: the **accessor** is settled and object-derived
  (`fstat` on fd) and needs no re-verification; what is unsettled is the
  **episode** that binds the object it reads. The Property B primitive audit is
  amended from "B is satisfiable on both Unix targets" to match.
* **R1b precondition 4** — which the lock scopes to **in-episode containment** —
  is recorded as failing on **`aarch64-apple-darwin` C**, since no containment
  mechanism was identified there. **Windows Part B and macOS Part A are not
  precondition-4 failures**; they fall under the per-target-triple feasibility
  clause, which expressly covers every fixed property including Part B.
* The formulation "**the macOS gap is R9 and A2 only**" is **withdrawn**
  wherever it appears. The macOS **escalation grounds** remain R9 and A2, which
  are unmet on **independent** evidence; the A/C candidate is a **separate,
  additional** unsettled item, neither a substitute ground nor a further
  escalation trigger.
* Settled item 7 is rewritten to settle only that the question is **open**, and
  to bar inheritance of A/C on macOS as evidence.
* The cycle-1 disposition of the `O_NOFOLLOW_ANY` verification finding is marked
  **superseded**.
* Operator determination 2 is restated to carry the unverified A/C candidate
  alongside — and explicitly apart from — the R9/A2 grounds.
* A **binding scope definition** for "gap" / "only" formulations is added below,
  so the newly-introduced *escalation grounds* vocabulary does not make the
  Linux and Windows phrasing read as a whole-surface clearance. No target
  determination is changed by it.

**No count changes.** Escalation still fires on **all three** supported target
triples; R9 is still the failing property on **two** of the three; **five**
operator determinations are still requested; all **twelve** residuals are still
carried forward, none closed. The macOS escalation ground is unchanged, so the
escalation set is **not** recomputed. The terminal state is likewise unchanged —
this remains a **feasibility escalation before planning**, the generation-3
plan-review attempt counter remains **zero**, the correction budget remains
**unopened** (it attaches to plan-review findings, and no plan-review occurred),
`027-D` remains **blocked**, and no plan, harvest, or shipment exists. This
review cycle ran against a **deliberation and its mirrors, not a plan**.

#### Binding scope of "gap" and "only" formulations

Cycle 2 introduced the phrase **escalation grounds** for macOS, which makes the
older "the *X* gap is *Y* **only**" phrasing used for the other two targets read
as broader than it is. That phrasing is **hereby defined, not withdrawn**, and
the definition binds every such formulation in this document and in all four
mirrors. **The lock governs on any conflict**, exactly as the fixed-input table
at the head of this document does.

> "The *T* gap is *P* **only**" means **solely** that, among the **fixed
> properties of the amended input** — Part **A** same-object binding, Part **B**
> object-derived identity, in-episode containment (**C**), and **R9**
> atomicity — the blocking ground on target *T* is *P*. It asserts **nothing**
> about the residual ledger, and it **never** implies that target *T*'s surface
> is otherwise settled or safe to inherit.

**This definition does not adjudicate A2.** It quantifies over the *blocking
ground* and settles nothing about whether A2 is an escalation ground on any
target. The summary table records A2 as **not closed on both
`x86_64-pc-windows-msvc` and `aarch64-apple-darwin`**, while macOS is written
**"ESCALATED (R9, A2)"** and Windows **"ESCALATED (Property B)"**. That
asymmetry **pre-dates cycle 2, is not created by this definition, and is not
resolved here** — cycle 2 takes **no position** on it, and every target verdict
stands exactly as recorded. It is flagged so a later generation sees it as an
open question rather than a settled reading; resolving it would require a
determination this cycle has no authority to make.

The **residual ledger governs** what is undischarged, and it governs
**uniformly**: R1b preconditions 1 and 2 (R6 soundness, R8 per-target hazard
enumeration) fail on **every** target including Linux; R10 is undischarged
everywhere; and **all twelve residuals are carried forward, none declared closed
by this deliberation** (A2 is closed *only* on Linux, by `RESOLVE_NO_XDEV`, as
settled item 1 records). Accordingly, "**the Linux gap is R9 only**" and "**the
sole blocking Windows gap is Property B**" are **fixed-property statements**,
not whole-surface clearances. **No target's surface is settled**, and **no
target is safe to enter `impl-plan` on**, which is precisely why all three
escalate. A later authorized generation that reads any "only" formulation as a
whole-surface clearance has **misread it**.

## Operator determination requested

The escalation requires an explicit operator determination on **five** points.
Stage has made none of them.

1. **`x86_64-unknown-linux-gnu` R9** — accept a hard fail-closed arm on the
   primary release target, or accept `openat2`'s **partial** final-state
   `-EAGAIN` detection as sufficient despite the undetected
   revert-before-completion window, or amend the fixed input so R9 no longer
   requires atomicity or whole-class detectability. Note that the Linux gap is
   **R9 only**: A, B, C, and A2 are all delivered by
   `openat2` with `BENEATH | NO_MAGICLINKS | NO_SYMLINKS | NO_XDEV`. **The
   fixed-input amendment option is program-wide, not Linux-scoped**: exercising
   it would also dispose of determination 2's R9 arm, and it requires a new
   recorded amendment to the lock. The first two options are Linux-scoped.
2. **`aarch64-apple-darwin`** — accept a hard fail-closed arm on a supported
   release target, or remove macOS from G0's supported set, or accept
   `kqueue`-based partial, window-bounded R9 detection as sufficient. The macOS
   **escalation grounds are R9 and A2**, unmet on independent evidence. Note
   separately that **A and C on macOS are `CANDIDATE — UNVERIFIED`**: the
   traversal shape would rest on `O_NOFOLLOW_ANY`, which `rustix 1.1.3` does
   not name, leaving safe flag access, flag semantics, and an on-target
   planted-symlink probe all outstanding — and **C's containment mechanism is
   unsupplied altogether**, since the flag refuses symlinks only. That
   candidate is **not** a further escalation ground and its verification would
   **not** clear this determination — but neither may A, C, or A/B/C as a
   package be treated as deliverable while answering it.
3. **Windows Part B** — authorize a new vetted dependency exposing
   `FILE_ID_INFO` from a handle, or authorize an isolated `unsafe` boundary
   outside `#![forbid(unsafe_code)]`, or accept fail-closed on Windows. This is
   the **sole** blocking Windows gap.
4. **Windows R9** — this is **not** a feasibility question but an authorization
   one. Prevention is reachable today via `cap_fs_ext::OpenOptionsExt::share_mode`.
   The ask is to authorize a **deliberate, user-visible,
   interoperability-blocking mechanism applied to every intermediate directory
   of every corpus traversal** — materially more than "accept R6's disclosed
   side effect", which concerns one long-lived boundary handle.
5. **The failure bound** — the lock records that whether a feasibility
   escalation counts against the renewed generation-3 failure bound is **not
   decided** and requires explicit operator determination. This deliberation
   reached escalation **without** a plan-review failure, and takes no position
   on whether the bound is consumed.

Determinations 1 and 2 are **not interchangeable** as *target* determinations:
each target must be determined on its own evidence, since delivering a property
on one supported target does not discharge it on another. The one exception is
determination 1's **fixed-input amendment** option, which is program-wide by
construction and would dispose of both R9 arms at once.

Until these are determined, `027-D` returns to **`blocked`** and G0 halts.
