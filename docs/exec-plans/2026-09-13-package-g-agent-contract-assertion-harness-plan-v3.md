---
type: exec-plan
date: 2026-09-13
package: G
version: v3
source: docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md
supersedes: docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v2.md
branch: chore/checkpoint-resolution-ordering-restage
head: fde75189
requires_plan_hardening: yes
---

# Package G v3 — agent-contract assertion harness (implementation plan)

## Source Document

`docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md`

Prior evidence (read, not re-litigated):
`docs/closure/2026-09-13-package-g-v2-plan-review-record.md`.

## Objective

Deliver a data-driven, fail-closed assertion runner that evaluates declarative
contracts (`required` / `prohibited` / `exactly_one` × `set` / `each_file`)
over the 92 markdown files in `.github/agents`, `.github/skills`,
`.github/instructions`, `.github/policies`, and `.github/prompts`; run it via
`cargo test --all-targets` and in CI on every PR that touches those files.

Packages A–F extend it by adding **registry rows**, not evaluator code.

## Verified Preconditions

Every row re-verified this session against `HEAD = fde75189`.

| # | Precondition | Observed | Consequence |
|---|---|---|---|
| P1 | 267 `[[test]]` targets, all explicit `name` + `path`; `autotests` key absent; 0 root `tests/*.rs`; 0 `tests/*/main.rs` | `Cargo.toml`, filesystem | A `tests/contract/agent_harness/` subtree **cannot** create a stray cargo target; the new target needs an explicit `[[test]]` block |
| P2 | 63 targets already use the `contract_` name prefix | `Cargo.toml` | `contract_agent_harness_contracts` matches the convention |
| P3 | Manifest surfaces for `src/**` declare `targets = ["contract_*", ...]` | `.cargo/test-coverage-manifest.toml:85` et al. | The new target is absorbed by existing globs; adding it does **not** break completeness |
| P4 | Oracle baseline: `TARGET_COUNT=267`, `UNMAPPED_TARGETS_COUNT=0`, `UNMAPPED_MODULES_COUNT=0`, `STATUS=PASS` | live run of `scripts/test-coverage-oracle.ps1 --mode completeness` | G.1 must leave this at `STATUS=PASS` with `TARGET_COUNT=268` |
| P5 | `'.github/**/*.md'` present in `on.push.paths-ignore` **and** `on.pull_request.paths-ignore` | `.github/workflows/ci.yml` | Exactly two lines to remove (G.31) |
| P6 | `#![forbid(unsafe_code)]` at `src/lib.rs:10`; `#![warn(clippy::pedantic)]` at `:11` | `src/lib.rs` | Crate-level attributes do **not** reach a separate test crate; the new target must carry its own |
| P7 | Edition 2024, MSRV 1.85 | `Cargo.toml:15-16` | No feature-gating concerns |
| P8 | `tempfile` is a regular `[dependencies]` entry (`:59`) imported at `tests/helpers/mod.rs:43` | both files | Regular dependencies link into test targets; `globset`, `toml`, `serde` are usable without a `[dev-dependencies]` addition |
| P9 | 92 files across the five roots, **all** `.md`; max relative depth **2**; max size **98,546** bytes | recursive enumeration | No depth or size policy is warranted (B1) |
| P10 | `ignore 0.4` is declared (`:75`) but has **zero** live call sites in `src/` | `Cargo.toml`, recursive grep | Not using `ignore` here removes nothing that is in use |
| P11 | `allow(dead_code)` occurs 11× in `tests/`; `#[path = ...]` occurs 46× | recursive grep | Corrects v2 advisory A-1; both are established, not novel |
| P12 | Exactly one `mod.rs` exists under `tests/` (`tests/helpers/mod.rs`), consumed via `#[path]` | filesystem | A `tests/contract/agent_harness/mod.rs` subtree is **novel topology** → hardening H2 |

## Architecture

### Module topology (created whole in G.1; no later unit adds a root module)

```text
tests/contract/agent_harness_contracts_test.rs  # target root: #![forbid(unsafe_code)],
                                                # #![warn(clippy::pedantic)], mod agent_harness;,
                                                # end-to-end #[test]s only
tests/contract/agent_harness/mod.rs             # declares the seven submodules
tests/contract/agent_harness/fs_access.rs       # FileAccess, RealFs, FakeFs
tests/contract/agent_harness/registry.rs        # model, load_from_str, validate
tests/contract/agent_harness/resolve.rs         # validate_root, resolve_file_set
tests/contract/agent_harness/read.rs            # read_resolved
tests/contract/agent_harness/assert.rs          # evaluate (set + each_file)
tests/contract/agent_harness/report.rs          # Diagnostic, DiagnosticCode, classify,
                                                # normalize_path, render
tests/contract/agent_harness/runner.rs          # run_registry
```

**Test placement rule (dead-code discipline).** Every `#[test]` lives in the
module whose items it exercises; only end-to-end tests live in the target root.
Every declared item therefore has a caller in its own module, so `dead_code`
cannot fire and **no `allow(dead_code)` appears anywhere in this target** —
notwithstanding P11, which shows the repository permits it elsewhere.

### Red-phase mechanism (NON-NEGOTIABLE, narrowed per B4)

* **API-introducing `[R]` units** add the minimal signatures they need with
  `todo!()` bodies, plus the `#[test]`s that call them. The target **compiles**
  at every commit (Constitution: each commit must be coherent and buildable);
  the tests fail at **runtime** on the `todo!()` panic. `todo!()` has the never
  type and coerces to any return type. The paired `[G]` unit replaces the bodies.
* **Data/config-asserting `[R]` units** (G.28, G.30, G.32) introduce no API.
  Their red phase is a compiling test that fails because the asserted repository
  state does not yet exist; the paired `[G]` unit creates that state.

No `[R]` unit may leave the crate non-compiling. No `[G]` or `[V]` unit exists
without a preceding `[R]` unit.

### Scenario-counting convention (SC)

One scenario = one `#[test]` function; **≤3 per unit**. A `#[test]` may be
table-driven only when every row drives the same public entry point at the same
pipeline stage and exercises one `form`×`scope` combination (positive and
negative cases together = one scenario). Rows spanning different `form`×`scope`
combinations, different pipeline stages, or different `DiagnosticCode` outcomes
are distinct scenarios requiring separate `#[test]` functions.

### Type surface (every item exercised by a caller in its own module)

| Item | Module | Signature added by | Implemented by |
|---|---|---|---|
| `FileAccess` (trait: `list_dir`, `read_to_string`), `Entry`, `RealFs`, `FakeFs` | `fs_access` | G.2 | G.3 |
| `ContractRegistry`, `ContractEntry`, `AssertionForm`, `AssertionScope` | `registry` | G.4 | G.5 |
| `load_from_str`, `RegistryError::Malformed` | `registry` | G.4 | G.5 |
| `validate`, `RegistryError::{RootNotAllowed, DuplicateId}`, `ALLOWED_ROOTS` | `registry` | G.6 | G.7 |
| `validate_root`, `ResolveError::OutOfWorkspace` | `resolve` | G.8 | G.9 |
| `FileSet`, `resolve_file_set` | `resolve` | G.10 | G.11 |
| `ResolveError::{EmptySet, DiscoveryFailed}` | `resolve` | G.12 | G.13 |
| `read_resolved`, `ReadError::Unreadable` | `read` | G.14 | G.15 |
| `evaluate`, `AssertionOutcome` (set scope) | `assert` | G.16 | G.17 |
| `evaluate` (each-file scope) | `assert` | G.18 | G.19 |
| `Diagnostic`, `DiagnosticCode` (F1–F8), `classify` | `report` | G.20 | G.21 |
| `normalize_path`, `render` | `report` | G.22 | G.23 |
| `run_registry`, `RunReport` | `runner` | G.24 | G.25 |
| `RunReport::aggregate` | `runner` | G.26 | G.27 |

## Assertion Model

| `form` | `scope = set` | `scope = each_file` |
|---|---|---|
| `required` | ≥1 match anywhere in the resolved set | ≥1 match in **every** file |
| `prohibited` | 0 matches anywhere in the set | 0 matches in every file |
| `exactly_one` | exactly 1 match across the whole set | exactly 1 match in **every** file |

`scope` is required with no default; omitting it is a schema error (F3).
`root` is restricted at load time to the five harness directories; any other
root is `RegistryError::RootNotAllowed` → F3.

**Deliberate expressive boundary** (v2 advisory A-5). The matrix cannot express
conditional (`if X then Y`), cross-file consistency, ordering, or numeric
thresholds beyond 0/1/≥1. These are legitimate deferrals. Extending the matrix
is a **plan-level change**, not a row-add; a Package A–F author who hits this
wall must not silently add an evaluator arm.

**`prohibited`+`set` vs. `prohibited`+`each_file`** (v2 advisory A-6) have
identical accept/reject predicates and differ **only** in diagnostic
granularity: `set` emits one diagnostic naming the set, `each_file` emits one
diagnostic per offending file. This is stated here so the matrix is not misread
as two distinct predicates.

## Failure Semantics

| Code | Condition | Owning layer | Red unit |
|---|---|---|---|
| F3 | Malformed registry: parse error, missing `scope`, disallowed `root` | `registry` | G.4, G.6 |
| F7 | Duplicate assertion `id` | `registry` | G.6 |
| F5 | Resolution-phase failure: containment violation, or `list_dir` / symlink-metadata access failure | `resolve` | G.8, G.12 |
| F4 | Valid contained root resolving to zero files | `resolve` | G.12 |
| F6 | Read-phase failure: an **admitted** set member cannot be read | `read` | G.14 |
| F1 | Required contract missing | `assert` | G.16, G.18 |
| F2 | Prohibited contract present | `assert` | G.16, G.18 |
| F8 | Ambiguous multiplicity (`exactly_one` matched >1) | `assert` | G.16, G.18 |

**Precedence (total, mutually exclusive)**: `F3 → F7 → F5 → F4 → F6 → F1/F2/F8`.
Load-time errors precede resolution errors, which precede read errors, which
precede assertion outcomes. F1/F2/F8 are mutually exclusive by construction — an
entry has exactly one `form`. F4, F5, and F6 are **fail-closed**: an empty,
escaping, inaccessible, or unreadable resolution is never success.

### Reachability matrix (B2 — the central correction)

| Code | Reachable through public `run_registry`? | Proven by |
|---|---|---|
| F3 | **Yes** | `registry` unit (G.6) **and** runner integration (G.24 sc.2) |
| F7 | **Yes** | `registry` unit (G.6) **and** runner integration (G.24 sc.3) |
| F5 | **No — by design** | `resolve` unit only (G.8, G.12) |
| F4 | **Yes** | `resolve` unit (G.12) **and** runner integration (G.26 sc.2) |
| F6 | **No — not deterministically** | `read` unit only (G.14) |
| F1 | **Yes** | `assert` units (G.16, G.18) **and** runner integration (G.26 sc.1) |
| F2 | **Yes** | `assert` units (G.16, G.18) **and** runner integration (G.26 sc.1) |
| F8 | **Yes** | `assert` units (G.16, G.18) **and** runner integration (G.26 sc.3) |

**Why F5 is unreachable through the runner, and why that is correct.** The
registry allow-list is a strict subset of the contained roots, so a registry that
passes `validate` can never hand the resolver an escaping root — prevalidation
maps it to F3 first. Asserting F5 end to end would require defeating
prevalidation. The runner suite instead proves the **complement** (G.24 sc.2:
a disallowed root surfaces as F3), which is the precedence contract itself.

**Why F6 is not asserted end to end.** F6 is a real runtime contract — an
admitted member that cannot be read must fail, never be skipped — but the real
filesystem cannot produce it deterministically (it is a race by construction,
and Windows file-locking semantics make v2's throwaway-delete trick
nondeterministic). It is proven at the read layer against `FakeFs`. **The
end-to-end suite does not claim F6 and must not.**

**The runner does not re-walk.** `run_registry` resolves once, then reads the
admitted set. A member deleted between discovery and read maps to **F6**, and
that is the documented public contract. G.26 sc.2 asserts F4 only for the case
that genuinely produces it: an allowed root whose declared glob matches nothing.

### Exhaustive discovery (B1 — the other central correction)

`resolve_file_set` performs a caller-driven `std::fs` recursion through
`FileAccess::list_dir`. It has **no filtering option of any kind**: no
`max_depth`, no `max_filesize`, no ignore sources, no hidden-file skipping.
Every regular file under each allowed root enters the candidate set. Symlinked
entries are **rejected as F5**, never silently skipped. Glob narrowing
(`globset`) is applied to the candidate set afterwards and is an intentional,
registry-visible narrowing declared by the contract itself. F4 is evaluated
**only after** exhaustive enumeration.

P9 (max depth 2, max size 98,546 bytes) shows v2's `max_depth(8)` and
`max_filesize(262_144)` were inert on every real input while being capable of
silently deleting the largest and most contract-dense file in the corpus once it
grew. They are removed, not merely relaxed. `ignore::WalkBuilder` is not used:
it filters by default in five separate ways, each a distinct opportunity to
reintroduce this defect.

### Diagnostic content policy

* `path` is rendered **workspace-relative with `/` separators** on every
  platform, never an absolute host path.
* `detail` carries assertion metadata only — entry `id`, form, scope, pattern,
  match count, expected count. It **never** contains raw file content.
* Sort key is the total order
  `(entry_order, normalized_path, code, match_ordinal)`.

## Work Units

`[R]` = compiling red harness unit. `[G]` = implementation greening exactly its
paired `[R]`. `[V]` = verification. Every unit is ≤3 scenarios under SC and
touches ≤3 files.

### G.1 `[R]` — Register target and create the module skeleton

* **Files owned**: `Cargo.toml` (one `[[test]]` block), plus creation of all
  nine files in the topology above.
* **Detail**: target `name = "contract_agent_harness_contracts"`,
  `path = "tests/contract/agent_harness_contracts_test.rs"`. Root file carries
  `#![forbid(unsafe_code)]` and `#![warn(clippy::pedantic)]` (P6) and declares
  `mod agent_harness;`. `mod.rs` declares all seven submodules; the seven
  submodule files are created empty. One placeholder `#[test]` in the root file
  fails with "Package G harness not yet implemented".
* **Scenarios (3)**: (1) `cargo test --test contract_agent_harness_contracts`
  **compiles** and runs; (2) it reports exactly one failure (the placeholder);
  (3) `scripts/test-coverage-oracle.ps1 --mode completeness` reports
  `STATUS=PASS`, `TARGET_COUNT=268`, `UNMAPPED_TARGETS_COUNT=0` — valid because
  P3 shows existing `contract_*` globs absorb the new target.
* **C6**: the entire module tree is created here. **No later unit adds a root
  module.**

### G.2 `[R]` — `FileAccess` boundary contract tests

* **Files owned**: `agent_harness/fs_access.rs`,
  `tests/fixtures/agent_harness/fs_access/` (two files, one subdirectory).
* **Detail**: adds `trait FileAccess { fn list_dir(&self, p: &Path) -> io::Result<Vec<Entry>>; fn read_to_string(&self, p: &Path) -> io::Result<String>; }`,
  `struct Entry { path: PathBuf, is_dir: bool, is_symlink: bool }`,
  `struct RealFs`, `struct FakeFs` — all with `todo!()` bodies.
* **Scenarios (3)**: (1) `RealFs::list_dir` over the fixture directory returns
  exactly its entries with correct `is_dir`/`is_symlink` flags, and
  `read_to_string` returns the fixture's bytes; (2) `FakeFs` returns its
  programmed listing and contents for a programmed tree; (3) `FakeFs` returns
  the specific `io::Error` programmed for a specific path, for both methods.

### G.3 `[G]` — `FileAccess` implementations

* **Files owned**: `agent_harness/fs_access.rs`.
* **Detail**: `RealFs` delegates to `std::fs::read_dir` +
  `std::fs::symlink_metadata` + `std::fs::read_to_string`. `FakeFs` holds a
  `BTreeMap<PathBuf, FakeNode>` where a node is a directory listing, file
  contents, or a programmed `io::ErrorKind`. No `unsafe`, no platform `cfg`.

### G.4 `[R]` — Registry parse and shape tests

* **Files owned**: `agent_harness/registry.rs`.
* **Detail**: adds `ContractRegistry`, `ContractEntry`, `AssertionForm`
  (`Required`/`Prohibited`/`ExactlyOne`), `AssertionScope` (`Set`/`EachFile`),
  `RegistryError::Malformed`, `load_from_str` — `todo!()` bodies.
* **Scenarios (3)**: (1) a valid TOML registry round-trips `id`, `root`,
  `glob`, `pattern`, `form`, `scope` for all three forms and both scopes;
  (2) syntactically malformed TOML yields `Malformed`; (3) an entry omitting
  `scope` — and, separately, one omitting `form` — yields `Malformed` (one
  scenario: same entry point, same stage, same outcome variant).

### G.5 `[G]` — Registry model and loader

* **Files owned**: `agent_harness/registry.rs`.
* **Detail**: `serde` derives over `toml 0.8` (P8). `scope` and `form` are
  non-`Option` fields with no `#[serde(default)]`, so omission is a
  deserialization error mapped to `Malformed`.

### G.6 `[R]` — Registry validation tests

* **Files owned**: `agent_harness/registry.rs`.
* **Detail**: adds `ALLOWED_ROOTS: [&str; 5]`, `validate`,
  `RegistryError::{RootNotAllowed, DuplicateId}` — `todo!()` bodies.
* **Scenarios (3)**: (1) each of the five allowed roots validates, and a
  sixth `.github` path, an unrelated repository path, an absolute path, and a
  `../`-escaping path each yield `RootNotAllowed` (one scenario: same entry
  point, same outcome variant); (2) two entries sharing an `id` yield
  `DuplicateId`; (3) `classify` maps `Malformed` and `RootNotAllowed` to **F3**
  and `DuplicateId` to **F7**, and F3 precedes F7 when both are present.

### G.7 `[G]` — Registry validator

* **Files owned**: `agent_harness/registry.rs`.
* **Detail**: allow-list membership by exact normalized-string match against
  `ALLOWED_ROOTS`; duplicate detection over a `BTreeSet<&str>` of `id`s,
  reported in first-offending-`id` order for determinism.

### G.8 `[R]` — Root containment tests

* **Files owned**: `agent_harness/resolve.rs`.
* **Detail**: adds `validate_root`, `ResolveError::OutOfWorkspace` —
  `todo!()` bodies. Pure path logic; no filesystem access, therefore
  deterministic on every platform.
* **Scenarios (3)**: (1) each of the five allowed roots passes containment;
  (2) an absolute root, a `../`-traversing root, and a root whose canonical form
  escapes the workspace each yield `OutOfWorkspace` (one scenario); (3) a root
  that escapes **and** contains no files yields `OutOfWorkspace` (F5), **not**
  `EmptySet` (F4) — proving precedence `F5 → F4`.

### G.9 `[G]` — Root validation

* **Files owned**: `agent_harness/resolve.rs`.
* **Detail**: canonicalize the workspace root once; reject absolute paths and
  any `..` component lexically; then canonicalize the joined root and assert it
  is a prefix of the workspace root. All checks run **before** any traversal.

### G.10 `[R]` — Exhaustive enumeration tests (no silent omission)

* **Files owned**: `agent_harness/resolve.rs`,
  `tests/fixtures/agent_harness/enumeration/` (a fixture tree containing a
  **393,216-byte** file — 1.5× v2's removed 262,144 cap — and a file nested
  **10 levels** deep — beyond v2's removed depth-8 cap — plus a dot-prefixed
  hidden file and a `.gitignore`d name).
* **Detail**: adds `FileSet`, `resolve_file_set(&dyn FileAccess, root, glob)` —
  `todo!()` body.
* **Scenarios (3)**: (1) **every** regular file in the fixture tree appears in
  the resolved set — explicitly including the oversized file, the depth-10 file,
  the hidden file, and the `.gitignore`d file; **no file may be absent**;
  (2) two consecutive resolutions return byte-identical sorted orders;
  (3) a symlinked entry yields `ResolveError` (F5) and is **never** silently
  dropped from the candidate set.
* **Note**: the fixture's oversized file is generated by the test's setup from a
  repeated byte pattern rather than committed, keeping the repository small
  while proving the property on a real filesystem through `RealFs`.

### G.11 `[G]` — Exhaustive resolver

* **Files owned**: `agent_harness/resolve.rs`.
* **Detail**: caller-driven recursion over `FileAccess::list_dir`; descend into
  directories, collect regular files, reject symlinks. **No filter options of
  any kind.** `globset` applied to the candidate set afterwards. Results sorted
  by normalized path.

### G.12 `[R]` — Empty-set and discovery-failure tests

* **Files owned**: `agent_harness/resolve.rs`,
  `tests/fixtures/agent_harness/empty/` (one directory holding one `.md` file
  that no test glob matches).
* **Detail**: adds `ResolveError::{EmptySet, DiscoveryFailed}` — `todo!()`
  discriminants wired into `resolve_file_set`.
* **Scenarios (3)**: (1) a valid contained root whose glob matches nothing
  yields `EmptySet` (**F4**), evaluated only **after** exhaustive enumeration
  confirms the candidate set was fully populated; (2) `FakeFs` programmed to
  fail `list_dir` on a nested directory yields `DiscoveryFailed` (**F5**) — the
  walk must **not** proceed with a partial set; (3) `FakeFs` programmed to fail
  on the **root** directory likewise yields `DiscoveryFailed` (**F5**), not
  `EmptySet`.

### G.13 `[G]` — Empty-set and discovery-failure handling

* **Files owned**: `agent_harness/resolve.rs`.
* **Detail**: any `list_dir` error aborts the whole resolution with
  `DiscoveryFailed` carrying the offending path. Emptiness is tested only on a
  fully successful traversal.

### G.14 `[R]` — Read-layer tests

* **Files owned**: `agent_harness/read.rs`.
* **Detail**: adds `read_resolved(&dyn FileAccess, &FileSet)`,
  `ReadError::Unreadable` — `todo!()` bodies.
* **Scenarios (3)**: (1) every admitted member is read and returned in resolved
  order; (2) `FakeFs` programmed to return `PermissionDenied` for one admitted
  member yields `Unreadable` (**F6**) naming that member, and the run **fails**
  rather than skipping it; (3) `FakeFs` programmed so an admitted member is
  absent at read time yields `Unreadable` (**F6**), **not** `EmptySet` (F4) —
  pinning the documented contract that the runner does not re-walk.

### G.15 `[G]` — Read layer

* **Files owned**: `agent_harness/read.rs`.
* **Detail**: read each admitted member through `FileAccess::read_to_string`;
  any error aborts with `Unreadable` carrying the offending path. No member is
  ever skipped.

### G.16 `[R]` — Set-scope assertion tests

* **Files owned**: `agent_harness/assert.rs`.
* **Detail**: adds `evaluate`, `AssertionOutcome` — `todo!()` bodies.
* **Scenarios (3)**, one per `form` at `scope = set`: (1) `required`+`set`
  pass (≥1 match) and fail (0 matches → F1); (2) `prohibited`+`set` pass
  (0 matches) and fail (≥1 → F2), with **one** diagnostic naming the set;
  (3) `exactly_one`+`set` pass (1), fail-zero (→ F1), fail-many (→ F8).

### G.17 `[G]` — Set-scope evaluator

* **Files owned**: `agent_harness/assert.rs`.

### G.18 `[R]` — Each-file-scope assertion tests

* **Files owned**: `agent_harness/assert.rs`.
* **Scenarios (3)**, one per `form` at `scope = each_file`: (1)
  `required`+`each_file` passes only when every file matches; a single
  non-matching file → F1 naming that file; (2) `prohibited`+`each_file` emits
  **one diagnostic per offending file** (the A-6 granularity distinction);
  (3) `exactly_one`+`each_file` requires exactly one match in **every** file;
  zero → F1, two → F8, each naming the offending file.

### G.19 `[G]` — Each-file-scope evaluator

* **Files owned**: `agent_harness/assert.rs`.

### G.20 `[R]` — Classification and precedence tests

* **Files owned**: `agent_harness/report.rs`.
* **Detail**: adds `Diagnostic`, `DiagnosticCode` (F1–F8), `classify` —
  `todo!()` bodies.
* **Scenarios (3)**: (1) each adjacent precedence pair (F3≻F7, F7≻F5, F5≻F4,
  F4≻F6, F6≻F1) resolves to the earlier code when both conditions co-occur —
  one scenario, one predicate, table-driven over adjacent pairs; (2) no input
  yields two codes (mutual exclusivity); (3) **every** `DiagnosticCode` variant
  is produced by at least one documented trigger — the no-unreachable-code
  assertion, cross-checked against the reachability matrix above.

### G.21 `[G]` — Classifier

* **Files owned**: `agent_harness/report.rs`.

### G.22 `[R]` — Rendering and ordering tests

* **Files owned**: `agent_harness/report.rs`.
* **Detail**: adds `normalize_path`, `render` — `todo!()` bodies.
* **Scenarios (3)**: (1) `normalize_path` yields workspace-relative
  `/`-separated paths for both `\`- and `/`-separated inputs and never an
  absolute host path; (2) diagnostics sort by the four-tuple total order and are
  byte-identical across two runs, including when one entry emits several
  diagnostics for one file; (3) `detail` contains entry metadata and **no** raw
  file content, asserted against a fixture containing a distinctive sentinel
  string that must not appear in any rendered output.

### G.23 `[G]` — Renderer

* **Files owned**: `agent_harness/report.rs`.

### G.24 `[R]` — Runner prevalidation and happy path

* **Files owned**: `agent_harness/runner.rs`,
  `tests/contract/agent_harness_contracts_test.rs`.
* **Detail**: adds `run_registry(&str) -> RunReport`, `RunReport` — `todo!()`
  bodies. `run_registry` takes **no** injection parameter and always uses
  `RealFs`.
* **Scenarios (3)**: (1) a fixture registry over the real roots succeeds, with a
  per-root **floor** assertion (≥1 file resolved for each of the five roots) —
  floors, not exact counts, per the v2 brittleness finding; (2) a registry whose
  entry names a disallowed root yields **F3**, proving prevalidation precedes
  resolution and that F5 is unreachable here **by design**; (3) a registry with
  a duplicate `id` yields **F7**.

### G.25 `[G]` — Runner pipeline

* **Files owned**: `agent_harness/runner.rs`.
* **Detail**: wires `registry::load_from_str → registry::validate →
  resolve::validate_root → resolve::resolve_file_set → read::read_resolved →
  assert::evaluate`, returning unaggregated per-entry outcomes.

### G.26 `[R]` — Runner aggregation and publicly reachable assertion codes

* **Files owned**: `agent_harness/runner.rs`,
  `tests/contract/agent_harness_contracts_test.rs`.
* **Detail**: adds `RunReport::aggregate` — `todo!()` body.
* **Scenarios (3)**: (1) a registry with one failing `required` and one failing
  `prohibited` entry emits exactly two diagnostics, F1 then F2, ordered by
  `entry_order`; (2) an **allowed** root whose glob matches nothing emits
  exactly one **F4** — the only case that genuinely produces F4 through the
  runner; (3) an `exactly_one` entry matching twice emits exactly one **F8**.
* **Explicit non-assertion**: this unit does **not** assert F5 or F6. Per the
  reachability matrix they are unreachable through `run_registry` and are owned
  by G.8/G.12 and G.14 respectively.

### G.27 `[G]` — Aggregation and render wiring

* **Files owned**: `agent_harness/runner.rs`.
* **Detail**: collect per-entry outcomes, classify, sort by the four-tuple,
  render, and return a nonzero result when any diagnostic is present.

### G.28 `[R]` — Seed registry contract tests

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs`.
* **Detail**: data-asserting red phase (B4) — fails because
  `.github/contracts/agent-harness.toml` does not yet exist.
* **Scenarios (3)**: (1) the committed seed registry loads and validates;
  (2) it declares ≥1 entry for **each** of the five allowed roots; (3)
  `run_registry` over the seed registry against the **live corpus** succeeds
  with zero diagnostics.

### G.29 `[G]` — Author the seed registry

* **Files owned**: `.github/contracts/agent-harness.toml`.
* **Detail**: one entry per root, each chosen against a contract the live corpus
  already satisfies — e.g. `required`+`each_file` that every
  `.github/agents/*.agent.md` contains a `## Role` heading; `exactly_one`+
  `each_file` that `_orchestrator.agent.md`, `_ship.agent.md`, and
  `_stage.agent.md` each carry exactly one `## Role Boundary (NON-NEGOTIABLE)`
  heading. Patterns are validated against the corpus before commit.

### G.30 `[R]` — CI trigger contract test

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs`.
* **Detail**: data-asserting red phase; parses `.github/workflows/ci.yml` with
  `serde_yaml` (P8, already a declared dependency).
* **Scenarios (3)**: (1) `on.push.paths-ignore` does **not** contain
  `.github/**/*.md`; (2) `on.pull_request.paths-ignore` does **not** contain it;
  (3) both blocks still contain `docs/**`, `*.md`, `.backlogit/**`, and
  `.autoharness/**` — a **no-over-widening** guard, so the fix cannot be made by
  deleting the blocks.

### G.31 `[G]` — Widen the CI trigger

* **Files owned**: `.github/workflows/ci.yml`.
* **Detail**: delete exactly the two `'.github/**/*.md'` lines confirmed by P5.
  No other change.

### G.32 `[R]` — Coverage-manifest surface test

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs`.
* **Detail**: data-asserting red phase; parses
  `.cargo/test-coverage-manifest.toml`.
* **Scenarios (3)**: (1) a `[[surface]]` exists for each of the five harness
  roots **and** for `.github/contracts/`, each listing
  `contract_agent_harness_contracts`; (2) `--mode select` for a changed
  `.github/agents/*.agent.md` includes the harness target in the required set;
  (3) `--mode completeness` reports `STATUS=PASS` with
  `UNMAPPED_TARGETS_COUNT=0` and `UNMAPPED_MODULES_COUNT=0`.

### G.33 `[G]` — Add coverage-manifest surfaces

* **Files owned**: `.cargo/test-coverage-manifest.toml`.
* **Detail**: six `[[surface]]` blocks appended; no existing block modified.

### G.34 `[V]` — Static gates

* **Files owned**: none.
* **Scenarios (3)**: (1) `cargo fmt --check` clean; (2)
  `cargo clippy --all-targets -- -D warnings` clean, with pedantic active on the
  new target via its own `#![warn(clippy::pedantic)]` (P6); (3) a repository grep
  confirms the new target contains **zero** occurrences of `allow(dead_code)`,
  zero `unsafe`, and that `#![forbid(unsafe_code)]` is present on the target
  root. Clippy cannot detect the missing attribute, so the grep is the oracle.

### G.35 `[V]` — Dynamic gates

* **Files owned**: none.
* **Scenarios (3)**: (1) `cargo test --all-targets` green, with the new target
  executing; (2) coverage oracle `--mode completeness` `STATUS=PASS` and
  `--mode select` over a `.github/agents` diff selecting the harness target;
  (3) `cargo audit` clean at or below the pre-existing advisory baseline.

## Dependency Graph

A **single total chain**; there are no independent branches.

```text
G.1 → G.2 → G.3 → G.4 → G.5 → G.6 → G.7 → G.8 → G.9 → G.10 → G.11
    → G.12 → G.13 → G.14 → G.15 → G.16 → G.17 → G.18 → G.19
    → G.20 → G.21 → G.22 → G.23 → G.24 → G.25 → G.26 → G.27
    → G.28 → G.29 → G.30 → G.31 → G.32 → G.33 → G.34 → G.35
```

Every `[G]` is immediately preceded by the `[R]` it greens. Every `[V]` follows
all implementation. No file is created by two units.

## Constitution Check

| Principle | Applies | Status | Evidence |
|---|---|---|---|
| I Local-first | Yes | Compliant | Test-only; no network, no daemon, no external service |
| II Test-first | Yes | Compliant | 17 `[R]`/`[G]` pairs; no `[G]` without a preceding `[R]`; red phases defined by B4 |
| III Workspace isolation | Yes | Compliant | G.9 validates every root before traversal; G.11 rejects symlinks; fixtures live under `tests/fixtures/` |
| IV Deterministic behaviour | Yes | Compliant | G.10 sc.2, G.22 sc.2 pin byte-identical ordering; G.14 replaces v2's nondeterministic throwaway-file trick with `FakeFs` |
| V Observability | Yes | Compliant | G.22/G.23 define diagnostic content and ordering; `detail` never leaks content |
| VI Simplicity | Yes | Compliant | B1 removes inert bounds; `ignore` not used; two-method boundary trait; 3×2 matrix with a documented expressive boundary |
| VII No new dependencies | Yes | Compliant | `globset`, `toml`, `serde`, `serde_yaml` all pre-declared (P8, E17); none added, removed, or version-changed |
| VIII Buildable commits | Yes | Compliant | Every `[R]` compiles; `todo!()` fails at runtime, not compile time |
| IX Lint discipline | Yes | Compliant | G.34; `#![forbid(unsafe_code)]` + `#![warn(clippy::pedantic)]` on the target root; zero `allow(dead_code)` |
| X CI integrity | Yes | Compliant | G.30 sc.3 forbids over-widening; G.31 is a two-line deletion |
| XI Traceability | Yes | Compliant | Reachability matrix maps every code to its owning layer and test level |

Cross-reference: Plan Hardening below contains **H1–H7**. This table is
consistent with that range (correcting v2 advisory A-4).

## Explicit Non-Goals

* No change to `src/`. No production code.
* No `verify_markdown` or `engram` coupling.
* No dependency addition, removal, or version change.
* No contract authoring for Packages A–F beyond the five seed entries.
* No conditional, cross-file-consistency, ordering, or numeric-threshold
  assertion forms.
* No depth or size policy, and no failure code for one.
* No CODEOWNERS entry for the registry (recorded as an out-of-scope follow-up
  from v2 advisory A-8).

## Plan Hardening

### H1 — CI trigger change (G.31)

* **Risk**: removing `.github/**/*.md` from `paths-ignore` re-arms the full
  fmt → clippy → test → audit sequence on doc-only PRs, increasing CI load and
  exposing doc-only PRs to unrelated failures.
* **Blast radius**: every PR touching `.github/**/*.md`.
* **Mitigation**: intended — the harness is worthless if it does not run when
  the corpus changes. G.30 sc.3 pins that the other ignore patterns survive, so
  the change cannot be over-applied. Reversible by restoring two lines.

### H2 — Novel module topology (G.1)

* **Risk**: P12 shows `tests/contract/agent_harness/mod.rs` is the first
  non-`helpers` `mod.rs` subtree under `tests/`; an unexpected cargo
  auto-discovery could create a stray target.
* **Mitigation**: P1 shows `autotests` is absent and auto-discovery covers only
  `tests/*.rs` and `tests/*/main.rs`. The subtree contains neither. G.1 sc.3
  pins `TARGET_COUNT=268` — exactly one new target — so a stray target fails the
  oracle loudly.

### H3 — Bounded runtime-red branch state

* **Risk**: between each `[R]` and its `[G]`, `cargo test` fails. A branch left
  mid-pair is red.
* **Mitigation**: the chain is total and every `[R]` is immediately followed by
  its `[G]`. The branch is green at every even-numbered boundary. `[R]`/`[G]`
  pairs should be landed together.

### H4 — Silent-skip traversal failures (the B1 defect class)

* **Risk**: the v2 failure mode — a candidate file leaving the resolved set
  without a diagnostic, turning a `prohibited` violation into a PASS.
* **Defence, four independent layers**:
  1. **Structural** — no filtering option exists to misconfigure. Plain
     `std::fs` recursion, no `ignore::WalkBuilder`, no `max_depth`, no
     `max_filesize`.
  2. **Positive proof** — G.10 sc.1 asserts an oversized (393,216-byte),
     depth-10, hidden, and `.gitignore`d file all appear in the resolved set.
     Each is chosen to exceed a bound v2 actually imposed.
  3. **Fail-closed errors** — any `list_dir` failure aborts with F5 (G.12
     sc.2/sc.3); any read failure aborts with F6 (G.14 sc.2/sc.3). No partial
     set ever reaches the evaluator.
  4. **Floors** — G.24 sc.1 asserts ≥1 resolved file per root against the live
     corpus, catching wholesale loss.
* **Residual**: a glob that matches nothing narrows the set legitimately. This
  is registry-visible and, when it narrows to zero, produces F4.

### H5 — Pedantic lint exposure (P6)

* **Risk**: `#![warn(clippy::pedantic)]` at `src/lib.rs:11` does **not** reach a
  separate test crate, so the new target could silently escape pedantic review.
* **Mitigation**: G.1 places both `#![forbid(unsafe_code)]` and
  `#![warn(clippy::pedantic)]` on the target root. G.34 sc.2 runs clippy with
  `-D warnings`; G.34 sc.3 greps for the attributes, because clippy cannot
  detect their **absence**.

### H6 — Registry as an attacker-influenceable surface

* **Risk**: `.github/contracts/agent-harness.toml` is repository data. A
  malicious edit could point `root` at a sensitive tree or at `target/`.
* **Mitigation**: `root` is allow-listed at load time to the five harness
  directories (F3, G.6 sc.1) and independently containment-checked by the
  resolver (F5, G.8). These two layers are orthogonal: the allow-list constrains
  *which* directories, containment constrains *where they may resolve to*.
  Symlinks are rejected, so an allow-listed directory cannot be re-pointed.
* **Explicit residual, stated rather than assumed**: the harness reads file
  **contents** from the allow-listed roots and matches patterns against them. It
  never executes, writes, or renders content — and G.22 sc.3 asserts no content
  reaches the output — so the worst outcome of a hostile registry is a false
  diagnostic, never disclosure or execution. No CODEOWNERS entry is added
  (non-goal); this is accepted because the registry is already covered by
  ordinary PR review.

### H7 — Inherited `serde_yaml` deprecation (v2 advisory A-8)

* **Risk**: `serde_yaml 0.9` (E17) is upstream-archived; G.30 adds a new
  consumer.
* **Mitigation**: inherited, not introduced — the dependency is already
  declared and used. The new consumer parses one repository-controlled workflow
  file in a test-only target. Migration is out of scope and recorded as a
  follow-up.

## Verification Strategy

| Layer | Proves | Units |
|---|---|---|
| Boundary | `RealFs` matches the real filesystem; `FakeFs` injects deterministic errors | G.2 |
| Registry | Schema, allow-list, duplicate detection → F3, F7 | G.4, G.6 |
| Resolve | Containment → F5; exhaustive non-filtering discovery; emptiness → F4; discovery failure → F5 | G.8, G.10, G.12 |
| Read | Admitted-member read failure → F6 | G.14 |
| Assert | All six `form`×`scope` combinations → F1, F2, F8 | G.16, G.18 |
| Report | Precedence totality, no unreachable code, path normalization, ordering, content policy | G.20, G.22 |
| Runner (integration) | Prevalidation precedence and the publicly reachable codes **only** — F1, F2, F3, F4, F7, F8 | G.24, G.26 |
| Corpus | The seed registry passes against the 92 live files | G.28 |
| Repository | CI trigger and coverage-manifest state | G.30, G.32 |
| Gates | fmt, clippy, attribute presence, full test run, oracle, audit | G.34, G.35 |

The end-to-end suite proves **only** errors reachable through the public runner.
Layer-level tests prove internal contracts. This table and the reachability
matrix together are the traceability record.

## Sizing

35 units, each ≤3 scenarios under SC and ≤3 files. Estimated ≤2 hours of
human-equivalent effort per unit.
