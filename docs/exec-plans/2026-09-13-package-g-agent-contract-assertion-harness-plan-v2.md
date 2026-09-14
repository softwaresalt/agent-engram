---
title: "Package G v2 implementation plan — executable agent-contract assertion harness"
description: "Test-first plan for a repository-owned Rust contract-test harness proving agent, skill, instruction, policy, and prompt contract assertions"
source_document: "docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md"
plan_status: "draft"
supersedes: "docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md"
package: G
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
(v1, now `superseded`). The v1 plan, its hardening section, and both review
rounds are retained unmodified as evidence.

**Requires plan hardening: yes** — see `## Plan Hardening`. The package changes a
CI trigger and introduces a module topology with no in-repo precedent.

## Objective

Build a repository-native, executable harness that asserts contract presence,
absence, and multiplicity across the 92 markdown files in `.github/agents`,
`.github/skills`, `.github/instructions`, `.github/policies`, and
`.github/prompts`; run it locally via `cargo dev-test` and in CI on every PR that
touches those files; and expose it as **data-extensible** so Packages A–F add
registry rows, never evaluator code.

## Verified Preconditions

Every fact below was re-derived on 2026-09-13 and is cited in the source
deliberation's Research Findings (R1–R8). A plan step that contradicts one of
these is a defect.

| Ref | Fact | Consequence for this plan |
|---|---|---|
| R1 | 267 `[[test]]` targets, all explicit `name` + `path`; no `autotests` key; no root `tests/*.rs` | New target needs an explicit `[[test]]` block; a `tests/contract/<dir>/` subtree cannot create a stray target |
| R2 | Regular `[dependencies]` link into test targets (proved by `tests/helpers/mod.rs` using `tempfile`, a regular dep at `Cargo.toml:59`) | `ignore` (L75), `globset` (L76), `serde_yaml` (L77), `toml` (L60), `serde` are usable with **zero** new dependencies |
| R3 | `[build] rustflags = ["-Dwarnings"]` global; clippy runs `--all-targets` at pedantic; `src/lib.rs`'s 20 crate-level allows do **not** apply to test crates; precedent is narrow per-target allows (21× `doc_markdown`, 15× `needless_raw_string_hashes`); `allow(dead_code)` exists only in `tests/helpers/mod.rs` because it is `#[path]`-shared; `unwrap_used`/`expect_used` are **not** configured | Test target must be pedantic-clean with at most narrow named allows; **no `allow(dead_code)` anywhere**; `unwrap()` is permitted in test code |
| R4 | The only multi-file idiom is `#[path = "../helpers/mod.rs"] mod helpers;`; the `pub mod f1;` in `canonical_call_resolution_test.rs` is fixture text inside a raw string, not a module | Plain `mod agent_harness;` → `tests/contract/agent_harness/mod.rs` is novel here but standard Rust |
| R5 | CI test step uses `--all-targets`, so a registered target runs automatically; the gap is `paths-ignore: '.github/**/*.md'` on **both** `push` and `pull_request`; `main` is unprotected (404 on branch protection) so `build` is not a required check | No CI edit is needed to *execute* the target; the only CI edit is removing that one entry twice |
| R6 | Oracle baseline `STATUS=PASS` (267 targets, 13 modules, 0 unmapped); surfaces match by target-**name** glob and every `src/` surface lists `"contract_*"`; `.github` is not a source surface so change-scoped `--mode run` selects nothing for harness edits; non-`src` surfaces are supported (`.cargo/config.toml`, `scripts/test-coverage-oracle`) | A `contract_`-prefixed name keeps completeness PASS with **no** manifest edit; harness surfaces must still be added so `--mode run` stops false-PASSing |
| R7 | 92 harness `.md` files: agents 23, skills 28, instructions 34, policies 2, prompts 5; 20 of 92 contain same-line `{{…}}` | Seed registry sized against real counts; `verify_markdown` would reject 20 files |
| R8 | `verify_markdown` emits `template.unresolved` at `Severity::Error` and sets `conformant = findings.is_empty()` | **Zero coupling** to `engram::services::verify` |

## Architecture

Single explicitly registered test target with a dedicated submodule tree
(deliberation Option **T-A**).

```text
Cargo.toml
  [[test]]
  name = "contract_agent_harness_contracts"
  path = "tests/contract/agent_harness_contracts_test.rs"

tests/contract/agent_harness_contracts_test.rs   # root: `mod agent_harness;` + end-to-end tests
tests/contract/agent_harness/mod.rs              # declares the four submodules
tests/contract/agent_harness/registry.rs         # typed registry model, load, schema validation
tests/contract/agent_harness/resolve.rs          # bounded walk, glob, containment, read
tests/contract/agent_harness/assert.rs           # assertion evaluation (F1/F2/F8)
tests/contract/agent_harness/report.rs           # deterministic structured diagnostics

.github/harness-contracts.toml                   # the registry DATA (Packages A–F extend this)
tests/fixtures/agent_harness/                    # negative fixtures for F3–F8
```

`.github/harness-contracts.toml` is `.toml`, so it is **not** matched by
`paths-ignore: '.github/**/*.md'` — a registry edit triggers CI today and after
the G.13 change.

### Type surface (all items exercised; no `allow(dead_code)`)

| Item | Module | Exercised by |
|---|---|---|
| `ContractRegistry`, `ContractEntry` | `registry` | G.2 |
| `AssertionForm` (`Required` / `Prohibited` / `ExactlyOne`) | `registry` | G.2, G.8 |
| `RegistryError` (`Malformed`, `DuplicateId`) | `registry` | G.2 |
| `FileSet`, `resolve_file_set` | `resolve` | G.4 |
| `ResolveError` (`EmptySet`, `OutOfWorkspace`, `Unreadable`) | `resolve` | G.4, G.6 |
| `read_resolved` | `resolve` | G.6 |
| `evaluate`, `AssertionOutcome` | `assert` | G.8 |
| `Diagnostic`, `DiagnosticCode` (F1–F8) | `report` | G.10 |
| `render` | `report` | G.10 |

## Failure Semantics (nonzero required for every row)

| Code | Condition | Asserted in |
|---|---|---|
| F1 | Expected contract missing (`required` resolved 0 matches) | G.8 |
| F2 | Prohibited contract present (`prohibited` resolved ≥1) | G.8 |
| F3 | Malformed registry | G.2 |
| F4 | Empty resolved file set | G.4 |
| F5 | Invalid / out-of-cwd path | G.4 |
| F6 | Unreadable file | G.6 |
| F7 | Duplicate assertion ID | G.2 |
| F8 | Ambiguous multiplicity (`exactly_one` resolved ≥2) | G.8 |

F4 and F5 are fail-closed: an empty or escaping resolution is never success.
Each code renders distinctly; none collapses to a generic "failed".

## Work Units

Every unit is `<4` scenarios and single-domain. `[R]` = red harness unit,
`[G]` = implementation unit that turns a specific red green, `[V]` = verification.
**No implementation unit exists without a preceding red unit.**

### G.1 `[R]` — Register target and create the complete module skeleton

* **Files owned**: `Cargo.toml` (one `[[test]]` block),
  `tests/contract/agent_harness_contracts_test.rs`,
  `tests/contract/agent_harness/mod.rs`,
  `tests/contract/agent_harness/{registry,resolve,assert,report}.rs`
* **Detail**: root declares `mod agent_harness;`; `mod.rs` declares all four
  submodules; the four files are created empty in the same unit. A single
  placeholder test asserts `false` with the message "Package G harness not yet
  implemented", establishing the red phase. Target name **must** start with
  `contract_` (R6).
* **Scenarios** (3): (1) `cargo test --test contract_agent_harness_contracts`
  compiles and runs; (2) it reports exactly one failure (the placeholder);
  (3) `scripts/test-coverage-oracle.ps1 --mode completeness` still reports
  `STATUS=PASS` with `UNMAPPED_TARGETS_COUNT=0`.
* **C6 compliance**: the entire module tree — every `mod.rs` and every submodule
  file — is created here. **No later unit adds a root module**, so no step can
  reference a missing module.
* **Width-isolation justification**: the `[[test]]` block and the files it
  references are one atomic registration. Splitting them would leave a manifest
  entry pointing at a non-existent path, which C6 forbids and which does not
  compile.
* **Depends on**: —

### G.2 `[R]` — Registry model contract tests

* **Files owned**: `tests/contract/agent_harness/registry.rs` (tests only),
  `tests/fixtures/agent_harness/registry_malformed.toml`,
  `tests/fixtures/agent_harness/registry_duplicate_id.toml`
* **Scenarios** (3): (1) a well-formed registry deserializes into
  `ContractRegistry` with the expected entry count and `AssertionForm` variants;
  (2) a malformed registry yields `RegistryError::Malformed` (**F3**);
  (3) two entries sharing an `id` yield `RegistryError::DuplicateId` (**F7**).
* **Red proof**: fails to compile/run because the types do not exist yet.
* **Depends on**: G.1

### G.3 `[G]` — Registry model and loader

* **Files owned**: `tests/contract/agent_harness/registry.rs` (implementation)
* **Detail**: `serde`-derived `ContractRegistry` / `ContractEntry`;
  `AssertionForm` enum; `load_from_str` via `toml`; schema validation rejecting
  duplicate `id`. No `engram` import.
* **Acceptance**: G.2's three scenarios pass; every item in the registry row of
  the type-surface table is exercised; no `allow(dead_code)`.
* **Depends on**: G.2

### G.4 `[R]` — File-set resolution contract tests

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (tests only),
  `tests/fixtures/agent_harness/tree/` (small deterministic fixture tree)
* **Scenarios** (3): (1) resolution over the fixture tree returns paths in stable
  sorted order across two runs; (2) a glob matching nothing yields
  `ResolveError::EmptySet` (**F4**); (3) a registry root escaping the workspace
  (`../`, absolute, or symlink) yields `ResolveError::OutOfWorkspace` (**F5**).
* **Red proof**: `resolve_file_set` does not exist yet.
* **Depends on**: G.3

### G.5 `[G]` — File-set resolver with containment and determinism

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (implementation)
* **Detail**: `ignore::WalkBuilder` with each harness directory added as its own
  root, `hidden(false)` **set explicitly** (`.github` is a dot-directory and the
  default `hidden(true)` would skip nested entries), `follow_links(false)`,
  `max_depth`, `max_filesize`, `sort_by_file_path`. `globset` for patterns.
  Every resolved path canonicalized and asserted inside the workspace root before
  any open. Read-only throughout.
* **Acceptance**: G.4's three scenarios pass; no writes performed.
* **Depends on**: G.4

### G.6 `[R]` — Unreadable-file diagnostic test

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (tests only)
* **Scenarios** (1): a resolved path that cannot be read yields
  `ResolveError::Unreadable` carrying the offending path, and does **not** panic.
* **Cross-platform note**: driven by a path removed between resolution and read
  rather than by OS permission bits, so it behaves identically on Windows,
  Linux, and macOS.
* **Depends on**: G.5

### G.7 `[G]` — Read layer

* **Files owned**: `tests/contract/agent_harness/resolve.rs` (implementation)
* **Detail**: `read_resolved` maps every `io::Error` into
  `ResolveError::Unreadable { path, source }`. No `unwrap` on I/O.
* **Acceptance**: G.6's scenario passes.
* **Depends on**: G.6

### G.8 `[R]` — Assertion evaluation contract tests

* **Files owned**: `tests/contract/agent_harness/assert.rs` (tests only)
* **Scenarios** (3): (1) a `required` assertion with zero matches produces
  **F1**; (2) a `prohibited` assertion with ≥1 match produces **F2**; (3) an
  `exactly_one` assertion with ≥2 matches produces **F8**.
* **Positive semantics also asserted inline**: `required` with ≥1 match,
  `prohibited` with 0 matches, and `exactly_one` with exactly 1 match all pass —
  these are assertions within the same three scenarios, not extra scenarios.
* **Depends on**: G.7

### G.9 `[G]` — Assertion evaluator

* **Files owned**: `tests/contract/agent_harness/assert.rs` (implementation)
* **Detail**: `evaluate(entry, file_set) -> AssertionOutcome`. Match counting is
  explicit; absence is a first-class result, never an ambiguous empty string.
* **Acceptance**: G.8's three scenarios pass.
* **Depends on**: G.8

### G.10 `[R]` — Deterministic diagnostics contract tests

* **Files owned**: `tests/contract/agent_harness/report.rs` (tests only)
* **Scenarios** (3): (1) each of F1–F8 renders a distinct `DiagnosticCode`, none
  collapsing to a generic failure; (2) diagnostics are ordered by
  `(entry_order, file_path)`; (3) two renders of the same input are
  byte-identical.
* **Depends on**: G.9

### G.11 `[G]` — Diagnostic renderer

* **Files owned**: `tests/contract/agent_harness/report.rs` (implementation)
* **Detail**: `Diagnostic { code, entry_id, path, detail }`; `render` emits
  structured, stably ordered text. Every code carries the entry ID and, where
  applicable, the path.
* **Acceptance**: G.10's three scenarios pass.
* **Depends on**: G.10

### G.12 `[R]` — CI trigger contract test

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs` (test added)
* **Scenarios** (2): (1) `.github/workflows/ci.yml`'s `on.push.paths-ignore`
  does **not** contain `.github/**/*.md`; (2) the same for
  `on.pull_request.paths-ignore`.
* **Detail**: parses `ci.yml` with `serde_yaml` (regular dependency, R2). This is
  the repository-native red phase for CI integration: **it fails today** and only
  passes after G.13 edits the workflow. There is no green-only CI unit.
* **Depends on**: G.1 (module skeleton only — independent of G.2–G.11)

### G.13 `[G]` — CI trigger and coverage-manifest surfaces

* **Files owned**: `.github/workflows/ci.yml`,
  `.cargo/test-coverage-manifest.toml`
* **Detail**: remove `- '.github/**/*.md'` from **both** the `push` and
  `pull_request` `paths-ignore` blocks. Add `[[surface]]` entries mapping
  `.github/agents/`, `.github/skills/`, `.github/instructions/`,
  `.github/policies/`, `.github/prompts/`,
  `.github/harness-contracts.toml`, and `tests/contract/agent_harness/` to
  `targets = ["contract_agent_harness_contracts"]`, closing the change-scoped
  false-PASS (R6).
* **Scenarios** (3): (1) G.12's two assertions pass; (2)
  `--mode completeness` reports `STATUS=PASS`; (3) `--mode report` over a diff
  touching a harness markdown file lists `contract_agent_harness_contracts` as
  required with `omitted == 0`.
* **Width-isolation justification**: both files are repository gate
  configuration, one domain. They are inseparable — removing the trigger
  exclusion without mapping the surface leaves the change-scoped oracle still
  false-PASSing on the exact class of change being enabled.
* **No CI edit is needed to execute the target** (R5): `--all-targets` already
  covers it. This unit changes only *when* CI runs.
* **Depends on**: G.12

### G.14 `[G]` — Seed registry and end-to-end wiring

* **Files owned**: `.github/harness-contracts.toml`,
  `tests/contract/agent_harness_contracts_test.rs` (wiring)
* **Detail**: author a minimal seed registry exercising all three assertion forms
  against real harness files; wire the root test
  `registry → resolve → read → assert → report`; **remove the G.1 placeholder**.
* **Scenarios** (3): (1) the seed registry loads and every entry resolves a
  non-empty file set across all five directories (guards the `hidden(false)`
  caveat against the R7 counts: 23/28/34/2/5); (2) the seed suite passes against
  the current tree; (3) mutating a fixture copy to delete a required clause
  produces a nonzero result with the F1 diagnostic.
* **Extensibility acceptance**: adding a Package A–F assertion requires editing
  only `.github/harness-contracts.toml` — no file under
  `tests/contract/agent_harness/` changes. Demonstrated by adding one seed row
  without touching evaluator code.
* **Depends on**: G.11, G.13

### G.15 `[V]` — Build verification (last)

* **Files owned**: none
* **Scenarios** (3): (1) `cargo fmt --all -- --check` clean; (2)
  `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`
  clean, with **zero** `allow(dead_code)` introduced and any `#![allow(clippy::…)]`
  narrow, named, and justified against the R3 precedent; (3) `cargo dev-test`
  green and `--mode completeness` `STATUS=PASS`.
* **Depends on**: G.14

## Dependency Graph

```text
G.1 ─┬─> G.2 ─> G.3 ─> G.4 ─> G.5 ─> G.6 ─> G.7 ─> G.8 ─> G.9 ─> G.10 ─> G.11 ─┐
     │                                                                          ├─> G.14 ─> G.15
     └─> G.12 ─> G.13 ───────────────────────────────────────────────────────── ┘
```

Every `[G]` node is immediately preceded by the `[R]` node it greens. The two
branches from G.1 are independent and may be executed in either order; both
converge before G.14.

## Constitution Check

| Principle | Status |
|---|---|
| I Safety-first Rust | Test-only code; `forbid(unsafe_code)` unaffected; no `unwrap` on I/O paths |
| II Test-first | Every `[G]` unit has a preceding `[R]` unit, including CI (G.12 → G.13) |
| III Workspace isolation | G.5 canonicalizes and contains every path; `follow_links(false)` |
| IV CLI containment | Read-only harness; no writes anywhere |
| V Observability | Structured, stably ordered diagnostics (G.10/G.11) |
| VI Single responsibility | **Zero new dependencies** (R2) |
| IX Git-friendly persistence | Registry is TOML, sorted, review-friendly |
| X Context efficiency | Registry is data; A–F add rows, not code |

## Explicit Non-Goals

No `src/` change. No MCP tool. No product runtime behavior. No modification of
`verify_markdown` or any `engram::services::verify` coupling. No authoring of
Package A–F assertion data. No new dependency. No PR. No change to Packages A–F.

## Plan Hardening

### H1 — CI trigger change (G.13)

* **Blast radius**: every push and PR to `main`.
* **Risk**: doc-only PRs that previously skipped CI now run the full suite.
* **Evidence bounding it**: `main` has **no branch protection** (404), so `build`
  is not a required status check and a slower or failing run cannot block a
  merge. Only `.github/copilot-instructions.md` lies outside the five harness
  directories, so no glob negation is needed.
* **Rollback**: re-add the single `- '.github/**/*.md'` line to both blocks.
  One-line, reversible, no data migration.
* **`ActionRisk`**: `moderate`. **Approval**: covered by the operator's Package G
  authorization. **`ActionResult`**: recorded at G.13 completion.

### H2 — Novel module topology (G.1)

* **Risk**: `tests/contract/agent_harness/` has no in-repo precedent (R4).
* **Bounding evidence**: R1 proves auto-discovery covers only `tests/*.rs` and
  `tests/*/main.rs`, neither of which this creates, so a stray target is
  impossible. The construct is ordinary Rust module resolution.
* **Mitigation**: G.1 is a compile-and-run-only unit. If the topology fails, it
  fails in the first unit with nothing else built on it, and the fallback is the
  deliberation's Option T-B (single file) at the cost of the G.4–G.11 split.
* **`ActionRisk`**: `low`.

### H3 — Red branch state between G.1 and G.14

* **Condition**: `cargo dev-test` is intentionally red from G.1 until G.14
  replaces the placeholder, and G.12 keeps the CI assertion red until G.13.
* **This is the required test-first state**, not a defect. It is bounded to the
  shipment and must be resolved before any PR. G.15 is the gate that proves it
  was.
* **`ActionRisk`**: `low`. Merge is impossible while red because G.15 fails.

### H4 — Hidden-directory traversal

* **Risk**: `WalkBuilder`'s default `hidden(true)` silently skips `.github`
  descendants, producing an empty file set that a naive design would read as
  "all assertions passed".
* **Mitigation**: `hidden(false)` set explicitly; each harness directory added as
  its own root; **F4 makes an empty resolution a hard failure**; G.14 scenario 1
  pins the resolution against the real R7 counts so a silent-skip regression
  fails loudly.
* **`ActionRisk`**: `moderate` — this is the single most likely silent-failure
  mode and is defended three ways.

### H5 — Pedantic lint exposure (R3)

* **Risk**: the test crate does not inherit `src/lib.rs`'s 20 allows, so lints
  suppressed everywhere in the library are live here.
* **Mitigation**: narrow, named, per-target `#![allow(clippy::…)]` only, matching
  the 21-file/15-file precedent. **`allow(dead_code)` is prohibited** — the
  single-target design makes every item reachable, and G.15 scenario 2 enforces
  it. A request for broad suppression is a review-blocking finding.
* **`ActionRisk`**: `low`.

### H6 — Coverage-oracle drift

* **Risk**: a new target unmapped by a `src/` surface fails `--mode completeness`.
* **Bounding evidence**: surfaces match by target-**name** glob and every `src/`
  surface already lists `"contract_*"`; the mandated `contract_` prefix therefore
  keeps completeness PASS with no manifest edit. G.1 scenario 3 verifies this at
  the first unit rather than discovering it at G.15.
* **`ActionRisk`**: `low`.

## Verification Strategy

1. Per-unit: the named scenarios, run via
   `cargo test --test contract_agent_harness_contracts`.
2. Package-level (G.15): `cargo fmt --all -- --check`, `cargo lint`,
   `cargo dev-test`, `scripts/test-coverage-oracle.ps1 --mode completeness`.
3. Negative-path proof: G.14 scenario 3 mutates a fixture to delete a required
   clause and asserts a nonzero result with the F1 code — the harness is proven
   to fail, not merely to pass.
