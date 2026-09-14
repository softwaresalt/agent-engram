---
type: exec-plan
date: 2026-09-13
package: G0
depends_on: none
source: docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
requires_plan_hardening: yes
plan_status: blocked
review_verdict: FAIL
review_rounds: 1
review_record: docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md
blocked_by: "Two architecture-level P1 findings: (A) FsError cannot represent a failing symlink_metadata or canonicalize — two of the four seam methods have no failure channel, raised independently by the Rust and Correctness reviewers; (B) corpus identity is undefined — duplicate or nested allow-listed roots emit a multiset, which would make G1's exactly_one assertion report false multiplicity. Correction budget was restricted to mechanical findings and did not open."
harvested: false
---

# Package G0 — contained test-filesystem seam (implementation plan)

## Source Document

`docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md`,
which inherits `docs/decisions/2026-09-13-package-g-split-deliberation.md`
(Decisions 2, 3, 6 binding) under program decision
`docs/decisions/2026-09-13-package-g-split-program-decision.md`.

## Objective

Ship a dev-only, `std`-only library crate that turns a list of allow-listed
repository-relative directories into a deterministic, exhaustively enumerated,
safely read corpus — through an injectable filesystem boundary that makes the
root-symlink, canonical-escape, entry-symlink, unreadable-path, disappearing-
file, and Windows-path cases **mechanically provable without platform
privileges**.

G0 introduces **no product runtime behaviour** and **no assertion semantics**.

## Verified Preconditions

Each verified this session by read-only command; cited with file:line where a
file is the source.

| # | Precondition | Value | Source |
|---|---|---|---|
| P1 | Workspace members declared | `[".", "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml:1-3` |
| P2 | Member-crate precedent | `publish = false`, own `Cargo.toml`, `src/lib.rs` | `crates/engram-indexer/Cargo.toml` |
| P3 | `dead_code` is a hard error | `rustflags = ["-Dwarnings"]` under `[build]` | `.cargo/config.toml` |
| P4 | `allow(dead_code)` already forced in `tests/` | 11 occurrences | recursive grep |
| P5 | Coverage-oracle completeness maps targets against `src/` **and** `crates/` surfaces only | unmapped target ⇒ FAIL | `scripts/test-coverage-oracle.ps1:203-208` |
| P6 | A `crates/` surface precedent exists | `path = "crates/powerbi-tmdl-parser/"` | `.cargo/test-coverage-manifest.toml:137-139` |
| P7 | Existing `crates/` surface globs do **not** match `agent_contract_fs_test` | globs are `contract_*`, `integration_*`, `unit_*`, `cold_*`, `helpers_*` | same:139 |
| P8 | CI and local aliases run the **root package only** (no `--workspace`) | `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` | `.github/workflows/ci.yml` |
| P9 | `dev-test = "test --all-targets"`, `ci = "test --all-targets --all-features"`, `lint`, `fmt-check` | — | `.cargo/config.toml` |
| P10 | `autotests` absent; all test targets explicit `name` + `path` | 267 targets | `Cargo.toml` |
| P11 | Harness corpus | 92 files, all `.md`; max depth 2; max size 98,546 B | fresh enumeration |
| P12 | `tempfile = "3"` is a regular root dependency | available to test targets | `Cargo.toml` |

**Consequence of P7 + P5**: a new `[[surface]]` is required, and it is required
under *any* topology. It is not an artefact of the chosen one.

**Consequence of P8**: the G0 crate carries **no internal `#[cfg(test)]`
tests**. All tests live in the root-package `[[test]]` target, which CI already
runs. G0 therefore requires **no CI workflow change whatsoever**.

## Architecture

### Topology (created whole in G0.1; no later unit adds a file)

```text
crates/agent-contract-fs/
  Cargo.toml            # publish = false, edition 2024, rust-version 1.85, NO dependencies
  src/lib.rs            # the entire public surface
tests/contract/agent_contract_fs_test.rs   # the sole exerciser; [[test]] in root Cargo.toml
```

Root `Cargo.toml` gains: one `[workspace] members` entry, one
`[dev-dependencies]` path entry, one `[[test]]` block.
`.cargo/test-coverage-manifest.toml` gains one `[[surface]]`.

### Crate-level lints

`src/lib.rs` opens with `#![forbid(unsafe_code)]` and
`#![warn(clippy::pedantic)]`, mirroring the root crate (`src/lib.rs:10-11`).
Combined with P3, a pedantic finding inside the crate is a hard error during any
root build that compiles it.

### Public type surface — every item has a named exerciser

| Item | Exercised by |
|---|---|
| `enum FileKind { File, Dir, Symlink, Other }` | G0.3, G0.11 |
| `struct DirEntry { file_name: OsString, kind: FileKind }` | G0.1, G0.11 |
| `enum FsError` (9 variants, each carrying the offending path) | G0.7, G0.9, G0.11, G0.13, G0.15 |
| `trait FileAccess` — `symlink_metadata`, `canonicalize`, `read_dir`, `read_to_string` | G0.1, G0.3, G0.5 |
| `struct RealFs` + `impl FileAccess` | G0.5 |
| `struct FakeFs` + builder (`dir`, `file`, `symlink`, `unreadable`, `canonical`, `remove`) | G0.1, G0.3, G0.7, G0.9, G0.11, G0.13, G0.15 |
| `enum FsCall` + `FakeFs::calls()` | G0.7 |
| `struct ResolvedFile { rel_path: String, abs_path: PathBuf }` | G0.11, G0.15 |
| `fn resolve_corpus<F: FileAccess>(&F, &Path, &[&str]) -> Result<Vec<ResolvedFile>, FsError>` | G0.7, G0.9, G0.11, G0.13 |
| `fn read_corpus<F: FileAccess>(&F, &[ResolvedFile]) -> Result<Vec<(String, String)>, FsError>` | G0.15 |

Every row is `pub` and reachable from the crate root, so `dead_code` does not
fire (P3). **No `allow(dead_code)` appears anywhere in this package.** This is
the structural claim the review is asked to probe: it holds because a library
crate's reachable `pub` items are not dead, not because of discipline.

### The seam — exactly four methods

```rust
pub trait FileAccess {
    fn symlink_metadata(&self, path: &Path) -> Result<FileKind, FsError>; // lstat; never follows
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, FsError>;      // injectable
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, FsError>;    // non-filtering; lstat-derived kinds
    fn read_to_string(&self, path: &Path) -> Result<String, FsError>;
}
```

No size accessor, no following `metadata`, no mtime, no permissions, and **no
mutating operation of any kind**. "G0 performs no writes" is therefore provable
by inspecting the trait, not by testing.

### Resolver ordering (NON-NEGOTIABLE — this is the v3 fix)

For each allow-listed relative root, in the order given:

1. Lexically join the relative root to the workspace root.
2. `symlink_metadata` the joined root.
   `Symlink` ⇒ `RootIsSymlink`. Missing ⇒ `RootMissing`. `File`/`Other` ⇒
   `RootNotDirectory`. **This step precedes step 3.**
3. `canonicalize` the joined root **through the seam**; component-wise
   `Path::starts_with` against the canonicalized workspace root; not contained
   ⇒ `RootOutOfWorkspace`.
4. Recurse with `read_dir`. Each entry's lstat-derived kind decides:
   `Symlink` ⇒ `EntryIsSymlink` (rejected, never followed, never skipped);
   `Dir` ⇒ recurse; `File` ⇒ collect; `Other` ⇒ `EntryNotReadable` (rejected).
   Every kind has an explicit arm; **no kind falls through to being dropped**.
5. **No depth filter, no size filter, no ignore-file filter, no hidden-file
   filter, no extension filter.** Omission is unexpressible because no filtering
   construct exists in the code path.
6. Build `rel_path` from workspace-relative components joined with `/`; reject a
   non-UTF-8 component with `NonUtf8Path`; sort the result by `rel_path`.

Steps 2 and 3 **cannot** be reordered: `canonicalize` follows symlinks, so a
containment check on a canonicalized root has already discarded the information
that the root was a symlink, and has already followed it.

### Red-phase mechanism

Every `[R]` unit leaves the workspace **compiling**. New public functions and
trait impls introduced in a red phase have `todo!()` bodies, which type-check
and panic at run time, so the test fails for the intended reason while
`cargo build` and `cargo clippy` stay green. No `[R]` unit may leave the crate
non-compiling.

### Scenario convention

Each `[R]` unit adds **at most 3** `#[test]` functions. **One test asserts
exactly one outcome** — one `FsError` variant, or one success shape. Packing two
distinct error variants into one test is forbidden (closes v3 M-4).

## Failure Semantics

| Variant | Fires when | Carries | Proven by |
|---|---|---|---|
| `RootMissing` | joined root absent | path | G0.7 |
| `RootNotDirectory` | joined root is a file/other | path | G0.7 |
| `RootIsSymlink` | lstat of joined root reports symlink | path | G0.7 |
| `RootOutOfWorkspace` | canonical root not component-wise contained | path | G0.9 |
| `EntryIsSymlink` | entry lstat kind is symlink | path | G0.11 |
| `EntryNotReadable` | entry lstat kind is `Other` (neither file, dir, nor symlink) | path | G0.13 |
| `NonUtf8Path` | a path component is not valid UTF-8 | lossy-rendered path | G0.13 |
| `EnumerationFailed` | `read_dir` fails | path | G0.13 |
| `ReadFailed` | `read_to_string` fails (incl. disappearance) | path | G0.15 |

**There is no `EmptySet` variant.** An empty corpus is a valid G0 result;
judging emptiness is G1's concern (operator constraint: G0's taxonomy carries no
assertion-registry semantics).

### Reachability — stated as two distinct columns (closes v3 M-5)

| Variant | Runtime-reachable through `resolve_corpus`/`read_corpus` against `RealFs` | Deterministically inducible in CI |
|---|---|---|
| `RootMissing` | Yes | **Yes** — `FakeFs` and `RealFs` (tempdir) |
| `RootNotDirectory` | Yes | **Yes** — both |
| `RootIsSymlink` | Yes | **Yes** — `FakeFs` only (a real one needs elevation) |
| `RootOutOfWorkspace` | Yes | **Yes** — `FakeFs` only |
| `EntryIsSymlink` | Yes | **Yes** — `FakeFs` only |
| `EntryNotReadable` | Yes | **Yes** — `FakeFs` only |
| `NonUtf8Path` | Yes | **Yes** — `FakeFs` only |
| `EnumerationFailed` | Yes | **Yes** — `FakeFs` only |
| `ReadFailed` | Yes | **Yes** — `FakeFs` only |

The two columns are kept separate deliberately. v3 was faulted for a matrix that
answered "No — by design" to a *reachability* question when it meant *test
inducibility*. Every variant here is genuinely runtime-reachable; the second
column records only which injector induces it deterministically.

## Work Units

16 units — **8 `[R]`/`[G]` pairs**, no unpaired unit, no placeholder test.

**File-count exception, declared once**: G0.1 touches **5** files. Every other
unit touches **1**. The blanket claim "every unit touches ≤3 files" is therefore
**not** made (v3 advisory A-2 was raised for exactly such an overstatement).
G0.1's 5 files are a single mechanical registration act with no logic —
two manifests, one surface entry, one skeleton, one test file — estimated ~20
minutes, well inside the 2-hour rule.

### G0.1 `[R]` — Topology, registration, first failing test

* **Files owned (5, declared exception)**: `crates/agent-contract-fs/Cargo.toml`
  (new), `crates/agent-contract-fs/src/lib.rs` (new), `Cargo.toml`,
  `.cargo/test-coverage-manifest.toml`,
  `tests/contract/agent_contract_fs_test.rs` (new).* Create the crate (`publish = false`, edition 2024, rust-version 1.85, **no
  `[dependencies]` section at all**). Declare the whole public type surface with
  `todo!()` bodies. Add the workspace member, the root `[dev-dependencies]` path
  entry, the `[[test]]` block (`name = "agent_contract_fs_test"`,
  `path = "tests/contract/agent_contract_fs_test.rs"`), and a `[[surface]]` with
  `path = "crates/agent-contract-fs/"` and
  `targets = ["agent_contract_fs_test"]`.
* Add the **first real test**: `FakeFs` with two files under one directory
  returns both from `read_dir` with `FileKind::File`.
* **No placeholder test is created**, so no later unit is burdened with removing
  one (closes v3 M-1).
* **Success criteria**: (1) `cargo build --all-targets` succeeds. (2)
  `cargo test --test agent_contract_fs_test` fails, and the failure is the
  `todo!()` panic in `FakeFs::read_dir`. (3) `--mode completeness` reports the
  new target as mapped.

### G0.2 `[G]` — `FakeFs` node map and `read_dir`

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement the node map, the builder (`dir`, `file`, `symlink`, `unreadable`,
  `remove`), and `read_dir` returning entries in insertion-independent sorted
  order with lstat-derived kinds and **no following of symlink nodes**.
* **Success criteria**: (1) G0.1's test passes. (2) `cargo dev-test` is green.

### G0.3 `[R]` — Remaining `FakeFs` seam methods

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios: (a) `symlink_metadata` of a symlink node returns
  `FileKind::Symlink` and does **not** resolve to its target; (b) `canonicalize`
  returns the mapped path when a canonicalization entry exists and the input
  otherwise; (c) `read_to_string` of a file node returns its content.
* **Success criteria**: compiles; all three fail on `todo!()`.

### G0.4 `[G]` — Implement them

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: G0.3's three tests pass; `cargo dev-test` green.

### G0.5 `[R]` — `RealFs` privilege-free contract

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios against a `tempfile::TempDir`, using **only** operations that
  need no elevation on any platform: (a) `read_dir` of a real directory returns
  its real children with correct `Dir`/`File` kinds; (b) `symlink_metadata` of a
  real regular file returns `File`, and of a missing path returns `RootMissing`;
  (c) `read_to_string` of a real file returns its bytes as UTF-8.
* **No real symlink is created anywhere in this package.**
* **Success criteria**: compiles; all three fail on `todo!()`.

### G0.6 `[G]` — `RealFs` over `std::fs`

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* `std::fs::symlink_metadata`, `std::fs::canonicalize`, `std::fs::read_dir`,
  `std::fs::read_to_string`. Entry kinds come from
  `DirEntry::file_type()` (lstat semantics, does not follow). Plain recursion
  only — `ignore::WalkBuilder` is not used and `ignore` is not a dependency.
* **Success criteria**: G0.5's tests pass; `cargo dev-test` green; the crate's
  `Cargo.toml` still has no `[dependencies]`.

### G0.7 `[R]` — Root validation **ordering** (closes v3 finding A)

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios: (a) **root is a symlink** ⇒ result is
  `Err(FsError::RootIsSymlink)` **and** `FakeFs::calls()` contains
  `SymlinkMetadata(root)` and contains **zero** `Canonicalize(_)` and **zero**
  `ReadDir(_)` entries — this is the positive proof of ordering, not merely of
  verdict; (b) missing root ⇒ `RootMissing`; (c) root is a regular file ⇒
  `RootNotDirectory`.
* **Success criteria**: compiles; all three fail on the `resolve_corpus`
  `todo!()`.

### G0.8 `[G]` — `resolve_corpus` root validation

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement steps 1–2 of the resolver ordering plus call recording.
* **Success criteria**: (1) G0.7's three tests pass. (2) The call-log assertion
  in (a) passes, i.e. no canonicalization or enumeration occurred. (3)
  `cargo dev-test` green.

### G0.9 `[R]` — Containment (closes v3 finding B and advisory A-9)

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios, all through `FakeFs`'s canonicalization map — **no on-disk
  symlink, no junction, no elevation, identical on Windows and Unix**: (a) a
  lexically contained root whose canonical form lies outside the canonical
  workspace root ⇒ `RootOutOfWorkspace`; (b) a contained root canonicalizing
  within the workspace is accepted; (c) a workspace root canonicalizing to a
  Windows extended-length `\\?\`-prefixed path, with the root canonicalizing to
  a path beneath it, is **accepted** — proving component-wise `starts_with`
  rather than string-prefix comparison.
* **Success criteria**: compiles; (a) and (c) fail against the not-yet-correct
  implementation.

### G0.10 `[G]` — Component-wise containment

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement step 3 with `Path::starts_with`, never string comparison.
* **Success criteria**: G0.9's three tests pass; `cargo dev-test` green.

### G0.11 `[R]` — Exhaustive enumeration, no silent omission

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios: (a) a tree containing a depth-10 file, a 393,216-byte file, a
  dot-prefixed hidden file, and a file matching a `.gitignore` pattern —
  **all four appear** in the resolved set (positive no-omission proof; carried
  forward unchanged from v3, where this probe passed unanimously); (b) a symlink
  entry ⇒ `EntryIsSymlink`, i.e. **rejected, not skipped and not followed**; (c)
  a nested directory tree is fully descended — a file at depth 4 under three
  intermediate directories appears.
* **Success criteria**: compiles; all three fail on the enumeration `todo!()`.

### G0.12 `[G]` — Exhaustive recursive resolver

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement steps 4–6. No filtering construct of any kind appears in the code
  path. `Other`-kind entries are **rejected**, never dropped.
* **Success criteria**: (1) G0.11's three tests pass. (2) The implementation
  contains no `max_depth`, `max_filesize`, extension filter, hidden filter, or
  ignore-file filter. (3) `cargo dev-test` green.

### G0.13 `[R]` — Enumeration faults and path validity

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios, one asserted outcome each: (a) a directory whose `read_dir`
  is unreadable ⇒ `EnumerationFailed` naming that directory; (b) an entry whose
  lstat kind is `Other` ⇒ `EntryNotReadable` naming it — rejected, never
  dropped; (c) an entry whose name is not valid UTF-8 ⇒ `NonUtf8Path`
  (constructed via a small `cfg`-split helper: `OsStringExt::from_vec` on Unix,
  `OsStringExt::from_wide` with an unpaired surrogate on Windows).
* **Success criteria**: compiles; all three fail.

### G0.14 `[G]` — Enumeration fault handling

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* Implement the exhaustive `FileKind` match with an explicit arm per kind and
  UTF-8 validation of each path component.
* **Success criteria**: (1) G0.13's three tests pass. (2) The `FileKind` match
  has no wildcard arm, so a future variant cannot be silently dropped. (3)
  `cargo dev-test` green.

### G0.15 `[R]` — Determinism and the read layer

* **Files owned (1)**: `tests/contract/agent_contract_fs_test.rs`.
* Three scenarios: (a) two `FakeFs` instances describing the same tree with
  different insertion orders produce **identical** `rel_path` sequences; (b)
  every `rel_path` uses `/` separators regardless of platform, asserted on a
  nested tree; (c) a file present at resolve time and removed before
  `read_corpus` ⇒ `ReadFailed` naming it.
* **Success criteria**: compiles; all three fail.

### G0.16 `[G]` — `read_corpus` and ordering

* **Files owned (1)**: `crates/agent-contract-fs/src/lib.rs`.
* **Success criteria**: (1) G0.15's three tests pass. (2) `cargo dev-test`,
  `cargo lint`, and `cargo fmt-check` all green. (3) `--mode completeness`
  passes. (4) No `allow(dead_code)` anywhere in the package. (5) The crate's
  `Cargo.toml` has no `[dependencies]` section.

## Verification

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo test --test agent_contract_fs_test
pwsh scripts/test-coverage-oracle.ps1 --mode completeness
```

Plus two explicit greps as acceptance evidence:

```text
rg "allow\(dead_code\)" crates/agent-contract-fs tests/contract/agent_contract_fs_test.rs   # expect 0
rg "max_depth|max_filesize|WalkBuilder" crates/agent-contract-fs                             # expect 0
```

## Constitution Check

Mapped against the real `.github/instructions/constitution.instructions.md`
principles by their real names. No principle name is invented (closes v3 M-3).

| Principle | Applies | How G0 complies |
|---|---|---|
| **I. Safety-First Rust** | Yes | Rust 2024; crate declares `#![forbid(unsafe_code)]` and `#![warn(clippy::pedantic)]`; all fallible operations return `Result<_, FsError>`; no `unwrap`/`expect` in crate code. *Deviation noted*: the crate returns `FsError`, not the root crate's `EngramError` — it is a standalone dev-only crate with no dependency on `engram`, so `EngramError` is not in scope. Introducing that dependency to satisfy the letter of the constraint would violate VI and couple a test seam to the product runtime. |
| **II. Test-First Development (NON-NEGOTIABLE)** | Yes | Every unit is an `[R]`/`[G]` pair; 7 pairs; each `[R]` compiles and is observed failing before its `[G]`. Tests live in `tests/contract/`, the mandated contract tier. All pass via `cargo dev-test`. |
| **III. Workspace Isolation and Security Boundaries** | Yes | This is G0's entire subject: two-layer containment (allow-listed roots plus canonical workspace containment), root-symlink rejection before canonicalization, entry-symlink rejection before following. No secrets are read or written. |
| **IV. CLI Workspace Containment (NON-NEGOTIABLE)** | Yes | The seam exposes **no mutating operation at all**, so G0 cannot create, modify, or delete anything anywhere. Reads are confined to canonically contained paths. |
| **V. Structured Observability** | Yes | Every failure is a typed variant carrying the offending path; `FakeFs::calls()` gives a structured call trace. No untyped string errors. |
| **VI. Single Responsibility** *(body: dependency minimality)* | Yes | **Zero new external dependencies.** The crate has no `[dependencies]` section; it is `std`-only. `tempfile` used by the exerciser is already a root dependency (P12). |
| **VII. Destructive Command Approval (NON-NEGOTIABLE)** | N/A | G0 proposes no destructive command. The seam has no mutating operation. Execution of this plan runs only `cargo` build/test/lint and read-only greps. |
| **VIII. Explicit Safety Modes for Elevated Risk** | Yes | Executed under **freeze-scope mode**: the declared boundary is the four files listed in G0.1 plus `crates/agent-contract-fs/src/lib.rs`. No file outside that set is touched by any unit. |
| **IX. Git-Friendly Persistence** | Yes | All planning artifacts are markdown with YAML frontmatter. The crate adds no serialized state. Manifest additions are single sorted entries that do not reorder existing content. |
| **X. Agent Context Efficiency** | Partial | The crate returns a structured `Vec<ResolvedFile>` rather than raw content from `resolve_corpus`; content is fetched separately by `read_corpus`, so a consumer that only needs paths never pays for bodies. *Honest limit*: `read_corpus` returns full file bodies, which is inherently bulk — but the 92-file, ≤98.5 KB corpus (P11) is bounded and the consumer is a test binary, not an agent context. |
| **XI. Merge Commit History Preservation (NON-NEGOTIABLE)** | Yes | G0 ships as its own shipment and its own PR, merged with a merge commit. Splitting G into three units produces three merge commits, which improves bisect granularity rather than degrading it. No squash, no rebase. |
| Development Workflow #5 — **No dead code** | Yes | Structurally satisfied: all public items are reachable from the crate root of a library crate, so `dead_code` does not fire under `-Dwarnings` (P3). **Zero `allow(dead_code)`**, verified by an explicit grep in Verification. |
| Task Granularity — **2-Hour Rule** | Yes | 16 units; 15 touch 1 file; G0.1 touches 5 as a declared, justified mechanical exception. Every unit has ≤3 test scenarios. |
| Task Granularity — **Width Isolation** | Yes | Each `[R]` unit touches only the test file; each `[G]` unit touches only the crate source. The single exception, G0.1, is registration-only and contains no logic. |
| Quality Gates | Yes | All four gates are in Verification, in the constitution's order. `cargo audit` is unaffected: the dependency graph does not change. |

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | `cargo clippy --all-targets` at the root lints the root package's targets but not the G0 crate's own pedantic lints. | Partly mitigated: `-Dwarnings` (P3) still applies to the crate at compile time, so `dead_code` and rustc lints are hard errors. The residual pedantic-lint gap is explicitly assigned to **G2** by split-deliberation Decision 2. G0 does not silently claim full pedantic coverage. |
| R2 | Adding a `[dev-dependencies]` path entry could change the root dependency graph. | Dev-dependencies are not linked into `cargo build --release`; the crate has no dependencies of its own, so the resolved graph gains exactly one local path node and zero registry crates. Verified by `cargo audit` being unchanged. |
| R3 | The `NonUtf8Path` test needs a `cfg`-split construction helper. | Bounded to ~8 lines in the test file, both arms using safe `std` APIs (`OsStringExt`). If a platform arm proves unworkable, the fallback is to `cfg`-gate the scenario to the platform where it is constructible and record the gap explicitly rather than deleting the variant. |
| R4 | `FakeFs` diverges from real filesystem semantics, so proofs are about the fake, not reality. | Bounded by design: G0.5/G0.6 pin `RealFs` against a real `tempfile` tree for every privilege-free operation, so the two implementations are cross-checked on the shared subset. The fake is authoritative only for the six fault classes that real filesystems cannot produce without privileges — which is precisely why the seam exists. This is stated as a limit, not hidden. |

## Plan Hardening

### H1 — Why the ordering fix cannot silently regress

The root-symlink property is not asserted through its error variant alone. G0.7
scenario (a) asserts the **call log**: `SymlinkMetadata(root)` present,
`Canonicalize(_)` absent, `ReadDir(_)` absent. Any future refactor that
canonicalizes first will still return `RootIsSymlink` and will still fail this
test, because the fail-open is the traversal, not the verdict. The test is
written against the mechanism, not the symptom.

### H2 — Why Windows determinism is real and not claimed

No test in this package creates an on-disk symlink or junction. The canonical-
escape case (G0.9a), the root-symlink case (G0.7a), and the extended-length-path
case (G0.9c) are all expressed as entries in `FakeFs`'s canonicalization and node
maps. They execute identically on `windows-latest` and `ubuntu-latest` with no
elevation and no developer mode. `RealFs` is exercised (G0.5) only on operations
that require no privilege on any platform. The plan therefore claims no proof it
cannot deliver — the specific criticism that terminated v3.

### H3 — Why no file can be silently omitted

Two independent arguments, both required:

1. **Structural**: after inherited Decision 6.1/6.2, the code path contains no
   filtering construct — no depth counter compared against a limit, no size
   check, no extension match, no hidden-file test, no ignore-file consultation,
   no `WalkBuilder`. Omission is not avoided; it is **unexpressible**. G0.12
   success criterion (2) and the Verification grep enforce this mechanically.
2. **Positive**: G0.11a proves a depth-10 file, a 393,216-byte file, a hidden
   file, and a `.gitignore`d file all appear. This probe passed unanimously in
   v3 and is carried forward unchanged.

Additionally, a symlink entry and an `Other`-kind entry are **rejected with a
named error**, never dropped — so "not present in the output" always implies
"not present in the tree", never "quietly filtered".

### H4 — Why there is no dead code, and what would break the claim

The claim rests on one mechanism: `dead_code` does not fire on `pub` items
reachable from a **library crate root**. It would break if (a) an item were made
private and left unused, or (b) the crate were converted to a binary or a
test-only module. Neither is proposed. The Verification grep asserts zero
`allow(dead_code)` as the observable consequence. If a reviewer finds any item
without a named exerciser in the Architecture table, that is a defect in this
plan, not a case for suppression.

### H5 — Why G0 is genuinely independently shippable

G0's change set is: two new files, three single-entry manifest additions, one new
test file. It is green under `cargo dev-test`, `cargo lint`, `cargo fmt-check`,
and `--mode completeness` **before G1 exists**, and requires no CI workflow
change (P8). Nothing in G0 references assertions, registries, contracts, or the
`.github` harness layout. If G1 and G2 were never built, G0 would still be a
complete, useful, fully exercised library.

### H6 — What G0 deliberately does **not** guarantee

Stated explicitly so no downstream package inherits a false guarantee:

* G0 does **not** guarantee the resolved corpus is non-empty. Emptiness is a
  valid result; judging it belongs to G1.
* G0 does **not** guarantee TOCTOU-freedom between `resolve_corpus` and
  `read_corpus`. A file can disappear in between; that is surfaced as
  `ReadFailed` (G0.13b), never as a silent short read.
* G0 does **not** guarantee that `RealFs` behaves identically to `FakeFs` for
  the six privileged fault classes. It guarantees only that the *resolver logic*
  is identical, because the resolver is generic over `FileAccess`.
* G0 does **not** lint its own crate under `clippy::pedantic` via the root
  `cargo lint` alias (R1). That gap is owned by G2.

### H7 — Rollback

G0's entire footprint is additive: one new directory, one new test file, and
three single-line manifest entries. Reverting the merge commit removes all of it
with no residue in the product runtime, no dependency-graph change, and no CI
configuration change.

### H8 — Blast radius

`src/` is not touched. `.github/workflows/` is not touched. No existing test,
target, surface, or alias is modified — only additions. The product binary is
unchanged byte-for-byte, because the crate is a dev-dependency and is never
linked into a release build.
