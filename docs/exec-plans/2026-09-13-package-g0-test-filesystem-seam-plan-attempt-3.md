---
type: exec-plan
date: 2026-09-13
package: G0
attempt: 3
supersedes: docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan-attempt-2.md
depends_on: none
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-3.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
prior_reviews:
  - docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
  - docs/closure/2026-09-13-package-g0-attempt-2-plan-review-record.md
branch: chore/checkpoint-resolution-ordering-restage
head: e0accf82
requires_plan_hardening: yes
plan_status: draft
gate: "final allowed G0 attempt"
---

# Package G0 attempt 3 — contained test-filesystem seam (implementation plan)

## Source Document

`docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-3.md`,
which records the operator-fixed API boundary (B1–B4) and settles questions
Q1–Q10 and decisions D1–D14. It inherits
`docs/decisions/2026-09-13-package-g-split-deliberation.md` (Decisions 2, 3, 6
binding) under program decision
`docs/decisions/2026-09-13-package-g-split-program-decision.md`.

Attempts 1 and 2 and both review records are preserved unmodified as evidence.

## Objective

Ship a dev-only workspace-member library crate that turns a list of allow-listed
repository-relative directories into a deterministic, exhaustively enumerated,
set-valued, safely read corpus, behind an injectable filesystem boundary, such
that:

* the corpus value that authorizes reading is **unforgeable outside the crate**;
* **every fallible seam call site** has its own independently injectable fault
  and its own proving scenario;
* **every entry's type is established by an explicit injected lstat**, never by
  trusted enumeration metadata;
* the workspace compiles warning-clean under `-Dwarnings` at **every commit**,
  with no enum variant added after the first unit.

G0 introduces **no product runtime behaviour** and **no assertion semantics**.

## What changed from attempt 2

| Attempt-2 finding | Sev | Closed by |
|---|---|---|
| **A** — `read_corpus` has no containment check; `ResolvedFile` is forgeable | P1 arch | B1/D4/D5/D6: `ResolvedFile { rel_path }` private, `ResolvedCorpus` opaque and the sole read authority, `read_corpus` re-lstats, re-canonicalizes and re-checks containment. Proved by a `compile_fail` doctest (D13) |
| **B** — failure channels not independently injectable; `ReadToString` unproven | P1 arch | B2/D3: fault map keyed `(FsOp, PathBuf)`; **all nine fallible call sites** have their own scenario; `ReadToString` is proven by G0.31(c) |
| **C** — entry `symlink_metadata` channel has no driving operation | P1 arch | B3/D1/D2: `read_dir` returns names only; the resolver lstats every entry explicitly; channel proven by G0.23(b) |
| **D** — red/green staging broken under `-Dwarnings` | P1 arch | B4/D9/D10/D11: channel-bundling rule, staged arm completion, private-state rule, and Phase 3 no longer sorts roots |
| **E** — `FsError` not compilation-complete | P1 mech | Complete taxonomy in G0.1: all derives on all nested types, an `#[error(...)]` on every variant |
| **M-1** — G0.11(a) call-log assertion unsatisfiable | P1 mech | G0.11 asserts `calls()` is **empty** (Phase 0 truly precedes Phase 1); G0.15(a) asserts the exact two-element prefix |
| **M-2** — `Cargo.lock` proof unsound | P2 | Claim narrowed to zero new **registry** crates; proved by asserting no added `source =` line in the lockfile diff |
| **M-3** — path rendering not implementable | P2 | D8/Q7: `FsError::io(workspace, site, fault)`; all payload paths are relative display `String`s; the two irreducible absolute classes are named; G0.15(b) asserts no absolute prefix leaks |
| **M-4** — H9 overstates the compile-time barrier | P2 | H9 rewritten around the lift function and the trait-impl break |
| **M-5** — TOCTOU residual broader than disclosed | P2 | Class (a) **closed** by the containment recheck; class (b) disclosed as open and untimed in H6 |
| **M-6** — zero-`ReadDir` assertion proves too little | P2 | G0.21(c) and G0.29(c) assert all root lstat+canonicalize calls precede the first `ReadDir` on multi-root input |
| **M-7** — two root-grammar cases undefined/false | P2 | Q9/D12: `RootIsWorkspaceRoot` added with a driving test; the Windows-spelling claim is withdrawn and the platform-dependent parse stated |
| **M-8** — `FsCall` never enumerated | P3 | Defined in full in the type surface below |
| **A-1** — `rel_path` uniqueness omits its premise | P3 | Q3/D7: `rel_path` is canonical-relative, so uniqueness follows directly from canonical identity |
| **A-2** — uniqueness stated for `ResolvedFile`, consumed as `ReadFile` | P3 | `read_corpus` preserves order 1:1; asserted by G0.31(a) |
| **A-3** — red-phase imports can trip `unused_imports` | P3 | Stated in the red-phase mechanism |

## Verified Preconditions

Each verified read-only this session against HEAD `e0accf82`.

| # | Precondition | Value | Source |
|---|---|---|---|
| P1 | Workspace members | `[".", "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml` |
| P2 | Member-crate precedent | `publish = false`, `edition = "2024"` | `crates/engram-indexer/Cargo.toml` |
| P3 | `-Dwarnings` is workspace-wide | `[build] rustflags = ["-Dwarnings"]` | `.cargo/config.toml` |
| P4 | `lint` alias | `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` | `.cargo/config.toml` |
| P5 | `dev-test` alias | `test --all-targets` | `.cargo/config.toml` |
| P6 | Coverage-oracle completeness rule | FAIL if any `[[test]]` target is unmapped by a surface | `scripts/test-coverage-oracle.ps1` |
| P7 | A `crates/` surface precedent exists | `path = "crates/powerbi-tmdl-parser/"` | `.cargo/test-coverage-manifest.toml` |
| P8 | That surface's globs include `unit_*` | `targets = ["contract_*", "integration_*", "unit_*", "cold_*", "helpers_*"]` | `.cargo/test-coverage-manifest.toml` |
| P9 | `unit_*` naming with `tests/unit/*_test.rs` paths | `unit_query_stats` → `tests/unit/query_stats_test.rs` | `Cargo.toml` |
| P10 | Oracle modes | `report`, `select`, `completeness`, `run` | `scripts/test-coverage-oracle.ps1` |
| P11 | CI runs the **root package only** (no `--workspace`) | `cargo test --no-default-features --features … --all-targets` | `.github/workflows/ci.yml` |
| P12 | `thiserror = "1"` is already a resolved root dependency | — | `Cargo.toml` |
| P13 | `tempfile = "3"` is already a root dependency | available to test targets | `Cargo.toml` |
| P14 | Repository error convention | `thiserror::Error`, struct-style variants, structured payloads, no embedded `io::Error` | `src/errors/mod.rs` |

**Consequence of P6 + P7 + P8.** The powerbi surface's `unit_*` glob would
already satisfy *completeness* for a target named `unit_agent_contract_fs`. A
dedicated `[[surface]]` for `crates/agent-contract-fs/` is nevertheless required
for *correctness* of the selection rule: without it, a change to the G0 crate
would select no test target at all.

**Consequence of P11.** The G0 crate carries **no internal `#[cfg(test)]`
tests**. All behavioural tests live in the root-package `[[test]]` target, which
CI already runs. G0 therefore requires **no CI workflow change**. The single
`compile_fail` doctest (D13) is the one gate CI does **not** run; see H10.

## Architecture

### Topology (created whole in G0.1; no later unit adds a file)

```text
crates/agent-contract-fs/
  Cargo.toml            # publish = false, edition 2024, thiserror = "1"
  src/lib.rs            # the entire public surface
tests/unit/agent_contract_fs_test.rs   # the sole behavioural exerciser
```

Root `Cargo.toml` gains **three** additions: one `[workspace] members` entry,
one `[dev-dependencies]` path entry, one `[[test]]` block
(`name = "unit_agent_contract_fs"`, `path = "tests/unit/agent_contract_fs_test.rs"`).
`.cargo/test-coverage-manifest.toml` gains **one** `[[surface]]`:

```toml
[[surface]]
path = "crates/agent-contract-fs/"
targets = ["unit_agent_contract_fs"]
```

Four manifest additions in total across two manifest files. **Five files
touched in total** (two created, two amended, one test file created).

### Dependency posture

The crate declares exactly one dependency, `thiserror = "1"`, already a resolved
root dependency (P12). **Zero new registry crates enter the dependency graph.**
Adding a workspace member and a path dev-dependency *does* add a local
`[[package]]` record to `Cargo.lock` and a dependency edge — that is expected
and is not a new external dependency. The proof is therefore that no **added**
lockfile entry carries a `source` line, not that the lockfile is unchanged
(closes M-2).

### Crate-level lints and documentation obligations

`src/lib.rs` opens with `#![forbid(unsafe_code)]` and
`#![warn(clippy::pedantic)]`. Enforcement is unconditional through the explicit
`-p` clippy command in Verification. Under pedantic (D14/Q10):

* every public `fn` returning `Result` carries an `# Errors` doc section —
  the four `FileAccess` methods, `resolve_corpus`, `read_corpus`
  (`clippy::missing_errors_doc`);
* every public `fn` returning a non-unit value carries `#[must_use]` —
  `rel_path`, `len`, `is_empty`, `iter`, `calls`, `node_kind`, `injected_fault`
  (`clippy::must_use_candidate`);
* `FakeFs` mutators return `()` and are called as statements, so
  `clippy::return_self_not_must_use` cannot fire (and post-resolve mutation
  scenarios need `&mut self` mutators anyway);
* every public item carries a `///` doc comment (Constitution, Documentation).

**No `allow(dead_code)` and no `allow` attribute of any kind appears anywhere in
this package.** H4 gives the exact mechanism and its two governed exceptions.

### The seam — exactly four methods

```rust
pub trait FileAccess {
    /// # Errors
    fn symlink_metadata(&self, path: &Path) -> Result<FileKind, FsFault>; // lstat; never follows the final component
    /// # Errors
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsFault>;
    /// # Errors
    fn read_dir(&self, path: &Path) -> Result<Vec<OsString>, FsFault>;    // names only (D1)
    /// # Errors
    fn read_to_string(&self, path: &Path) -> Result<String, FsFault>;
}
```

`read_dir` returns **`Vec<OsString>` — contained entry names only** (D1). It
returns no kind, no metadata, and no path. The resolver joins each name to the
directory it enumerated and establishes the type by an explicit
`symlink_metadata` call (D2). There is no enumeration-supplied type authority
anywhere in the package, and no `WalkBuilder` or equivalent is used.

No size accessor, no following `metadata`, no mtime, no permissions, and **no
mutating operation**. "G0 performs no writes" is a structural property of the
trait, provable by inspection.

**Why the seam returns `FsFault` and not `FsError`** (settled, attempt 2, item 1):
the adapter knows the operation and the path but not why the resolver called it;
the resolver knows the call site. Splitting them makes `site` trustworthy,
because a `FileAccess` implementor is never handed one. The resolver lifts every
fault through a single total function (D8):

```rust
impl FsError {
    fn io(workspace: Option<&Path>, site: FsSite, fault: FsFault) -> FsError;
}
```

`workspace` is `Option` solely because Phase 1 — the one call site at which no
canonical workspace root exists yet — must still be able to lift.

### Public type surface — complete in G0.1, every item with a named exerciser

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind { File, Dir, Symlink, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FsOp { SymlinkMetadata, Canonicalize, ReadDir, ReadToString }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FsSite { WorkspaceRoot, Root, Entry }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsCall {
    SymlinkMetadata(PathBuf), Canonicalize(PathBuf), ReadDir(PathBuf), ReadToString(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsFault { pub operation: FsOp, pub path: PathBuf, pub kind: std::io::ErrorKind }
```

`FsOp` derives `Ord` because it is half of a `BTreeMap` key (D3). `FileKind`,
`FsSite` and `FsOp` derive `Debug` because `FsError`'s `#[error(...)]` messages
render them; they derive `Clone, Copy, PartialEq, Eq` because `FsError` derives
`Clone, PartialEq, Eq` (closes finding E).

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FsError {
    #[error("configured root `{root}` is not a relative path (component `{component}`)")]
    RootNotRelative { root: String, component: String },
    #[error("configured root `{path}` does not exist")]
    RootMissing { path: String },
    #[error("configured root `{path}` is not a directory (kind {kind:?})")]
    RootNotDirectory { path: String, kind: FileKind },
    #[error("configured root `{path}` is a symlink")]
    RootIsSymlink { path: String },
    #[error("configured root `{path}` resolves to the workspace root")]
    RootIsWorkspaceRoot { path: String },
    #[error("{site:?} path `{path}` resolves outside the workspace")]
    OutOfWorkspace { site: FsSite, path: String },
    #[error("roots `{first}` and `{second}` resolve to the same directory `{canonical}`")]
    DuplicateRoot { canonical: String, first: String, second: String },
    #[error("root `{descendant}` is nested inside root `{ancestor}`")]
    OverlappingRoots { ancestor: String, descendant: String },
    #[error("files `{first_rel}` and `{second_rel}` resolve to the same file `{canonical}`")]
    DuplicateFileIdentity { canonical: String, first_rel: String, second_rel: String },
    #[error("entry `{path}` is a symlink")]
    EntryIsSymlink { path: String },
    #[error("entry `{path}` is not a readable regular file (kind {kind:?})")]
    EntryNotReadable { path: String, kind: FileKind },
    #[error("path `{path}` is not valid UTF-8")]
    NonUtf8Path { path: String },
    #[error("{operation:?} failed on {site:?} path `{path}`: {kind:?}")]
    Io { site: FsSite, operation: FsOp, path: String, kind: std::io::ErrorKind },
}
```

Thirteen variants, each with a concrete `#[error(...)]` message. No variant
carries `#[source]` or `#[from]`, so `Error::source()` is `None` for every
variant — stated, not implied. `std::io::ErrorKind` is carried instead of
`std::io::Error` so `FsError` can derive `PartialEq` and tests can assert whole
values. **No variant is added after G0.1** (B4).

| Item | Visibility | Exercised by |
|---|---|---|
| `FileKind`, `FsOp`, `FsSite`, `FsCall`, `FsFault` | pub | G0.1 onward |
| `FsError` (13 variants) | pub | G0.1, G0.11, G0.13, G0.15, G0.17, G0.19, G0.21, G0.23, G0.25, G0.27, G0.31, G0.33, G0.35 |
| `trait FileAccess` (4 methods) | pub | G0.1, G0.3, G0.5, G0.7, G0.9 |
| `struct RealFs` + impl | pub | G0.7, G0.9 |
| `struct FakeNode { kind, content, canonical }` | pub fields | G0.1 onward (fixture vocabulary) |
| `struct FakeFs` + impl | pub, private state | every unit |
| `FakeFs` mutators `dir`, `file`, `symlink`, `other`, `set_canonical`, `fail`, `remove`, `replace_with_symlink` | pub, `-> ()` | G0.1, G0.3, G0.5, G0.31, G0.33, G0.35 |
| `FakeFs` accessors `calls()`, `node_kind()`, `injected_fault()` | pub | G0.1, G0.3, G0.33 |
| `struct ResolvedFile` — **private** `rel_path: String` | pub type, private field | G0.27, G0.29 |
| `ResolvedFile::rel_path() -> &str` | pub | G0.27, G0.29 |
| `struct ResolvedCorpus` — **private** `files`, `workspace` | pub type, private fields | G0.13, G0.29 |
| `ResolvedCorpus::len/is_empty/iter` | pub | G0.13, G0.29 |
| `struct ReadFile { rel_path: String, content: String }` | pub fields | G0.31 |
| `resolve_corpus<F: FileAccess>(&F, &Path, &[&str]) -> Result<ResolvedCorpus, FsError>` | pub | G0.11 onward |
| `read_corpus<F: FileAccess>(&F, &ResolvedCorpus) -> Result<Vec<ReadFile>, FsError>` | pub | G0.31, G0.33, G0.35 |

`ReadFile` keeps public fields deliberately: it is an **output** carrying a
display identity and file content, and it authorizes nothing. `ResolvedFile` and
`ResolvedCorpus` are opaque precisely because they **do** authorize a read.

### Containment types — the unforgeability contract (B1)

```rust
pub struct ResolvedFile { rel_path: String }                      // private
pub struct ResolvedCorpus { workspace: PathBuf, files: Vec<ResolvedFile> }  // private
```

* `ResolvedFile` stores **no canonical path at all** (D4). There is no public
  accessor that yields a `Path`. A `ResolvedFile` in isolation carries no
  filesystem authority.
* `ResolvedCorpus` privately pairs the validated canonical workspace root with
  the validated files (D5, Q2). Because the pairing is internal, the
  "corpus checked against the wrong workspace root" failure mode is
  **unrepresentable**, not merely checkable — which is why no separate
  workspace-root token parameter is introduced.
* `resolve_corpus` is the **sole constructor** of both, and constructs them only
  after exhaustive contained resolution has succeeded.
* `read_corpus` accepts `&ResolvedCorpus` and nothing else, and re-validates
  every invariant that can change between resolution and read (D6).

**How the claim is proved.** `src/lib.rs` carries one `compile_fail` doctest on
`ResolvedCorpus` that attempts struct-literal construction from outside the
crate and asserts the compiler rejects it. It is run by
`cargo test -p agent-contract-fs --doc` in Verification. It is **exempt from the
red/green convention**, because it asserts a compile-time negative that is true
from the instant the type is declared; there is no state in which it is red and
then green. That exemption is stated, and H10 discloses that CI (P11) does not
execute it.

A secondary, cheaper check greps `src/lib.rs` for public constructors of the two
opaque types. It proves the absence of a spelling, not of a capability, and is
recorded as secondary for exactly that reason.

### `FakeFs` — operation-keyed fault injection (B2/D3)

```rust
pub struct FakeFs {
    nodes:  BTreeMap<PathBuf, FakeNode>,                          // read by node_kind() + seam
    faults: BTreeMap<(FsOp, PathBuf), std::io::ErrorKind>,        // read by injected_fault() + seam
    log:    RefCell<Vec<FsCall>>,                                 // read by calls()
}
```

The fault map is keyed by the explicit **(operation, path)** pair. Every
fallible seam operation is therefore independently injectable at every path:
`fail(FsOp::SymlinkMetadata, p, k)` leaves `canonicalize(p)` succeeding, and
vice versa. `fail(FsOp::ReadToString, p, k)` is a first-class scenario with its
own proving test (G0.31(c)).

All three fields are private and each has a public reader, so no field is dead
under `-Dwarnings` at any commit. `node_kind()` and `injected_fault()` are not
padding: `node_kind()` asserts that the post-resolve mutators actually mutated
the fixture (G0.33), and `injected_fault()` asserts operation-keyed
independence at the map level before the seam consumes it (G0.3).

`log` is a `RefCell` because the seam methods take `&self`. `FakeFs` is not
`Sync` and does not claim to be; each test function is single-threaded.

Every seam method, in order: record the `FsCall`, consult `faults` for its
`(FsOp, path)` key and return `FsFault` if present, then consult `nodes`.
Recording **before** the fault check is deliberate — an injected failure is
still an observed call, so the call log is a faithful record of what the
resolver attempted.

### Path rendering policy (D8/Q7, closes M-3)

One private helper renders every payload path:

```rust
fn render(workspace: Option<&Path>, path: &Path) -> String;
```

It strips the canonical workspace prefix when one is supplied and the strip
succeeds, joins the remaining components with `/`, and otherwise renders the
path lossily as given. Two payload classes **cannot** be relativized and the
plan says so rather than claiming otherwise:

* `Io { site: WorkspaceRoot, .. }` — no canonical root exists yet;
* `OutOfWorkspace { .. }` — the path is by definition not under the workspace.

Every other variant renders relative. G0.15(b) asserts that an
`Io { site: Root, .. }` payload does not contain the absolute fixture prefix.
The `workspace` field is deliberately **absent** from `OutOfWorkspace`: it was a
caller-supplied constant, and dropping it removes a second absolute-path
disclosure at no cost.

### Resolver ordering (NON-NEGOTIABLE)

Four phases. **Every phase completes for every root before the next begins.**

**Phase 0 — lexical root grammar. Zero filesystem calls.**
Each configured root must be non-empty and every `Path::components()` item must
be `Component::Normal`; otherwise `RootNotRelative`, naming the offending
component (the empty string when the root has no components at all). This
precedes every seam call, asserted by a `calls().is_empty()` check (closes M-1,
which was unsatisfiable because attempt 2 asserted this against a state that had
already made a Phase 1 call).

**Platform note (Q9(ii), closes M-7(ii)).** On Unix, `C:\foo` and `\\?\C:\foo`
parse as single `Component::Normal` items and are **accepted** by Phase 0; they
are then lexically contained, and fail at Phase 2 as `RootMissing`. Attempt 2's
claim that they are rejected as `Component::Prefix` on every platform was false
and is withdrawn. No escape is possible either way, so no extra lexical rule is
added.

**Phase 1 — workspace root.**
`canonicalize(workspace_root)` through the seam. Failure ⇒
`Io { site: WorkspaceRoot, operation: Canonicalize, .. }`. The workspace root is
**not** lstat-checked; it is a caller-supplied trust anchor (H6). Phase 1 runs
**unconditionally**, including for an empty allow-list, so a `ResolvedCorpus`
always carries a validated canonical workspace root.

**Phase 2 — per-root validation, for all roots, in configured order, before any traversal.**

1. lexically join the root to the workspace root;
2. `symlink_metadata(joined)`. Failure ⇒ `NotFound` becomes `RootMissing`,
   **every other kind** becomes `Io { site: Root, operation: SymlinkMetadata, .. }`.
   `Symlink` ⇒ `RootIsSymlink`; `File` or `Other` ⇒ `RootNotDirectory`;
   `Dir` ⇒ continue. **This step precedes step 3.**
3. `canonicalize(joined)`. Failure ⇒
   `Io { site: Root, operation: Canonicalize, .. }`;
4. component-wise `Path::starts_with` against the canonical workspace root.
   Not contained ⇒ `OutOfWorkspace { site: Root, .. }`. Canonically **equal** to
   the workspace root ⇒ `RootIsWorkspaceRoot` (Q9(i), closes M-7(i)).

Steps 2 and 3 cannot be reordered: `canonicalize` follows symlinks, so a
containment check on a canonicalized root has already discarded the information
that the root *was* a symlink, and has already followed it.

**Phase 3 — root-set uniqueness, before any traversal. No sort (D11).**
Compare **all pairs** of canonical roots: equal ⇒ `DuplicateRoot`; one
component-wise contained in another ⇒ `OverlappingRoots`. Full pairwise
comparison needs no lexicographic-prefix proof and `n` is the configured root
count, not a data-dependent quantity. **Deliberately not sorted**, so traversal
proceeds in *configured* root order and cross-root output order stays
non-deterministic until the final sort — which is what makes the determinism
test in G0.29 reliably red (closes finding D part 3).

**Phase 4 — traversal, per canonical root, in configured order.**
`read_dir(dir)` ⇒ failure `Io { site: Entry, operation: ReadDir, .. }`. The
returned **names** are sorted (`OsString` byte order) for within-directory
determinism. For each name, in sorted order:

1. join the name to the enumerated directory;
2. `symlink_metadata(joined)` — **explicit, injected, per entry** (D2). Failure ⇒
   `Io { site: Entry, operation: SymlinkMetadata, .. }`;
3. match the kind with an **explicit arm per variant and no wildcard**:
   * `Symlink` ⇒ `EntryIsSymlink` — rejected, never followed, never skipped;
   * `Other` ⇒ `EntryNotReadable { kind: Other }` — rejected, never dropped;
   * `Dir` ⇒ recurse;
   * `File` ⇒ `canonicalize(joined)` (failure ⇒
     `Io { site: Entry, operation: Canonicalize, .. }`); component-wise
     containment re-check (not contained ⇒ `OutOfWorkspace { site: Entry, .. }`);
     build `rel_path` by stripping the **canonical** workspace root from the
     **canonical** file path and joining the remaining components with `/`,
     rejecting a non-UTF-8 component with `NonUtf8Path`; insert into the identity
     map keyed by the canonical path — a second occurrence ⇒
     `DuplicateFileIdentity { canonical, first_rel, second_rel }`.

**No depth filter, no size filter, no ignore-file filter, no hidden-file filter,
no extension filter.** Omission is unexpressible because no filtering construct
exists in the code path (H3).

Finally, sort by `rel_path` and construct the `ResolvedCorpus`. An empty
allow-list yields an empty corpus carrying the canonical workspace root.

**Why `rel_path` is canonical-relative (D7/Q3).** It makes `rel_path` an
injective function of the canonical path under a fixed canonical workspace root,
so `rel_path` uniqueness follows **directly** from the identity map's canonical
uniqueness. Attempt 2's lexical construction needed an auxiliary premise about
symlink-free lexical descent (advisory A-1); that premise is now unnecessary.
It also makes `workspace.join(rel_path)` reconstruct the canonical path exactly,
which is what lets `ResolvedFile` store no path.

### `read_corpus` (D6)

For each `ResolvedFile`, in corpus order:

1. reconstruct `candidate = corpus.workspace.join(rel_path components)`;
2. `symlink_metadata(candidate)` — failure ⇒
   `Io { site: Entry, operation: SymlinkMetadata, .. }`; `Symlink` ⇒
   `EntryIsSymlink`; `Dir` or `Other` ⇒ `EntryNotReadable { kind }`;
   `File` ⇒ continue;
3. `canonicalize(candidate)` — failure ⇒
   `Io { site: Entry, operation: Canonicalize, .. }`;
4. component-wise containment against `corpus.workspace` — not contained ⇒
   `OutOfWorkspace { site: Entry, .. }`;
5. `read_to_string(candidate)` — failure ⇒
   `Io { site: Entry, operation: ReadToString, .. }`.

Returns `Vec<ReadFile { rel_path, content }>`, **1:1 and order-preserving** with
the corpus (closes advisory A-2; asserted by G0.31(a)).

**Why lstat precedes canonicalize.** lstat declines to follow only the *final*
component, so it is the only check that can detect a leaf that became a symlink.
Canonicalize then resolves the whole path, which is the only check that can
detect a substituted *ancestor*. Running them in this order gives both
detections with no redundant call. H6 states exactly what this closes and what
it does not.

### Red-phase mechanism

Every `[R]` unit leaves the workspace **compiling and warning-clean** under
`-Dwarnings`. Functions and impls introduced in a red phase:

* take **underscore-prefixed parameters** (`_fs`, `_path`, `_roots`), so
  `unused_variables` cannot fire (P3);
* have `todo!()` as the **terminal expression**, so the return type checks and no
  unreachable-code warning fires;
* are renamed to their real parameter names in the paired `[G]` unit;
* introduce an `use` item **only in the unit that uses it**, so
  `unused_imports` cannot fire (closes advisory A-3).

### Compilation-completeness rule (B4/D9)

1. **Complete in G0.1**: every enum and its full variant set, every
   `#[error(...)]`, every derive, the full `FileAccess` signature, every public
   struct with its public accessors, and the signatures of `resolve_corpus` and
   `read_corpus`. Public items in a library crate are externally reachable, so
   `dead_code` does not fire on them. **No enum variant is added after G0.1**,
   so no `match` can become non-exhaustive in any later commit.
2. **Private-state rule**: a private field or private helper is introduced in the
   green unit that first **reads or calls** it, because `-Dwarnings` (P3) makes
   an unread private field a hard error. Exactly **two** items in the whole
   package are governed by this rule, and both are named here:
   * `ResolvedCorpus::workspace` — written by `resolve_corpus`, first read by
     `read_corpus`'s containment recheck. Introduced in **G0.36**.
   * `FsError::io` and `render` — first called in **G0.14** (Phase 1's channel).
   Every `FakeFs` field is exempt because each has a public reader from G0.1.
3. **Staged arm completion**: an exhaustive `match` may carry `todo!()` arms.
   Each green unit replaces exactly the arms its paired red unit drives. A
   `todo!()` arm is a *handled* variant: it compiles, it is warning-clean, and it
   panics rather than silently succeeding — so a test that depends on an
   unimplemented arm is genuinely red.

### Channel-bundling rule (D10, closes finding D part 1)

> The `Io` channel test for a seam call lives in the **same red unit** that
> drives that call's introduction. A failure channel may never be owned by a unit
> later than the one introducing its call.

This makes finding D's defect structurally impossible rather than individually
patched. Under `-Dwarnings` a green unit that introduces a seam call *must*
handle its `Result`; bundling means the test that proves the mapping is already
red at that moment.

### Scenario convention

Each `[R]` unit adds **at most 3** `#[test]` functions. **One test asserts
exactly one outcome** — one `FsError` value, one `FsFault` value, or one success
shape. Packing two distinct error variants into one test is forbidden. A
call-log assertion accompanying the outcome in the same test is **not** a second
outcome: it is the proof of *how* that single outcome was reached.

## Failure Semantics

| Variant | Fires when | Carries | Proven by |
|---|---|---|---|
| `RootNotRelative` | root empty or has a non-`Normal` component | root, component | G0.11 |
| `RootMissing` | root lstat returns `NotFound` | rel path | G0.15(a) |
| `RootNotDirectory` | root lstat kind is `File` or `Other` | rel path, kind | G0.17(a), G0.17(b) |
| `RootIsSymlink` | root lstat kind is `Symlink` | rel path | G0.15(c) |
| `RootIsWorkspaceRoot` | canonical root equals the canonical workspace root | rel path | G0.19(b) |
| `OutOfWorkspace` | canonical root or canonical file not contained | site, path | G0.19(a), G0.19(c), G0.27(a), G0.35(a) |
| `DuplicateRoot` | two roots share one canonical path | canonical, both spellings | G0.21(a) |
| `OverlappingRoots` | one canonical root contained in another | ancestor, descendant | G0.21(b) |
| `DuplicateFileIdentity` | two files share one canonical path | canonical, both rel paths | G0.27(b) |
| `EntryIsSymlink` | entry lstat kind is `Symlink` | rel path | G0.23(c), G0.33(a) |
| `EntryNotReadable` | entry lstat kind is `Other` (traversal) or `Dir`/`Other` (read) | rel path, kind | G0.25(a), G0.33(b), G0.33(c) |
| `NonUtf8Path` | a canonical path component is not valid UTF-8 | lossy path | G0.27(c) |
| `Io` | any seam operation fails | site, operation, path, `ErrorKind` | see the call-site table |

### The nine fallible seam call sites, each with its own scenario

Attempt 2 tabulated eight "channels" keyed by `(site, operation)`, which
conflated two distinct *call sites* that share one discriminant. This table is
keyed by **call site**, which is what a fault must be targeted at.

| # | Call site | `(site, operation)` | Introduced by | Proven by (same red unit) |
|---|---|---|---|---|
| 1 | Phase 1 workspace canonicalize | `(WorkspaceRoot, Canonicalize)` | G0.14 | **G0.13(b)** |
| 2 | Phase 2 root lstat | `(Root, SymlinkMetadata)` | G0.16 | **G0.15(b)** |
| 2s | …specialized on `NotFound` | → `RootMissing` | G0.16 | **G0.15(a)** |
| 3 | Phase 2 root canonicalize | `(Root, Canonicalize)` | G0.18 | **G0.17(c)** |
| 4 | Phase 4 directory enumeration | `(Entry, ReadDir)` | G0.24 | **G0.23(a)** |
| 5 | Phase 4 entry lstat | `(Entry, SymlinkMetadata)` | G0.24 | **G0.23(b)** |
| 6 | Phase 4 entry canonicalize | `(Entry, Canonicalize)` | G0.26 | **G0.25(c)** |
| 7 | `read_corpus` re-lstat | `(Entry, SymlinkMetadata)` | G0.32 | **G0.31(b)** |
| 8 | `read_corpus` read | `(Entry, ReadToString)` | G0.32 | **G0.31(c)** |
| 9 | `read_corpus` re-canonicalize | `(Entry, Canonicalize)` | G0.36 | **G0.35(b)** |

Every "Proven by" unit is the red half of the pair whose green half appears in
"Introduced by". That is the channel-bundling rule, checkable row by row.

The operator's "metadata/type/size failure if retained" case folds into call
sites 2, 5 and 7: **type** is retained and comes only from `symlink_metadata`;
**size** is not retained; a following `metadata` is not in the seam. Recorded as
folded, not silently dropped.

## Work Units

Eighteen `[R]`/`[G]` pairs, thirty-six units. Each pair is one backlog task at
roughly two hours, produces a verifiable state change (red observed, then
green), and touches a single skill domain.

### Pair 1 — crate topology, complete taxonomy, `FakeFs` enumeration

**G0.1 `[R]`** creates all five files, the three root-`Cargo.toml` additions and
the one manifest surface; declares the **entire** public surface above with all
derives and `#[error]` messages; `FakeFs` with its three private fields, all
eight mutators and all three accessors implemented; `RealFs`, all four `FakeFs`
and `RealFs` seam methods, `resolve_corpus` and `read_corpus` as `todo!()`;
the `compile_fail` doctest. Tests: (a) `read_dir` on a fixture directory returns
exactly the contained names, sorted; (b) `read_dir` with
`fail(ReadDir, dir, PermissionDenied)` returns
`FsFault { operation: ReadDir, path: dir, kind: PermissionDenied }`; (c) after
one `read_dir`, `calls() == [FsCall::ReadDir(dir)]`.
**G0.2 `[G]`** implements `FakeFs::read_dir`: log, fault lookup, name listing,
sort.

### Pair 2 — `FakeFs` lstat and canonicalize, operation-keyed independence

**G0.3 `[R]`**: (a) `symlink_metadata` on a declared symlink returns
`FileKind::Symlink` (proving non-following); (b) with only
`fail(SymlinkMetadata, p, PermissionDenied)` injected, `injected_fault` reports
the fault for `(SymlinkMetadata, p)` and `canonicalize(p)` still succeeds;
(c) with only `fail(Canonicalize, p, NotFound)` injected, `symlink_metadata(p)`
still succeeds.
**G0.4 `[G]`** implements `FakeFs::symlink_metadata` and `FakeFs::canonicalize`.

### Pair 3 — `FakeFs` read, mixed call log

**G0.5 `[R]`**: (a) `read_to_string` returns the declared content;
(b) `fail(ReadToString, p, PermissionDenied)` faults the read while
`symlink_metadata(p)` still succeeds; (c) a mixed sequence records all four
`FsCall` variants in invocation order.
**G0.6 `[G]`** implements `FakeFs::read_to_string`.

### Pair 4 — `RealFs` enumeration and lstat

**G0.7 `[R]`** over a `tempfile` directory: (a) `read_dir` returns exactly the
contained names; (b) `symlink_metadata` on a regular file returns `File`;
(c) on a directory returns `Dir`.
**G0.8 `[G]`** implements `RealFs::read_dir` and `RealFs::symlink_metadata` over
`std::fs`.

### Pair 5 — `RealFs` canonicalize, read, fault labelling

**G0.9 `[R]`**: (a) `canonicalize` equals `std::fs::canonicalize`;
(b) `read_to_string` round-trips written content; (c) reading a nonexistent path
yields `FsFault { operation: ReadToString, path, kind: NotFound }` — proving the
adapter labels the operation it performed.
**G0.10 `[G]`** implements `RealFs::canonicalize` and `RealFs::read_to_string`.

**No `RealFs` test creates a symlink**, because symlink creation requires
elevation or developer mode on Windows. Symlink kinds are exercised only through
`FakeFs`, which models the platform contract rather than reimplementing it.

### Pair 6 — Phase 0 root grammar, zero seam calls

**G0.11 `[R]`**: (a) an empty root ⇒
`RootNotRelative { root: "", component: "" }` and `calls()` is **empty**;
(b) a `..` component ⇒ `RootNotRelative { component: ".." }` and `calls()` is
empty; (c) a `.` root ⇒ `RootNotRelative { component: "." }` and `calls()` is
empty.
**G0.12 `[G]`** implements Phase 0.

### Pair 7 — Phase 1, call site 1, empty corpus

**G0.13 `[R]`**: (a) an empty allow-list ⇒ `Ok`, `corpus.is_empty()`, and
`calls() == [FsCall::Canonicalize(workspace)]` — proving Phase 1 runs
unconditionally and nothing else does; (b) `fail(Canonicalize, workspace, …)` ⇒
`Io { site: WorkspaceRoot, operation: Canonicalize, path: <as given>, kind }`.
**G0.14 `[G]`** implements Phase 1, the private `FsError::io` lift and the
private `render` helper, and constructs the empty `ResolvedCorpus`.

### Pair 8 — Phase 2 root lstat, call sites 2 and 2s, `Symlink` arm

**G0.15 `[R]`**: (a) root lstat `NotFound` ⇒ `RootMissing` and
`calls() == [Canonicalize(ws), SymlinkMetadata(joined)]` — the exact prefix
(closes M-1); (b) `fail(SymlinkMetadata, joined, PermissionDenied)` ⇒
`Io { site: Root, operation: SymlinkMetadata, .. }`, asserted to be **not**
`RootMissing` and to carry no absolute fixture prefix (closes M-3);
(c) a root declared as a symlink ⇒ `RootIsSymlink`.
**G0.16 `[G]`** implements Phase 2 steps 1–2 with the `NotFound` specialization
and the `Symlink` arm. The `File`, `Other` and `Dir` arms are `todo!()`.

### Pair 9 — root kind arms, Phase 2 step 3, call site 3

**G0.17 `[R]`**: (a) root kind `File` ⇒ `RootNotDirectory { kind: File }`;
(b) root kind `Other` ⇒ `RootNotDirectory { kind: Other }`; (c) root kind `Dir`
with `fail(Canonicalize, joined, PermissionDenied)` ⇒
`Io { site: Root, operation: Canonicalize, .. }`.
**G0.18 `[G]`** implements the `File`/`Other` arms and the `Dir` arm's
`canonicalize(joined)` call with its mapping. Containment remains `todo!()`.

### Pair 10 — Phase 2 step 4 containment

**G0.19 `[R]`**: (a) a root whose declared canonical target lies outside the
workspace ⇒ `OutOfWorkspace { site: Root, .. }`; (b) a root whose declared
canonical target **equals** the canonical workspace root ⇒
`RootIsWorkspaceRoot` (closes M-7(i)); (c) workspace `…/ws` with a root
canonicalizing into `…/ws-evil/x` ⇒ `OutOfWorkspace` — proving component-wise
containment rather than string `starts_with`.
**G0.20 `[G]`** implements Phase 2 step 4.

### Pair 11 — Phase 3 root-set uniqueness

**G0.21 `[R]`**: (a) two spellings canonicalizing to the same directory ⇒
`DuplicateRoot { canonical, first, second }`; (b) one canonical root a proper
ancestor of another ⇒ `OverlappingRoots { ancestor, descendant }`; (c) three
roots whose third duplicates the first ⇒ `DuplicateRoot`, with `calls()` showing
**all three** root lstat and canonicalize calls before the error and **zero**
`ReadDir` — proving Phase 2 completes for every root before Phase 3 decides
(closes M-6).
**G0.22 `[G]`** implements Phase 3 as a full pairwise comparison, **without
sorting** (D11).

### Pair 12 — Phase 4 enumeration and explicit entry lstat, call sites 4 and 5

**G0.23 `[R]`**: (a) `fail(ReadDir, canonical_root, PermissionDenied)` ⇒
`Io { site: Entry, operation: ReadDir, .. }`; (b) `fail(SymlinkMetadata,
entry_path, PermissionDenied)` ⇒ `Io { site: Entry, operation: SymlinkMetadata,
.. }` — **this is the operation that finding C said did not exist** (B3/D2);
(c) a directory whose names are declared in the order `["b.md", "a.md"]` where
`a.md` is a symlink ⇒ `EntryIsSymlink` naming `a.md` — proving sorted iteration
**and** explicit per-entry lstat in one outcome.
**G0.24 `[G]`** implements Phase 4's `read_dir`, name sort, per-entry join and
explicit `symlink_metadata`, both `Io` mappings, and the `Symlink` arm. The
`File`, `Dir` and `Other` arms are `todo!()`.

### Pair 13 — entry kind arms, entry canonicalize, call site 6

**G0.25 `[R]`**: (a) an entry of kind `Other` ⇒
`EntryNotReadable { kind: Other }`; (b) a nested directory containing a symlink
⇒ `EntryIsSymlink` naming the depth-2 path, proving recursion; (c) a `File`
entry with `fail(Canonicalize, entry, PermissionDenied)` ⇒
`Io { site: Entry, operation: Canonicalize, .. }`.
**G0.26 `[G]`** implements the `Other` arm, the `Dir` recursion and the `File`
arm's `canonicalize` call with its mapping. Entry containment, `rel_path` and
the identity map remain `todo!()`.

### Pair 14 — entry containment, `rel_path`, UTF-8, identity

**G0.27 `[R]`**: (a) a file whose declared canonical target lies outside the
workspace ⇒ `OutOfWorkspace { site: Entry, .. }`; (b) two distinct entries whose
declared canonical targets coincide ⇒
`DuplicateFileIdentity { canonical, first_rel, second_rel }`; (c) a canonical
path with a non-UTF-8 component ⇒ `NonUtf8Path`.
**G0.28 `[G]`** implements entry containment, canonical-relative `rel_path`
construction with UTF-8 validation, and the identity map — **together**, because
(b) requires `rel_path` strings to exist (closes finding D part 2).

### Pair 15 — final sort, success shape, determinism

**G0.29 `[R]`**: (a) a two-root, nested-directory fixture — including the
adversarial sibling pair `.github/agents` and `.github/agents-evil` — resolves
to `Ok`, and `iter().map(rel_path)` yields the expected `/`-joined paths in
sorted order; (b) the same fixture with the configured root order reversed and
the directory names declared in reverse yields the **identical** sequence;
(c) on that ordinary multi-root success, `calls()` shows every root lstat and
canonicalize before the first `ReadDir` (closes M-6 for the success path).
**G0.30 `[G]`** implements the final sort by `rel_path` and returns the
populated `ResolvedCorpus`.

Scenario (b) is reliably red before G0.30 precisely because Phase 3 does not sort
(D11): until the final sort lands, traversal follows configured root order and
reversing it reverses the output.

### Pair 16 — `read_corpus`, call sites 7 and 8

**G0.31 `[R]`**: (a) a resolved two-file corpus reads to
`Vec<ReadFile>` with `rel_path` values **1:1 and in corpus order** and the
declared contents (closes advisory A-2); (b) a file removed after resolve ⇒
`Io { site: Entry, operation: SymlinkMetadata, kind: NotFound }`;
(c) `fail(ReadToString, path, PermissionDenied)` ⇒
`Io { site: Entry, operation: ReadToString, .. }` — **the channel attempt 2
could not prove at all**.
**G0.32 `[G]`** implements `read_corpus`'s reconstruction, re-lstat with its
mapping, the `File` arm, and the read with its mapping. The `Symlink`, `Dir` and
`Other` arms are `todo!()`.

### Pair 17 — `read_corpus` kind arms

**G0.33 `[R]`**: (a) a file replaced by a symlink after resolve ⇒
`EntryIsSymlink`, with `node_kind` asserting the fixture actually mutated;
(b) replaced by a directory ⇒ `EntryNotReadable { kind: Dir }`; (c) replaced by
an `Other` node ⇒ `EntryNotReadable { kind: Other }`.
**G0.34 `[G]`** implements the three arms.

### Pair 18 — `read_corpus` containment revalidation, call site 9

**G0.35 `[R]`**: (a) after resolve, the file's declared canonical target is
rewritten to a path outside the workspace — modelling an ancestor directory
replaced by a symlink ⇒ `OutOfWorkspace { site: Entry, .. }`;
(b) `fail(Canonicalize, path, PermissionDenied)` during read ⇒
`Io { site: Entry, operation: Canonicalize, .. }`.
**G0.36 `[G]`** implements the re-canonicalize call, its mapping and the
component-wise containment recheck, and introduces the private
`ResolvedCorpus::workspace` field together with its first reader
(compilation-completeness rule part 2).

### Obligation ownership

No behaviour is implemented before the test that drives it, and no obligation is
claimed by two units.

| Obligation | Owned by |
|---|---|
| Topology, complete taxonomy, `FakeFs` state/mutators/accessors, doctest | G0.1 |
| `FakeFs::read_dir` | G0.2 |
| `FakeFs::symlink_metadata`, `FakeFs::canonicalize` | G0.4 |
| `FakeFs::read_to_string` | G0.6 |
| `RealFs::read_dir`, `RealFs::symlink_metadata` | G0.8 |
| `RealFs::canonicalize`, `RealFs::read_to_string` | G0.10 |
| Phase 0 | G0.12 |
| Phase 1, `FsError::io`, `render`, empty corpus | G0.14 |
| Phase 2 steps 1–2 + `NotFound` specialization + `Symlink` arm | G0.16 |
| Root `File`/`Other` arms + Phase 2 step 3 | G0.18 |
| Phase 2 step 4 containment + `RootIsWorkspaceRoot` | G0.20 |
| Phase 3 pairwise uniqueness | G0.22 |
| Phase 4 enumeration, name sort, explicit entry lstat, `Symlink` arm | G0.24 |
| Entry `Other` arm, `Dir` recursion, entry canonicalize | G0.26 |
| Entry containment, `rel_path`, UTF-8, identity map | G0.28 |
| Final sort, populated corpus | G0.30 |
| `read_corpus` reconstruction, re-lstat, `File` arm, read | G0.32 |
| `read_corpus` `Symlink`/`Dir`/`Other` arms | G0.34 |
| `read_corpus` re-canonicalize, containment recheck, `workspace` field | G0.36 |

## Verification

Run in constitution order, plus three package-specific checks.

```text
# Gate 1 — format
cargo fmt --all -- --check

# Gate 2 — lint (crate-scoped, unconditional; then workspace-wide)
cargo clippy -p agent-contract-fs --all-targets -- -D warnings -D clippy::pedantic
cargo lint

# Gate 3 — test
cargo dev-test
cargo test -p agent-contract-fs --doc      # the compile_fail unforgeability assertion (H10)

# Gate 4 — audit
cargo audit
```

Package-specific checks:

```text
# Coverage-oracle completeness: the new [[test]] target must be mapped
pwsh scripts/test-coverage-oracle.ps1 -Mode completeness

# Zero new REGISTRY crates (closes M-2): no added lockfile entry may carry a source
git diff -U0 Cargo.lock | Select-String '^\+source = '        # expect zero matches

# No filtering construct in the resolver (H3), and no lint allowance anywhere
Select-String -Path crates/agent-contract-fs/src/lib.rs -Pattern '\.filter|\.skip|\bcontinue\b'   # expect zero matches
Select-String -Path crates/agent-contract-fs/src/lib.rs -Pattern '#\[allow'                        # expect zero matches

# Secondary unforgeability check (the doctest is primary)
Select-String -Path crates/agent-contract-fs/src/lib.rs -Pattern 'pub fn .*-> *(ResolvedCorpus|ResolvedFile)'
# expect exactly one match: resolve_corpus
```

## Constitution Check

| # | Principle | How this plan satisfies it |
|---|---|---|
| I | Safety-First Rust | `#![forbid(unsafe_code)]`; `#![warn(clippy::pedantic)]` with an explicit `-p` deny command; every fallible operation returns `Result`; no `unwrap`/`expect` in crate code; `-Dwarnings` holds at **every** commit by the compilation-completeness rule |
| II | Test-First Development (NON-NEGOTIABLE) | Eighteen `[R]`/`[G]` pairs; every behaviour has a red test observed failing before its green unit; the channel-bundling rule prevents premature green; the **one** exemption — the `compile_fail` doctest, which asserts a compile-time negative — is named explicitly rather than hidden |
| III | Workspace Isolation | `resolve_corpus` rejects any root whose canonical path is not a proper descendant of the canonical workspace root; Phase 4 re-checks every file; `read_corpus` re-checks again at read time |
| IV | CLI Workspace Containment (NON-NEGOTIABLE) | **What the mechanism delivers:** `resolve_corpus` touches only paths it has canonically contained, and `read_corpus` accepts only a `ResolvedCorpus`, which cannot be constructed outside the crate (proved by the `compile_fail` doctest) and which it re-validates before every read. **What it does not:** `RealFs` is an unconstrained `std::fs` adapter and makes no containment claim of its own; containment is a property of the resolver, not of the seam. Both halves are stated |
| V | Destructive Command Approval | The seam has no mutating method; "G0 performs no writes" is structural |
| VI | Single Responsibility | One dependency, `thiserror = "1"`, already resolved at the root (P12); zero new registry crates, proved by the lockfile-`source` check |
| VII | Structured Observability | Every failure is a typed `FsError` with a site, an operation and a rendered path; `FakeFs::calls()` exposes the full call log |
| VIII | Safety Modes | Planning-only change; no destructive operation is proposed |
| IX | Git-Friendly Persistence | Plan, deliberation and review records are markdown with YAML frontmatter |
| X | Agent Context Efficiency | G0 exists so G1 can query a resolved corpus instead of scanning the tree |
| XI | Merge Commit History Preservation | Ship's concern; this plan proposes no merge and creates no PR |

Quality gates run in the constitution's order, including `cargo audit`
(Verification above).

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | `clippy::pedantic` fires on a construct the plan did not anticipate | The three pedantic lints that bind this surface are named and designed around (Q10/D14); the `-p` command runs from G0.2 onward, so a surprise surfaces in the first green unit, not at the end |
| R2 | The lockfile gains a local `[[package]]` record and a reviewer reads it as a new dependency | The Dependency posture section states the expected diff shape and the check is written to match it (closes M-2) |
| R3 | Eighteen pairs is a long sequence for one package | Each pair is independently verifiable and independently committable; a halt at any pair boundary leaves the workspace green |
| R4 | The `compile_fail` doctest does not run in CI (P11) | Disclosed in H10 and listed as an explicit Verification command; G0 deliberately makes no CI change, which is G2's scope |
| R5 | Per-entry lstat adds one syscall per file over attempt 2's design | Accepted deliberately (B3). The harness corpus is ~92 files; one extra lstat per file is immaterial, and the alternative is trusting enumeration metadata as a type authority, which finding C rejected |
| R6 | `FakeFs`'s declarative canonical map can express states a real filesystem cannot | Accepted and disclosed in H2. The fake models the platform *contract*; `RealFs` tests (Pairs 4–5) pin the contract to `std::fs` behaviour |

## Plan Hardening

### H1 — Why the ordering fix cannot silently regress

Three call-log assertions pin the order, and each names the exact expected
sequence rather than a negative:

* G0.11 asserts `calls()` is **empty** after a Phase 0 rejection. This is exact
  and satisfiable because Phase 0 genuinely precedes Phase 1 — unlike attempt
  2's G0.11(a), which asserted zero `Canonicalize` entries against a state in
  which Phase 1 had already canonicalized the workspace (M-1).
* G0.15(a) asserts the exact two-element prefix
  `[Canonicalize(ws), SymlinkMetadata(joined)]`, pinning lstat-before-canonicalize
  at the root.
* G0.21(c) and G0.29(c) assert that every root lstat and canonicalize precedes
  the first `ReadDir`, on a failing and on a succeeding multi-root fixture
  respectively (M-6).

Reordering steps 2 and 3 of Phase 2, or interleaving validation with traversal,
breaks at least one of these assertions on a recorded call log, not on prose.

### H2 — Why Windows determinism is real, not claimed

The resolver contains **no case folding and no `cfg` fork**. Platform case
semantics arrive entirely through `canonicalize`, which on Windows returns the
on-disk casing. Two root spellings differing only in case therefore canonicalize
to the identical `PathBuf` and are rejected as `DuplicateRoot` by the same
comparison that catches any other duplicate — no special case exists to drift.

`rel_path` is produced by stripping the canonical workspace root from the
canonical file path, so the `\\?\` extended-length prefix cancels on both sides
and never appears in output. Sorting is byte-wise on `String` and `OsString`,
which is identical on both platforms.

`FakeFs` does **not** reimplement these semantics; its canonical map is
declarative, so a test states the platform behaviour it is modelling. Pairs 4
and 5 pin `RealFs` to the real `std::fs` contract on whichever platform the
suite runs. The limit is stated plainly: `FakeFs` can express canonical maps a
real filesystem could not produce (R6). That is a property of a fake, and it is
why the `RealFs` pairs exist.

### H3 — Why no file can be silently omitted

Three independent arguments, and the grep claim is narrowed to exactly what it
checks:

1. **Structural.** Phase 4 matches `FileKind` with four explicit arms and no
   wildcard. Every arm either produces a `ResolvedFile`, recurses, or returns an
   error. No arm drops an entry.
2. **Type-level.** `read_dir` returns names with no kind attached (D1), so there
   is no metadata an implementation could filter on before the resolver's own
   lstat. Attempt 2's `DirEntry.kind` — the thing finding C identified as an
   untrustworthy authority — no longer exists.
3. **Mechanical, narrowly.** The Verification grep checks for `.filter`, `.skip`
   and `continue` in `lib.rs` and expects zero matches. It proves the absence of
   those three constructs only; the structural argument above carries the rest.
   This narrowing is deliberate (closes attempt-1 M-15).

### H4 — Why there is no dead code, and exactly what governs the exceptions

Every public item in a library crate is externally reachable, so `dead_code`
cannot fire on the taxonomy, the trait, the adapters or the accessors. That is
what makes B4's "complete taxonomy in unit 1" compatible with `-Dwarnings`.

Private state is the only exception, and the compilation-completeness rule
part 2 governs it. **Exactly two items** are affected, both named in that rule:
`ResolvedCorpus::workspace` (introduced in G0.36 with its first reader) and the
`FsError::io` / `render` helper pair (introduced in G0.14 with their first call
site). Every `FakeFs` private field has a public reader from G0.1 — `calls()`,
`node_kind()`, `injected_fault()` — so none of them is ever dead.

Two things would break this claim, and neither is present: an `#[allow]`
attribute anywhere in the package (checked by grep in Verification), and a
private helper introduced before its caller.

### H5 — Why G0 is genuinely independently shippable

G0 creates two files, amends two manifests and adds one test file. It changes no
`src/**` file of the root package, no runtime behaviour, and no CI workflow. The
freeze scope is exactly these five paths:

```text
crates/agent-contract-fs/Cargo.toml
crates/agent-contract-fs/src/lib.rs
tests/unit/agent_contract_fs_test.rs
Cargo.toml
.cargo/test-coverage-manifest.toml
```

G1 consumes `resolve_corpus`, `read_corpus`, `ResolvedCorpus` and `ReadFile` and
**must not edit this crate**. If G1 needs a new capability, that is a G0
follow-up release unit, not an in-place edit — otherwise G0's review verdict
stops describing the shipped code.

### H6 — What G0 deliberately does **not** guarantee

1. **The workspace root is a trust anchor.** It is canonicalized but never
   lstat-checked. If the caller supplies a workspace root that is itself a
   symlink, G0 resolves relative to its target. That is by design: the caller
   owns that path. The canonical-root comparison in Phase 3 still detects the
   symlinked-intermediate collision case.
2. **TOCTOU, stated precisely** (closes M-5). `read_corpus` re-lstats,
   re-canonicalizes and re-checks containment before every read. That **closes**
   two classes attempt 2 left open:
   * a leaf replaced by a symlink after resolve (caught by the re-lstat), and
   * **an ancestor directory replaced by a symlink after resolve** — caught by
     the re-canonicalize, because `symlink_metadata` declines to follow only the
     *final* component and would report `File`, while `canonicalize` resolves
     through the substituted ancestor and fails the containment recheck.

   One class remains **open, and is open for the entire interval between
   `resolve_corpus` and `read_corpus` — it does not require winning a race**:
   a regular file replaced by a *different regular file* at the same canonical
   path. The kind is unchanged, containment is unchanged, and no error fires.
   Closing it needs handle-based atomic read (open once, then fstat and read the
   same handle), which the settled four-method seam does not have and which this
   attempt does not widen.

   Separately, and genuinely narrowly, a syscall-level window remains between
   the containment recheck and the read itself.
3. **`RealFs` makes no containment claim.** It is an unconstrained `std::fs`
   adapter. Containment is a property of the resolver.
4. **No assertion semantics, no CI change, no product runtime behaviour.**
5. **`clippy::too_many_lines`** may fire on the resolver once the arms are
   complete. If it does, the repair is extracting a phase into a private
   function **with a caller in the same commit**, never an `#[allow]`.

### H7 — Rollback

Delete `crates/agent-contract-fs/` and `tests/unit/agent_contract_fs_test.rs`,
revert the three root-`Cargo.toml` additions and the one manifest surface, and
run `cargo dev-test`. No other file is touched, so rollback is a clean revert of
a single commit range with no data migration and no runtime impact.

### H8 — Blast radius

Zero at runtime: nothing in `src/**` links the crate, and the dev-dependency
edge is test-only. The realistic failure mode is a **build-time** one — a
pedantic lint or a coverage-oracle completeness failure — which surfaces in
`cargo dev-test` and `cargo lint` before merge, never in a running daemon.

### H9 — Why the failure channel cannot fall behind the seam again

Attempt 2 claimed a compile-time barrier that does not exist: adding a fifth
seam method does **not** force a new `FsOp` variant, because the new method's
`FsFault` can reuse an existing `FsOp` value. Nothing at the type level binds a
method to a distinct operation label (closes M-4). The claim is restated around
the two mechanisms that are real:

1. **Every fault flows through one total lift function.** `FsError::io` is the
   only construction path for `Io`, and it consumes an `FsFault`. A seam call
   whose `Result` is handled therefore always has an error channel — it cannot
   be dropped silently, because `-Dwarnings` (P3) makes an unhandled `Result` a
   hard error.
2. **Adding a trait method breaks every implementor at compile time.** `RealFs`
   and `FakeFs` both stop compiling until the new method is implemented, and the
   fault map's `(FsOp, PathBuf)` key means the new method's fault is injectable
   the moment it exists.

One-`FsOp`-per-method is therefore a **stated convention**, not a compiler
guarantee, and the plan says so. What the compiler *does* guarantee is that the
method exists on every implementor and that its `Result` is handled.

### H10 — The one gate CI does not run

The `compile_fail` doctest is the primary proof of the unforgeability claim on
which Constitution row IV rests. Precondition P11 records that CI runs the root
package only, without `--workspace`, so **CI does not execute it**. It runs
under the explicit `cargo test -p agent-contract-fs --doc` command in
Verification, which is a local and review-time gate.

This is disclosed rather than resolved because G0 makes **no CI change by
design** — CI wiring is G2's scope. If G2 does not pick it up, the residual is
that a future edit could make `ResolvedCorpus` forgeable and CI would not catch
it. That is a named, inherited obligation on G2, not a silent gap.

### H11 — What would make this plan wrong

Stated so a reviewer can aim at it:

* If a private field or helper other than the two named in the
  compilation-completeness rule turns out to need late introduction, the
  `-Dwarnings` claim is incomplete and the unit topology needs re-cutting.
* If any "Proven by" red unit in the call-site table is not the pair-partner of
  its "Introduced by" green unit, the channel-bundling rule is violated and
  finding D recurs.
* If any `FsError` variant were added after G0.1, a later commit could carry a
  non-exhaustive match and B4 would be violated.
* If `read_corpus` could be reached with a value the crate did not construct,
  Constitution row IV would again claim more than the mechanism delivers.

Each of these is checkable against the tables in this plan without running the
code.
