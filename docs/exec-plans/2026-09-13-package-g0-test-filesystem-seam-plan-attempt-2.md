---
type: exec-plan
date: 2026-09-13
package: G0
attempt: 2
supersedes: docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan.md
depends_on: none
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-2.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_review: docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
branch: chore/checkpoint-resolution-ordering-restage
head: 0cd3d957
requires_plan_hardening: yes
plan_status: blocked
review_verdict: FAIL
review_rounds: 1
review_record: docs/closure/2026-09-13-package-g0-attempt-2-plan-review-record.md
blocked_by: "Four architecture-level P1 clusters, three with multi-model consensus: (A) read_corpus performs no containment check and ResolvedFile's public fields make its input forgeable, so the Constitution IV claim is not delivered by the mechanism - raised independently by Rust, Correctness and Security; (B) the failure channels are not independently injectable because FakeFs::unreadable is pinned to symlink_metadata only, and the ReadToString channel has no proving scenario at all - raised independently by Rust and Maintainability; (C) the entry symlink_metadata channel does not exist during traversal since Phase 4 uses DirEntry.kind, leaving G0.21(c)/G0.22 with no driving operation - raised by Correctness and Rust; (D) red/green staging is broken because -Dwarnings forces a [G] unit introducing a seam call to wire its failure mapping in the same commit, so G0.13(c), G0.21(a) and G0.21(c) pass green before their owning unit ships - raised independently by Maintainability and Correctness. Plus P1 mechanical finding E: the declared FsError is not compilation-complete (missing nested derives and per-variant #[error] attributes). Correction budget was restricted to mechanical findings and did not open."
harvested: false
---

# Package G0 attempt 2 — contained test-filesystem seam (implementation plan)

## Source Document

`docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-2.md`,
which supersedes the attempt-1 deliberation and inherits
`docs/decisions/2026-09-13-package-g-split-deliberation.md` (Decisions 2, 3, 6
binding) under program decision
`docs/decisions/2026-09-13-package-g-split-program-decision.md`.

This plan **supersedes**
`docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan.md` (attempt 1),
which is preserved unmodified as evidence alongside its review record
`docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md`.

## Objective

Ship a dev-only workspace-member library crate that turns a list of allow-listed
repository-relative directories into a deterministic, exhaustively enumerated,
**set-valued**, safely read corpus — through an injectable filesystem boundary
that makes the root-symlink, canonical-escape, entry-symlink, unreadable-path,
non-UTF-8, duplicate-identity, disappearing-file, substituted-file, and
Windows-path cases **mechanically provable without platform privileges** — and
whose failure channel is **complete for every operation the seam can perform**.

G0 introduces **no product runtime behaviour** and **no assertion semantics**.

## What changed from attempt 1

| Attempt-1 finding | Sev | Where closed in this plan |
|---|---|---|
| **A** — `FsError` cannot represent a failing `symlink_metadata` or `canonicalize` | P1 arch | Deliberation Q1; `Io { site, operation, path, kind }`; Failure Semantics; units G0.11, G0.13, G0.21, G0.23, G0.29 |
| **B** — corpus identity undefined; duplicate/nested roots yield a multiset | P1 arch | Deliberation Q2; three-layer set enforcement; units G0.17, G0.18, G0.23, G0.27 |
| **C-1** — workspace root canonicalized without lstat (downgraded P3) | P3 | Documented as trust anchor in H6; *and* closed by mechanism — canonical-root comparison (G0.17) detects the symlinked-intermediate collision |
| **M-1** — red/green staging contradictions; two units claim one obligation | P0 | Units re-cut: 30 units / 15 pairs, one obligation per unit; see "Obligation ownership" table |
| **M-2** — `read_corpus` happy path asserted nowhere | P1 | G0.29(a) |
| **M-3** — red-phase stubs not warning-clean | P1 | "Red-phase mechanism": `_`-prefixed params mandated |
| **M-4** — root grammar unvalidated before first seam call | P1 | G0.9/G0.10, with a zero-call assertion |
| **M-5** — G0.9c does not prove what it claims (`Prefix` mismatch) | P2 | G0.15(c): both sides mapped to extended-length form; component-vs-string proof moved to the sibling-name case G0.15(b) |
| **M-6** — motivating adversarial fixture never instantiated | P2 | G0.15(b) uses `.github/agents-evil` vs `.github/agents` |
| **M-7** — TOCTOU substitution not covered by read failure | P2 | G0.29(c): re-lstat before each read; kind change rejected; limit stated in H6 |
| **M-8** — `FileKind::Other` has no `FakeFs` producer | P2 | Deliberation Q5; `FakeFs::other()` in G0.5/G0.6; consumed by G0.21(b) |
| **M-9** — crate pedantic lint is decorative | P2 | Explicit `cargo clippy -p agent-contract-fs …` in Verification; no Constitution row claims uncovered enforcement |
| **M-10** — `cargo audit` missing from Verification | P2 | Present, in constitution order |
| **M-11** — pair-count error (7 vs 8) | P2 | 30 units / **15 pairs**, stated identically everywhere |
| **M-12** — cross-reference errors in hardening | P2 | All hardening cross-references point at real unit IDs; manifest-addition counts restated exactly |
| **M-13** — dead-code claim over-stated | P2 | H4 narrowed to effective external reachability; private-state rule added |
| **M-14** — one scenario packs two outcomes | P2 | One asserted outcome per test, enforced by the Scenario convention; no scenario asserts two variants |
| **M-15** — grep proof narrower than its claim | P2 | H3 claim narrowed to the three checked constructs; the structural argument carries the rest |
| **M-16** — undefined input/traversal invariants | P2 | Deliberation Q4: root grammar, empty allow-list, `.`-root, cycle policy |
| **A-1** — positional tuple at the G0→G1 boundary | P3 | `read_corpus` returns `Vec<ReadFile>` with named fields |
| **A-2** — trust-anchor assumption omitted | P3 | H6 |
| **A-3** — `RealFs` symlink detection rests on the `std::fs` lstat contract | P3 | H2 |
| **A-4** — `FsError` path rendering unspecified | P3 | "Path rendering policy" |
| **A-5** — `FileKind::Other` possibly speculative | P3 | Deliberation Q5; two-column reachability |
| **A-6** — contract-tier over-claim | P3 | Tests live in `tests/unit/`; tier justified by the repository's own unit-tier definition |
| **A-7** — G0.1 described as containing no logic | P3 | G0.1 is described accurately: registration **plus** the declared type surface **plus** the first real test |
| **A-8** — principle IV claim over-broad for `RealFs` | P3 | Constitution row IV split into resolver and adapter claims |
| **A-9** — freeze-scope file count off by one | P3 | H-scope lists exactly 5 files, matching G0.1 |
| **A-10** — `too_many_lines` risk once lints close | P3 | H6 |
| **A-11** — G1 must not edit G0's crate | P3 | H5 |
| **A-12** — `FakeFs::calls()` should return a snapshot | P3 | `calls()` returns `Vec<FsCall>` by clone; G0.5(b) |
| **A-13** — H1 prose inaccurate | P3 | H1 rewritten to argue from the call log only |

## Verified Preconditions

Each verified read-only this session against HEAD `0cd3d957`.

| # | Precondition | Value | Source |
|---|---|---|---|
| P1 | Workspace members | `[".", "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml:1-3` |
| P2 | Member-crate precedent | `publish = false`, `edition = "2024"`, `rust-version = "1.85"` | `crates/engram-indexer/Cargo.toml` |
| P3 | `dead_code` is a hard error workspace-wide | `[build] rustflags = ["-Dwarnings"]` | `.cargo/config.toml` |
| P4 | `lint` alias includes `--all-features` | `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` | `.cargo/config.toml` |
| P5 | `dev-test` alias | `test --all-targets` | `.cargo/config.toml` |
| P6 | Coverage-oracle completeness rule | "FAIL if any `[[test]]` target is unmapped by a src/crates surface" | `scripts/test-coverage-oracle.ps1:13-14` |
| P7 | A `crates/` surface precedent exists | `path = "crates/powerbi-tmdl-parser/"` | `.cargo/test-coverage-manifest.toml:137-138` |
| P8 | That surface's globs include `unit_*` | `targets = ["contract_*", "integration_*", "unit_*", "cold_*", "helpers_*"]` | `.cargo/test-coverage-manifest.toml:139` |
| P9 | `unit_*` naming with `tests/unit/*_test.rs` paths | `unit_query_stats` → `tests/unit/query_stats_test.rs` | `Cargo.toml:216-217` |
| P10 | `autotests` absent; 267 explicit `[[test]]` targets | — | `Cargo.toml` |
| P11 | CI runs the **root package only** (no `--workspace`) | `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` | `.github/workflows/ci.yml` |
| P12 | `thiserror = "1"` is already a resolved root dependency | — | `Cargo.toml` |
| P13 | `tempfile = "3"` is already a root dependency | available to test targets | `Cargo.toml` |
| P14 | Repository error convention | `thiserror::Error`, struct-style variants, structured payloads, no embedded `io::Error` | `src/errors/mod.rs:13,19-44` |
| P15 | Harness corpus (the eventual G2 roots) | 92 files, all `.md`; max depth 2; max size 98,546 B | fresh enumeration |

**Consequence of P6 + P7 + P8.** The powerbi surface's `unit_*` glob would
already satisfy *completeness* for a target named `unit_agent_contract_fs`.
A dedicated `[[surface]]` for `crates/agent-contract-fs/` is nevertheless
required for *correctness*: without it, a change to the G0 crate would require
no test target at all. The surface is added for the selection rule, not to
satisfy completeness.

**Consequence of P11.** The G0 crate carries **no internal `#[cfg(test)]`
tests**. All tests live in the root-package `[[test]]` target, which CI already
runs. G0 therefore requires **no CI workflow change whatsoever**.

## Architecture

### Topology (created whole in G0.1; no later unit adds a file)

```text
crates/agent-contract-fs/
  Cargo.toml            # publish = false, edition 2024, rust-version 1.85, thiserror = "1"
  src/lib.rs            # the entire public surface
tests/unit/agent_contract_fs_test.rs   # the sole exerciser; [[test]] in root Cargo.toml
```

Root `Cargo.toml` gains **three** additions: one `[workspace] members` entry,
one `[dev-dependencies]` path entry, one `[[test]]` block.
`.cargo/test-coverage-manifest.toml` gains **one** `[[surface]]`.
Four manifest additions in total, across two manifest files.

### Dependency posture

The crate declares exactly one dependency: `thiserror = "1"`, which is already a
resolved root dependency (P12). **Zero new external crates enter the workspace
dependency graph**; `Cargo.lock` gains no package and `cargo audit` output is
unchanged. Verification asserts both directly. See deliberation Q6 for why the
attempt-1 "no `[dependencies]` section" purity claim was dropped.

### Crate-level lints

`src/lib.rs` opens with `#![forbid(unsafe_code)]` and
`#![warn(clippy::pedantic)]`. Enforcement is made unconditional by the explicit
`cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic`
command in Verification, so no claim depends on how default workspace member
selection resolves (closes M-9).

### Public type surface — every item has a named exerciser

| Item | Exercised by |
|---|---|
| `enum FileKind { File, Dir, Symlink, Other }` | G0.3, G0.5, G0.19, G0.21 |
| `struct DirEntry { file_name: OsString, kind: FileKind }` | G0.1, G0.19 |
| `enum FsSite { WorkspaceRoot, Root, Entry }` | G0.11, G0.13, G0.15, G0.21, G0.23 |
| `enum FsOp { SymlinkMetadata, Canonicalize, ReadDir, ReadToString }` | G0.5, G0.11, G0.13, G0.21, G0.23, G0.29 |
| `struct FsFault { operation: FsOp, path: PathBuf, kind: std::io::ErrorKind }` | G0.5, G0.7 |
| `enum FsError` (12 variants, each carrying the offending path) | G0.9, G0.11, G0.13, G0.15, G0.17, G0.21, G0.23, G0.25, G0.29 |
| `trait FileAccess` — `symlink_metadata`, `canonicalize`, `read_dir`, `read_to_string` | G0.1, G0.3, G0.7 |
| `struct RealFs` + `impl FileAccess` | G0.7 |
| `struct FakeFs` + builder (`dir`, `file`, `symlink`, `other`, `unreadable`, `canonical`, `remove`, `replace_with_symlink`) | G0.1, G0.3, G0.5, and every later `[R]` unit |
| `enum FsCall` + `FakeFs::calls() -> Vec<FsCall>` (cloned snapshot) | G0.5, G0.9, G0.11 |
| `struct ResolvedFile { rel_path: String, canonical_path: PathBuf }` | G0.19, G0.25, G0.27 |
| `struct ReadFile { rel_path: String, content: String }` | G0.29 |
| `fn resolve_corpus<F: FileAccess>(&F, &Path, &[&str]) -> Result<Vec<ResolvedFile>, FsError>` | G0.9 onward |
| `fn read_corpus<F: FileAccess>(&F, &[ResolvedFile]) -> Result<Vec<ReadFile>, FsError>` | G0.29 |

Every row is `pub` and reachable from the crate root. **No `allow(dead_code)`
appears anywhere in this package** — see H4 for the exact mechanism and its
limits.

### The seam — exactly four methods (inherited, not widened)

```rust
pub struct FsFault { pub operation: FsOp, pub path: PathBuf, pub kind: std::io::ErrorKind }

pub trait FileAccess {
    fn symlink_metadata(&self, path: &Path) -> Result<FileKind, FsFault>; // lstat; never follows
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsFault>;      // injectable
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsFault>;    // non-filtering; lstat-derived kinds
    fn read_to_string(&self, path: &Path) -> Result<String, FsFault>;
}
```

No size accessor, no following `metadata`, no mtime, no permissions, and **no
mutating operation of any kind**. "G0 performs no writes" is a structural
property of the trait, provable by inspection rather than by test.

**Why the seam returns `FsFault` and not `FsError`.** The *adapter* knows which
operation failed and on which path; it does **not** know why the resolver was
calling it. The *resolver* knows the call site. Splitting the two is what makes
`site` trustworthy: an implementation of `FileAccess` cannot mislabel a site
because it is never given one. The resolver lifts every fault with a single
total function,

```rust
impl FsError { fn io(site: FsSite, fault: FsFault) -> FsError { /* Io { site, .. } */ } }
```

so there is exactly one construction path for `Io`, and it is impossible to
originate an `Io` value without naming a site. `FsFault` is `pub` because
`FakeFs` fault injection and any future `FileAccess` implementor must construct
it.

### The error contract (closes finding A)

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FsError {
    RootNotRelative { root: String, component: String },
    RootMissing { path: PathBuf },
    RootNotDirectory { path: PathBuf, kind: FileKind },
    RootIsSymlink { path: PathBuf },
    OutOfWorkspace { site: FsSite, path: PathBuf, workspace: PathBuf },
    DuplicateRoot { canonical: PathBuf, first: String, second: String },
    OverlappingRoots { ancestor: PathBuf, descendant: PathBuf },
    DuplicateFileIdentity { canonical: PathBuf, first_rel: String, second_rel: String },
    EntryIsSymlink { path: PathBuf },
    EntryNotReadable { path: PathBuf },
    NonUtf8Path { path: PathBuf },
    Io { site: FsSite, operation: FsOp, path: PathBuf, kind: std::io::ErrorKind },
}
```

`std::io::ErrorKind` is carried rather than `std::io::Error` so `FsError` can
derive `PartialEq` and tests can assert whole values (deliberation Q1).
`source()` returns `None`; no inner error is retained, and that is stated rather
than implied.

**The eight failure channels, and where each is proven:**

| # | Channel | `(site, operation)` | Proven by |
|---|---|---|---|
| 1 | workspace-root canonicalize failure | `(WorkspaceRoot, Canonicalize)` | G0.13(c) |
| 2 | root lstat failure (non-`NotFound`) | `(Root, SymlinkMetadata)` | G0.11(b) |
| 3 | root canonicalize failure | `(Root, Canonicalize)` | G0.13(b) |
| 4 | directory enumeration failure | `(Entry, ReadDir)` | G0.21(a) |
| 5 | entry lstat failure | `(Entry, SymlinkMetadata)` | G0.21(c) |
| 6 | entry canonicalize failure | `(Entry, Canonicalize)` | G0.23(a) |
| 7 | file read failure | `(Entry, ReadToString)` | G0.29(b) |
| 8 | root lstat `NotFound` (specialized) | → `RootMissing` | G0.11(c) |

Channel 2 and channel 8 are the two halves of the narrow specialization rule:
**only** `ErrorKind::NotFound` at `(Root, SymlinkMetadata)` becomes
`RootMissing`; every other kind becomes `Io`. G0.11(b) asserts that a
`PermissionDenied` root lstat is **not** reported as `RootMissing`.

The operator's channel 6 ("metadata/type/size failure if retained") folds into
channels 2 and 5: **type** is retained and comes only from `symlink_metadata`;
**size** is not retained; following `metadata` is not in the seam. This is
recorded as folded, not silently dropped.

### Path rendering policy (closes A-4)

Every `FsError` payload path is **workspace-relative wherever the workspace root
is known** — that is, for every variant produced after the workspace root has
been canonicalized. `Io { site: WorkspaceRoot, .. }` is the sole exception and
carries the caller-supplied workspace root as given, because at that point no
canonical root exists to relativize against. No absolute, runner-specific path
is rendered into any other variant, so CI logs cannot leak a machine-specific
layout. `Display` renders the path with `/` separators.

### Resolver ordering (NON-NEGOTIABLE)

Four phases. **Every phase completes for every root before the next begins** —
this is the change that makes cross-root identity expressible (finding B).

**Phase 0 — lexical root grammar. Zero filesystem calls.**
Each configured root must be non-empty and every `Path::components()` item must
be `Component::Normal`. Otherwise `RootNotRelative`, naming the offending
component. This precedes every seam call, asserted through the call log.

**Phase 1 — workspace root.**
`canonicalize(workspace_root)` through the seam. Failure ⇒
`Io { site: WorkspaceRoot, operation: Canonicalize, .. }`. The workspace root is
**not** lstat-checked; it is a caller-supplied trust anchor (H6).

**Phase 2 — per-root validation, for all roots, before any traversal.**
For each root, in configured order:

1. lexically join to the workspace root;
2. `symlink_metadata(joined)`;
   failure ⇒ `NotFound` → `RootMissing`, else
   `Io { site: Root, operation: SymlinkMetadata, .. }`;
   `Symlink` ⇒ `RootIsSymlink`; `File` or `Other` ⇒ `RootNotDirectory`;
   `Dir` ⇒ continue. **This step precedes step 3.**
3. `canonicalize(joined)` through the seam;
   failure ⇒ `Io { site: Root, operation: Canonicalize, .. }`;
4. component-wise `Path::starts_with` against the canonical workspace root;
   not contained ⇒ `OutOfWorkspace { site: Root, .. }`.

Steps 2 and 3 **cannot** be reordered: `canonicalize` follows symlinks, so a
containment check on a canonicalized root has already discarded the information
that the root *was* a symlink, and has already followed it.

**Phase 3 — root-set uniqueness, before any traversal.**
Sort the canonical roots. Compare **all pairs**: equal canonical paths ⇒
`DuplicateRoot`; one component-wise contained in another ⇒ `OverlappingRoots`.
Full pairwise comparison is used instead of an adjacent-pair scan because it is
obviously correct and needs no lexicographic-prefix proof; `n` is the configured
root count, not a data-dependent quantity.

**Phase 4 — traversal, per canonical root in sorted canonical order.**
`read_dir(dir)`; failure ⇒ `Io { site: Entry, operation: ReadDir, .. }`. Entries
are sorted by `file_name` for determinism. Each entry's lstat-derived kind
decides, with an **explicit arm per kind and no wildcard**:

* `Symlink` ⇒ `EntryIsSymlink` — rejected, never followed, never skipped;
* `Other` ⇒ `EntryNotReadable` — rejected, never dropped;
* `Dir` ⇒ recurse;
* `File` ⇒ `canonicalize(entry)` (failure ⇒
  `Io { site: Entry, operation: Canonicalize, .. }`); component-wise containment
  re-check (not contained ⇒ `OutOfWorkspace { site: Entry, .. }`); build
  `rel_path` from the **lexical** workspace-relative components joined with `/`,
  rejecting a non-UTF-8 component with `NonUtf8Path`; insert into the identity
  map keyed by `canonical_path` — a second occurrence ⇒
  `DuplicateFileIdentity`.

**No depth filter, no size filter, no ignore-file filter, no hidden-file filter,
no extension filter.** Omission is unexpressible because no filtering construct
exists in the code path.

Finally, sort by `rel_path` and return. An empty allow-list returns `Ok(vec![])`.

### `read_corpus`

For each `ResolvedFile`, in order: `symlink_metadata(canonical_path)` —
failure ⇒ `Io { site: Entry, operation: SymlinkMetadata, .. }`, kind `Symlink` ⇒
`EntryIsSymlink`, kind other than `File` ⇒ `EntryNotReadable` — then
`read_to_string(canonical_path)`, failure ⇒
`Io { site: Entry, operation: ReadToString, .. }`. Returns
`Vec<ReadFile { rel_path, content }>`, named fields rather than a positional
tuple (A-1). The re-lstat narrows the substitution race (M-7); H6 states the
limit it cannot remove.

### Red-phase mechanism (closes M-3)

Every `[R]` unit leaves the workspace **compiling and warning-clean**. New
functions and trait impls introduced in a red phase:

* take **underscore-prefixed parameters** (`_fs`, `_path`, `_roots`), so
  `unused_variables` cannot fire under `-Dwarnings` (P3);
* have `todo!()` as the **terminal expression**, so the return type checks and
  no unreachable-code warning fires;
* are renamed to their real parameter names in the paired `[G]` unit.

No `[R]` unit may leave the crate non-compiling, and no `[R]` unit may introduce
a private field or private helper that its own test does not reach (H4).

### Scenario convention (closes M-14)

Each `[R]` unit adds **at most 3** `#[test]` functions. **One test asserts
exactly one outcome** — one `FsError` value, or one success shape. Packing two
distinct error variants into one test is forbidden. A call-log assertion
accompanying the outcome in the same test is **not** a second outcome: it is
the proof of *how* the single outcome was reached, and is used only in G0.9(a),
G0.11(a), and G0.5(b).

### Obligation ownership (closes M-1)

No behaviour is implemented before the test that drives it, and no obligation is
claimed by two units.

| Obligation | Owned by |
|---|---|
| `FakeFs` node map + `read_dir` | G0.2 |
| `FakeFs` lstat / canonicalize-map / read | G0.4 |
| `FakeFs` fault injection, `other()`, call log | G0.6 |
| `RealFs` over `std::fs` | G0.8 |
| Phase 0 root grammar | G0.10 |
| Phase 1 + Phase 2 steps 1–2 | G0.12 |
| Phase 2 step 2 non-`Dir` arms + step 3 failure | G0.14 |
| Phase 2 step 4 containment | G0.16 |
| Phase 3 root-set uniqueness | G0.18 |
| Phase 4 recursion, `File`/`Dir`/`Symlink` arms | G0.20 |
| Phase 4 `Other` arm + entry lstat failure | G0.22 |
| Entry canonicalization, entry containment, identity map | G0.24 |
| `rel_path` construction + UTF-8 validation | G0.26 |
| Final sort + empty allow-list | G0.28 |
| `read_corpus` | G0.30 |

## Failure Semantics

| Variant | Fires when | Carries | Proven by |
|---|---|---|---|
| `RootNotRelative` | root empty or has a non-`Normal` component | root string, component | G0.9 |
| `RootMissing` | root lstat returns `NotFound` | path | G0.11 |
| `RootNotDirectory` | root lstat kind is `File` or `Other` | path, kind | G0.13 |
| `RootIsSymlink` | root lstat kind is `Symlink` | path | G0.11 |
| `OutOfWorkspace` | canonical root or canonical file not contained | site, path, workspace | G0.15 (Root), G0.23 (Entry) |
| `DuplicateRoot` | two roots share one canonical path | canonical, both spellings | G0.17 |
| `OverlappingRoots` | one canonical root contained in another | ancestor, descendant | G0.17 |
| `DuplicateFileIdentity` | two files share one canonical path | canonical, both display paths | G0.23 |
| `EntryIsSymlink` | entry lstat kind is `Symlink` | path | G0.19, G0.29 |
| `EntryNotReadable` | entry lstat kind is `Other` | path | G0.21 |
| `NonUtf8Path` | a path component is not valid UTF-8 | lossy-rendered path | G0.25 |
| `Io` | any seam operation fails | site, operation, path, `ErrorKind` | G0.11, G0.13, G0.21, G0.23, G0.29 |

**There is no `EmptySet` variant.** An empty corpus is a valid G0 result;
judging emptiness is G1's concern.

### Reachability — two distinct columns

| Variant | Runtime-reachable against `RealFs` | Deterministically inducible in CI |
|---|---|---|
| `RootNotRelative` | Yes | **Yes** — no filesystem at all |
| `RootMissing` | Yes | **Yes** — `FakeFs` and `RealFs` (tempdir) |
| `RootNotDirectory` | Yes | **Yes** — both |
| `RootIsSymlink` | Yes | **Yes** — `FakeFs` only (a real one needs elevation) |
| `OutOfWorkspace` | Yes | **Yes** — `FakeFs` only |
| `DuplicateRoot` | Yes | **Yes** — `FakeFs` only |
| `OverlappingRoots` | Yes | **Yes** — `FakeFs` and `RealFs` |
| `DuplicateFileIdentity` | Yes, but **not** while Phase 3 holds and entry symlinks are rejected — it is a defence-in-depth invariant | **Yes** — `FakeFs` canonicalization map only |
| `EntryIsSymlink` | Yes | **Yes** — `FakeFs` only |
| `EntryNotReadable` | Yes (a device node inside a root) — **not** in a Git-tracked corpus | **Yes** — `FakeFs` only |
| `NonUtf8Path` | Yes | **Yes** — `FakeFs` only |
| `Io` (each of 7 channels) | Yes | **Yes** — `FakeFs` only |

The two columns are kept separate deliberately, and the two honest caveats
(`DuplicateFileIdentity`, `EntryNotReadable`) are stated in the table rather
than hidden behind a uniform "Yes".

## Work Units

**30 units — 15 `[R]`/`[G]` pairs.** No unpaired unit, no placeholder test.

**File-count exception, declared once**: G0.1 touches **5** files. Every other
unit touches **1**. The blanket claim "every unit touches ≤3 files" is therefore
**not** made.

Size and complexity are recorded per unit below because the installed backlog
registry advertises no structured sizing fields; they are carried as labelled
prose into each harvested task.

### G0.1 `[R]` — Topology, registration, first failing test

* **Files owned (5, declared exception)**: `crates/agent-contract-fs/Cargo.toml`
  (new), `crates/agent-contract-fs/src/lib.rs` (new), `Cargo.toml`,
  `.cargo/test-coverage-manifest.toml`, `tests/unit/agent_contract_fs_test.rs`
  (new).
* Create the crate: `publish = false`, `edition = "2024"`,
  `rust-version = "1.85"`, `[dependencies] thiserror = "1"`.
* Declare the **whole public type surface** (all types in the Architecture
  table) with `todo!()` bodies and `_`-prefixed stub parameters.
* Add the workspace member entry, the root `[dev-dependencies]` path entry, the
  `[[test]]` block (`name = "unit_agent_contract_fs"`,
  `path = "tests/unit/agent_contract_fs_test.rs"`), and a `[[surface]]` with
  `path = "crates/agent-contract-fs/"` and
  `targets = ["unit_agent_contract_fs"]`.
* Add the **first real test**: a `FakeFs` with two files under one directory
  returns both from `read_dir` with `FileKind::File`.
* **No placeholder test is created**, so no later unit is burdened with removing
  one.
* *Accurately described*: this unit is registration **plus** the declared type
  surface **plus** the first real test. It is not "registration only" (A-7).
* **Success criteria**: (1) `cargo build --all-targets` succeeds with no
  warnings. (2) `cargo test --test unit_agent_contract_fs` fails, and the
  failure is the `todo!()` panic in `FakeFs::read_dir`. (3)
  `--mode completeness` passes. (4) `Cargo.lock` gains no package.
* *Size: M | Complexity: low*

### G0.2 `[G]` — `FakeFs` node map and `read_dir`

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement the node map and the `dir` / `file` builder methods, and `read_dir`
  returning entries sorted by `file_name` with lstat-derived kinds and **no
  following of symlink nodes**.
* **Success criteria**: G0.1's test passes; `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.3 `[R]` — Remaining `FakeFs` seam methods

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) `symlink_metadata` of a symlink node returns
  `FileKind::Symlink` and does **not** resolve to its target; (b) `canonicalize`
  returns the mapped path when a canonicalization entry exists and the input
  otherwise; (c) `read_to_string` of a file node returns its content.
* **Success criteria**: compiles warning-clean; all three fail on `todo!()`.
* *Size: S | Complexity: low*

### G0.4 `[G]` — Implement them

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Adds the `symlink` and `canonical` builder methods and the three seam methods.
* **Success criteria**: G0.3's three tests pass; `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.5 `[R]` — `FakeFs` fault injection, `other()`, call log

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) a node marked `unreadable(ErrorKind::PermissionDenied)`
  makes `symlink_metadata` return
  `Err(FsFault { operation: SymlinkMetadata, kind: PermissionDenied, path })`;
  (b) `calls()` returns a **cloned snapshot** recording each seam invocation
  with its path in invocation order, and the snapshot can be held while a
  further seam call is made (A-12); (c) a node created by `other()` reports
  `FileKind::Other`.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.6 `[G]` — Implement fault injection, `other()`, `FsCall` log

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* `RefCell<Vec<FsCall>>` interior mutability (the seam takes `&self`);
  `calls()` returns `self.log.borrow().clone()`, never a `Ref`. Adds `other`,
  `unreadable`, `remove`, `replace_with_symlink` builder methods.
* **Success criteria**: G0.5's three tests pass; `cargo dev-test` green; no
  `unsafe`.
* *Size: M | Complexity: medium*

### G0.7 `[R]` — `RealFs` privilege-free contract

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios against a `tempfile::TempDir`, using **only** operations that
  need no elevation on any platform: (a) `read_dir` of a real directory returns
  its real children with correct `Dir`/`File` kinds; (b) `symlink_metadata` of a
  real regular file returns `FileKind::File`; (c) `read_to_string` of a real
  file returns its content.
* **No real symlink is created anywhere in this package.**
* **Success criteria**: compiles warning-clean; all three fail on `todo!()`.
* *Size: S | Complexity: low*

### G0.8 `[G]` — `RealFs` over `std::fs`

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* `std::fs::symlink_metadata`, `std::fs::canonicalize`, `std::fs::read_dir`,
  `std::fs::read_to_string`. Entry kinds come from `DirEntry::file_type()`
  (lstat semantics, does not follow). Every `io::Error` is mapped to
  `FsFault { operation, path, kind: e.kind() }`; the adapter never names a
  site. Plain recursion only — `ignore::WalkBuilder` is not used and `ignore`
  is not a dependency.
* **Success criteria**: G0.7's tests pass; `cargo dev-test` green; the crate's
  only dependency is still `thiserror`.
* *Size: M | Complexity: medium*

### G0.9 `[R]` — Root grammar, before any seam call (closes M-4)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios, each asserting one `RootNotRelative` outcome **and** that
  `calls()` is **empty**: (a) `../outside`; (b) an absolute root (`/etc` on
  Unix-shaped input, and a `Prefix`-bearing `C:\Windows` input — both are
  non-`Normal`, so one assertion covers the class and the test names which
  component was rejected); (c) the empty string.
* **Success criteria**: compiles warning-clean; all three fail on the
  `resolve_corpus` `todo!()`.
* *Size: S | Complexity: low*

### G0.10 `[G]` — Phase 0

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Reject any root that is empty or has a component other than
  `Component::Normal`, naming the offending component. Zero seam calls on this
  path.
* **Success criteria**: G0.9's three tests pass, **including the zero-call
  assertions**; `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.11 `[R]` — Root lstat: ordering proof and failure channel (closes A + M-4 ordering)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) **root is a symlink** ⇒ `Err(FsError::RootIsSymlink)`
  **and** `calls()` contains `SymlinkMetadata(root)` and contains **zero**
  `Canonicalize(_)` and **zero** `ReadDir(_)` entries — the positive proof of
  ordering, not merely of verdict; (b) root lstat fails with
  `PermissionDenied` ⇒
  `Io { site: Root, operation: SymlinkMetadata, kind: PermissionDenied, .. }`
  and **not** `RootMissing`; (c) root lstat fails with `NotFound` ⇒
  `RootMissing`.
* Scenarios (b) and (c) are the two halves of the narrow specialization rule and
  are the direct answer to finding A's sharpest case.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.12 `[G]` — Phase 1 and Phase 2 steps 1–2

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Canonicalize the workspace root; join each root; lstat it; classify `Symlink`
  and the two lstat failure kinds. `Dir` continues; the `File`/`Other` arms and
  step 3 belong to G0.14.
* **Success criteria**: (1) G0.11's three tests pass. (2) The call-log assertion
  in (a) passes, i.e. no canonicalization or enumeration occurred. (3)
  `cargo dev-test` green.
* *Size: M | Complexity: medium*

### G0.13 `[R]` — Root kind rejection and the two canonicalize channels

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) root is a regular file ⇒
  `RootNotDirectory { kind: FileKind::File, .. }`; (b) root canonicalize fails ⇒
  `Io { site: Root, operation: Canonicalize, .. }`; (c) **workspace root**
  canonicalize fails ⇒
  `Io { site: WorkspaceRoot, operation: Canonicalize, .. }`.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: S | Complexity: medium*

### G0.14 `[G]` — Root kind arms and canonicalize failure mapping

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: G0.13's three tests pass; `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.15 `[R]` — Containment, component-wise (closes M-5, M-6)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios, all through `FakeFs`'s canonicalization map — **no on-disk
  symlink, no junction, no elevation, identical on Windows and Unix**:
  (a) a lexically contained root whose canonical form lies outside the canonical
  workspace root ⇒ `OutOfWorkspace { site: Root, .. }`;
  (b) the **sibling-name fixture named by the deliberation**: workspace root
  canonicalizes to `…/.github/agents`, a candidate canonicalizes to
  `…/.github/agents-evil` ⇒ `OutOfWorkspace` — this is the case string-prefix
  comparison would wrongly **accept**, and it is the component-vs-string proof
  (M-6, and the proof M-5 relocated here);
  (c) canonical workspace root and canonical root **both** rendered in Windows
  extended-length `\\?\` form, root beneath workspace ⇒ **accepted** — proving
  consistent-form handling, with no `Prefix::Disk` vs `Prefix::VerbatimDisk`
  mismatch (M-5 repair).
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.16 `[G]` — Phase 2 step 4

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Component-wise `Path::starts_with`, never string comparison; **no case
  folding anywhere** (deliberation Q3).
* **Success criteria**: G0.15's three tests pass; `cargo dev-test` green.
* *Size: S | Complexity: medium*

### G0.17 `[R]` — Root-set uniqueness (closes finding B, layer 2)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) the same root spelling listed twice ⇒ `DuplicateRoot`;
  (b) two **different** spellings mapped by the canonicalization map to one
  canonical path — the deterministic model of Windows case-insensitivity, and
  also of a symlinked intermediate component ⇒ `DuplicateRoot`, which is the
  mechanical closure of the attempt-1 C-1 dissent; (c) three roots where one
  canonical root is an ancestor of another ⇒ `OverlappingRoots` naming the
  ancestor and the descendant.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.18 `[G]` — Phase 3

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Sort canonical roots; full pairwise equality and containment comparison;
  report the first conflict in sorted order so the result is deterministic under
  multiple conflicts. Runs to completion **before** any `read_dir`.
* **Success criteria**: (1) G0.17's three tests pass. (2) In scenario (c) the
  call log contains **zero** `ReadDir(_)` entries, proving Phase 3 precedes
  traversal. (3) `cargo dev-test` green.
* *Size: M | Complexity: medium*

### G0.19 `[R]` — Exhaustive enumeration, no silent omission

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) a tree containing a depth-10 file, a 393,216-byte file, a
  dot-prefixed hidden file, and a file matching a `.gitignore` pattern — **all
  four appear** in the resolved set (positive no-omission proof; carried forward
  unchanged, this probe passed unanimously in v3 and again in attempt 1);
  (b) a symlink entry ⇒ `EntryIsSymlink`, i.e. rejected, **not** skipped and
  **not** followed; (c) a nested tree is fully descended — a file at depth 4
  under three intermediate directories appears.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.20 `[G]` — Phase 4 recursion, `File`/`Dir`/`Symlink` arms

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Recursive `read_dir` descent with entries sorted by `file_name`. The `Other`
  arm, entry canonicalization, `rel_path` construction, UTF-8 validation, the
  identity map, and the final sort are **not** implemented here — they belong to
  G0.22, G0.24, G0.26, and G0.28 respectively. Files are collected in traversal
  order with a provisional lexical path.
* **Success criteria**: (1) G0.19's three tests pass. (2) The implementation
  contains no `max_depth`, `max_filesize`, or `WalkBuilder` construct. (3)
  `cargo dev-test` green.
* *Size: M | Complexity: medium*

### G0.21 `[R]` — Entry fault channels

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) a directory whose `read_dir` fails ⇒
  `Io { site: Entry, operation: ReadDir, .. }` naming that directory; (b) an
  entry whose lstat kind is `Other` ⇒ `EntryNotReadable` naming it — rejected,
  never dropped; (c) an entry whose lstat fails ⇒
  `Io { site: Entry, operation: SymlinkMetadata, .. }`.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: S | Complexity: medium*

### G0.22 `[G]` — `Other` arm and entry lstat failure

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: (1) G0.21's three tests pass. (2) The `FileKind` match
  has **no wildcard arm**, so a future variant cannot be silently dropped. (3)
  `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.23 `[R]` — File identity (closes finding B, layer 3)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) an entry whose canonicalize fails ⇒
  `Io { site: Entry, operation: Canonicalize, .. }`; (b) two files under two
  **non-overlapping** roots whose canonicalization-map entries collide on one
  canonical path ⇒ `DuplicateFileIdentity` carrying the canonical path and both
  display paths — never deduped, never emitted twice; (c) a file whose canonical
  path lies outside the canonical workspace root ⇒
  `OutOfWorkspace { site: Entry, .. }`.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: high*

### G0.24 `[G]` — Entry canonicalization, containment, identity map

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: G0.23's three tests pass; `cargo dev-test` green.
* *Size: M | Complexity: medium*

### G0.25 `[R]` — Display path and UTF-8

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) on a nested tree, every `rel_path` is workspace-relative,
  includes its configured root prefix, and uses `/` separators regardless of
  platform; (b) an entry whose name is not valid UTF-8 ⇒ `NonUtf8Path`
  (constructed via a small `cfg`-split helper: `OsStringExt::from_vec` on Unix,
  `OsStringExt::from_wide` with an unpaired surrogate on Windows); (c) for a file
  reached through a canonicalization-map entry, `rel_path` is the **lexical**
  display path while `canonical_path` is the **mapped** canonical path — the two
  fields are proven distinct and independently derived.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.26 `[G]` — `rel_path` construction and UTF-8 validation

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: G0.25's three tests pass; `cargo dev-test` green.
* *Size: S | Complexity: medium*

### G0.27 `[R]` — Determinism and set-ness

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) two `FakeFs` instances describing the same tree with
  different insertion orders produce **identical** `rel_path` sequences;
  (b) on a multi-root tree, the returned `rel_path` sequence is **strictly
  increasing** — a complete uniqueness proof, and the direct assertion that the
  result is a set and not a multiset; (c) an **empty allow-list** returns
  `Ok(vec![])` with zero seam calls after Phase 1.
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: S | Complexity: medium*

### G0.28 `[G]` — Final sort and the empty case

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Sort by `rel_path` (byte-wise `String` `Ord`, identical on every platform).
* **Success criteria**: G0.27's three tests pass; `cargo dev-test` green.
* *Size: S | Complexity: low*

### G0.29 `[R]` — `read_corpus` (closes M-2, M-7)

* **Files owned (1)**: `tests/unit/agent_contract_fs_test.rs`.
* Three scenarios: (a) **happy path** — a multi-file tree returns `Ok` with
  `rel_path` values matching the resolved set in the same order and `content`
  matching each file's bytes (an implementation returning unconditional `Err`
  fails here, closing M-2); (b) a file present at resolve time and removed
  before the read ⇒
  `Io { site: Entry, operation: SymlinkMetadata, kind: NotFound, .. }`;
  (c) a file **replaced by a symlink** between resolve and read ⇒
  `EntryIsSymlink` — the substituted-tree case relocated to the read boundary
  (M-7).
* **Success criteria**: compiles warning-clean; all three fail.
* *Size: M | Complexity: medium*

### G0.30 `[G]` — `read_corpus` with re-lstat

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: (1) G0.29's three tests pass. (2) Every Verification
  command below is green. (3) No `allow(dead_code)` anywhere in the package.
  (4) The crate's only dependency is `thiserror`, and `Cargo.lock` has gained no
  package.
* *Size: M | Complexity: medium*

## Verification

The four constitution quality gates, in the constitution's order, plus the
package-specific commands:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo audit
```

```text
cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic
cargo test --test unit_agent_contract_fs
pwsh scripts/test-coverage-oracle.ps1 --mode completeness
```

Plus explicit acceptance evidence:

```text
rg "allow\(dead_code\)" crates/agent-contract-fs tests/unit/agent_contract_fs_test.rs   # expect 0
rg "max_depth|max_filesize|WalkBuilder" crates/agent-contract-fs                        # expect 0
git diff --stat Cargo.lock                                                              # expect no change
```

The `cargo clippy -p agent-contract-fs` line is present because the root alias's
package selection is not relied upon (M-9). The `Cargo.lock` check is the
mechanical proof of the "zero new external dependencies" claim.

## Constitution Check

Mapped against `.github/instructions/constitution.instructions.md` by the real
principle names and numbering.

| Principle | Applies | How G0 complies |
|---|---|---|
| **I. Safety-First Rust** | Yes | Rust 2024; crate declares `#![forbid(unsafe_code)]` and `#![warn(clippy::pedantic)]`, enforced by the explicit `-p` clippy command in Verification; all fallible operations return `Result<_, FsError>`; no `unwrap`/`expect` in crate code. *Deviation noted*: the crate returns `FsError`, not `EngramError` — it is a standalone dev-only crate with no dependency on `engram`, so `EngramError` is not in scope. Introducing that dependency to satisfy the letter of the constraint would violate VI and couple a test seam to the product runtime. `FsError` follows the repository's `thiserror` + struct-variant convention (P14). |
| **II. Test-First Development (NON-NEGOTIABLE)** | Yes | Every unit is an `[R]`/`[G]` pair; **15 pairs**; each `[R]` compiles warning-clean and is observed failing before its `[G]`. Tests live in `tests/unit/`, which this repository defines as the isolated-logic tier — the tier G0's tests actually belong to. All pass via `cargo dev-test`. |
| **III. Workspace Isolation and Security Boundaries** | Yes | G0's entire subject: two-layer containment (allow-listed roots plus canonical workspace containment), lexical root-grammar rejection before any filesystem call, root-symlink rejection before canonicalization, entry-symlink rejection before following, entry containment re-check. No secrets are read or written. |
| **IV. CLI Workspace Containment (NON-NEGOTIABLE)** | Yes | Two distinct claims, stated separately (A-8). *Resolver*: `resolve_corpus`/`read_corpus` touch only canonically contained paths, and a malformed root produces zero filesystem calls. *Adapter*: `RealFs` is a thin `std::fs` adapter that faithfully performs whatever path it is given — it is not itself containment-enforcing, and containment is the resolver's property. Neither can write: the seam exposes **no mutating operation at all**. |
| **V. Structured Observability** | Yes | Every failure is a typed variant carrying the offending path, and every seam failure additionally carries site, operation, and `io::ErrorKind`. `FakeFs::calls()` gives a structured call trace. No untyped string errors. |
| **VI. Single Responsibility** *(body: dependency minimality)* | Yes | **Zero new external crates in the workspace dependency graph.** The crate's single dependency, `thiserror = "1"`, is already resolved at the root (P12); `Cargo.lock` and `cargo audit` output are unchanged, asserted in Verification. `tempfile` used by the exerciser is likewise already a root dependency (P13). |
| **VII. Destructive Command Approval (NON-NEGOTIABLE)** | N/A | G0 proposes no destructive command. The seam has no mutating operation. Execution runs only `cargo` build/test/lint/audit and read-only greps. |
| **VIII. Explicit Safety Modes for Elevated Risk** | Yes | Executed under **freeze-scope mode**. The declared boundary is exactly the **five** files listed in G0.1 — two new crate files, two manifests, one new test file. No file outside that set is touched by any unit (A-9). |
| **IX. Git-Friendly Persistence** | Yes | All planning artifacts are markdown with YAML frontmatter. The crate adds no serialized state. The four manifest additions are single entries that do not reorder existing content. |
| **X. Agent Context Efficiency** | Partial | `resolve_corpus` returns structured `ResolvedFile` records rather than content, so a consumer needing only paths never pays for bodies; `read_corpus` is a separate call. *Honest limit*: `read_corpus` returns full file bodies, which is inherently bulk — bounded here by a 92-file, ≤98.5 KB corpus (P15) consumed by a test binary, not an agent context. |
| **XI. Merge Commit History Preservation (NON-NEGOTIABLE)** | Yes | G0 ships as its own shipment and its own PR, merged with a merge commit. No squash, no rebase. |
| Development Workflow #5 — **No dead code** | Yes | See H4 for the exact mechanism and its two stated limits. **Zero `allow(dead_code)`**, verified by an explicit grep in Verification. |
| Task Granularity — **2-Hour Rule** | Yes | 30 units; 29 touch 1 file; G0.1 touches 5 as a declared, justified mechanical exception. Every `[R]` unit has ≤3 test scenarios. |
| Task Granularity — **Width Isolation** | Yes | Each `[R]` unit touches only the test file; each `[G]` unit touches only the crate source. The single exception is G0.1, which spans registration, type surface, and the first test — declared, not hidden. |
| Quality Gates | Yes | All four gates appear in Verification in the constitution's order, including `cargo audit`. |

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | The root `cargo lint` alias may not lint the G0 crate's own pedantic lints, depending on default workspace member selection. | Not relied upon. Verification runs `cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic` explicitly, so enforcement holds regardless of which mechanism applies. `-Dwarnings` (P3) independently makes rustc lints hard errors at compile time. |
| R2 | Adding a `[dev-dependencies]` path entry changes the root dependency graph. | The resolved graph gains exactly one **local path** node and zero registry crates, because `thiserror` is already resolved (P12). Dev-dependencies are not linked into `cargo build --release`. Asserted by `git diff --stat Cargo.lock` showing no change. |
| R3 | The `NonUtf8Path` scenario needs a `cfg`-split construction helper. | Bounded to ~8 lines in the test file, both arms using safe `std` APIs (`OsStringExt`). If a platform arm proves unworkable, the fallback is to `cfg`-gate that scenario to the platform where it is constructible and record the gap explicitly rather than deleting the variant. |
| R4 | `FakeFs` diverges from real filesystem semantics, so proofs are about the fake, not reality. | Bounded by design: G0.7/G0.8 pin `RealFs` against a real `tempfile` tree for every privilege-free operation, so the two implementations are cross-checked on the shared subset. The fake is authoritative only for the fault classes real filesystems cannot produce without privileges — which is precisely why the seam exists. Stated as a limit in H6, not hidden. |
| R5 | Canonicalizing every discovered file adds one seam call per file, and `RealFs::canonicalize` is a real syscall. | Bounded: the corpus is 92 files (P15) and the consumer is a test binary. Correctness requires it — canonical identity is the only identity stable across spelling and case (deliberation Q2, Q3). No caching is introduced, because a cache would be state the plan would then have to prove correct. |
| R6 | 15 `[R]` units append to one test file, which will grow large. | Accepted and bounded: the file is organized into one `mod` per phase (`root_grammar`, `root_validation`, `containment`, `root_set`, `traversal`, `identity`, `determinism`, `read`), each unit appending to exactly one module. `clippy::pedantic`'s `too_many_lines` is a per-function lint and is unaffected by file length. |

## Plan Hardening

### H1 — Why the ordering fix cannot silently regress

The root-symlink property is asserted through the **call log**, not through its
error variant. G0.11(a) asserts `SymlinkMetadata(root)` present,
`Canonicalize(_)` absent, `ReadDir(_)` absent. The safety property is the
*absence of the traversal*, and the absence of those two call kinds is exactly
that property expressed as an assertion. Any refactor that canonicalizes before
lstat necessarily records a `Canonicalize(_)` entry and fails the test,
regardless of what it subsequently returns. This argument rests only on the call
log; attempt-1's prose about what a reordered implementation would *return* was
inaccurate and is not repeated (A-13).

G0.18 applies the same technique one level up: its scenario (c) asserts **zero**
`ReadDir(_)` calls when the root set is rejected, proving Phase 3 completes
before traversal begins.

### H2 — Why Windows determinism is real and not claimed

No test in this package creates an on-disk symlink or junction. The
canonical-escape case (G0.15a), the sibling-name case (G0.15b), the
extended-length case (G0.15c), the root-symlink case (G0.11a), the Windows
case-collision model (G0.17b), and the duplicate-identity case (G0.23b) are all
expressed as entries in `FakeFs`'s node and canonicalization maps. They execute
identically on `windows-latest` and `ubuntu-latest` with no elevation and no
developer mode.

`RealFs` is exercised (G0.7) only on operations requiring no privilege on any
platform. Its symlink-kind detection rests on the documented `std::fs`
`symlink_metadata` / `DirEntry::file_type` lstat contract, which is **not
privilege-testable in CI** and is therefore taken as a documented platform
guarantee rather than an asserted one (A-3). It is backstopped by the second
containment layer: even a misclassified symlink entry whose target escapes the
workspace is rejected by the Phase-4 entry containment check.

The resolver contains **no case-folding and no `cfg` fork**; platform case
semantics enter only through `canonicalize`, which is the injected boundary
(deliberation Q3).

### H3 — Why no file can be silently omitted

Two independent arguments, both required:

1. **Structural**: after inherited Decision 6.1/6.2 the code path contains no
   filtering construct — no depth counter compared against a limit, no size
   check, no extension match, no hidden-file test, no ignore-file consultation,
   no `WalkBuilder`. Omission is not avoided; it is unexpressible. G0.20 success
   criterion (2) states this, and the Verification grep checks it **for the
   three named constructs only** (`max_depth`, `max_filesize`, `WalkBuilder`).
   The grep is corroborating evidence for those three; it is not claimed to
   mechanically exclude the whole class (M-15). The structural argument carries
   the rest, and it is reviewable by reading one function.
2. **Positive**: G0.19(a) proves a depth-10 file, a 393,216-byte file, a hidden
   file, and a `.gitignore`d file all appear.

Additionally, a symlink entry and an `Other`-kind entry are **rejected with a
named error**, never dropped — so "not present in the output" always implies
"not present in the tree", never "quietly filtered". And under Phase 3 plus the
identity map, "present twice in the tree" is a reported conflict, never a
duplicated output row.

### H4 — Why there is no dead code, and the two things that would break the claim

The claim rests on one mechanism: `dead_code` does not fire on items that are
**effectively externally reachable** — `pub` items reachable from the root of a
library crate. It does **not** protect:

* **unused private struct fields**, which still fire under `-Dwarnings`;
* **unused private helper functions**, and `pub` items nested inside a private
  module, which are not effectively externally reachable.

The plan therefore adds a rule rather than relying on the mechanism alone: **a
private field or private helper may be introduced only in the unit that first
uses it.** Every item in the public type surface table has a named exerciser.
The Verification grep asserts zero `allow(dead_code)` as the observable
consequence. If a reviewer finds a public item without a named exerciser, that
is a defect in this plan, not a case for suppression (M-13).

### H5 — Why G0 is genuinely independently shippable, and the freeze invariant

G0's change set is: two new crate files, one new test file, three root
`Cargo.toml` additions, and one coverage-manifest surface — **four manifest
additions across two manifest files**, five files touched in total. It is green
under every Verification command **before G1 exists**, and requires no CI
workflow change (P11). Nothing in G0 references assertions, registries,
contracts, or the `.github` harness layout. If G1 and G2 were never built, G0
would still be a complete, useful, fully exercised library.

**Freeze invariant, restated from split-deliberation Decision 2 (A-11)**: G1
adds its **own** crate and **must not edit** `crates/agent-contract-fs`. If G1
finds it needs a fifth seam method, that is a G0 contract change requiring its
own deliberation and its own shipment — not an in-flight edit during G1.

### H6 — What G0 deliberately does **not** guarantee

Stated explicitly so no downstream package inherits a false guarantee:

* **Trust anchor.** The workspace root is a caller-supplied trust anchor. It is
  canonicalized but **not** lstat-checked, because it is not attacker-influenced
  corpus content. G0 does not defend against a caller supplying a malicious
  workspace root; it defends the corpus *inside* a trusted workspace root
  (A-2). The related concern that a symlinked *intermediate* component could
  make two configured roots denote one subtree is separately closed by
  mechanism: both canonicalize identically and `DuplicateRoot` fires (G0.17b).
* **Emptiness.** G0 does not guarantee the corpus is non-empty. Emptiness is a
  valid result; judging it belongs to G1.
* **TOCTOU.** G0 does not guarantee TOCTOU-freedom between `resolve_corpus` and
  `read_corpus`. The re-lstat immediately before each read (G0.30) **narrows**
  the window and converts the substituted-file case into `EntryIsSymlink`
  (G0.29c) and the disappeared-file case into `Io { .. NotFound }` (G0.29b), but
  it **cannot eliminate** the syscall-level race between the re-lstat and the
  read. That residual window is stated, not papered over.
* **Fake vs real.** G0 does not guarantee `RealFs` behaves identically to
  `FakeFs` for the privileged fault classes. It guarantees the *resolver logic*
  is identical, because the resolver is generic over `FileAccess`.
* **Layout stability.** `resolve_corpus` may trip `clippy::pedantic`'s
  `too_many_lines` as phases accumulate. The single-file crate layout is
  therefore **not** guaranteed to survive unchanged; splitting `lib.rs` into
  modules is a permitted, non-contract-changing refactor (A-10). The public
  surface is the contract; the file layout is not.

### H7 — Rollback

G0's entire footprint is additive: one new crate directory, one new test file,
three root `Cargo.toml` entries, and one coverage-manifest surface. Reverting
the merge commit removes all of it with no residue in the product runtime, **no
dependency-graph change**, and no CI configuration change. `Cargo.lock` is
unchanged by the addition, so it is unchanged by the removal.

### H8 — Blast radius

`src/` is not touched. `.github/workflows/` is not touched. No existing test,
target, surface, or alias is modified — only additions. The product binary is
unchanged byte-for-byte, because the crate is a dev-dependency and is never
linked into a release build.

### H9 — Why the failure channel cannot fall behind the seam again

Finding A arose because the taxonomy was written by enumerating *outcomes the
author thought of* rather than *operations the seam can perform*. The repair is
structural, not diligence-based: `Io` is discriminated by `FsSite` × `FsOp`, and
`FsOp` has exactly one variant per seam method. Adding a fifth seam method
therefore forces a new `FsOp` variant, which breaks every exhaustive match over
`FsOp` in the crate and in its consumers. The compiler, not the reviewer, is the
thing that notices.

The same argument applies to `FileKind`: the Phase-4 match has no wildcard arm
(G0.22 success criterion 2), so adding a kind is a compile error rather than a
silent drop.
