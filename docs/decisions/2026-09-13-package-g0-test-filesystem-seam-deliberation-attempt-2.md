---
type: deliberation
date: 2026-09-13
package: G0
attempt: 2
supersedes: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md
depends_on: none
source: docs/decisions/2026-09-13-package-g-split-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_review: docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 0cd3d957
---

# Package G0 attempt 2 — contained test-filesystem seam (deliberation)

## Authority and scope

This deliberation **supersedes**
`docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md`
(attempt 1). The superseded document is preserved unmodified as evidence.

It inherits, and does **not** reopen:

* `docs/decisions/2026-09-13-package-g-split-deliberation.md` Decisions 2
  (topology T3), 3 (four-method seam), and 6 (inherited constraints).
* `docs/decisions/2026-09-13-package-g-split-program-decision.md` gate policy.
* Every item in the attempt-1 review record's **"What this review SETTLES"**
  section (items 1–9). Those are affirmed and inherited, not re-argued: the
  split itself, root-symlink fail-open closure, injectable canonicalization,
  the four-method seam width, the no-bounds decision, the Constitution Check
  mapping, zero new *external* dependencies, and scope discipline.

G0 remains the program's in-degree-0 root: **no prerequisite, independently
shippable.**

## Why attempt 1 failed

Two architecture-level P1 findings, both contract defects rather than wording
defects:

* **Finding A** — `FsError` had no channel for a *failing* `symlink_metadata`
  or a *failing* `canonicalize`. Two of four seam methods could fail in ways
  the taxonomy could not express. Raised independently by two reviewers on two
  models.
* **Finding B** — corpus identity was undefined. Duplicate or nested
  allow-listed roots emitted the same file twice, so G0's "corpus" was a
  multiset. G1's `exactly_one` assertion would then report a false
  `AmbiguousMultiplicity` caused entirely by G0.

This deliberation settles exactly those two contracts, plus the one tension the
review left explicitly open (`FileKind::Other`, A-5 vs M-8). Everything else is
inherited.

## Evidence

Inherited: S1–S14 (as corrected by S10a) and G0-E1..G0-E4 from attempt 1.
Re-verified read-only this session against HEAD `0cd3d957`:

| # | Evidence | Value | Source |
|---|---|---|---|
| E1 | Workspace members | `[".", "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml:1-3` |
| E2 | Member-crate precedent | `publish = false`, `edition = "2024"`, `rust-version = "1.85"` | `crates/engram-indexer/Cargo.toml` |
| E3 | `-Dwarnings` is a build-wide rustflag | `[build] rustflags = ["-Dwarnings"]` | `.cargo/config.toml` |
| E4 | `lint` alias is `--all-features` | `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` | `.cargo/config.toml` |
| E5 | `dev-test` alias | `test --all-targets` | `.cargo/config.toml` |
| E6 | Declared `[[test]]` targets | 267, all explicit `name` + `path`; `autotests` absent | `Cargo.toml` |
| E7 | `unit_*` target naming with `tests/unit/*_test.rs` paths | e.g. `unit_query_stats` → `tests/unit/query_stats_test.rs` | `Cargo.toml:216-217` |
| E8 | Coverage-oracle completeness rule | "FAIL if any `[[test]]` target is unmapped by a src/crates surface" | `scripts/test-coverage-oracle.ps1:13-14` |
| E9 | A `crates/` surface precedent exists | `path = "crates/powerbi-tmdl-parser/"`, `targets = ["contract_*", "integration_*", "unit_*", "cold_*", "helpers_*"]` | `.cargo/test-coverage-manifest.toml:137-139` |
| E10 | Repository error convention | `thiserror::Error` derive, struct-style variants with named fields, structured payloads (`PathBuf`/`String`), no embedded `io::Error` | `src/errors/mod.rs:13,19-44` |
| E11 | `thiserror = "1"` is already a root dependency | resolved in the workspace graph today | `Cargo.toml` |
| E12 | `tempfile = "3"` is already a root dependency | available to test targets | `Cargo.toml` |

## Question 1 — what must the failure channel express? (closes finding A)

The defect is not "a variant is missing". It is that the plan's Failure
Semantics table **claimed completeness it did not have**. Any repair must be
complete by construction, not by adding variants until no reviewer objects.

Enumerate every fallible thing the resolver can do. There are exactly four seam
operations, invoked at exactly three sites.

| Site | `symlink_metadata` | `canonicalize` | `read_dir` | `read_to_string` |
|---|---|---|---|---|
| workspace root | not invoked (trust anchor, Q4) | **invoked** | not invoked | not invoked |
| allow-listed root | **invoked** | **invoked** | **invoked** | not invoked |
| entry | **invoked** | **invoked** (Q2) | **invoked** | **invoked** |

Every cell marked *invoked* needs a failure channel. That is 8 channels, and
the table is closed because the seam has exactly four methods and the resolver
has exactly three sites.

### Option A-1 — one operation-failure variant per site/operation pair

Eight new variants (`RootSymlinkMetadataFailed`, `RootCanonicalizeFailed`,
`EntrySymlinkMetadataFailed`, …).

* Pro: maximally explicit; each channel is its own name.
* Con: eight near-identical variants carrying identical payloads. The
  Maintainability reviewer flagged speculative surface in v3; this is the
  same defect with the opposite sign — real surface, mechanically duplicated.
  **Rejected.**

### Option A-2 — one stringly `Io(String)` catch-all

* Con: the operator explicitly forbids "broad stringly catch". It also destroys
  the site/operation distinction the finding demands. **Rejected.**

### Option A-3 — one structured variant discriminated by site and operation *(SELECTED)*

```rust
pub enum FsSite { WorkspaceRoot, Root, Entry }
pub enum FsOp   { SymlinkMetadata, Canonicalize, ReadDir, ReadToString }

Io { site: FsSite, operation: FsOp, path: PathBuf, kind: std::io::ErrorKind }
```

One variant, three orthogonal discriminants, one contained path. Every one of
the eight channels is a distinct, matchable, assertable `(site, operation)`
pair. Adding a seam method or a site would force a new enum variant in `FsOp`
or `FsSite` and break every exhaustive match — so the taxonomy cannot silently
fall behind the seam again, which is the exact regression that produced
finding A.

**Who supplies `site`.** The seam adapter knows which *operation* failed on
which *path*; it does not know why the resolver called it. So the trait returns
a site-less `FsFault { operation, path, kind }`, and the resolver lifts it with
a single total constructor `FsError::io(site, fault)`. There is exactly one
construction path for `Io`, and it is impossible to produce one without naming
a site — so an implementor of `FileAccess` cannot mislabel the site, because it
is never handed one.

* Pro: complete by construction; no duplication; each channel independently
  injectable and assertable; `PartialEq`-derivable so tests assert the whole
  value rather than a discriminant.
* Con: a consumer matching on `Io` must inspect fields to distinguish channels.
  Accepted — that is precisely the information the finding says must survive.

**Selected: A-3.**

### The `RootMissing` specialization — stated as a rule, not a fallback

`RootMissing` remains a distinct variant, because "this allow-listed root does
not exist" is a configuration error with different meaning to the caller than
"the filesystem refused the call". The mapping rule is **narrow, explicit, and
tested**:

> Only `ErrorKind::NotFound`, returned by `symlink_metadata` at site `Root`,
> becomes `RootMissing`. Every other `ErrorKind` at that site becomes
> `Io { site: Root, operation: SymlinkMetadata, .. }`.

A named red/green pair asserts that a `PermissionDenied` root lstat produces
`Io`, **not** `RootMissing`. This is the direct, mechanical answer to the
finding's sharpest sentence ("mapping an `EACCES` on `symlink_metadata` to
`RootMissing` would be a false statement").

### Source preservation

The operator permits "source `std::io::Error` **or** a stable testable error
kind consistent with repository conventions". We carry
`std::io::ErrorKind`, because:

1. `std::io::Error` is neither `PartialEq` nor `Clone`, so carrying it would
   forbid `#[derive(PartialEq)]` on `FsError` and force every test to match on
   shape instead of asserting a value. The whole point of this package is
   mechanical assertability.
2. `ErrorKind` is `Copy + Eq + Debug` and is stable across platforms for the
   kinds we assert (`NotFound`, `PermissionDenied`).
3. It matches E10: the repository's own `EngramError` tree carries structured
   fields and does **not** embed `io::Error` in its variants.

`FsError` implements `std::error::Error`; `source()` returns `None` because no
inner error is retained. That is stated, not left implicit.

### Metadata / type / size

The operator's channel 6 ("metadata/type/size failure if retained") is
**folded into `SymlinkMetadata` and recorded as folded, not silently dropped**:

* **type** is retained, and is obtained *only* from `symlink_metadata` (and
  from `read_dir` entry kinds, which are lstat-derived). Its failure channel is
  `(site, SymlinkMetadata)`.
* **size** is **not retained** — inherited Decision 6.1 removes all size policy,
  so a size accessor would be unexercised surface (Constitution VI).
* **following `metadata`** is not in the seam at all; exposing it would create a
  second fail-open beside the one G0 exists to close.

### Taxonomy boundary

The taxonomy stays inside G0's filesystem/discovery semantics. There is **no**
`EmptySet`, no assertion semantics, no registry semantics. An empty corpus is a
valid G0 result; judging it is G1's concern.

## Question 2 — what makes the corpus a set? (closes finding B)

The operator's constraint is absolute: **a set, never a multiset**, and a
duplicate identity is a *conflict to report*, not a duplicate to silently drop.

### Option B-1 — deduplicate silently by normalized `rel_path`

* Con: directly forbidden ("do not dedupe silently"). It also hides a real
  misconfiguration: if two roots resolve to the same subtree, the allow-list is
  wrong and the operator should learn that, not get a quietly-correct answer.
  **Rejected.**

### Option B-2 — reject at the root set, key files by canonical path, fail on collision *(SELECTED)*

Three layers, in this order:

1. **Canonicalize and validate every configured root before any traversal.**
   Root validation is a complete phase that finishes before the first
   `read_dir`. Attempt 1 validated and traversed each root in one pass, which
   made cross-root properties unexpressible.
2. **Reject a duplicate or overlapping canonical root set.**
   * two roots with the *same* canonical path ⇒ `DuplicateRoot`
   * one canonical root component-wise contained in another ⇒
     `OverlappingRoots`
   Comparison is on **canonical** roots, never on the configured spellings.
3. **Key every discovered file by its canonical path.** A second occurrence of
   an already-seen canonical path ⇒ `DuplicateFileIdentity`, carrying the
   canonical path and both display paths. Never deduped, never emitted twice.

* Pro: the set property is enforced at the only two places multiplicity can
  enter — the root set and the file set. Each layer has a named error and a
  named test.
* Con: files are canonicalized, which adds a seam call per file and a failure
  channel (`(Entry, Canonicalize)`). Accepted: that channel is *required* by
  finding A anyway, and canonical identity is the only identity that is stable
  across spelling and case.

**Selected: B-2.**

### Why canonical-root comparison also closes the attempt-1 C-1 dissent

The Correctness reviewer's downgraded C-1 finding was that
`symlink_metadata(joined_root)` does not detect a symlinked *intermediate*
component, so two configured roots could silently denote the same subtree via a
symlinked ancestor. Under B-2 that case is no longer silent: both roots
canonicalize to the same path and `DuplicateRoot` fires. The dissent is closed
by mechanism, not by argument — while the majority position (workspace root is
a caller-supplied trust anchor) is *also* documented explicitly, as both the
Architecture and Security reviewers requested.

### Overlap detection algorithm

Full pairwise comparison over the canonical root set, in sorted canonical
order. `n` is the number of configured roots (5 today, bounded by
configuration, not by data). An O(n log n) adjacent-pair argument exists but
requires a lexicographic-prefix proof; the pairwise form is obviously correct
and needs none. Determinism comes from sorting first, so the *reported* pair in
a multi-conflict allow-list is stable.

### Identity versus display — two separate fields

```rust
pub struct ResolvedFile {
    pub rel_path: String,       // display: workspace-relative, '/'-joined, stable
    pub canonical_path: PathBuf // identity: canonical, platform-true
}
```

* `rel_path` is derived from the **lexical** traversal path relative to the
  workspace root, joined with `/` on every platform. It is the stable,
  human-meaningful, platform-independent display key, and it is what ordering
  is computed on (byte-wise `String` `Ord`, identical everywhere).
* `canonical_path` is the identity key. Collisions on it are the conflict.

They are deliberately not the same field. Attempt 1 conflated them into
`rel_path` alone, which is why identity was undefined.

### What G1 receives

The G0→G1 contract is stated explicitly so `exactly_one` cannot be poisoned:

> In the returned `Vec<ResolvedFile>`, `canonical_path` is unique (enforced by
> `DuplicateFileIdentity`) and `rel_path` is unique and strictly increasing
> (derived: non-overlapping canonical roots produce disjoint lexical subtrees;
> asserted directly by a named test). **One logical entry per file. Never
> false multiplicity originating in G0.**

## Question 3 — Windows case behaviour

Windows default filesystems are case-insensitive and case-preserving; Linux and
macOS-on-APFS-case-sensitive are not. A rule must be correct on both without a
`cfg` fork in the resolver.

### Option W-1 — case-fold paths before comparison

* Con: wrong on Linux, where `Agents/` and `agents/` are genuinely two
  directories; folding would merge them and report a phantom `DuplicateRoot`.
  It also invents a Unicode case-folding policy G0 has no business owning.
  **Rejected.**

### Option W-2 — `cfg(windows)` case-insensitive comparison

* Con: the comparison rule becomes platform-forked, so the FakeFs proofs no
  longer test the same code path on both platforms — reintroducing exactly the
  environment-dependence that v3 died of. **Rejected.**

### Option W-3 — exact comparison of canonical paths; the platform decides *(SELECTED)*

`canonicalize` returns the true on-disk form. On Windows, `.github/Agents` and
`.github/agents` both canonicalize to the single real on-disk spelling, so
exact comparison finds them equal and `DuplicateRoot` fires. On Linux they are
distinct files, canonicalize differently, and are correctly treated as two
roots. **The resolver contains no case logic at all**; platform semantics enter
only through the seam, which is the one boundary G0 already injects.

`FakeFs` models this deterministically: the Windows case is expressed by adding
two canonicalization-map entries that map both spellings to one canonical value.
That scenario runs identically on `ubuntu-latest` and `windows-latest`, with no
elevation, no developer mode, and no real case-insensitive volume.

Containment comparison stays component-wise `Path::starts_with` on canonical
paths (inherited Decision, attempt-1 Question 5), which is correct for both
Windows extended-length (`\\?\C:\…`) prefixes and sibling-name near-misses
(`.github/agents-evil` vs `.github/agents`).

**Selected: W-3.**

## Question 4 — root grammar and the trust anchor

Attempt-1 mechanical findings M-4 and M-16 (unvalidated root grammar; undefined
input invariants) converge here.

**Accepted root grammar** — validated lexically, **before any seam call**:

> A configured root is a non-empty relative path every one of whose components
> is `Component::Normal`.

This single rule rejects the empty string, `.` (`CurDir`), `..` (`ParentDir`),
`/foo` (`RootDir`), and `C:\foo` or `\\?\C:\foo` (`Prefix`) — with
`RootNotRelative`, and with **zero filesystem calls**, asserted through the call
log. Attempt 1 lexically joined such roots and then lstat'd them, touching the
filesystem outside the workspace before containment was checked.

**Trust anchor, stated explicitly** (attempt-1 advisory A-2, requested by both
Architecture and Security):

> The workspace root is a **caller-supplied trust anchor**. It is canonicalized
> but is *not* lstat-checked, because it is not attacker-influenced corpus
> content. G0 does not defend against a caller that supplies a malicious
> workspace root; it defends the corpus *inside* a trusted workspace root.

**Cycle policy** (M-16): directory cycles require a symlink, junction, or other
reparse point. `symlink_metadata` reports all of these as `FileKind::Symlink`,
and a symlink entry is rejected with `EntryIsSymlink` **before** being followed.
Hard links to directories are not creatable on the supported platforms. A cycle
is therefore structurally unreachable in the traversed graph — not bounded by a
depth limit, which inherited Decision 6.1 forbids.

## Question 5 — `FileKind::Other` (resolves the A-5 / M-8 tension)

The attempt-1 review left this open and explicitly demanded a deliberate
resolution: advisory A-5 argued `Other` is speculative (Git cannot track FIFOs,
sockets, or device nodes, so it is unreachable against `RealFs` in this corpus);
mechanical findings M-1 and M-8 argued the opposite way — the `FileKind` match
must be exhaustive with no wildcard arm, and `FakeFs` had no producer for it.

**Resolution: keep `FileKind::Other`, and give it an explicit `FakeFs`
producer.**

* Dropping it would make `FileKind` a three-variant enum. `RealFs` must then map
  a real `FileType` that is neither file, dir, nor symlink onto one of the
  three — which is a silent misclassification, the exact fail-open class G0
  exists to eliminate. There is no honest three-variant mapping.
* Keeping it without a producer (attempt 1) makes the driving test
  unimplementable, which is M-8.
* So: `FakeFs::other(path)` is a first-class builder method, `EntryNotReadable`
  is produced by a named scenario, and the match over `FileKind` is exhaustive
  with **no wildcard arm**, so a future variant cannot be silently dropped.

Honesty is preserved by the two-column reachability table: `EntryNotReadable`
is *runtime-reachable* through `RealFs` (a device node inside a root would
produce it) but *not deterministically inducible* against `RealFs` in a Git
corpus — inducible only through `FakeFs`. Recording both columns separately is
what attempt 1's table did right and is retained.

## Question 6 — `thiserror`, and what "zero new dependencies" now means

Attempt 1 claimed the crate would have "no `[dependencies]` section at all".
Repository convention (E10) is `thiserror::Error` with struct-style variants —
the entire `EngramError` tree is built that way. `FsError` has 12 variants with
named fields and needs `Display`; hand-writing that is ~60 lines of boilerplate
that diverges from every other error type in the repository.

`thiserror = "1"` is **already a resolved root dependency** (E11). Depending on
it from the G0 crate adds **zero new external crates to the workspace
dependency graph** and leaves `Cargo.lock` and `cargo audit` output unchanged.

The inherited constraint is "zero new external dependencies" (split Decision
6.3). It is satisfied in substance. The attempt-1 phrasing ("no `[dependencies]`
section") was a stricter, self-imposed *purity* claim that bought nothing and
cost convention alignment. **Selected: depend on `thiserror = "1"`, and state
the distinction explicitly rather than letting a reviewer discover the change.**

Verification asserts the substance directly: `Cargo.lock` shows no added
package, and `cargo audit` is unchanged.

## Question 7 — test tier and lint enforcement

**Tier.** Attempt-1 advisory A-6 correctly found that calling `tests/contract/`
"the mandated contract tier" over-reads Constitution II, whose contract tier is
*MCP tool response verification*. G0's tests are isolated logic over an injected
seam, which is exactly this repository's definition of the **unit** tier. The
exerciser therefore lives at `tests/unit/agent_contract_fs_test.rs` with target
name `unit_agent_contract_fs`, matching E7's naming convention. The placement is
justified by the tier definition, and A-6's over-claim is not reproduced.

**Lint enforcement.** Attempt-1 mechanical finding M-9 observed that
`#![warn(clippy::pedantic)]` inside the crate may be decorative, because the
root `cargo lint` alias's package selection may not include a workspace member
compiled as a dev-dependency. Rather than assert which mechanism applies — an
assertion this deliberation cannot verify without writing code — G0 takes M-9's
**stronger** repair option and makes enforcement unconditional:

```text
cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic
```

is an explicit, named Verification command. Enforcement is then true by
construction regardless of how default member selection resolves, and no
Constitution Check row claims coverage that the gate set does not contain.

## Inherited, not re-argued

Binding on G0 attempt 2, reproduced from split-deliberation Decision 6:

1. No `max_depth`, no `max_filesize`, no walker filters. **Do not reopen.**
2. Plain `std::fs` recursion, never `ignore::WalkBuilder`.
3. Zero new external dependencies (see Q6 for the exact reading).
4. No `verify_markdown`, no `engram` coupling, no product runtime behaviour.
5. Test-first with compiling red phases; ≤3 scenarios per unit.
6. Two-layer containment: allow-listed roots plus workspace canonical
   containment.

## Failure taxonomy

Filesystem and discovery semantics only. No assertion or registry semantics.

| Variant | Fires when |
|---|---|
| `RootNotRelative` | a configured root is empty or has a non-`Normal` component — **before any seam call** |
| `RootMissing` | root `symlink_metadata` returns `ErrorKind::NotFound` |
| `RootNotDirectory` | root lstat kind is `File` or `Other` |
| `RootIsSymlink` | root lstat kind is `Symlink` — **before** canonicalization |
| `OutOfWorkspace` | a canonical root (`site: Root`) or canonical file (`site: Entry`) is not component-wise contained in the canonical workspace root |
| `DuplicateRoot` | two configured roots share one canonical path |
| `OverlappingRoots` | one canonical root is component-wise contained in another |
| `DuplicateFileIdentity` | two discovered files share one canonical path |
| `EntryIsSymlink` | an entry's lstat kind is `Symlink` — rejected, never followed, never skipped |
| `EntryNotReadable` | an entry's lstat kind is `Other` — rejected, never dropped |
| `NonUtf8Path` | a path component is not valid UTF-8 |
| `Io` | any seam operation fails, discriminated by `site` × `operation`, carrying the contained path and `std::io::ErrorKind` |

Twelve variants. Every one carries the offending path. Every one is proven by a
named work unit.

## Independent usefulness

G0 ships a real library with a real public API: deterministic, contained,
exhaustively enumerated, **set-valued** path discovery and reading over an
injectable filesystem boundary, with a failure channel that is complete for
every operation the seam can perform. It has no notion of assertions,
registries, contracts, or this repository's harness layout. Any future tool
needing "enumerate these directories safely, deterministically, exactly once
each, and prove it" can depend on it. G1 is the first consumer, and G0 is
complete and green before G1 exists.
