---
title: "Package G v2 implementation plan — executable agent-contract assertion harness"
description: "Test-first plan for a repository-owned Rust contract-test harness proving agent, skill, instruction, policy, and prompt contract assertions"
source_document: "docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md"
plan_status: "blocked"
review_verdict: "FAIL"
review_record: "docs/closure/2026-09-13-package-g-v2-plan-review-record.md"
supersedes: "docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md"
package: G
review_round: 2
tags:
  - "verification"
  - "contract-tests"
  - "harness"
  - "ci"
---

## Source Document

`docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md`

This plan **supersedes**
`docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md`
(v1, now `superseded`). The v1 plan, its hardening section, and both v1 review
rounds are retained unmodified as evidence.

**Requires plan hardening: yes** — see `## Plan Hardening`.

## Revision History

| Round | Outcome | Change |
|---|---|---|
| R1 | 7 reviewers: 4 PASS, 3 FAIL. All findings bounded and mechanical; the core architecture (single `[[test]]` target, Option T-A, zero new dependencies, zero `verify_markdown` coupling) was affirmed by all 7. | — |
| R2 | This revision. | 19 corrections applied — see `## Round-1 Corrections Applied`. |
| R2 verdict | **FAIL** — 2 PASS (Constitution, Architecture), 2 FAIL (Rust, Correctness). Correction budget exhausted. Package G **BLOCKED**; no harvest, no backlog IDs, no shipment. | All 20 round-1 findings resolved, but the correction introduced two new P1 defects: silent fail-open resource bounds (B1, 3 of 4 reviewers) and unreachable F5/F6 end-to-end coverage (B2). See `docs/closure/2026-09-13-package-g-v2-plan-review-record.md`. |

## Objective

Build a repository-native, executable harness that asserts contract presence,
absence, and multiplicity across the 92 markdown files in `.github/agents`,
`.github/skills`, `.github/instructions`, `.github/policies`, and
`.github/prompts`; run it locally via `cargo dev-test` and in CI on every PR that
touches those files; and expose it as **data-extensible** so Packages A–F add
registry rows, never evaluator code.

## Verified Preconditions

Every fact below was re-derived on 2026-09-13 and cross-checked in review round 1.

| Ref | Fact | Consequence |
|---|---|---|
| R1 | 267 `[[test]]` targets, all explicit `name` + `path`; no `autotests` key; no root `tests/*.rs`; auto-discovery covers only `tests/*.rs` and `tests/*/main.rs` | New target needs an explicit `[[test]]` block; a `tests/contract/agent_harness/` subtree cannot create a stray target |
| R2 | Regular `[dependencies]` link into test targets — proved by `tests/helpers/mod.rs:39-42` importing `tempfile::TempDir`, where `tempfile = "3"` is a regular dep at `Cargo.toml:59` | `ignore` (L75), `globset` (L76), `serde_yaml` (L77), `toml` (L60), `serde` (L31) are usable with **zero** new dependencies |
| R3 | `.cargo/config.toml:1-2` sets `rustflags = ["-Dwarnings"]`; `lint` alias is `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`; `src/lib.rs:9-32` crate attributes (including `#![forbid(unsafe_code)]` and 20 allows) do **not** apply to test crates; `clippy::unwrap_used`/`expect_used` are configured nowhere | Test target must carry its own `#![forbid(unsafe_code)]`, must be pedantic-clean, may use `unwrap()` |
| R3a | **[R1 correction]** Crate-level `#![allow(dead_code)]` appears only in `tests/helpers/mod.rs:34`, but *item-level* `#[allow(dead_code)]` appears in at least 6 test files (`shim_lifecycle_test.rs:26`, `shim_stdio_initialize_test.rs:33`, `get_workspace_status_atomicity_test.rs:44,47,54,81,86`, `hcl_security_test.rs:15`, `hybrid_graph_vector_test.rs:16`) | The precedent is real but broader than v2-R1 stated. Package G still bans **both** forms; the ban is a deliberate tightening, not a claim of uniqueness |
| R3b | Narrow per-target clippy allows are established: `#![allow(clippy::doc_markdown)]` in 21 files, `#![allow(clippy::needless_raw_string_hashes)]` in 15 | Narrow named clippy allows are permitted; broad suppression is not |
| R4 | **[R1 correction]** The established multi-file idiom is `#[path]`-included helper modules, used for more than one helper (`../helpers/mod.rs`, `../helpers/fake_health_responder.rs`, `../helpers/mcp_catalog_capture.rs`). The `pub mod f1;` in `tests/integration/canonical_call_resolution_test.rs:247-260` is fixture text inside a raw string passed to `write_file`, not a module declaration | No in-repo precedent for a subdirectory submodule tree; plain `mod agent_harness;` → `tests/contract/agent_harness/mod.rs` is standard Rust and, per R1, safe |
| R5 | CI test step uses `--all-targets` (`.github/workflows/ci.yml:82-92`), so a registered target runs automatically; `'.github/**/*.md'` is present under **both** `on.push.paths-ignore` and `on.pull_request.paths-ignore` (`.github/workflows/ci.yml:29-47`); `main` is unprotected (404 on branch-protection API) so `build` is not a required check | No CI edit needed to *execute* the target; the only CI edit is removing that one entry twice |
| R6 | Oracle baseline `STATUS=PASS` (267 targets, 13 modules, 0 unmapped); surfaces match by target-**name** glob and every `src/` surface lists `"contract_*"`; `.github` is not a source surface; non-`src` surfaces are supported (`.cargo/config.toml`, `scripts/test-coverage-oracle`) | A `contract_`-prefixed name keeps completeness PASS with no manifest edit; harness surfaces must still be added so change-scoped selection stops false-PASSing |
| R6a | **[R1 correction]** `--mode report` prints counts only; `--mode select` prints required target **names** as `TARGET=<name>` (`scripts/test-coverage-oracle.ps1:11-12`) | Proving the target is *required* for a harness diff must use `--mode select`, not `--mode report` |
| R7 | 92 harness `.md` files: agents 23, skills 28, instructions 34, policies 2, prompts 5; 20 of 92 contain same-line `{{…}}` | `verify_markdown` would reject 20 files |
| R8 | `verify_markdown` emits `template.unresolved` at `Severity::Error` and sets `conformant = findings.is_empty()` | **Zero** coupling to `engram::services::verify` |

## Architecture

Single explicitly registered test target with a dedicated submodule tree
(deliberation Option **T-A**).

```text
Cargo.toml
  [[test]]
  name = "contract_agent_harness_contracts"
  path = "tests/contract/agent_harness_contracts_test.rs"

tests/contract/agent_harness_contracts_test.rs   # root: #![forbid(unsafe_code)], `mod agent_harness;`, end-to-end tests
tests/contract/agent_harness/mod.rs              # declares the four submodules
tests/contract/agent_harness/registry.rs         # typed registry model, load, schema validation
tests/contract/agent_harness/resolve.rs          # root validation, bounded walk, glob, containment, read
tests/contract/agent_harness/assert.rs           # assertion evaluation (F1/F2/F8 under both scopes)
tests/contract/agent_harness/report.rs           # deterministic structured diagnostics

.github/harness-contracts.toml                   # the registry DATA (Packages A–F extend this)
tests/fixtures/agent_harness/                    # negative fixtures for F3–F8
```

`.github/harness-contracts.toml` is `.toml`, so it is **not** matched by
`paths-ignore: '.github/**/*.md'` — a registry edit triggers CI today and after
the G.17 change.

### Red-phase mechanism (NON-NEGOTIABLE)

Every `[R]` unit produces a **compiling but failing** harness, never a
compile error. Each `[R]` unit adds, in the same unit:

1. the minimal type and function **signatures** it needs, with `todo!()` bodies; and
2. the tests that call them.

The target therefore compiles at every commit (Constitution "each commit MUST be
coherent and buildable"), the tests fail at **runtime** via the `todo!()` panic,
and `dead_code` cannot fire because every declared item is called by the tests
introduced alongside it. The paired `[G]` unit replaces the `todo!()` bodies with
real implementations. No `[R]` unit may leave the crate non-compiling.

### Type surface (all items exercised; no `allow(dead_code)` in any form)

| Item | Module | Signature added by | Implemented by |
|---|---|---|---|
| `ContractRegistry`, `ContractEntry` | `registry` | G.2 | G.3 |
| `AssertionForm` (`Required` / `Prohibited` / `ExactlyOne`) | `registry` | G.2 | G.3 |
| `AssertionScope` (`Set` / `EachFile`) | `registry` | G.2 | G.3 |
| `RegistryError` (`Malformed`, `DuplicateId`, `RootNotAllowed`) | `registry` | G.2 | G.3 |
| `load_from_str`, `validate` | `registry` | G.2 | G.3 |
| `FileSet`, `resolve_file_set` | `resolve` | G.4 | G.5 |
| `ResolveError` (`EmptySet`, `OutOfWorkspace`, `Unreadable`) | `resolve` | G.4 | G.5 |
| `validate_root` | `resolve` | G.4 | G.5 |
| `read_resolved` | `resolve` | G.6 | G.7 |
| `evaluate`, `AssertionOutcome` (set scope) | `assert` | G.8 | G.9 |
| `evaluate` (each-file scope) | `assert` | G.10 | G.11 |
| `Diagnostic`, `DiagnosticCode` (F1–F8), `classify` | `report` | G.12 | G.13 |
| `render`, `normalize_path` | `report` | G.12 | G.13 |
| `run_registry` (root runner) | root file | G.14 | G.15 |

## Assertion Model

An entry declares a **form** and a **scope**. The 3×2 matrix covers every
contract shape Packages A–F are expected to need, so they extend by adding rows,
not variants.

| `form` | `scope = set` | `scope = each_file` |
|---|---|---|
| `required` | ≥1 match anywhere in the resolved set | ≥1 match in **every** file of the set |
| `prohibited` | 0 matches anywhere in the set | 0 matches in every file |
| `exactly_one` | exactly 1 match across the whole set | exactly 1 match in **every** file |

`scope` is a required field with no default; omitting it is a schema error
(F3). This resolves the R1 ambiguity about whether counting is per-file or
set-aggregate, and removes the need for a future `ForAll` evaluator variant.

Registry `root` values are restricted at load time to the five harness
directories; any other root is `RegistryError::RootNotAllowed` → **F3**. This is
an allow-list at the data layer; the resolver's containment check (F5) remains as
defence in depth for direct API use.

## Failure Semantics

Each row produces a **nonzero** result. Exactly one code is emitted per failure,
resolved by the precedence below.

| Code | Condition | Red unit |
|---|---|---|
| F3 | Malformed registry (parse error, missing `scope`, disallowed `root`) | G.2 |
| F7 | Duplicate assertion ID | G.2 |
| F5 | Invalid / out-of-workspace root or path | G.4 |
| F4 | Valid contained root resolving to zero files | G.4 |
| F6 | Unreadable file | G.6 |
| F1 | Expected contract missing | G.8 (set), G.10 (each-file) |
| F2 | Prohibited contract present | G.8 (set), G.10 (each-file) |
| F8 | Ambiguous multiplicity | G.8 (set), G.10 (each-file) |

**Precedence (total, mutually exclusive)**: `F3 → F7 → F5 → F4 → F6 → F1/F2/F8`.
Load-time errors precede resolution errors, which precede read errors, which
precede assertion outcomes. A condition matching two rows is always reported as
the earlier code. F1/F2/F8 are mutually exclusive by construction — a single
entry has exactly one `form`.

F4 and F5 are **fail-closed**: an empty or escaping resolution is never success.

### Diagnostic content policy

* `path` is rendered **workspace-relative with `/` separators** on every
  platform, never an absolute host path.
* `detail` carries assertion metadata only — entry ID, form, scope, pattern,
  match count, expected count. It **never** contains raw file content.
* Sort key is the total order `(entry_order, normalized_path, code, match_ordinal)`,
  which remains total when one entry emits several diagnostics for one file.

## Work Units

`[R]` = red harness unit (compiling, failing). `[G]` = implementation unit that
greens exactly its paired `[R]`. `[V]` = verification. Every unit is `<4`
scenarios. **No `[G]` or `[V]` unit exists without a preceding `[R]` unit.**

### G.1 `[R]` — Register target and create the module skeleton

* **Files owned**: `Cargo.toml` (one `[[test]]` block),
  `tests/contract/agent_harness_contracts_test.rs`,
  `tests/contract/agent_harness/mod.rs`,
  `tests/contract/agent_harness/{registry,resolve,assert,report}.rs` (empty)
* **Detail**: root file carries `#![forbid(unsafe_code)]` (R3 — the attribute in
  `src/lib.rs:9` does not reach this crate) and declares `mod agent_harness;`;
  `mod.rs` declares all four submodules; the four files are created empty in the
  same unit. A single placeholder test asserts failure with the message
  "Package G harness not yet implemented". Target name **must** start with
  `contract_` (R6).
* **Scenarios** (3): (1) `cargo test --test contract_agent_harness_contracts`
  **compiles** and runs; (2) it reports exactly one failure (the placeholder);
  (3) `scripts/test-coverage-oracle.ps1 --mode completeness` reports
  `STATUS=PASS` with `UNMAPPED_TARGETS_COUNT=0`.
* **C6 compliance**: the entire module tree is created here. **No later unit adds
  a root module.**
* **Granularity conflict (documented per Governance)**: this unit owns 7 files,
  exceeding the 3-file heuristic. **Principle in tension**: Task Granularity.
  **Justification**: a `[[test]]` `path` pointing at a missing file does not
  compile, and `mod agent_harness;` without `mod.rs` does not compile; four of
  the seven files are zero-byte stubs and total authored content is ~15 lines.
  **Rejected alternative**: splitting registration from file creation, rejected
  because it produces a non-compiling intermediate state, violating C6 and the
  red-phase mechanism above.
* **Depends on**: —

### G.2 `[R]` — Registry model contract tests

* **Files owned**: `tests/contract/agent_harness/registry.rs` (signatures +
  tests), `tests/fixtures/agent_harness/registry_malformed.toml`,
  `tests/fixtures/agent_harness/registry_duplicate_id.toml`
* **Scenarios** (3): (1) a well-formed registry deserializes with the expected
  entry count, `AssertionForm` variants, and `AssertionScope` variants;
  (2) a malformed registry — bad TOML, missing `scope`, or a `root` outside the
  five harness directories — yields `RegistryError::Malformed` or
  `RootNotAllowed`, both classified **F3**; (3) two entries sharing an `id`
  yield `RegistryError::DuplicateId` (**F7**).
* **Red proof**: signatures exist with `todo!()` bodies; the crate compiles and
  all three tests fail at runtime.
* **Depends on**: G.1

### G.3 `[G]` — Registry model and loader

* **Files owned**: `tests/contract/agent_harness/registry.rs` (bodies)
* **Detail**: `serde`-derived `ContractRegistry` / `ContractEntry`;
  `AssertionForm` and `AssertionScope` enums with `scope` required and no
  `#[serde(default)]`; `load_from_str` via `toml`; `validate` rejecting duplicate
  `id` and any `root` outside the five harness directories. No `engram` import.
* **Acceptance**: G.2's three scenarios pass; every `registry` item in the type
  surface is exercised; no `allow(dead_code)` in any form.
* **Loader composability**: `load_from_str` operates on a single TOML document
  and `ContractRegistry` concatenates trivially, so Packages A–F may later adopt
  per-package registry files with no evaluator change (deliberation unresolved
  question 2).
* **Depends on**: G.2

### G.4 `[R]` — Root validation and file-set resolution tests

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (signatures +
  tests), `tests/fixtures/agent_harness/tree/`
* **Scenarios** (3): (1) resolution over the fixture tree returns paths in stable
  sorted order across two runs; (2) a valid contained root whose glob matches
  nothing yields `ResolveError::EmptySet` (**F4**); (3) `validate_root` rejects
  an absolute root, a `../`-traversing root, and a symlinked root **before any
  walk is constructed**, yielding `ResolveError::OutOfWorkspace` (**F5**) — and a
  root that escapes but contains no files still yields F5, not F4, proving the
  precedence.
* **Red proof**: compiling `todo!()` signatures; runtime failures.
* **Depends on**: G.3

### G.5 `[G]` — Root validation and resolver

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (bodies)
* **Order of operations (load-bearing — R1 finding)**:
  1. canonicalize the workspace root once;
  2. for each registry root, reject absolute paths and `..` components
     **lexically**, then canonicalize and assert the result is a prefix-descendant
     of the workspace root, rejecting symlinked roots — **all before
     `WalkBuilder` is constructed**, so no traversal ever occurs outside the
     workspace;
  3. only then build the walker;
  4. canonicalize and re-assert containment for each yielded path before any open.
* **`WalkBuilder` configuration**: each harness directory added as its own root;
  `hidden(false)` (explicit inclusion of hidden descendants — defence in depth,
  since supplying `.github/agents` directly means the `.github` ancestor is not
  itself traversed); `follow_links(false)`; `max_depth(Some(8))`;
  `max_filesize(Some(262_144))`; `sort_by_file_path`. `globset` for patterns.
  Read-only throughout; no writes anywhere.
* **Acceptance**: G.4's three scenarios pass.
* **Depends on**: G.4

### G.6 `[R]` — Ignore-source and read-error tests

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (signatures +
  tests), `tests/fixtures/agent_harness/ignored/` (containing a `.gitignore`
  that excludes a matching `.md` file)
* **Scenarios** (2): (1) a harness-shaped file excluded by a `.gitignore` inside
  the fixture tree **is still resolved** — `ignore::WalkBuilder` honours
  `.gitignore`, `.ignore`, git global excludes, and `.git/info/exclude` by
  default, so a silently-skipped harness file is a real false-PASS vector;
  (2) a resolved path that cannot be read yields `ResolveError::Unreadable`
  carrying the path, without panicking.
* **Unreadable mechanism (III/IV-safe)**: create a throwaway file **inside**
  `tests/fixtures/agent_harness/`, resolve it, delete that throwaway, then read.
  No committed fixture is deleted; no out-of-workspace temp path is used as the
  resolved path, which would be F5 rather than F6.
* **Depends on**: G.5

### G.7 `[G]` — Ignore-source disabling and read layer

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (bodies)
* **Detail**: disable every ignore source on the walker —
  `ignore(false)`, `git_ignore(false)`, `git_global(false)`, `git_exclude(false)`,
  `parents(false)` — so repository or user ignore rules can never silence a
  harness assertion. `read_resolved` maps every `io::Error` to
  `ResolveError::Unreadable { path, source }`; no `unwrap` on I/O.
* **Acceptance**: G.6's two scenarios pass.
* **Depends on**: G.6

### G.8 `[R]` — Set-scope assertion tests

* **Files owned**: `tests/contract/agent_harness/assert.rs` (signatures + tests)
* **Scenarios** (3): (1) `required` + `scope = set` — 0 matches across the set
  produces **F1**, ≥1 match passes; (2) `prohibited` + `scope = set` — ≥1 match
  produces **F2**, 0 matches passes; (3) `exactly_one` + `scope = set` — ≥2
  matches across the set produces **F8**, exactly 1 passes, 0 produces F1.
* **Each scenario is one form**, asserting its negative and positive semantics
  together; forms are not split across scenarios to pad the count.
* **Depends on**: G.7

### G.9 `[G]` — Set-scope evaluator

* **Files owned**: `tests/contract/agent_harness/assert.rs` (bodies)
* **Detail**: `evaluate(entry, file_set) -> AssertionOutcome` for
  `AssertionScope::Set`. Match counting is explicit; absence is a first-class
  result, never an ambiguous empty string.
* **Acceptance**: G.8's three scenarios pass.
* **Depends on**: G.8

### G.10 `[R]` — Each-file-scope assertion tests

* **Files owned**: `tests/contract/agent_harness/assert.rs` (signatures + tests)
* **Scenarios** (3): (1) `required` + `scope = each_file` — a set where one file
  lacks the contract produces **F1** naming that file, while all-present passes;
  (2) `prohibited` + `scope = each_file` — one offending file produces **F2**
  naming it; (3) `exactly_one` + `scope = each_file` — a file with two matches
  produces **F8** naming it, while one-each passes. Boundary case asserted: two
  matches in one file versus one match in each of two files give different
  results, pinning the counting domain.
* **Depends on**: G.9

### G.11 `[G]` — Each-file-scope evaluator

* **Files owned**: `tests/contract/agent_harness/assert.rs` (bodies)
* **Detail**: `evaluate` handles `AssertionScope::EachFile`, producing one
  outcome per offending file so diagnostics can name the file.
* **Acceptance**: G.10's three scenarios pass.
* **Depends on**: G.10

### G.12 `[R]` — Diagnostic classification and ordering tests

* **Files owned**: `tests/contract/agent_harness/report.rs` (signatures + tests)
* **Scenarios** (3): (1) `classify` maps each of F1–F8 to a distinct
  `DiagnosticCode` and honours the declared precedence
  `F3 → F7 → F5 → F4 → F6 → F1/F2/F8` — an input satisfying two conditions
  renders the earlier code only; (2) the sort key
  `(entry_order, normalized_path, code, match_ordinal)` is a **total** order,
  proven by one entry emitting several diagnostics for the same file;
  (3) `normalize_path` renders workspace-relative `/`-separated paths and
  `render` is byte-identical across two runs and across path-separator
  conventions, and `detail` contains no file content.
* **Depends on**: G.11

### G.13 `[G]` — Diagnostic renderer

* **Files owned**: `tests/contract/agent_harness/report.rs` (bodies)
* **Detail**: `Diagnostic { code, entry_id, path, detail }`; `classify`
  implements the precedence table; `normalize_path` strips the workspace prefix
  and converts separators; `render` emits structured, totally ordered text.
* **Acceptance**: G.12's three scenarios pass.
* **Depends on**: G.12

### G.14 `[R]` — End-to-end runner tests

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs` (the
  `run_registry` signature and tests; placeholder still present),
  `tests/fixtures/agent_harness/e2e/`
* **Scenarios** (3): (1) a table-driven case per failure code drives
  `run_registry` end to end for **F1 through F8** and asserts each returns a
  nonzero result carrying the correct rendered `DiagnosticCode` — closing the R1
  gap where only F1 was proven end to end; (2) a passing registry over the
  fixture tree returns a zero result with no diagnostics; (3) resolution over the
  five real harness directories is **non-empty for each**, with each per-directory
  count `>=` a documented minimum floor (agents ≥10, skills ≥10, instructions ≥10,
  policies ≥1, prompts ≥1 — well below the R7 snapshot of 23/28/34/2/5, which is
  recorded as a comment only). This defeats the `hidden(false)` and ignore-source
  silent-skip regressions without breaking when a legitimate harness file is added.
* **Depends on**: G.13

### G.15 `[G]` — Seed registry and root wiring

* **Files owned**: `.github/harness-contracts.toml`,
  `tests/contract/agent_harness_contracts_test.rs` (bodies)
* **Detail**: author the seed registry exercising all three forms under both
  scopes against real harness files; implement `run_registry` wiring
  `registry → validate_root → resolve → read → evaluate → classify → render`;
  **remove the G.1 placeholder**.
* **Acceptance**: G.14's three scenarios pass. **Extensibility acceptance**:
  adding one further assertion row requires editing only
  `.github/harness-contracts.toml` with **no** change under
  `tests/contract/agent_harness/`; demonstrated by adding one seed row.
* **Depends on**: G.14

### G.16 `[R]` — CI trigger contract test

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs` (test added)
* **Scenarios** (2): (1) `.github/workflows/ci.yml`'s `on.push.paths-ignore`
  does **not** contain `.github/**/*.md`; (2) the same for
  `on.pull_request.paths-ignore`.
* **Detail**: reads `ci.yml` and deserializes **only the two `paths-ignore`
  arrays** as `Vec<String>` via `serde_yaml` (a regular dependency, R2) rather
  than round-tripping the whole workflow document, keeping the coupling narrow
  and cheap to migrate if `serde_yaml 0.9`'s upstream deprecation forces a change.
  **This test fails today** (R5) and passes only after G.17 — the repository-native
  red phase for CI integration. There is no green-only CI unit.
* **Depends on**: G.15

### G.17 `[G]` — CI trigger widening

* **Files owned**: `.github/workflows/ci.yml`
* **Detail**: remove `- '.github/**/*.md'` from **both** the `push` and
  `pull_request` `paths-ignore` blocks. No other workflow change is required:
  `--all-targets` already executes the new target (R5).
* **Scenarios** (2): (1) G.16's two assertions pass; (2) the workflow still
  parses and the `build` job's trigger set is otherwise unchanged.
* **Depends on**: G.16

### G.18 `[R]` — Coverage-manifest surface test

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs` (test added)
* **Scenarios** (2): (1) `.cargo/test-coverage-manifest.toml` declares a
  `[[surface]]` for each of `.github/agents/`, `.github/skills/`,
  `.github/instructions/`, `.github/policies/`, `.github/prompts/`, and
  `.github/harness-contracts.toml`, each listing
  `contract_agent_harness_contracts`; (2) it also declares surfaces for
  `tests/contract/agent_harness/` and `tests/fixtures/agent_harness/` — submodule
  and fixture files are not declared `[[test]]` paths and therefore get no
  self-coverage, so without these a fixture-only or submodule-only change still
  false-PASSes the change-scoped oracle.
* **Detail**: parses the manifest with `toml` (regular dependency). Fails today.
* **Depends on**: G.17

### G.19 `[G]` — Coverage-manifest surfaces

* **Files owned**: `.cargo/test-coverage-manifest.toml`
* **Detail**: add the eight `[[surface]]` entries from G.18. Every referenced
  path already exists (`.github/harness-contracts.toml` is created in G.15,
  `tests/fixtures/agent_harness/` in G.2/G.4/G.6/G.14), so no surface names a
  file that does not yet exist.
* **Scenarios** (3): (1) G.18's two assertions pass; (2)
  `--mode completeness` reports `STATUS=PASS`; (3) `--mode select` over a diff
  touching a harness markdown file emits
  `TARGET=contract_agent_harness_contracts` (R6a — `--mode report` prints counts
  only and cannot prove this).
* **Width-isolation justification**: `ci.yml` (G.17) and the coverage manifest
  (G.19) are both repository gate configuration, but are now separate units with
  separate red predecessors, so neither mixes authoring domains within a unit.
* **Depends on**: G.18

### G.20 `[V]` — Static gates

* **Files owned**: none
* **Scenarios** (3): (1) `cargo fmt --all -- --check` clean; (2)
  `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`
  clean; (3) a text search over `tests/contract/agent_harness_contracts_test.rs`
  and `tests/contract/agent_harness/` finds **zero** occurrences of
  `allow(dead_code)` in either crate-level or item-level form, and confirms
  `#![forbid(unsafe_code)]` is present on the target root. Clippy cannot detect
  a suppression it has been told to ignore, so the ban must be searched, not
  inferred. Any `#![allow(clippy::…)]` present must be narrow, named, and
  justified against R3b.
* **Depends on**: G.19

### G.21 `[V]` — Dynamic gates

* **Files owned**: none
* **Scenarios** (3): (1) `cargo dev-test` green; (2) `cargo audit` clean —
  required by the constitutional Quality Gates, which run
  fmt → clippy → test → audit **in order with no gate skipped**, and are not
  waived by the absence of new dependencies; (3)
  `scripts/test-coverage-oracle.ps1 --mode completeness` reports `STATUS=PASS`.
* **Depends on**: G.20

## Dependency Graph

```text
G.1 → G.2 → G.3 → G.4 → G.5 → G.6 → G.7 → G.8 → G.9 → G.10 → G.11
        → G.12 → G.13 → G.14 → G.15 → G.16 → G.17 → G.18 → G.19 → G.20 → G.21
```

The graph is a single total chain. **Round-1 correction**: v2-R1 declared two
"independent" branches, but every unit compiles into the same test binary, so a
unit that leaves any test runtime-red suspends observation of the whole target.
Ordering is therefore strictly sequential, with the CI and coverage-manifest
chain deliberately placed **after** `.github/harness-contracts.toml` exists
(G.15), so no unit references a file another unit has not yet created.

Every `[G]` node is immediately preceded by the `[R]` node it greens:
G.2→G.3, G.4→G.5, G.6→G.7, G.8→G.9, G.10→G.11, G.12→G.13, G.14→G.15,
G.16→G.17, G.18→G.19. G.20 and G.21 are verification gates over the completed
chain, each depending on the fully green G.19.

## Constitution Check

| Principle | Applicable | Status | Evidence |
|---|---|---|---|
| I Safety-first Rust | Yes | Compliant | G.1 adds `#![forbid(unsafe_code)]` to the test crate (R3: `src/lib.rs`'s attribute does not reach it); G.20 verifies it and the pedantic gate; G.7 forbids `unwrap` on I/O. Custom `RegistryError`/`ResolveError` rather than `EngramError` is correct for a test crate with no `engram` coupling |
| II Test-first (NON-NEGOTIABLE) | Yes | Compliant | Nine `[R]`→`[G]` pairs; the red-phase mechanism guarantees compiling-but-failing harnesses, not compile errors; G.14 gives the end-to-end runner its own red predecessor |
| III Workspace isolation | Yes | Compliant | G.5 validates every root **before** the walk and re-checks containment per path before open; `follow_links(false)`; G.6's throwaway file stays inside the workspace |
| IV CLI containment (NON-NEGOTIABLE) | Yes | Compliant | Harness is read-only; the only writes are the throwaway fixture in G.6, inside `tests/fixtures/agent_harness/` |
| V Structured observability | Yes | Compliant | F1–F8 distinct codes with declared precedence; total sort order; byte-identical render (G.12/G.13) |
| VI Single responsibility | Yes | Compliant | **Zero** new dependencies (R2) |
| VII Destructive approval (NON-NEGOTIABLE) | Weakly | Compliant | No deletion of operator data, no VCS history rewrite, no system package change. G.6 deletes only a file it created in the same test |
| VIII Safety modes | Yes | Compliant | `## Plan Hardening` H1–H7 is the careful-mode artifact with `ActionRisk` per item |
| IX Git-friendly persistence | Yes | Compliant | Registry is TOML, sorted, reviewable |
| X Context efficiency | Yes | Compliant | Registry is data; the form×scope matrix means A–F add rows, not variants |
| XI Merge-commit preservation (NON-NEGOTIABLE) | No | N/A | No PR and no merge in scope |
| Task granularity | Yes | Compliant | All 21 units `<4` scenarios; G.1's 7-file conflict documented with principle, justification, and rejected alternative |
| Quality gates | Yes | Compliant | G.20 + G.21 run fmt → clippy → test → **audit** in the mandated order, no gate skipped |

## Explicit Non-Goals

No `src/` change. No MCP tool. No product runtime behavior. No modification of
`verify_markdown` or any `engram::services::verify` coupling. No authoring of
Package A–F assertion data. No new dependency. No PR. No change to Packages A–F.

## Plan Hardening

### H1 — CI trigger change (G.17)

* **Blast radius**: every push and PR to `main`.
* **Risk**: doc-only PRs that previously skipped CI now run the full suite.
* **Bounding evidence**: `main` has **no branch protection** (404), so `build` is
  not a required status check and a slower run cannot block a merge. The change
  direction is fail-closed — more runs, not fewer. Only
  `.github/copilot-instructions.md` lies outside the five harness directories, so
  deleting the entry outright is correct and glob negation is unnecessary. A
  tighter positive `paths:` allow-list is recorded as a future refinement should
  `build` ever become a required check.
* **Security review (R1)**: `ci.yml` uses `pull_request` (not
  `pull_request_target`), declares `permissions: contents: read`, sets
  `persist-credentials: false`, and references no secrets. Widening the trigger
  changes only *when* an already read-only, secret-free job runs — no privilege
  escalation for fork PRs.
* **Rollback**: re-add one line to both blocks. Reversible, no data migration.
* **`ActionRisk`**: `moderate`. **Approval**: covered by the operator's Package G
  authorization. **`ActionResult`**: recorded at G.17 completion.

### H2 — Novel module topology (G.1)

* **Risk**: `tests/contract/agent_harness/` has no in-repo precedent (R4).
* **Bounding evidence**: R1 proves auto-discovery covers only `tests/*.rs` and
  `tests/*/main.rs`, neither of which this creates.
* **Mitigation**: G.1 is compile-and-run-only. If the topology fails it fails in
  the first unit with nothing built on it; the fallback is the deliberation's
  Option T-B at the cost of the G.4–G.13 split.
* **`ActionRisk`**: `low`.

### H3 — Bounded runtime-red branch state

* **Condition**: the target is runtime-red from G.1 until G.15 replaces the
  placeholder, and G.16 keeps the CI assertion red until G.17.
* **Crucially, the crate compiles at every commit.** The red-phase mechanism
  forbids compile-error reds, so "each commit MUST be coherent and buildable" is
  satisfied while Principle II's observed-failing-test requirement is also met.
  This was the R1 blocking distinction.
* **Hazard**: an interrupted shipment leaves the branch with failing tests.
  Bounded to the shipment; G.20/G.21 are the gates that prove resolution before
  any PR.
* **`ActionRisk`**: `low`.

### H4 — Silent-skip traversal failures

* **Risk**: two independent mechanisms can silently empty the resolved set —
  `WalkBuilder`'s default `hidden(true)`, and its default honouring of
  `.gitignore`, `.ignore`, git global excludes, and `.git/info/exclude`. A naive
  design would read an empty set as "all assertions passed".
* **Mitigation, four layers**: `hidden(false)`; all ignore sources disabled
  (G.7); **F4 makes an empty resolution a hard failure**; G.14 scenario 3 asserts
  per-directory non-emptiness against documented minimum floors. G.6 scenario 1
  is a dedicated regression test for the ignore-source path.
* **`ActionRisk`**: `moderate` — the most likely silent-failure mode, defended
  four ways.

### H5 — Pedantic lint exposure and suppression discipline (R3)

* **Risk**: the test crate does not inherit `src/lib.rs`'s 20 allows.
* **Mitigation**: narrow, named, per-target `#![allow(clippy::…)]` only, matching
  the R3b precedent. **`allow(dead_code)` is prohibited in both crate-level and
  item-level form**, a deliberate tightening beyond the R3a repository norm.
  Because clippy cannot detect a suppression it was told to honour, G.20
  scenario 3 **searches** for the string rather than inferring absence.
* **`ActionRisk`**: `low`.

### H6 — Registry as an attacker-influenceable surface

* **Risk**: any contributor can edit `.github/harness-contracts.toml`. An
  unbounded `root` could target `target/` (restored by `Swatinem/rust-cache`) or
  the whole repo, creating a CI resource-exhaustion vector that containment
  checking alone does not close, since it stops escape but not in-workspace
  scope creep.
* **Mitigation**: `root` is allow-listed at load time to the five harness
  directories (F3); `max_depth(8)` and `max_filesize(262_144)` bound the walk;
  diagnostics never echo file content and never emit absolute paths.
* **Accepted residual**: a PR weakening a registry row also weakens the check
  that would catch it. This is an inherent property of every in-repo linter
  config and is **not introduced by Package G**; a CODEOWNERS entry for the
  registry is recorded as an out-of-scope follow-up.
* **`ActionRisk`**: `moderate`.

### H7 — Inherited `serde_yaml` deprecation

* **Risk**: `serde_yaml 0.9` is upstream-archived. Package G does not introduce
  it (it is already a regular dependency) but G.16 adds a new consumer.
* **Mitigation**: G.16 deserializes only the two `paths-ignore` arrays as
  `Vec<String>` rather than the whole document, keeping the migration surface to
  a few lines. Replacement is recorded as an out-of-scope follow-up.
* **`ActionRisk`**: `low`.

### H8 — Coverage-oracle drift

* **Risk**: a new target unmapped by a `src/` surface fails `--mode completeness`.
* **Bounding evidence**: surfaces match by target-**name** glob and every `src/`
  surface lists `"contract_*"`, so the mandated prefix keeps completeness PASS
  with no manifest edit. G.1 scenario 3 verifies this at the first unit rather
  than at the end. The G.18/G.19 pair separately closes the change-scoped
  false-PASS, including for submodule and fixture edits.
* **`ActionRisk`**: `low`.

## Verification Strategy

1. Per-unit: the named scenarios via
   `cargo test --test contract_agent_harness_contracts`.
2. Static gates (G.20): `cargo fmt --all -- --check`; `cargo lint`; suppression
   search for `allow(dead_code)` and presence check for `#![forbid(unsafe_code)]`.
3. Dynamic gates (G.21): `cargo dev-test`; `cargo audit`;
   `scripts/test-coverage-oracle.ps1 --mode completeness`.
4. Operational confirmation (G.19 scenario 3):
   `scripts/test-coverage-oracle.ps1 --mode select` over a harness-markdown diff
   must emit `TARGET=contract_agent_harness_contracts`.
5. Negative-path proof (G.14 scenario 1): a table-driven end-to-end case for each
   of F1–F8 asserts a nonzero result with the correct code — the harness is
   proven to fail, not merely to pass.

## Round-1 Corrections Applied

| # | R1 finding (reviewer) | Correction |
|---|---|---|
| 1 | G.14 green-only (Constitution P0; Rust P1; Correctness P1) | Split into G.14 `[R]` end-to-end tests and G.15 `[G]` implementation |
| 2 | `[R]` units were compile-fail, not compiling-but-failing (Constitution P1) | Red-phase mechanism section added: every `[R]` adds `todo!()` signatures plus tests; crate compiles at every commit |
| 3 | Coverage-manifest edit had no red predecessor (Rust P1; Correctness P1) | New G.18 `[R]` / G.19 `[G]` pair |
| 4 | Containment checked only after the walk (Rust P1; Correctness P1) | G.5 validates every root lexically then canonically **before** `WalkBuilder` is constructed |
| 5 | `.gitignore` / ignore-source filtering not disabled (Correctness P1) | G.6 scenario 1 + G.7 disable `ignore`, `git_ignore`, `git_global`, `git_exclude`, `parents` |
| 6 | F1–F8 precedence undefined (Correctness P1) | Total precedence `F3 → F7 → F5 → F4 → F6 → F1/F2/F8` declared and tested in G.12 |
| 7 | `exactly_one` counting domain undefined (Correctness P1; Architecture P2) | `AssertionScope` (`Set` / `EachFile`) added as a required field; 3×2 matrix specified |
| 8 | Per-file universal contracts would need a new evaluator form (Architecture P2) | `scope = each_file` covers them as **data**, preserving the A–F zero-code extensibility claim |
| 9 | `(entry_order, file_path)` not a total order; separators differ per platform (Correctness P1) | Sort key extended to `(entry_order, normalized_path, code, match_ordinal)`; `normalize_path` added |
| 10 | `.github/harness-contracts.toml` referenced before creation (Correctness P1) | Chain reordered: G.15 creates it; G.18/G.19 follow |
| 11 | Only F1 proven end to end (Correctness P1) | G.14 scenario 1 is table-driven across F1–F8 |
| 12 | Missing `#![forbid(unsafe_code)]` on the test crate (Rust P1) | Added in G.1, verified in G.20 |
| 13 | `cargo audit` omitted from the gate sequence (Constitution P1) | G.21 scenario 2 |
| 14 | Unit sizing: G.8 packed 6 cases; G.13/G.15 exceeded (Rust P1) | Assertion work split into G.8/G.9 (set) and G.10/G.11 (each-file); CI split from manifest; verification split into G.20/G.21 |
| 15 | `--mode report` cannot prove the target is required (Correctness P1) | G.19 scenario 3 uses `--mode select` and asserts `TARGET=…` |
| 16 | Exact count pinning is brittle (Maintainability P2; Correctness P2) | Replaced with non-emptiness plus documented minimum floors |
| 17 | No resource bounds; `root` unconstrained (Security P2) | `max_depth(8)`, `max_filesize(262_144)`, and a load-time `root` allow-list (F3) |
| 18 | `allow(dead_code)` inventory and `#[path]` precedent misstated (Rust P2) | R3a and R4 corrected with verified file/line citations |
| 19 | Missing `tests/fixtures/agent_harness/` surface (Rust P2); diagnostic content policy unspecified (Security P3); Constitution Check incomplete (Constitution P2); branch independence overstated (Architecture P2); loader composability dropped (Maintainability P3); G.6 mechanism underspecified (Constitution P2) | All addressed in G.18/G.19, the diagnostic content policy, the full I–XI mapping, the single-chain graph, G.3's composability note, and G.6's throwaway-file mechanism |
