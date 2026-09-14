---
type: deliberation
date: 2026-09-13
package: G0
attempt: 3
supersedes: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-2.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_reviews:
  - docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
  - docs/closure/2026-09-13-package-g0-attempt-2-plan-review-record.md
branch: chore/checkpoint-resolution-ordering-restage
head: e0accf82
status: accepted
gate: "final allowed G0 attempt; any P0/P1 after the permitted review cycle opens the G0 circuit"
---

# Package G0 attempt 3 — contained test-filesystem seam (deliberation)

## Standing

This is the **third and final allowed** G0 attempt. Attempts 1 and 2 are
preserved unmodified as evidence together with their review records. If this
plan carries any P0/P1 finding after its one permitted correction cycle, the G0
circuit opens and no fourth plan is authored.

## What is NOT re-deliberated

The operator has fixed the coherent API boundary for this attempt. The four
decisions below are **inputs**, not open questions. They are recorded here so
the plan can cite a settled source, and they are not re-argued.

| # | Fixed decision |
|---|---|
| **B1** | Opaque containment types. `ResolvedFile` fields are private. An opaque `ResolvedCorpus` with private entries and a read-only accessor is the unit `read_corpus` consumes. Only the G0 resolver constructs either, and only after exhaustive contained resolution. `read_corpus` re-checks every invariant that can change between resolution and read. The public API exposes stable workspace-relative display identity and content, never a mutable canonical path. |
| **B2** | Operation-specific deterministic fault injection. The `FakeFs` fault map is keyed by an explicit *(operation, path)* pair, not a single path-keyed "unreadable" set. Every fallible seam operation is independently injectable, and `ReadToString` has its own proving scenario. |
| **B3** | Explicit entry lstat. Directory enumeration returns contained entry **names only**. The resolver explicitly invokes the injected `symlink_metadata` for every entry before any canonicalization or following. No directory-entry kind metadata is trusted as a type or security authority. |
| **B4** | Compilation-complete same-unit wiring. A unit that adds a seam call also adds its `FsCall` variant, its public error mapping, its fake implementation, and every exhaustive match arm required to compile — in the same green unit. No intermediate commit leaves an unhandled enum variant or a dead method under `-Dwarnings`. |

Also **not** reopened: everything in the "What this review SETTLES" sections of
the attempt-1 and attempt-2 review records. In particular the site-less
`FsFault` / site-bearing `FsError::Io` split with a single total lift function,
the three-layer identity design, the `thiserror` dependency, the `tests/unit/`
tier, the Constitution I–XI mapping, the explicit `-p` clippy command, the
`canonicalize`-driven Windows case model, and the privilege-free determinism
design.

## Retained affirmed contracts

Dev-only workspace library crate plus a root `[[test]]` target; no broad
dead-code allowances; root lstat before canonicalize; entry lstat before
canonicalize; injectable `canonicalize`; exhaustive non-filtering enumeration;
reject duplicate and overlapping roots and duplicate canonical file identity;
deterministic Windows case model; read-only containment; no new external
dependency; a real Constitution I–XI mapping; all quality gates including
`cargo audit`.

## Open questions settled by this deliberation

The attempt-2 review closed with eight items the next attempt "must settle
first". B1–B4 settle items 1–4 by operator decision. This deliberation settles
the remainder, plus the four new questions that B1–B4 open.

---

### Q1 — How is the unforgeability claim *proved*, rather than asserted?

**Context.** B1 makes `ResolvedCorpus` and `ResolvedFile` opaque. The whole
Constitution IV claim now rests on the proposition that no code outside the
crate can produce a `ResolvedCorpus`. Attempt 1 died claiming completeness it
did not have; attempt 2 died claiming containment its types did not deliver.
An *asserted* unforgeability claim would be the same defect a third time.

**Options.**

* **P-1 — Assert it in prose.** Rejected. This is precisely the failure mode
  that terminated three consecutive attempts.
* **P-2 — Prove it by inspection only.** A Verification step greps `lib.rs`
  for public constructors of `ResolvedCorpus`. Cheap, but a grep proves the
  absence of a *spelling*, not the absence of a *capability*.
* **P-3 — Prove it with a `compile_fail` doctest.** A single doctest on
  `ResolvedCorpus` attempts struct-literal construction from outside the crate
  and asserts the compiler rejects it. This is a mechanical, re-run-able,
  compiler-checked proof of exactly the claimed property.

**Decision: P-3, with P-2 retained as a cheap secondary check, and with the
CI gap disclosed.**

The doctest is the only mechanism in the package that *proves* a negative
compile-time property. It is exempt from the red/green convention because it
asserts a property that is true from the instant the type is declared with
private fields; there is no state in which it is red and then green. That
exemption is stated in the plan rather than hidden.

**Disclosed limitation.** Precondition P11 — CI runs the root package only,
without `--workspace` — means CI does **not** execute this doctest. It runs
under the explicit `cargo test -p agent-contract-fs --doc` command in the
Verification section, which is a local and review-time gate, not a CI gate.
G0 does not change CI (that is G2's scope), so this is a real, named residual
and is recorded in the plan's hardening section rather than papered over.

---

### Q2 — Where does `read_corpus` get its workspace root, and does it need a separate token?

**Context.** B1 says `read_corpus` accepts the opaque validated corpus "and, if
defense-in-depth requires, a validated workspace-root token".

**Options.**

* **W-1 — Separate token parameter.** `read_corpus(fs, &corpus, &WorkspaceRoot)`.
  A second opaque type, also only constructible by the resolver.
* **W-2 — The corpus carries its own canonical workspace root.**
  `read_corpus(fs, &corpus)`; the corpus privately stores the canonical
  workspace root established in Phase 1.

**Decision: W-2.**

A separate token adds a second opaque type and a second construction path
without adding a guarantee. The corpus is *already* unforgeable, and the
workspace root it carries is *already* the canonicalized value the resolver
validated in Phase 1. Pairing them in one value makes the mismatch case —
a corpus checked against the wrong workspace root — **unrepresentable**, which
is strictly stronger than making it checkable. W-1 would reintroduce exactly
that mismatch as a caller responsibility.

---

### Q3 — What is `rel_path` relative to: the lexical join, or the canonical path?

**Context.** Attempt 2 built `rel_path` from lexical workspace-relative
components. Advisory A-1 observed that lexical subtree disjointness follows from
canonical non-overlap only under an additional, unstated premise about
symlink-free descent — and that premise is fragile because Phase 2 lstats only
the *final* component of a root, not its intermediate components.

**Options.**

* **R-1 — Lexical, plus the A-1 premise stated.** Keeps attempt 2's shape and
  documents the extra clause.
* **R-2 — Canonical-relative.** `rel_path` is the canonical file path stripped
  of the canonical workspace root, components joined with `/`.

**Decision: R-2.**

R-2 removes the premise instead of documenting it. Because `rel_path` becomes a
*function* of `canonical_path` under a fixed canonical workspace root, and that
function is injective, `rel_path` uniqueness follows **directly** from canonical
identity uniqueness, which the identity map already enforces. No auxiliary
argument about symlink-free lexical descent is required at all. A-1 is closed by
construction rather than by disclosure.

R-2 has a second consequence that Q4 depends on: `workspace.join(rel_path)`
reconstructs the canonical path exactly, so `ResolvedFile` does not need to
store a `PathBuf` at all.

---

### Q4 — What does `ResolvedFile` store?

**Decision.** `ResolvedFile { rel_path: String }` — one private field, and a
single public accessor `rel_path() -> &str`. The canonical path is **not**
stored per file and is **not** exposed anywhere.

This is a direct strengthening of B1. Attempt 2's `ResolvedFile` carried a
public `canonical_path: PathBuf`; the review found it forgeable. Here the
canonical path is not merely private — it does not exist as a per-file value.
`read_corpus` reconstructs it as `corpus.workspace.join(rel_path components)`
and then *re-derives and re-validates* it through the seam. A `ResolvedFile`
in isolation carries no filesystem authority whatsoever; authority lives only
in the corpus that pairs it with a validated workspace root.

---

### Q5 — What exactly must be compilation-complete in the first unit?

**Context.** B4 requires the complete `FsError` / `FsFault` taxonomy in the
earliest owning unit. Taken naively — "every declared item, complete, in unit
one" — this collides with `-Dwarnings` (precondition P3): a private field that
nothing *reads* yet, or a private helper function that nothing *calls* yet,
trips `dead_code` and breaks the build.

**Decision — a precise three-part rule, stated in the plan and enforced by it.**

1. **Complete in the first unit**: every `enum` and its full variant set, every
   `#[error(...)]` attribute, every derive, the full `FileAccess` trait
   signature, every public struct and its public accessors, and the signatures
   of `resolve_corpus` and `read_corpus`. Public items in a library crate are
   externally reachable, so `dead_code` does not fire on them. This is the part
   that matters for B4: **no enum variant is ever added later**, so no match
   can ever become non-exhaustive in a later commit.
2. **Private state rule (inherited from attempt 2's H4, affirmed by review)**:
   a *private field* or *private helper* is introduced in the green unit that
   first **reads or calls** it. Exactly two items are governed by this rule in
   the whole package, and the plan names both.
3. **Staged arm completion**: an exhaustive `match` may carry `todo!()` arms.
   Each green unit replaces exactly the arms its paired red unit drives. A
   `todo!()` arm is a *handled* variant — it compiles, it is warning-clean, and
   it panics rather than silently succeeding. This is the mechanism that keeps
   B4 and test-first simultaneously satisfiable.

Part 3 is the general answer to attempt-2 terminal finding D. Part 1 is the
answer to finding E.

---

### Q6 — How is "no premature green" guaranteed for failure channels?

**Context.** Finding D observed that under `-Dwarnings` a green unit that
introduces a seam call *must* handle that call's `Result`, which wires the
failure channel in the same commit — so a channel test owned by a *later* unit
is already green when it lands.

**Decision — bundling is mandatory, not advisory.**

> **Channel-bundling rule.** The `Io` channel test for a seam call lives in the
> **same red unit** that drives that call's introduction. A failure channel may
> never be owned by a unit later than the one introducing its call.

Applied consistently this makes finding D's defect structurally impossible
rather than individually patched. The plan's channel table and its work-unit
table are generated from the same assignment, so they cannot disagree.

Two secondary staging breaks from finding D are closed by construction:

* **`rel_path` before its driving test** — closed by bundling entry
  containment, `rel_path` construction, and the identity map into a **single**
  red/green pair. `DuplicateFileIdentity` carries `first_rel` and `second_rel`,
  so its driving test and `rel_path` construction are necessarily the same unit.
* **Determinism already satisfied before its unit** — closed by **not sorting
  canonical roots in Phase 3**. Full pairwise comparison needs no sort (already
  settled in attempt 2 for a different reason). Traversal therefore proceeds in
  *configured* root order, so cross-root output order is genuinely
  non-deterministic until the final sort lands. The determinism test is
  reliably red.

---

### Q7 — What is the path-rendering type, given the disclosure requirement?

**Context.** Mechanical finding M-3, raised by two reviewers: attempt 2's lift
function received no workspace root, so seven of eight `Io` channels could not
be relativized and runner-specific absolute paths would reach CI logs.

**Decision.** Every `FsError` payload path is a **`String` in
workspace-relative, `/`-separated display form**, produced by one private
rendering helper, and the single total lift function takes the workspace root:

```rust
fn io(workspace: Option<&Path>, site: FsSite, fault: FsFault) -> FsError
```

`Option` preserves "exactly one construction path for `Io`" while admitting the
one call site — Phase 1 — at which no canonical workspace root exists yet.

Two payload classes **cannot** be relativized and the plan says so rather than
claiming otherwise:

* `Io { site: WorkspaceRoot, .. }`, where no canonical root exists yet;
* `OutOfWorkspace { .. }`, whose path is by definition not under the workspace.

Both render lossily as absolute. Every other variant is relative, and one
assertion checks that an `Io { site: Root, .. }` payload does not contain the
absolute fixture prefix. The `workspace` field is **dropped** from
`OutOfWorkspace` — it was a constant the caller already supplied, and removing
it removes a second absolute-path disclosure for no loss of information.

Using `String` rather than `PathBuf` also removes a real test-authoring hazard:
on Windows, canonical paths carry the `\\?\` extended-length prefix, so
`PathBuf` payload assertions would have to be written platform-specifically.

---

### Q8 — What remains genuinely open after `read_corpus` revalidates?

**Context.** Mechanical finding M-5: attempt 2's hardening described the TOCTOU
residual as "the syscall-level race between the re-lstat and the read", implying
a nanosecond window, when in fact two whole classes were open for the entire
interval between resolve and read.

**Decision.** The re-canonicalize-and-recheck step in `read_corpus` (B1)
**closes** M-5 class (a) — ancestor-directory substitution. `symlink_metadata`
declines to follow only the *final* component, so a substituted ancestor yields
kind `File`; the subsequent `canonicalize` resolves through the substituted
ancestor and the containment recheck rejects it.

M-5 class (b) — a regular file replaced by a *different* regular file at the
same canonical path — remains **open and is disclosed as open**. Kind is
unchanged, containment is unchanged, and no error fires. Closing it requires
handle-based atomic read (open once, fstat and read the same handle), which the
settled four-method seam does not have and which this attempt does not widen.
It does not require winning a race: the window is the whole interval between
`resolve_corpus` and `read_corpus`.

The irreducible residual between the containment recheck and the read itself is
also stated, separately, as a genuinely narrow syscall-level window.

---

### Q9 — Two undefined root-grammar cases (finding M-7)

**M-7(i) — a root that canonically equals the workspace root.** Attempt 2's
component-wise `starts_with` is inclusive, so such a root was silently accepted
and would traverse the entire workspace. **Decision: reject**, with a dedicated
variant `RootIsWorkspaceRoot`. A root that resolves to the whole workspace is
not a bounded allow-list entry. The arm has a real driving test: the `FakeFs`
canonical map is declarative, so a non-symlink root can be declared to
canonicalize to the workspace root and the arm is exercised deterministically.

**M-7(ii) — the false platform claim.** Attempt 2 claimed `C:\foo` and
`\\?\C:\foo` are `Component::Prefix` and rejected on every platform. That is
**false on Unix**, where Rust's path parser treats those spellings as ordinary
`Normal` components. **Decision: withdraw the claim.** Phase 0 rejects every
non-`Normal` component, which is platform-correct as written; on Unix a
Windows-style spelling is simply an unusual file name, is lexically contained,
and fails at Phase 2 as `RootMissing`. No escape is possible, so no extra
lexical rule is added. The plan states the platform-dependent parse explicitly
instead of asserting a uniform rejection it does not deliver.

---

### Q10 — Do `clippy::pedantic` obligations change the declared surface?

Yes, and attempt 2 never said so. Under `#![warn(clippy::pedantic)]` plus
`-D warnings`:

* `clippy::missing_errors_doc` requires an `# Errors` section on **every**
  public function returning `Result` — that is all four `FileAccess` methods,
  `resolve_corpus`, and `read_corpus`.
* `clippy::must_use_candidate` requires `#[must_use]` on public functions
  returning a non-unit value — `rel_path`, `len`, `is_empty`, `iter`, `calls`.
* `clippy::return_self_not_must_use` fires on builder methods returning `Self`
  or `&mut Self`. **Decision:** `FakeFs` mutators return `()` and are called as
  statements rather than chained. This avoids the lint outright rather than
  annotating around it, and it is also what the post-resolve mutation scenarios
  (file removal, symlink substitution, canonical-map rewrite) need anyway.

`FakeFs` records its call log through a `RefCell`, because the seam methods take
`&self`. Tests are single-threaded per test function, so the crate does not
claim `Sync` for `FakeFs` and does not need to.

---

## Decisions carried into the plan

| ID | Decision |
|---|---|
| D1 | `read_dir` returns `Vec<OsString>` — contained entry **names** only. No kind, no metadata. (B3) |
| D2 | The resolver explicitly calls `symlink_metadata` for every entry, before any canonicalize. (B3) |
| D3 | `FakeFs` fault map keyed by `(FsOp, PathBuf)`; builder `fail(op, path, kind)`. (B2) |
| D4 | `ResolvedFile { rel_path: String }` private; public accessor `rel_path()` only. (B1, Q4) |
| D5 | `ResolvedCorpus { workspace: PathBuf, files: Vec<ResolvedFile> }` private; `len`, `is_empty`, `iter`. Sole constructor `resolve_corpus`. (B1, Q2) |
| D6 | `read_corpus(&F, &ResolvedCorpus)`: re-lstat, re-canonicalize, re-check containment, then read. (B1, Q8) |
| D7 | `rel_path` is canonical-relative, `/`-joined, UTF-8 validated. (Q3) |
| D8 | All `FsError` payload paths are display-form `String`s; one lift function `FsError::io(workspace, site, fault)`. (Q7) |
| D9 | Complete taxonomy in unit 1; private-state rule; staged arm completion. (B4, Q5) |
| D10 | Channel-bundling rule: an `Io` channel test lives in the red unit that drives its call's introduction. (Q6) |
| D11 | Phase 3 does **not** sort canonical roots; traversal follows configured order until the final sort. (Q6) |
| D12 | `RootIsWorkspaceRoot` variant; Windows-spelling claim withdrawn. (Q9) |
| D13 | One `compile_fail` doctest proves unforgeability; the CI gap is disclosed. (Q1) |
| D14 | `# Errors` docs, `#[must_use]`, and unit-returning `FakeFs` mutators. (Q10) |

## Explicit non-goals

G0 introduces **no product runtime behaviour**, no assertion semantics, no CI
workflow change, no new external dependency, and no change to any file outside
the five it creates or the two manifests it amends. G1 and G2 are not touched.
