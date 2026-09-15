---
title: "Package G0 generation 2 — contained test-filesystem seam"
type: decision
doc_type: decision
date: 2026-09-15
agent: stage
decision_status: accepted
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 2
attempt: 1
depth: deep
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
branch: chore/stage-g0-generation-2-planning
base_commit: f9425943ec6bdb6431389b25d1f2057f51755d3c
supersedes: none
inherits_as_evidence:
  - "chore/checkpoint-resolution-ordering-restage@3b3edf05:docs/closure/2026-09-13-package-g0-attempt-3-plan-review-record.md"
  - "chore/checkpoint-resolution-ordering-restage@3b3edf05:docs/memory/2026-09-13-stage-package-g0-attempt-3-review-fail-circuit-open.md"
promote_to: plan
---

## Status and authority

This is a **new deliberation** opening Package G0 **generation 2**, a separate
operator-authorized work unit created under the Circuit semantics section of the
program lock. It is **not** a fourth attempt at generation 1 and **not** a reset
of the generation-1 circuit, which remains OPEN/triggered.

| Input | Source | Mutability here |
|---|---|---|
| Program structure, package boundary, prohibited scope | `docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` | Read-only |
| Current status, eligibility, cursor | Backlog item `027-D` (`queued`) | Read-only in this artifact |
| Fixed invariant, root-cause evidence | Program lock § *G0 generation-2 fixed input* | **Non-negotiable input** |
| Generation-1 settled items | Gen-1 review record (evidence branch `3b3edf05`) | Read-only evidence, inheritable |
| Storage, type, API shape | **Deliberately undecided** by the lock | **Settled by this document** |

Generation-1 artifacts were read as evidence and **not modified**. No `143.*`
artifact, PR #396, or evidence-branch file was touched.

## Phase 1 — Problem frame

### The problem being solved

Package G0 owns the **contained test-filesystem seam**: the exerciser and
manifest surface that later packages (G1 typed assertion evaluation, G2 registry
and CI wiring) consume. The seam resolves a *corpus* of files under a workspace
and then reads them, with an injectable fake filesystem so the fallible call
sites can be driven deterministically in tests.

Generation 1 died at plan-review attempt 3 on a single architecture-level
finding with two-model consensus:

> `read_corpus` re-canonicalizes and re-checks **workspace containment** but
> never compares the result against the **authorized identity**. An ancestor
> directory replaced by a symlink that redirects to another location *inside*
> the workspace passes every check and yields a different file.

The defect class is precise, and it is not a TOCTOU race. It is a **category
error**: *containment* answers "is this path inside the boundary?"; *identity*
answers "is this the same object I authorized?". Containment is invariant under
any redirect that stays inside the workspace, so it cannot detect one.

### Operator-fixed constraints (non-negotiable)

1. **Read-time canonical identity must EQUAL the opaque stored authorized
   canonical identity.** Remaining inside the workspace is not sufficient.
2. The authorized identity is **opaque**: compared for equality, never
   reconstructed, parsed, or re-derived at read time.
3. **Ancestor replacement inside the workspace must fail.** An ancestor
   directory replaced by a symlink redirecting elsewhere still inside the
   workspace must be rejected.
4. A workspace-containment check is **not a substitute** for identity equality.

### Success criteria

* S1 — The ancestor-substitution-inside-the-workspace scenario is **rejected**,
  and a test proves it fails for the identity reason, not a containment reason.
* S2 — The stored authorized identity is opaque at the type level: no public
  constructor, no accessor that yields its contents, no `Debug`/`Display` that
  discloses an absolute path.
* S3 — The minimal API contract is fully settled **in this document**, before
  `impl-plan` begins.
* S4 — Every guarantee the plan asserts is delivered by a named mechanism, and
  every residual is disclosed. (This is the *pattern* that killed generation 1
  three times — see Phase 2.)
* S5 — No G1 or G2 scope: no typed assertion evaluation, no registry, no CI
  wiring.

### Scope boundaries

**In scope (G0):** the test-filesystem seam; the exerciser; the manifest
surface; resolve-time and read-time path authority; the fake filesystem and its
fault injection; the error taxonomy.

**Out of scope:** typed contract assertion evaluation (G1); harness registry,
real-root binding, CI/oracle registration (G2); any other program package;
changes to `src/db/workspace.rs` production code.

## Phase 2 — Research

### R1. The generation-1 failure pattern (most important input)

Across package-G v3 and G0 attempts 1–3, **every** failure had one root cause:
*the plan asserted a guarantee its mechanism did not deliver.*

| Attempt | Over-claim |
|---|---|
| v3 / attempt 1 | Failure-semantics table claimed completeness it lacked |
| attempt 2 | Containment claim was forgeable at the type level |
| attempt 3 | Hardening item H6 claimed the ancestor-substitution class was closed; it was closed only for redirects *leaving* the workspace |

Attempt 3 closed attempts 1 and 2's blockers convincingly and still died on a
**newly introduced over-claim inside the hardening section written to disclose
residuals honestly.** The lesson is not "harden more". It is: *claim less, and
make every claim's scope explicit.* This deliberation therefore carries an
explicit **Residuals** section (Phase 4) and the plan must carry a residual
table whose rows are scoped, not global.

### R2. Generation-1 settled items (inherited as evidence)

The gen-1 review record affirms eleven items "must not be re-argued". Generation
2 inherits these as **evidence**, in a **new** plan:

| # | Settled item | Generation-2 disposition |
|---|---|---|
| 1 | Opaque containment design: `ResolvedFile` with no stored public path, private `ResolvedCorpus`, `resolve_corpus` sole constructor, `read_corpus` accepts only `&ResolvedCorpus` | **Keep.** Extended, not replaced — see D3 |
| 2 | Corpus carries its own canonical workspace root; no separate validated token | **Keep.** Do not reintroduce a token |
| 3 | `compile_fail` doctest proving struct-literal forgery is rejected | **Keep** |
| 4 | Operation-keyed `(FsOp, PathBuf)` fault injection, all nine call sites proven | **Keep.** Extended to ten — see D6 |
| 5 | Explicit per-entry lstat with `read_dir -> Vec<OsString>` | **Keep** |
| 6 | Compilation-complete `FsError` taxonomy, 13 variants | **Keep.** Extended by one variant — see D5 |
| 7 | Channel-bundling rule for task staging | **Keep** |
| 8 | `todo!()`-arm staged completion under `-Dwarnings` | **Keep** |
| 9 | Security core: SEC-1/2/3 closed; path-rendering policy; no implicit symlink follow | **Keep.** D5 must not regress SEC-3 |
| 10 | Scope discipline; no G1/G2 leakage | **Keep** |
| 11 | Mechanical closures M-1..M-8, advisories A-1..A-3 | **Keep** |

Carried-forward unresolved findings from gen-1, which generation 2 must address
or explicitly re-scope: **finding A** (the architecture-level identity gap — this
is the whole subject of generation 2), **finding B** (`ResolvedCorpus::workspace`
staged at G0.36 but first read at G0.32 — a *staging-order* defect), **M-1**
(missing `FakeFs` constructor), **M-2** (`clippy::missing_panics_doc` on staged
`todo!()` arms).

### R3. Repository precedent — `CapRoot` in `src/db/workspace.rs`

The repository already contains a mature, reviewed solution to the adjacent
problem, and it is the strongest available precedent:

```rust
/// `dir` is the load-bearing authority: every child is reached from it with an
/// `openat`-style no-follow operation, one component at a time, so validation
/// and use address the same object. `display` is carried for error messages
/// only and is NEVER re-resolved for access (invariant 1).
struct CapRoot { dir: Dir, display: PathBuf }
```

Relevant properties, all already proven in-tree:

* `cap-std = "=4.0.2"` is a **direct, pinned dependency** — no new dependency is
  required.
* `open_child_dir` uses `open_dir_nofollow` and validates through
  **handle-derived** metadata, with a runtime guard that the name is exactly one
  component.
* `read_child_file` takes metadata from the **open file handle** and reads
  content from that **same** handle — never a reopen by path. The record notes
  this was itself "the load-bearing P0 constraint from plan review S1".
* `normalize_canonical` (line 21) already normalizes Windows verbatim prefixes
  (`\\?\C:\` and `\\?\UNC\`) by **component inspection, not string round-trip**.
* The *asymmetry* is deliberate and documented: a workspace root that is itself
  a junction is a legitimate, supported layout; the reparse gate is scoped to
  the chain **below** a root.

`tests/unit/workspace_toctou_test.rs` already stages the exact adversarial
fixture class this package needs, including a cross-platform `make_dir_link`
(Unix symlink / Windows `mklink /J` junction, no elevation needed) and an
explicit `SKIPPED:` discipline — *never a silent pass* — when privileges are
unavailable. **This is a reusable fixture pattern, not a new invention.**

### R4. Compound learnings retrieved

| Learning | Bearing on G0 generation 2 |
|---|---|
| `security/canonicalize-both-sides-workspace-path-containment-2026-07-01.md` | Canonicalize both sides; never resolve relative paths against CWD; on Windows normalize the verbatim prefix before comparison. **Directly reused by D2.** Note it prescribes `starts_with` containment — which is exactly the check the fixed invariant says is *insufficient*. The learning is correct for its own scope (a CLI boundary) and is **not** authority for read-time identity. |
| `path-handling/windows-drive-relative-path-traversal-component-prefix-2026-05-06.md` | Drive-relative traversal must be caught by **component** inspection, not string matching |
| `testing/tempdir-lifetime-in-contract-tests-2026-03-30.md` | `TempDir` must outlive the harness or the fixture root vanishes mid-test |
| `testing/pub-visibility-for-external-test-harness-2026-04-20.md` | An external `tests/` harness can only reach `pub` items — constrains where the seam can live (D7) |
| `workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-defer-to-harness-architect-2026-07-25.md` | Over-splitting tasks cascades review churn. Bounds the plan's task count (D8) |
| `workflow-issues/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md` | Single-model plan review diverges; use the multi-model adversarial panel. **Governs the Step 4 review method** |
| `testing/hermetic-all-target-per-test-process-explosion-2026-08-02.md` | Per-test process spawning explodes under `--all-targets`; keep the seam in-process |

### R5. What a "canonical identity" can be

Three candidate representations were evaluated against the fixed invariant.

| Representation | Detects ancestor substitution? | Portable? | Derivable in `FakeFs`? | Notes |
|---|---|---|---|---|
| **(a) Canonicalized absolute path**, normalized via the existing `normalize_canonical` shape | **Yes.** `canonicalize` resolves symlinks, so `ws/a/b/f` becomes `ws/z/b/f` after `ws/a -> ws/z`; the values differ | Yes — already handled for Windows verbatim/UNC prefixes | Yes — the fake owns a synthetic path space | Does **not** detect same-path content replacement |
| **(b) OS file id** (Unix `dev`+`ino`; Windows `VolumeSerialNumber`+`FileIndex`) | Yes, and also detects same-path file replacement | Weaker: Windows requires an open handle; ReFS uses a 128-bit id; `FileIndex` is not guaranteed stable across all filesystems | Awkward — the fake must synthesize and reissue ids | Strictly stronger *and* strictly less portable |
| **(c) Opaque hash of (a)** | Yes | Yes | Yes | Adds a hashing dependency and a (negligible) collision surface for **zero** additional detection over (a) |

## Phase 3 — Options

### Option A — Stored opaque per-file canonical identity, compared for equality at read time

At resolve time, derive and store an opaque identity for each resolved file
inside the private `ResolvedFile`. At read time, derive the identity **again**
from the live filesystem and compare it for **equality** against the stored
value. Any inequality rejects.

* **Pros:** literal, direct expression of the fixed invariant; preserves all
  eleven settled items; adds exactly one error variant and one seam operation;
  the fake can drive it; no new dependency.
* **Cons:** still a check-then-read pair, so it *detects* substitution rather
  than *preventing* it; a true temporal race in the window between compare and
  read remains (narrow, and see D4).
* **Effort:** medium. **Fit:** exact.

### Option B — Retained capability handle per resolved file (the `CapRoot` shape)

Open every corpus file at resolve time, retain the handle, and read from the
same handle. Identity becomes structural: there is no re-resolution to attack.

* **Pros:** strongest possible guarantee; *eliminates* the class rather than
  detecting it; strong in-repo precedent.
* **Cons, and they are decisive:**
  1. It **does not satisfy the fixed invariant**, which mandates a *stored
     opaque identity compared for equality at read time*. Option B has no such
     comparison — it makes the question moot. The invariant is a non-negotiable
     *input*, not a goal to be met by an alternative route.
  2. **Unbounded file descriptors.** A corpus is arbitrarily large; retaining a
     handle per file risks descriptor exhaustion.
  3. It **invalidates settled items 4 and 5** by deleting call sites that the
     nine-call-site fault-injection design and the explicit-lstat design depend
     on — re-litigating two items the gen-1 review affirmed.
  4. `FakeFs` has no real handles; the trait would need an associated handle
     type, complicating the fake for no test-visible gain.
* **Effort:** high. **Fit:** violates a fixed input.

### Option C — Hybrid: retained root anchor + per-file identity equality

Retain a single capability-rooted anchor handle for the corpus root
(bounded: exactly one descriptor), and additionally store and compare per-file
opaque identity as in Option A.

* **Pros:** all of Option A, plus the anchor removes the ancestor chain from the
  read path entirely, so the residual temporal window in Option A is narrowed to
  the leaf component.
* **Cons:** the anchor is a second mechanism whose *marginal* benefit must be
  separately proven; `FakeFs` must model an anchor; increases the surface G1
  inherits. The gen-1 death pattern warns specifically against adding mechanisms
  whose claimed closure exceeds what they deliver.
* **Effort:** high. **Fit:** satisfies the invariant, but exceeds *minimal*.

### Comparison

| Criterion | A | B | C |
|---|---|---|---|
| Satisfies the fixed invariant literally | **Yes** | **No** | Yes |
| Rejects ancestor substitution inside the workspace (S1) | Yes | Yes | Yes |
| Preserves all 11 settled items | **Yes** | No (breaks 4, 5) | Yes |
| Minimal API surface (lock: "minimal API contract") | **Yes** | No | No |
| Bounded descriptors | Yes | **No** | Yes |
| `FakeFs` drivable without redesign | **Yes** | No | Partly |
| New dependency | None | None | None |
| Over-claim risk (the gen-1 killer) | **Low** | Medium | **High** |

### Decision

**Option A is adopted.** Option C is explicitly **deferred, not rejected on
merit**: it is a legitimate strengthening, but it is not minimal, it is not
required by the fixed invariant, and adopting a second mechanism is precisely
the move that produced attempt 3's fatal over-claim. If a future package proves
the leaf-window residual (Res-1) matters, C is the named upgrade path.

Option B is **rejected** because it does not satisfy a non-negotiable input.

## Phase 4 — Settled minimal API contract

This section discharges the lock's requirement that *"the minimal API contract
must be settled inside the deliberation, before `impl-plan` begins."* `impl-plan`
may **detail** these decisions; it may **not** revise them.

### D1 — Identity representation

The authorized canonical identity is the **canonicalized absolute path,
normalized for platform prefix representation** (option (a) of R5).

* Rejected (b) on portability and fake-drivability; rejected (c) as pure
  overhead.
* Normalization follows the **component-inspection** shape already proven in
  `normalize_canonical` (verbatim disk and verbatim UNC), never a string
  round-trip. G0 **re-implements this shape inside the seam** rather than making
  `src/db/workspace.rs::normalize_canonical` public — G0 must not modify
  production code.

### D2 — Identity type

```rust
/// Opaque authorized canonical identity. Compared for equality only.
#[derive(Clone, PartialEq, Eq)]
pub struct CanonicalIdentity(PathBuf);   // field PRIVATE
```

Binding constraints:

* The inner field is **private**. There is **no** public constructor, **no**
  accessor returning the inner value, and **no** `From`/`Deref`/`AsRef` escape.
* Derives are exactly `Clone, PartialEq, Eq`. **No `Debug`**, **no `Display`**,
  **no `Serialize`**, **no `Hash`**, **no `Ord`** — `Debug` is omitted
  deliberately so the type cannot be printed into a test failure or a log and
  disclose an absolute path (preserves settled item 9 / SEC-3). Any container
  that must derive `Debug` implements it **manually** and renders the identity
  as a fixed opaque placeholder.
* Sole construction route is a private seam function invoked from
  `resolve_corpus` and from the read-time derivation.
* The stored value is **never parsed, re-derived, or reconstructed** — the only
  operation performed against it is `==`.

### D3 — Storage location

```rust
pub struct ResolvedFile {
    rel_path: String,               // private, existing (settled item 1)
    identity: CanonicalIdentity,    // private, NEW
}
impl ResolvedFile { pub fn rel_path(&self) -> &str { &self.rel_path } }
```

`ResolvedCorpus { workspace, files }` stays private-fielded with `resolve_corpus`
as sole constructor (settled items 1 and 2). Adding a private field **preserves**
unforgeability: the `compile_fail` doctest (settled item 3) keeps proving
struct-literal forgery is rejected, and in fact the new private field makes
forgery *harder*, never easier. **No accessor for `identity` is added** — it is
not part of the consumed surface G1 inherits.

### D4 — Where containment and identity each apply (the core correction)

This is the decision that closes gen-1 finding A. The two checks are separated
by **phase**, and neither substitutes for the other:

| Phase | Check | Authority | Error on failure |
|---|---|---|---|
| **Resolve time** | Workspace containment, canonicalized both sides | Input validation — rejects a caller-supplied path outside the workspace | `OutOfWorkspace` (existing) |
| **Resolve time** | Derive and store `CanonicalIdentity` | Establishes the authorized identity | — |
| **Read time** | **Identity equality only** | **Sole authority.** Re-derive the identity and compare `==` against the stored value | `IdentityMismatch` (new, D5) |

**Read time performs NO containment check.** The lock left "whether a
workspace-containment check is retained at all" undecided; it is retained at
**resolve time only**, and is explicitly **demoted from a security boundary to
input validation**. Re-checking containment at read time is exactly what
generation 1 did, and it is what made the ancestor-substitution attack pass.

Why this rejects the fixed root-cause scenario, mechanically:

1. Resolve time: `ws/a/b/file` canonicalizes to `ws/a/b/file`; that value is
   stored as the authorized identity.
2. Attacker replaces ancestor `ws/a` with a link to `ws/z`, still inside `ws`.
3. Read time: canonicalization now yields `ws/z/b/file`.
4. `ws/z/b/file != ws/a/b/file` → **`IdentityMismatch`, rejected.**
5. Containment would have returned *inside the workspace* at both steps — which
   is why it is no longer consulted at read time.

The inverse case is symmetric: if `ws/a` was *already* a link at resolve time
(stored `ws/z/b/file`) and is later replaced by a real directory, read time
yields `ws/a/b/file`, which is again unequal, and is again rejected.

### D5 — Error taxonomy extension

Exactly **one** variant is added to `FsError`, bringing it from 13 to 14:

```rust
#[error("resolved file '{rel_path}' no longer has its authorized identity")]
IdentityMismatch { rel_path: String },
```

* Carries **only the relative path**. It must **not** carry either identity,
  either absolute path, or any workspace path — preserving SEC-3 and the settled
  path-rendering policy (settled item 9).
* The variant must satisfy the existing `Clone + PartialEq + Eq` derive chain
  (settled item 6): `String` satisfies all three, so the chain is preserved.

### D6 — Seam operation extension

`FsOp` gains exactly **one** discriminant, `CanonicalIdentity`, bringing the
call-site count from nine to **ten**. The existing operation-keyed
`(FsOp, PathBuf)` fault-injection design (settled item 4) carries over unchanged
and must give the tenth call site its own independent injector. `FsOp: Ord` is
required for the `BTreeMap` key and a fieldless-enum `Ord` remains sound with
one more variant (settled item 6).

The seam trait gains one method, whose signature is fixed here:

```rust
fn canonical_identity(&self, path: &Path) -> Result<CanonicalIdentity, FsError>;
```

`FakeFs` implements it over its synthetic path space, which is what lets the
ancestor-substitution scenario be exercised **deterministically and
hermetically**, with no real symlink and therefore **no privilege requirement**
— see D9.

### D7 — Seam location and visibility

The seam lives in the **library crate** (`src/`), not under `tests/`, so both
the in-crate unit tests and the external `tests/` harness can reach it, and so
G1 can consume it. Compound learning
`pub-visibility-for-external-test-harness-2026-04-20.md` applies: items the
external harness touches must be `pub`. The exact module path is an `impl-plan`
detail; the **visibility rule** is settled here:

* `pub`: `ResolvedCorpus`, `ResolvedFile`, `ResolvedFile::rel_path`,
  `resolve_corpus`, `read_corpus`, `CanonicalIdentity` (as an opaque type),
  `FsError`, `FsOp`, the seam trait, `FakeFs`.
* **Never `pub`:** every field of `CanonicalIdentity`, `ResolvedFile`, and
  `ResolvedCorpus`; the identity-derivation function.

### D8 — Plan shape

**Verified precondition:** the seam does **not** exist in the tree. A read-only
search of `src/`, `tests/`, and `crates/` for `ResolvedCorpus`, `ResolvedFile`,
`resolve_corpus`, `read_corpus`, `FakeFs`, and `enum FsOp` returns **zero
matches** at `f9425943`. Generation 1 failed at plan-review and never harvested,
so no G0 code was ever written.

The consequence must be stated precisely, because conflating the two halves of
it would itself be an over-claim:

* The **design** is a delta — eleven items are settled and inherited (R2), and
  generation 2 adds exactly one mechanism (D4).
* The **code** is **greenfield**. Every type, trait, and function in D2–D7 is
  newly authored.

Generation 1's plan was **18 pairs / 36 units** for the same greenfield scope,
and `in-plan-task-granularity-splitting-cascades-review-churn-...` warns that
over-splitting cascades review churn. Generation 2 is bounded to **9–12 tasks**,
each within the 2-hour rule, staged under the settled channel-bundling rule
(settled item 7) so that the identity channel — resolve-time capture, read-time
comparison, and the containment demotion — is **bundled into one task**, which
also closes carried-forward gen-1 **finding B** (the staged-before-read ordering
defect) by construction.

### D9 — Test strategy and the red test that proves S1

Two layers, deliberately:

1. **Hermetic, always-runs (primary).** Drive the ancestor substitution through
   `FakeFs`'s synthetic path space. No real filesystem, no symlink, no
   privilege, no skip. This is the test that *must* be red before the fix and
   green after.
2. **Real-filesystem corroboration (secondary).** Reuse the proven
   `make_dir_link` pattern from `tests/unit/workspace_toctou_test.rs` (Unix
   symlink / Windows `mklink /J` junction — no elevation). Where privilege is
   genuinely unavailable, it prints an explicit `SKIPPED:` line and still
   asserts everything it can — **never a silent pass**, per that file's
   established discipline.

The primary test must assert the failure is **`IdentityMismatch`**, not merely
that an error occurred — otherwise it would pass for a containment reason and
reproduce the generation-1 category error in the test itself.

### Residuals — explicitly NOT closed by this design

Stated at their true scope. This section exists because a global closure claim
is what terminated generation 1.

| ID | Residual | Scope, precisely | Disposition |
|---|---|---|---|
| **Res-1** | Temporal race between the identity comparison and the content read | The **leaf component only**. The ancestor chain is covered, because any ancestor redirect changes the derived identity | **Disclosed, not closed.** Option C is the named upgrade path if a later package proves it matters |
| **Res-2** | Same-canonical-path content replacement | An attacker who overwrites the file **in place**, leaving the canonical path identical, is not detected | **Disclosed, not closed.** Out of scope: the fixed invariant is about *identity of the object named*, not content integrity. Representation (b) would narrow but not close this, at a portability cost |
| **Res-3** | Hard links | Two hard links to one inode canonicalize to different paths and compare unequal | **Accepted as correct-by-design.** The seam authorizes a *path*, so a different path is a different authorization |
| **Res-4** | Real-filesystem symlink tests may skip without privilege | Secondary layer only; the primary hermetic test never skips | **Bounded by D9** |
| **Res-5** | Identity is derived through the seam trait, so a *malicious* `TestFs` impl could lie | Test-only surface; no production trust boundary depends on it | **Accepted.** `FakeFs` is test scaffolding, not a sandbox |

## Phase 5 — Decision record

**Recommendation: Option A, with the contract fixed by D1–D9 and residuals
Res-1..Res-5 disclosed at their stated scope.**

**Rejected alternatives:** Option B (does not satisfy a non-negotiable fixed
input; breaks settled items 4 and 5; unbounded descriptors). Option C (satisfies
the invariant but is not minimal; deferred as the named upgrade path).

**Risks and mitigations**

| Risk | Mitigation |
|---|---|
| Re-introducing an over-claim (the gen-1 killer) | Residual table scoped per row; D4 states the mechanism step-by-step; D9 requires the test assert the *specific* error |
| Scope creep into G1/G2 | D7 fixes visibility only; no assertion evaluation, registry, or CI wiring appears in D1–D9 |
| Re-litigating settled items | R2 dispositions every one of the eleven explicitly; only items 1, 4 and 6 are *extended*, none replaced |
| Disclosing an absolute path through the new type | D2 omits `Debug`/`Display`; D5 restricts the variant to `rel_path` |
| Plan-review churn from over-splitting | D8 bounds the plan to 9–12 tasks |
| Greenfield volume underestimated because the design is settled | D8 states the design/code split explicitly and verifies the zero-match precondition rather than assuming a delta |

**Unresolved questions:** none that block `impl-plan`. Res-1..Res-5 are
*disclosed residuals*, not open questions — each has a stated scope and
disposition.

**Promotion:** `plan`. This artifact is the source document for `impl-plan`
within this same Stage operation, as the lock requires.

## Appendix — Stage disposition of stash `F937D77C`

Handled under Stage's own backlog authority within this session; **not** part of
the G0 release unit, **not** added to the G0 shipment.

* **P-021 C5 duplicate scan (unconditional): CLEAN.** Scanned all active stash
  entries for a second entry describing 027-D body-text staleness. Exactly one
  exists (`F937D77C`). No duplicate; no merge; no archival of any other entry.
* **P-021 C6 late-identifier reconciliation: triggered** (`task`, `feature`,
  `shipment` all recorded `N/A`). Reconciliation result: **no late identifier
  found, and none is expected** — PR #398 was a bounded no-shipment,
  PR-only Ship operation, so no task, feature, or shipment identifier ever
  existed for it. The recorded `N/A` values **stand as a truthful terminal
  record**. Non-blocking, per C6.
* **Applicability: confirmed still applicable.** `027-D` carries
  `status: queued` while its body still opens "BLOCKED pending publication".
* **Disposition:** the entry is marked `Requires deliberation: false` (mechanical
  wording update, no design decision). It is dispositioned here rather than
  routed to a separate deliberation, and the corrective body edit is applied to
  `027-D` by Stage under its own backlog authority.
* **Explicitly untouched:** stash `4EF24729`, which the program lock forbids
  consuming, archiving, or mutating.
