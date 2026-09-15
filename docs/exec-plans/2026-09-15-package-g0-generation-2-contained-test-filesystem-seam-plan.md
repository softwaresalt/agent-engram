---
title: "Package G0 generation 2 — contained test-filesystem seam (implementation plan)"
type: exec-plan
doc_type: exec-plan
date: 2026-09-15
agent: stage
plan_status: blocked
program: checkpoint-resolution finalization write-boundary
program_item: 027-D
package: G0
generation: 2
plan_attempt: 1
source_document: docs/decisions/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-deliberation.md
authority: docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md
branch: chore/stage-g0-generation-2-planning
base_commit: f9425943ec6bdb6431389b25d1f2057f51755d3c
---

## Source

This plan implements **only** the decisions settled in
`docs/decisions/2026-09-15-package-g0-generation-2-contained-test-filesystem-seam-deliberation.md`
(D1–D9). It does not revise them. Program boundary authority is the program
lock; live status is backlog item `027-D`.

## Problem frame

Package G0 must author a **contained test-filesystem seam** — the exerciser and
manifest surface that G1 (typed assertion evaluation) and G2 (registry, CI) will
consume. The seam resolves a corpus of files under a workspace and later reads
them through an injectable filesystem abstraction.

Generation 1 died because `read_corpus` re-checked **workspace containment** at
read time instead of comparing **authorized identity**. Containment is invariant
under any redirect that stays inside the workspace, so an ancestor directory
replaced by a symlink pointing elsewhere *inside* the workspace passed every
check and yielded a different file.

The fix, per D4, is a **phase separation**: containment is input validation at
resolve time only; **identity equality is the sole read-time authority**.

### Verified preconditions (read-only, at `f9425943`)

| Fact | Evidence |
|---|---|
| The seam does not exist | Zero matches for `ResolvedCorpus`, `ResolvedFile`, `resolve_corpus`, `read_corpus`, `FakeFs`, `enum FsOp` across `src/`, `tests/`, `crates/` |
| `-Dwarnings` is global | `.cargo/config.toml` `[build] rustflags = ["-Dwarnings"]` |
| Canonical local gate | `cargo dev-test` = `test --all-targets` (includes colocated `--lib` tests) |
| Lint gate is pedantic | `cargo lint` = `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` |
| `thiserror = "1"` available | `Cargo.toml:44` |
| No new dependency needed | Design uses `std` only; `cap-std` is present but not required by the adopted Option A |
| `src/services/` is a **declared** coverage surface | `.cargo/test-coverage-manifest.toml:120`, targets `["contract_*","integration_*","unit_*","cold_*","helpers_*"]` |
| A `unit_*` target needs **no** manifest edit | `unit_*` is already globbed by the `src/services/` surface |
| `unit_*` target precedent | `Cargo.toml:1065` — `unit_workspace_toctou` → `tests/unit/workspace_toctou_test.rs` |
| Cross-platform dir-link fixture precedent | `tests/unit/workspace_toctou_test.rs::make_dir_link` (Unix symlink / Windows `mklink /J`, no elevation) |

**Module placement decision.** The seam lives at **`src/services/testfs/`**.
Rationale: the coverage oracle's `completeness` mode fails if *any top-level
module under `src/` is not a declared surface*. A new top-level module would
force a `.cargo/test-coverage-manifest.toml` edit, and **oracle registration is
explicitly G2 scope**. Nesting under the already-declared `src/services/`
surface means G0 adds **no** manifest entry and therefore **no G2 leakage**.

## Requirements trace

| Source decision | Implementation unit(s) |
|---|---|
| D1 — identity is the normalized canonicalized absolute path | U2 |
| D2 — `CanonicalIdentity` opaque; `Clone + PartialEq + Eq` only; no `Debug`/`Display` | U2, U8 |
| D3 — identity stored privately in `ResolvedFile`; no accessor | U6 |
| D4 — containment at resolve time; **identity equality only** at read time | U6 (capture), U7 (compare) |
| D5 — one new `FsError::IdentityMismatch { rel_path }`, 14 variants total | U1, U7 |
| D6 — one new `FsOp::CanonicalIdentity`, ten call sites; trait method signature | U1, U3, U11 |
| D7 — seam in the library crate; fixed visibility rules | U1–U7 |
| D8 — 9–12 tasks; identity channel bundled into one unit | This plan: 11 units; U6 bundles the identity channel |
| D9 — hermetic primary test + real-FS secondary, asserting the *specific* error | U9 (primary), U10 (secondary) |
| S1 — ancestor substitution inside the workspace is rejected for the identity reason | U9, U10 |
| S2 — identity opaque at the type level | U2, U8 |
| S4 — every claim delivered by a named mechanism; residuals disclosed | U7 rustdoc, Risks section below |
| S5 — no G1/G2 scope | Module placement decision; no assertion/registry/CI unit exists |

## Implementation units

Every unit carries, as a standing acceptance criterion: **`cargo dev-test`
passes and `cargo lint` is clean** (pedantic, `-D warnings`). This closes gen-1
finding **M-2** (`clippy::missing_panics_doc` on staged `todo!()` arms) by
making rustdoc `# Errors` / `# Panics` sections a per-unit gate rather than a
cleanup pass.

Units are staged under the settled channel-bundling rule (settled item 7): a
unit never leaves a type readable before it is writable, and never leaves a
match non-exhaustive under `-Dwarnings`.

---

### U1 — Error and operation taxonomy

* **Domain:** code. **Size: S | Complexity: low.**
* **Files:** `src/services/testfs/mod.rs`, `src/services/testfs/error.rs` (2).
* **Changes:** Declare the module under `src/services/`. Author `FsError` with
  all **14** variants (13 inherited per settled item 6, plus
  `IdentityMismatch { rel_path: String }` per D5) and `FsOp` with all **10**
  discriminants (9 inherited per settled item 4, plus `CanonicalIdentity` per
  D6). Derives: `FsError: Debug + Clone + PartialEq + Eq + thiserror::Error`;
  `FsOp: Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord`.
* **Tests:** colocated `--lib` unit tests — every variant renders a message; no
  variant's `Display` contains an absolute path or a path separator sequence
  originating from a workspace root.
* **Posture:** test-first.
* **Acceptance:** 14 variants and 10 discriminants exist; `FsOp: Ord` compiles
  (required for the `BTreeMap` fault key); `IdentityMismatch` renders only
  `rel_path`.

### U2 — `CanonicalIdentity` opaque type and prefix normalization

* **Domain:** code. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/identity.rs`, `src/services/testfs/mod.rs` (2).
* **Changes:** `pub struct CanonicalIdentity(PathBuf)` with a **private** field
  and derives **exactly** `Clone, PartialEq, Eq` (D2). No `Debug`, `Display`,
  `Hash`, `Ord`, `Serialize`, `From`, `Deref`, or `AsRef`. Private constructor
  only. Platform-prefix normalization by **component inspection** — handling
  `Prefix::VerbatimDisk` and `Prefix::VerbatimUNC` — re-implemented inside the
  seam (G0 must not modify `src/db/workspace.rs`).
* **Tests:** colocated — `\\?\C:\a\b` and `C:\a\b` normalize equal;
  `\\?\UNC\srv\share\x` and `\\srv\share\x` normalize equal; two different
  paths compare unequal; non-UTF-8 path components survive normalization.
* **Posture:** test-first.
* **Acceptance:** equality holds across verbatim/non-verbatim prefix forms; the
  type exposes no route to its inner value.

### U3 — Seam trait and real implementation

* **Domain:** code. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/fs.rs`, `src/services/testfs/mod.rs` (2).
* **Changes:** Author the seam trait with all ten operations, including
  `fn canonical_identity(&self, path: &Path) -> Result<CanonicalIdentity, FsError>`
  (D6), and `read_dir -> Result<Vec<OsString>, FsError>` (names only, settled
  item 5). Author `RealFs`, the `std`-backed implementation.
* **Tests:** colocated — `RealFs` against a `TempDir`: `canonical_identity` of
  the same path twice is equal; of two distinct files, unequal.
* **Posture:** test-first.
* **Acceptance:** all ten operations are trait methods; `read_dir` returns names
  without kind, so the caller must lstat explicitly.

### U4 — `FakeFs` synthetic path space and constructor

* **Domain:** code. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/fake.rs`, `src/services/testfs/mod.rs` (2).
* **Changes:** `FakeFs` over an in-memory path space supporting files,
  directories, and **link entries** (so ancestor substitution is modelled
  without touching a real filesystem). Provide an **explicit public
  constructor** and builder methods — this closes gen-1 finding **M-1**
  (missing `FakeFs` constructor). Implement the seam trait, resolving link
  entries during `canonical_identity` so a redirected ancestor yields a
  different identity.
* **Tests:** colocated — build a tree; assert `canonical_identity` follows a
  link entry; assert an unlinked path is stable.
* **Posture:** test-first.
* **Acceptance:** `FakeFs` is constructible from outside the module; no real
  filesystem or privilege is used; interior mutability (`RefCell`) imposes no
  `Sync` bound (settled item 6).

### U5 — Operation-keyed fault injection and call log

* **Domain:** code. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/fake.rs`, `src/services/testfs/fault.rs` (2).
* **Changes:** `(FsOp, PathBuf)`-keyed injection over a `BTreeMap` (settled
  item 4), plus a call log recording `(FsOp, PathBuf)` in order. Post-resolve
  mutation support so the two call-site pairs sharing a `(site, operation)`
  discriminant remain independently addressable.
* **Tests:** colocated — an injected fault fires for its exact key and for no
  other; the call log records order faithfully.
* **Posture:** test-first.
* **Acceptance:** each of the ten operations is independently injectable;
  injection is keyed on the pair, never on the operation alone.

### U6 — Corpus types, `resolve_corpus`, and identity capture *(identity channel — bundled)*

* **Domain:** code. **Size: M | Complexity: high.**
* **Files:** `src/services/testfs/corpus.rs`, `src/services/testfs/mod.rs` (2).
* **Changes:** `ResolvedFile { rel_path: String, identity: CanonicalIdentity }`
  (both **private**; only `pub fn rel_path(&self) -> &str`) and
  `ResolvedCorpus { workspace, files }` (private fields, `resolve_corpus` the
  **sole** constructor) — settled items 1 and 2, extended per D3.
  `resolve_corpus` performs: canonicalize-both-sides workspace containment as
  **input validation** producing `OutOfWorkspace`; explicit per-entry lstat over
  `read_dir -> Vec<OsString>` (settled item 5); and **capture of the authorized
  `CanonicalIdentity` for every resolved file**.
* **Why bundled:** capturing the identity and storing it in `ResolvedFile` must
  land in the **same** unit that makes the field readable. Splitting them is
  exactly gen-1 carried-forward finding **B** (`ResolvedCorpus::workspace`
  staged at G0.36 but first read at G0.32). Bundling closes it by construction.
* **Tests:** colocated — a path outside the workspace yields `OutOfWorkspace`; a
  resolved corpus stores one identity per file; `rel_path()` never returns an
  absolute path.
* **Posture:** test-first.
* **Acceptance:** no public field and no public constructor other than
  `resolve_corpus`; no accessor exposes `identity`; containment error is
  produced **here**, at resolve time.

### U7 — `read_corpus` with identity-equality-only read gate

* **Domain:** code. **Size: M | Complexity: high.**
* **Files:** `src/services/testfs/corpus.rs`, `src/services/testfs/read.rs` (2).
* **Changes:** `read_corpus(&ResolvedCorpus, &dyn TestFs)` re-derives each
  file's `CanonicalIdentity` from the live filesystem and compares it for
  **equality** against the stored authorized value. Inequality →
  `FsError::IdentityMismatch { rel_path }`. **No containment check is performed
  at read time** (D4). Rustdoc on `read_corpus` states the guarantee *and*
  discloses residuals **Res-1** (leaf-component temporal window) and **Res-2**
  (same-canonical-path content replacement) at their stated scope.
* **Tests:** colocated — a corpus read with an unmodified filesystem succeeds;
  a corpus whose stored identity is made stale fails with `IdentityMismatch`.
* **Posture:** test-first.
* **Acceptance:** `grep` of the read path finds **no** containment/`starts_with`
  check; the identity comparison is `==` against the stored value and never
  parses, re-derives, or reconstructs it; rustdoc residual scope matches the
  deliberation verbatim in meaning.

### U8 — `compile_fail` forgery and opacity proof

* **Domain:** tests. **Size: S | Complexity: low.**
* **Files:** `src/services/testfs/corpus.rs`, `src/services/testfs/identity.rs`
  (doctests) (2).
* **Changes:** `compile_fail` doctests proving (a) `ResolvedCorpus` and
  `ResolvedFile` cannot be struct-literal forged (settled item 3), and (b)
  `CanonicalIdentity`'s inner value is unreachable and the type cannot be
  `Debug`-formatted (D2/S2).
* **Tests:** the doctests are the test.
* **Posture:** test-first.
* **Acceptance:** each doctest fails to compile for the intended reason. **Note
  (inherited H10):** doctest execution in CI is **G2 scope**; locally
  `cargo test --doc` proves them. This plan claims *local* proof only.

### U9 — Hermetic ancestor-substitution proof *(primary — never skips)*

* **Domain:** tests. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/corpus.rs` colocated test module (1).
* **Changes:** Using `FakeFs` only: build `ws/a/b/file`, `resolve_corpus`, then
  replace ancestor `ws/a` with a link entry redirecting to `ws/z` — **still
  inside the workspace** — and `read_corpus`. Add the symmetric inverse case
  (ancestor was a link at resolve time, is a real directory at read time).
* **Tests:** both assert the error is **specifically** `FsError::IdentityMismatch`.
* **Posture:** test-first (red before U7's comparison exists, green after).
* **Acceptance:** proves **S1**. Runs on every platform with **no** privilege,
  **no** real symlink, and **no** skip path. A test asserting merely "an error
  occurred" fails this acceptance criterion, because it would also pass for a
  containment reason and thereby reproduce the generation-1 category error
  inside the test itself.

### U10 — Real-filesystem corroboration *(secondary)*

* **Domain:** tests. **Size: M | Complexity: medium.**
* **Files:** `tests/unit/testfs_identity_test.rs`, `Cargo.toml` `[[test]]` entry
  (2).
* **Changes:** New `[[test]] name = "unit_testfs_identity"` target, following
  the `unit_workspace_toctou` precedent. Reuse the proven `make_dir_link`
  pattern (Unix symlink / Windows `mklink /J` junction, no elevation). Stage the
  same ancestor substitution against `RealFs` under a `TempDir`.
* **Tests:** asserts `IdentityMismatch`; where privilege is genuinely
  unavailable, prints an explicit `SKIPPED:` line and still asserts everything
  it can — **never a silent pass**.
* **Posture:** test-first.
* **Acceptance:** `unit_*` requires **no** coverage-manifest edit (already
  globbed by the `src/services/` surface); `TempDir` outlives the fixture
  (compound learning `tempdir-lifetime-in-contract-tests`).

### U11 — Ten-call-site fault-injection coverage proof

* **Domain:** tests. **Size: M | Complexity: medium.**
* **Files:** `src/services/testfs/fault.rs` colocated test module,
  `src/services/testfs/corpus.rs` colocated test module (2).
* **Changes:** A table-driven proof that each of the **ten** fallible call sites
  has an independent injector that genuinely reaches and drives it, including
  the new `FsOp::CanonicalIdentity` site. For each site, assert the fixture
  reaches the intended operation rather than short-circuiting at an earlier one
  — the exact trap that killed attempt 2's `ReadToString` case.
* **Tests:** ten cases plus the call-log assertion (gen-1 M-1 closure).
* **Posture:** test-first.
* **Acceptance:** every site fires independently; the call log proves the target
  operation was actually reached in each case.

## Dependency graph

```text
U1 ──> U2 ──> U3 ──> U4 ──> U5
                      │      │
                      └──────┴──> U6 ──> U7 ──> U9
                                   │       │
                                   └─> U8  └─> U10
                                           └─> U11
```

Acyclic. Critical path: `U1 → U2 → U3 → U4 → U5 → U6 → U7 → U9`. `U8`, `U10`,
and `U11` are leaves and may be sequenced in any order after their predecessor.

## Decisions and rationale

| # | Decision | Rationale |
|---|---|---|
| P1 | Seam at `src/services/testfs/` | Avoids a new top-level `src/` module, which the oracle's `completeness` mode would require be declared — and oracle registration is G2 scope. Zero manifest edits, zero G2 leakage. |
| P2 | Test target named `unit_testfs_identity` | `unit_*` is pre-globbed by the `src/services/` surface, so no manifest edit is required. Follows `unit_workspace_toctou`. |
| P3 | Primary proof is hermetic (`FakeFs`), not real-FS | A real-symlink test can skip without privilege; a skippable test cannot be the proof of the package's central invariant. |
| P4 | Identity capture and storage bundled in U6 | Closes gen-1 carried-forward finding B by construction rather than by ordering discipline. |
| P5 | No containment at read time | The direct correction of gen-1 finding A. Containment is demoted to resolve-time input validation. |
| P6 | Re-implement prefix normalization inside the seam | G0 must not modify production code; making `src/db/workspace.rs::normalize_canonical` public would be an out-of-scope production change. |
| P7 | No new dependency | Option A needs only `std` + `thiserror` (already present). `cap-std` remains unused by G0. |
| P8 | Lint/rustdoc compliance is a per-unit criterion | Closes gen-1 M-2 without a separate cleanup task, keeping the unit count inside D8's 9–12 bound. |

## Risks and caveats

| Risk | Severity | Mitigation |
|---|---|---|
| Asserting a guarantee the mechanism does not deliver (killed gen-1 three times) | **High** | U7 rustdoc discloses Res-1/Res-2 at their stated scope; U9 acceptance forbids a generic "an error occurred" assertion; the residual table below is scoped per row, never global |
| Res-1: leaf-component temporal window between compare and read | Medium | **Disclosed, not closed.** Ancestor chain *is* covered. Option C (retained root anchor) is the named upgrade path if a later package proves it matters |
| Res-2: same-canonical-path in-place content replacement | Medium | **Disclosed, not closed.** Out of scope — the invariant concerns identity of the object named, not content integrity |
| Res-3: hard links compare unequal | Low | **Correct by design** — the seam authorizes a path, so a different path is a different authorization |
| Res-4: real-FS test may skip without privilege | Low | Bounded to the secondary layer (U10); the primary layer (U9) never skips |
| Res-5: a malicious `TestFs` impl could lie about identity | Low | **Accepted.** Test-only surface; no production trust boundary depends on it |
| Windows verbatim-prefix mismatch causes false `IdentityMismatch` | Medium | U2 normalizes by component inspection with explicit tests for both `VerbatimDisk` and `VerbatimUNC` |
| Non-UTF-8 path components | Low | Identity holds `PathBuf`, not `String`; U2 covers a non-UTF-8 case |
| Unit count drifts upward, cascading review churn | Medium | D8 bounds to 9–12; this plan is 11; P8 folds lint work into units rather than adding one |
| Scope creep into G1/G2 | Medium | No unit performs assertion evaluation, registry seeding, real-root binding, or CI wiring; P1/P2 keep the oracle manifest untouched |

## Runtime verification and closure

**Runtime surface changed: none.** The seam is test infrastructure in the
library crate. It is not reachable from the CLI, the MCP server, the daemon, or
any background job. No feature flag, no migration, no rollout, no data or config
mutation.

| Unit | Runtime surface | Verification | Closure artifact |
|---|---|---|---|
| U1–U8, U11 | None | `cargo dev-test`, `cargo lint` | Plan + review record |
| U9 | None | Hermetic red→green on the ancestor-substitution scenario, on every platform | The red→green transition is the closure evidence for S1 |
| U10 | None | Real-FS corroboration; explicit `SKIPPED:` where unprivileged | Skip reasons recorded in test output |

**Rollback:** the entire package is additive and self-contained under
`src/services/testfs/` plus one `[[test]]` entry. Reverting the release unit
removes it wholly; nothing else depends on it until G1, which is `blocked`.

**Ownership and validation window:** not applicable — no production surface, no
SLI, no on-call responder. This is stated explicitly rather than inventing a
monitoring table, because gen-1's fatal defect was an unearned claim, and an
unobservable SLI is the same error in a different section.

## Plan Hardening Signals (REQUIRED)

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | **PRESENT** | D2/D3/D6/D7 introduce a new public surface (`CanonicalIdentity`, `ResolvedCorpus`, `ResolvedFile`, the seam trait, `FsError`, `FsOp`, `FakeFs`) that **G1 inherits and must not redesign**. A contract error here propagates to two downstream packages. |
| Security, auth, permission, or compliance-sensitive behavior | **PRESENT** | The identity-equality gate *is* the security-relevant mechanism, and the package exists because its predecessor got exactly this wrong. Symlink/junction handling and the absolute-path non-disclosure rule (SEC-3) are both in scope. |
| Migration, backfill, destructive data/config action, irreversible step | Absent | Additive only; no data, schema, or config migration. Fully revertible. |
| External integration, operator checkpoint, or external dependency | Absent | No new dependency (P7); no network, no external service, no operator checkpoint. |
| High runtime, rollout, or rollback risk | Absent | No runtime surface; rollback is deletion of an additive, unconsumed module. |

**Requires plan hardening: yes**

## Plan Hardening

### Was hardening required, and why

**Yes.** Two signals are present: a **public API/contract change** that two
downstream packages (G1, G2) inherit and are forbidden to redesign, and
**security-sensitive behavior** — the identity gate is the exact mechanism whose
predecessor got it wrong.

**Hardening discipline for this package (H0).** Generation 1 did not die from
insufficient hardening. It died *inside* its hardening section: item H6 claimed
the ancestor-substitution class was closed when it was closed only for redirects
leaving the workspace. **This section therefore hardens by narrowing claims,
adding falsifiable checks, and naming non-closures — never by asserting
closure.** Any sentence here that reads as a global guarantee is a defect.

### Learnings and instructions consulted

| Source | Applied as |
|---|---|
| `security/canonicalize-both-sides-workspace-path-containment-2026-07-01.md` | H3 — both-sides canonicalization; never resolve against CWD; normalize Windows verbatim prefix before comparison. Explicitly **not** used as authority for read-time gating |
| `path-handling/windows-drive-relative-path-traversal-component-prefix-2026-05-06.md` | H3 — component inspection, never string matching |
| `testing/tempdir-lifetime-in-contract-tests-2026-03-30.md` | H6 — `TempDir` binding lifetime in U10 |
| `testing/pub-visibility-for-external-test-harness-2026-04-20.md` | H7 — U10 reaches the seam only through `pub` items |
| `testing/hermetic-all-target-per-test-process-explosion-2026-08-02.md` | H6 — U9/U11 stay in-process; no process spawning |
| `workflow-issues/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md` | H9 — review method is the multi-model adversarial panel |
| `workflow-issues/in-plan-task-granularity-splitting-cascades-review-churn-...-2026-07-25.md` | H8 — unit count held at 11 |
| `.github/instructions/strict-safety.instructions.md` | `ProposedAction` / `ActionRisk` table below |
| `.github/instructions/rust.instructions.md`, `technology-rust.instructions.md` | H2, H4 — derive discipline, error taxonomy, pedantic rustdoc |
| `tests/unit/workspace_toctou_test.rs` (read-only precedent) | H6 — `make_dir_link`, `SKIPPED:` discipline |
| `src/db/workspace.rs` (read-only precedent) | H3 — `normalize_canonical` component-inspection shape |

### Protected invariants

| ID | Invariant | Falsifiable check |
|---|---|---|
| **INV-1** | Read-time canonical identity must EQUAL the stored authorized identity | U9 primary hermetic test asserts `FsError::IdentityMismatch` specifically |
| **INV-2** | The authorized identity is compared for equality only — never parsed, re-derived, or reconstructed | H1 grep gate below |
| **INV-3** | No containment check occurs on the read path | H1 grep gate below |
| **INV-4** | The identity value is never disclosed in any error, log, or `Debug` output | H2 gate below |
| **INV-5** | `resolve_corpus` is the sole constructor of a corpus; no forgery route exists | U8 `compile_fail` doctests |
| **INV-6** | G0 modifies no production code and no CI/oracle registration | H5 gate below |

### Reinforced instructions

* **H1 — Mechanical read-path gate (closes INV-2, INV-3).** Before U7 is
  considered complete, a reviewer must confirm, by inspection of the read path
  (`read.rs` plus the `read_corpus` body), that it contains **no** occurrence of
  `starts_with`, `contains`, `strip_prefix`, or any `workspace`-rooted
  comparison, and that the only operation performed against the stored identity
  is `==`. This is a **mechanical, falsifiable** check, deliberately not a
  judgement call — gen-1 finding A was missed precisely because the read path
  *looked* defended.
* **H2 — Disclosure gate (closes INV-4).** `CanonicalIdentity` derives no
  `Debug`. Any type transitively containing it must implement `Debug`
  **manually**, rendering the identity as a fixed opaque placeholder. U1's test
  asserts no `FsError` variant's `Display` contains an absolute path. Note the
  precise scope: this gate covers *the seam's own* rendering. It cannot prevent
  a future caller from deriving `Debug` on a wrapper — that is what omitting the
  `Debug` derive makes a compile error rather than a silent leak.
* **H3 — Normalization gate.** Prefix normalization is by `Component`/`Prefix`
  inspection, never a string round-trip, and covers `VerbatimDisk` **and**
  `VerbatimUNC`. U2 carries a case for each. Rationale: a string round-trip
  misses the UNC variant and corrupts non-UTF-8 paths.
* **H4 — Staging gate.** No unit may leave a `match` non-exhaustive under
  `-Dwarnings`. Where a `todo!()` arm is used for staging (settled item 8), the
  same unit adds the `# Panics` rustdoc section, so `clippy::pedantic` stays
  clean — closing gen-1 **M-2** at the unit that creates the arm, not later.
* **H5 — Boundary gate (closes INV-6).** The release unit's diff must touch
  **only** `src/services/testfs/**`, `tests/unit/testfs_identity_test.rs`, and
  one `[[test]]` block in `Cargo.toml`. Any diff touching
  `.cargo/test-coverage-manifest.toml`, `src/db/workspace.rs`, `.github/workflows/**`,
  or any other production file is **out of scope** and must be rejected in
  review, not fixed forward.
* **H6 — Test-environment precheck.** U9 must run and pass on Windows, Linux,
  and macOS with no privilege. U10 must, before staging a link, verify
  `make_dir_link` succeeded; on failure it prints `SKIPPED: <reason>` and still
  asserts every privilege-independent property. A silent pass is a defect.
* **H7 — Visibility gate.** U10 lives in `tests/`, so it can reach only `pub`
  items. If U10 needs a non-`pub` item, that is a signal the **contract** (D7)
  is wrong — escalate rather than widening visibility ad hoc.

### Risky actions

| ProposedAction | ActionRisk | Approval needed | ActionResult |
|---|---|---|---|
| Introduce a new public API surface (`CanonicalIdentity`, corpus types, seam trait, `FsError`, `FsOp`, `FakeFs`) that G1/G2 inherit | `moderate` | No — settled by D2/D3/D6/D7 and gated by plan-review | `planned` |
| Author a security-relevant identity gate on the read path | `high` | No — the mechanism is operator-fixed; approval already granted via `027-D` | `planned` |
| Add one `[[test]]` block to `Cargo.toml` | `low` | No | `planned` |
| Edit `.cargo/test-coverage-manifest.toml` | `moderate` | **Would require approval — and is PROHIBITED here** (G2 scope). P1/P2 make it unnecessary | `rejected` |
| Modify `src/db/workspace.rs` to export `normalize_canonical` | `moderate` | **PROHIBITED** — production change, out of G0 scope. P6 re-implements instead | `rejected` |
| Delete, rewrite, or amend any generation-1 artifact | `destructive` | **PROHIBITED** by the program lock | `rejected` |

### Deepened runtime verification

**No runtime surface changes.** Verification is therefore build-, test-, and
inspection-based, and the plan makes no monitoring claim.

| Gate | Command / method | Pass condition |
|---|---|---|
| Build + unit | `cargo dev-test` | All targets pass, including colocated `--lib` |
| Lint | `cargo lint` | Clean under `-D warnings -D clippy::pedantic` |
| Format | `cargo fmt-check` | Clean |
| Doctests | `cargo test --doc` | U8 `compile_fail` doctests fail to compile for the intended reason. **CI execution of doctests is G2 scope (inherited H10); this plan claims local proof only** |
| Exhaustive backstop | `cargo ci` | Passes under `--all-features` |
| INV-2/INV-3 | H1 mechanical read-path inspection | No containment token on the read path; identity used only with `==` |
| INV-6 | H5 diff-boundary inspection | Diff touches only the three permitted locations |
| S1 red→green | Run U9 before U7's comparison exists, then after | Red before, green after, and red for the `IdentityMismatch` reason specifically |

### Deepened operational closure

| Item | Value |
|---|---|
| Monitoring signals | **None, deliberately.** No production surface emits a signal for this package. Inventing an SLI here would repeat gen-1's unearned-claim defect in a new section |
| Rollback trigger | Any post-merge failure of `cargo dev-test` or `cargo lint` attributable to `src/services/testfs/**` |
| Rollback procedure | Revert the release unit's merge commit as a unit (`git revert -m 1 <merge-sha>`, per `git-revert-merge-commit-requires-m1-2026-04-27`). The module is additive and has **no consumers** — G1 is `blocked` — so revert is clean |
| Owner | The Ship session executing the G0 shipment, for the duration of that shipment only. **No long-lived human owner is asserted**, because none is assigned and asserting one would be false |
| Validation window | None. There is no runtime behavior to observe over time; the gates above are terminal |

### Unresolved operator decisions

**None block execution.** Res-1 through Res-5 are *disclosed residuals* with
stated scope and disposition, not open questions. Option C (retained root
anchor) remains the named upgrade path for Res-1 and requires a **future**
package authorization — it is explicitly **not** requested here.

### What this hardening section does NOT claim

Stated explicitly, because the symmetric omission is what terminated
generation 1:

1. It does **not** claim the TOCTOU class is closed. Res-1 (leaf-component
   window) remains open and is disclosed in U7's rustdoc.
2. It does **not** claim content integrity. Res-2 (same-canonical-path in-place
   replacement) is undetected by design.
3. It does **not** claim the `compile_fail` doctests run in CI. That is G2.
4. It does **not** claim cross-platform real-symlink coverage. U10 may skip
   without privilege; only the hermetic U9 is unconditional.
5. It does **not** claim `FakeFs` is a security sandbox. Res-5 stands.

<!-- plan-review-attempt: 1 -->
<!-- plan-review-verdict: FAIL — 2 P0, 15 P1 (10 architecture-level). Correction budget NOT opened per program lock advancement contract item 8. Program HALTED pending operator authorization. Record: docs/closure/2026-09-15-package-g0-generation-2-plan-review-record.md -->
