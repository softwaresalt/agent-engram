---
type: deliberation
date: 2026-09-13
package: G0
depends_on: none
source: docs/decisions/2026-09-13-package-g-split-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
---

# Package G0 — contained test-filesystem seam (deliberation)

## Authority and scope

Inherits `docs/decisions/2026-09-13-package-g-split-deliberation.md` in full.
Decisions 2 (topology T3), 3 (seam contract), and 6 (inherited, not re-argued)
of that document are **binding and not reopened here**. This deliberation
settles only the questions the split deliberation explicitly left to G0.

G0 is the program's in-degree-0 root. It has **no prerequisite** and is
independently shippable.

## Problem

Future contract tooling needs a way to turn a set of allow-listed repository
directories into a deterministic, fully enumerated, safely read corpus of files
— and to prove, in CI, on Windows, without elevation, that it cannot be tricked
into reading a substituted tree or into silently omitting a file.

v3 failed because its seam could express neither proof. `std::fs::canonicalize`
follows a symlinked root, so an allow-listed root replaced by a symlink was
traversed rather than rejected (finding A); and canonicalization was not routed
through the injectable boundary, so the canonical-escape case could only be
produced with a real on-disk symlink — which on Windows requires elevation or
developer mode, the exact thing the plan forbade (finding B).

G0 exists to make both proofs mechanical.

## Evidence

Inherited from the split deliberation: S1–S14, as corrected by S10a.
Additional G0-specific evidence gathered this session:

| # | Evidence | Value | Source |
|---|---|---|---|
| G0-E1 | `tempfile` is a regular root dependency, available to test targets | `tempfile = "3"` | `Cargo.toml` |
| G0-E2 | Member-crate precedent uses `publish = false` and its own `[dependencies]` | `crates/engram-indexer/Cargo.toml` | same |
| G0-E3 | No crate in `crates/` currently depends only on `std` | both existing members have dependencies | `crates/*/Cargo.toml` |
| G0-E4 | Root crate lints | `#![forbid(unsafe_code)]`, `#![warn(clippy::pedantic)]` at `src/lib.rs:10-11` | `src/lib.rs` |

## Question 1 — how does `FakeFs` model the filesystem?

### Option F-1 — string-keyed map of path → content, directories implied

* Pro: minimal.
* Con: cannot represent a symlink, a directory that is not a prefix of any file,
  an unreadable path, or a canonicalization that differs from the input. Five of
  the six fault classes G0 must prove are inexpressible. **Rejected.**

### Option F-2 — explicit node map plus an explicit canonicalization map *(SELECTED)*

`FakeFs` holds two independent maps:

1. **Node map**: `PathBuf → Node`, where `Node` is `File(String)`, `Dir`,
   `Symlink`, or `Unreadable(FsErrorKind)`. `symlink_metadata` and `read_dir`
   consult this map and **never follow** a `Symlink` node — that is what makes
   lstat semantics modellable at all.
2. **Canonicalization map**: `PathBuf → PathBuf`, consulted by `canonicalize`,
   defaulting to identity. This is the decisive piece: a root that is lexically
   inside the workspace but canonicalizes to a path outside it is expressed as a
   single map entry — **no on-disk symlink, no junction, no elevation, no
   developer mode, identical on Windows and Unix.** This is the direct,
   structural answer to v3 finding B.

Every one of the six required fault classes becomes a map entry:

| Fault class | How `FakeFs` expresses it |
|---|---|
| root symlink | node map: root → `Symlink` |
| entry symlink | node map: child → `Symlink` |
| canonical escape | canonicalization map: root → a path outside the workspace |
| unreadable path | node map: path → `Unreadable(...)` |
| disappearing file | node map entry removed between resolve and read (`FakeFs::remove`) |
| Windows behaviour | canonicalization map: workspace root → a `\\?\`-prefixed path |

* Pro: total expressiveness for exactly the fault set, no more; pure in-memory;
  deterministic; privilege-free; platform-independent.
* Con: two maps instead of one. Accepted — the second map is what buys finding B.

### Option F-3 — a real temp directory with privileged symlink creation

* Pro: highest fidelity.
* Con: reproduces v3 finding B exactly. Requires Windows elevation or developer
  mode; CI determinism becomes environment-dependent. **Rejected.**

**Selected: F-2.**

## Question 2 — how is the lstat-before-canonicalize *ordering* proven, not merely implemented?

An implementation can be correct and its plan still "claim a proof it cannot
deliver" — that was v3's terminal criticism. Ordering is a temporal property, so
asserting only the returned error is insufficient: `RootIsSymlink` could be
returned by an implementation that canonicalized first and checked afterwards,
which would already have followed the symlink.

### Option O-1 — assert only the returned error variant

* Con: proves the verdict, not the ordering. The fail-open is a *traversal* that
  happens before the verdict. **Rejected as insufficient.**

### Option O-2 — `FakeFs` records every call; the test asserts the call log *(SELECTED)*

`FakeFs` keeps a `RefCell<Vec<FsCall>>` recording each seam invocation with its
path. The root-symlink test then asserts **both**:

1. the result is `Err(FsError::RootIsSymlink { .. })`, and
2. the call log contains `SymlinkMetadata(root)` and contains **no**
   `Canonicalize(_)` and **no** `ReadDir(_)` at all.

Claim (2) is the actual safety property: the symlinked root was never
canonicalized and never traversed. It is a positive, mechanical proof of
ordering, and it fails loudly if a future refactor reorders the checks.

`RefCell` is `std`; interior mutability is required because `FileAccess` methods
take `&self`. No `unsafe`, no new dependency.

* Pro: converts an ordering claim into an assertion.
* Con: `FakeFs` gains a small observability surface. Accepted — it is `pub`, and
  it is exercised, so it is not dead code.

**Selected: O-2.**

## Question 3 — what is in the seam, and what is deliberately not?

Binding from split-deliberation Decision 3: exactly four methods —
`symlink_metadata`, `canonicalize`, `read_dir`, `read_to_string`.

Settled here, against the operator's "metadata/type/size if used" instruction:

* **File size is NOT exposed.** No resolver decision consults size, because
  there is no max-filesize policy (inherited Decision 6.1, affirmed 7/7 in v3).
  An unused accessor would be speculative surface, contrary to Constitution VI.
* **File type IS exposed**, but only as the lstat-derived `FileKind` returned by
  `symlink_metadata` and carried on each `read_dir` entry. This is not optional:
  rejecting a symlink entry *before following it* is impossible without it.
* **Modification time, permissions, and `metadata` (stat, following) are NOT
  exposed.** Nothing consults them, and `metadata` in particular follows
  symlinks, so exposing it would create a second fail-open surface beside the
  one G0 exists to close.
* **No mutating operation is exposed at all.** There is no create, write,
  remove, or rename in the trait. "G0 performs no writes" is therefore a
  structural property of the type, provable by inspection rather than by test.

## Question 4 — path identity and ordering determinism

### Option P-1 — carry `PathBuf`, order by `OsStr`

* Con: ordering for non-ASCII names can differ between Windows (UTF-16) and Unix
  (bytes). The claim "deterministic ordering" would be true for today's corpus
  and false in general — the kind of overstatement v3 was faulted for.
  **Rejected.**

### Option P-2 — carry a UTF-8 workspace-relative `String` with `/` separators; reject non-UTF-8 paths *(SELECTED)*

Each resolved file carries `rel_path: String`, built from its
workspace-relative components joined with `/` regardless of platform, plus the
absolute `PathBuf` for reading. Ordering is `String` `Ord`, which is byte-wise
and therefore **identical on every platform**.

A path component that is not valid UTF-8 is **rejected** with
`FsError::NonUtf8Path` rather than lossily converted. Lossy conversion would map
two distinct files to the same key — a silent-collision fail-open, which is the
defect class G0 exists to eliminate. Rejection is fail-closed and deterministic.

* Pro: the determinism claim is unconditionally true, not corpus-conditional.
* Con: one extra error variant. Accepted; it is exercised by a named unit.

**Selected: P-2.**

## Question 5 — containment comparison

Settled, closing v3 advisory A-9: containment uses **component-wise
`Path::starts_with`**, never string-prefix comparison. String prefixing
spuriously rejects Windows extended-length canonical paths (`\\?\C:\...`) and
spuriously accepts a sibling directory whose name extends the root's name
(`.../agents-evil` vs `.../agents`). Component-wise comparison is correct in
both directions. A Windows extended-length root is a named test scenario.

## Failure taxonomy (filesystem and discovery only)

Per the operator's instruction, G0's taxonomy contains **no assertion or
registry semantics**. There is deliberately **no `EmptySet` variant**: an empty
corpus is a legitimate G0 result, and judging it is G1's concern.

| Variant | Fires when |
|---|---|
| `RootMissing` | an allow-listed root does not exist |
| `RootNotDirectory` | it exists but is a regular file |
| `RootIsSymlink` | lstat of the joined root reports a symlink — **before** canonicalization |
| `RootOutOfWorkspace` | the canonical root is not component-wise contained in the canonical workspace root |
| `EntryIsSymlink` | an entry's lstat-derived kind is a symlink — rejected, never followed, never skipped |
| `NonUtf8Path` | a path component is not valid UTF-8 |
| `EnumerationFailed` | a `read_dir` invocation fails |
| `ReadFailed` | a `read_to_string` invocation fails, including disappear-between-discovery-and-read |

Every variant carries the offending path. Every variant is proven by a named
work unit (see the plan's Failure Semantics table).

## Inherited, not re-argued

Reproduced from split-deliberation Decision 6 and binding on G0:

1. No `max_depth`, no `max_filesize`, no walker filters. **Do not reopen.**
2. Plain `std::fs` recursion, never `ignore::WalkBuilder`.
3. Zero new external dependencies.
4. No `verify_markdown`, no `engram` coupling, no product runtime behaviour.
5. Test-first with compiling red phases; ≤3 scenarios per unit.
6. Two-layer containment: allow-listed roots plus workspace canonical
   containment.

## Independent usefulness

G0 ships a real library with a real public API: deterministic, contained,
exhaustively enumerated path discovery and reading over an injectable
filesystem boundary. It has no notion of assertions, registries, contracts, or
this repository's harness layout. Any future repository tool needing
"enumerate these directories safely and deterministically, and prove it" can
depend on it. Its independence is not a packaging claim — G1 is the first
consumer, and G0 is complete and green before G1 exists.
