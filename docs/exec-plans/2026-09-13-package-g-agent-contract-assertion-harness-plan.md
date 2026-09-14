---
title: "Package G implementation plan — executable agent-contract assertion harness"
description: "Test-first plan for a repository-owned Rust contract-test harness that proves agent, prompt, instruction, and policy contract assertions"
source_document: "docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md"
plan_status: "superseded"
superseded_by: "docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v2.md"
tags:
  - "verification"
  - "contract-tests"
  - "harness"
---

## Source Document

`docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md`

Decision adopted: **Option A** — a Rust contract-test harness with a typed
assertion registry, plus a scoped narrowing of the CI `paths-ignore` entry.

This plan is independent. It encodes **no** dependency on Packages A–F, and in
particular none on the failed Packages B and C. It ships the mechanism only; no
package-specific policy is baked into G.

## Objective

Give the repository an executable, repository-owned gate that returns non-zero
when a declared harness contract over tracked markdown under `.github/agents/`,
`.github/skills/`, `.github/instructions/`, `.github/policies/`, and
`.github/prompts/` is violated — so that policy-only release units can prove
their acceptance criteria instead of asserting them.

## Constraints Carried From the Source Document

* No product runtime behavior change. Nothing under `src/` changes behavior;
  `engram::services::verify` is consumed as a **library call only**.
* No new dependency. `std::fs` traversal and plain string matching only.
  `regex`, `walkdir`, and `glob` remain absent.
* No `Select-String` / `grep` success ambiguity. Absence of an expected contract
  and presence of a prohibited contract both fail.
* Explicit workspace containment via the compile-time repository root. No writes
  outside the working tree; the harness is read-only over tracked files.
* Deterministic output suitable for CI and local use.
* Only tracked, repository-owned surfaces. The globally installed `autoharness`
  is not modified.

## Architecture Summary

**Corrected in Round 1.** Module layout, traversal mechanism, and assertion
vocabulary were all revised in response to plan review.

Support code lives in a dedicated subdirectory, **not** inside
`tests/helpers/mod.rs`. That file is itself a declared `[[test]]` target
(`name = "helpers_daemon_harness"`) carrying a file-level `#![allow(dead_code)]`;
nesting here would inherit that allow — defeating the fail-closed posture this
harness exists to model — and would couple markdown validation to an unrelated
daemon fixture.

```text
tests/helpers/harness_contract/
  mod.rs         — re-exports; declares the submodules below
  workspace.rs   — root resolution, marker validation, path containment
  corpus.rs      — deterministic bounded document scanning and loading
  model.rs       — ContractAssertion, AssertionKind, ContractFinding
  evaluator.rs   — pure evaluation producing a deterministic report
  registry.rs    — provider catalog, composition, completeness checking
  providers/
    self_ci.rs   — G's own provider (the worked example; see G.8)

tests/contract/
  harness_corpus_test.rs      — [[test]] contract_harness_corpus
  harness_assertions_test.rs  — [[test]] contract_harness_assertions
  harness_registry_test.rs    — [[test]] contract_harness_registry
```

Three targets, not one. Each red/green pair compiles independently, so every red
phase can be *observed* to fail for its stated reason — Principle II requires
this, and a single shared target made it unreachable.

### Module inclusion mechanism (corrected)

Rust does not auto-discover files under `tests/`. Every consumer includes support
code explicitly; the repository already does this 41 times. Each contract target
begins with:

```rust
#[path = "../helpers/harness_contract/mod.rs"]
mod harness_contract;
```

Submodules declared inside `mod.rs` resolve normally relative to that file, so no
further `#[path]` attributes are needed below the root.

### Traversal mechanism (corrected)

Review established that `ignore = "0.4"` and `globset = "0.4"` are **existing
regular dependencies**. The original research listed only `regex`, `walkdir`, and
`glob` as absent and never considered them. Principle VI prefers reusing existing
project dependencies over hand-rolled logic, and `ignore::WalkBuilder` resolves
three separate review findings natively:

| Finding | `WalkBuilder` setting |
|---|---|
| Symlinks must not be followed | `follow_links(false)` — the default, set explicitly |
| Traversal must be bounded | `max_depth(..)`, `max_filesize(..)` |
| Order must be deterministic | `sort_by_file_path(..)` keyed on the UTF-8 repo-relative path |

Using `ignore` adds no dependency and replaces custom `std::fs` recursion. Its
gitignore semantics are acceptable and desirable here: untracked scratch files
under `.github/` are not harness contract content.

### Containment (corrected)

Containment is a designed, tested property, not an assumed one:

* The repository root is resolved from `env!("CARGO_MANIFEST_DIR")`, canonicalized
  once, and validated by the presence of **both** `Cargo.toml` and
  `.github/agents/`. Either absent is a hard failure before any assertion runs.
* Every target selector must be repo-relative. Absolute selectors and selectors
  containing `.` or `..` components are rejected outright — `Path::join` discards
  the base when given an absolute path, and `Path::starts_with` is lexical and
  does not resolve `..`.
* Every candidate path is canonicalized and must be a component-wise child of the
  canonical root. **Both sides** are canonicalized, so Windows `\\?\` prefixes and
  junction/reparse resolution are handled consistently.
* Symlinks and reparse points are refused, not followed, which also eliminates
  link cycles.

### Assertion vocabulary (corrected — trimmed)

Four kinds, down from six. `SectionPresent` was dropped as expressible via
`Contains`; `FrontmatterFieldEquals` was dropped as speculative — no downstream
package cites a concrete need, and adding a kind later is a small self-contained
change, exactly as the design intends.

| Kind | Semantics |
|---|---|
| `DocumentExists` | The selected document is present and readable. |
| `Contains` | The expected needle occurs in the document. |
| `DoesNotContain` | The prohibited needle does not occur in the document. |
| `FrontmatterFieldPresent` | The named YAML frontmatter key is present. |

Needle matching searches the **whole document string**, then derives the 1-based
line number from the match byte offset. Per-line searching would silently never
match a multi-line needle — reintroducing match-absence ambiguity at the matcher
layer, the exact defect this package exists to remove.

`DocumentExists`, `Contains`, and `DoesNotContain` accept any tracked text
document by explicit repo-relative path. Directory scans are markdown-only and
restricted to the five harness directories.

### Diagnostics contract

Findings use a fixed schema — stable `rule_id`, repo-relative path, 1-based line
number, and a static message. Diagnostics **never** interpolate document content,
matched substrings, or frontmatter values. After G.8 broadens CI triggers, these
run on public `pull_request` logs; echoing a matched line would leak any
credential that a `DoesNotContain` rule ever matched. Absolute paths appear only
in the unresolvable-root diagnostic.

The evaluator aggregates **all** findings before failing, so an agent can correct
everything in one pass rather than one defect per compile cycle (Principle X).

### Frontmatter handling

`FrontmatterFieldPresent` reuses `engram::services::parsing::frontmatter::parse`
rather than re-implementing YAML block extraction. That function maps
*present-but-malformed* frontmatter to `metadata: None`, indistinguishable from
*absent*. The evaluator therefore consults
`engram::services::verify::verify_markdown` first and surfaces a
`frontmatter.malformed` finding in preference to a misleading "field absent".

### Registry and provider catalog (corrected)

Comparing registered rule identifiers against evaluation results catches evaluator
gaps and duplicate identifiers, but **cannot** detect a provider module omitted
from composition entirely — it contributes no identifier and passes silently. The
corrected design adds an explicit catalog:

* A provider catalog maps a stable `package_id` to a provider function. Package
  IDs and rule IDs must each be unique.
* A completeness check compares the catalog against the conventionally-named files
  in `tests/helpers/harness_contract/providers/`. A provider file with no catalog
  entry, or a catalog entry with no file, fails the harness.
* Every registered rule must produce exactly one evaluation result.
* Rollback is atomic and documented: delete the provider file and its catalog
  entry. Adding or removing a package necessarily edits the catalog — this is an
  index, not a monolith, and the plan states it plainly rather than claiming zero
  shared edits, which Rust's module system cannot deliver.

## Implementation Units

Test-first throughout. Every red-phase unit must compile and be **observed** to
fail for its stated reason, with that output captured in the commit or PR body,
before its paired green-phase unit begins.

### G.1 — Red: workspace containment and corpus contract tests

* **Domain**: tests
* **Files**: `tests/contract/harness_corpus_test.rs` (new), `Cargo.toml`
  (`[[test]]` entry `contract_harness_corpus`)
* **Work**: Failing tests for root resolution and canonicalization; rejection of a
  root missing `Cargo.toml` or `.github/agents/`; deterministic sorted traversal;
  and **failure on an empty resolved set**. Plus the containment escape cases:
  file symlink pointing outside the repo, directory symlink pointing outside,
  symlink loop, `..` selector, absolute selector, and a Windows junction case
  (skipped where unsupported). Plus bound cases: over-depth and oversize inputs
  must fail with a bound-specific diagnostic rather than diverging.
* **Milestone**: `cargo test --test contract_harness_corpus` compiles and fails
  because `harness_contract::workspace` and `::corpus` do not exist.
* **Acceptance criteria**:
  * The target uses `#[path = "../helpers/harness_contract/mod.rs"]`.
  * Every escape case and every bound case has a distinct named test.
  * The empty-set test asserts failure; an empty set is never a pass.
  * Red output is captured before G.2 begins.
* **Depends on**: none.

### G.2 — Green: workspace and corpus modules

* **Domain**: code
* **Files**: `tests/helpers/harness_contract/mod.rs`, `workspace.rs`, `corpus.rs`
  (all new). `tests/helpers/mod.rs` is **not** modified.
* **Work**: Implement root resolution, marker validation, and canonical-child
  containment; implement bounded deterministic scanning via `ignore::WalkBuilder`
  with `follow_links(false)`, explicit `max_depth`, `max_filesize`, and
  `sort_by_file_path` keyed on the UTF-8 repo-relative path.
* **Milestone**: All G.1 tests pass.
* **Acceptance criteria**:
  * `cargo test --test contract_harness_corpus` is green.
  * `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`
    passes with no new `#![allow(..)]`; `pub` items carry `# Errors` / `# Panics`
    docs and `#[must_use]` where pedantic requires, matching the
    `tests/helpers/mod.rs` convention.
  * No new dependency entry appears in `Cargo.toml`.
  * The module performs no writes.
* **Depends on**: G.1.

### G.3 — Red: assertion model and evaluator contract tests

* **Domain**: tests
* **Files**: `tests/contract/harness_assertions_test.rs` (new), `Cargo.toml`
  (`[[test]]` entry `contract_harness_assertions`),
  `tests/fixtures/harness_contract/` (fixtures)
* **Work**: Failing tests for: missing target document; `Contains` needle absent;
  `DoesNotContain` needle present, reporting the correct line number; a
  **multi-line** needle matching correctly; malformed frontmatter surfacing
  `frontmatter.malformed` rather than "field absent"; empty body failing via
  `verify_markdown`; and a diagnostic-hygiene test asserting that a
  `DoesNotContain` failure message does **not** contain the prohibited fixture
  substring.
* **Milestone**: Compiles and fails because `harness_contract::model` and
  `::evaluator` do not exist.
* **Acceptance criteria**:
  * Fixtures cover conformant, malformed-frontmatter, empty-body, and
    prohibited-content documents.
  * Diagnostics are asserted, not only boolean outcomes.
  * No Package A–F policy content appears in any fixture or test.
  * Red output is captured before G.4 begins.
* **Depends on**: G.2.

### G.4 — Green: assertion model and evaluator

* **Domain**: code
* **Files**: `tests/helpers/harness_contract/model.rs`, `evaluator.rs` (new)
* **Work**: Implement `ContractAssertion`, the four-variant `AssertionKind`,
  `ContractFinding`, and the evaluator. Whole-document needle search with byte
  offset to line-number derivation. Frontmatter via
  `parsing::frontmatter::parse`, gated on the `verify_markdown`
  `frontmatter.malformed` finding. Findings aggregated before failure.
* **Milestone**: All G.3 tests pass.
* **Acceptance criteria**:
  * Each `AssertionKind` variant is exercised by a passing test.
  * `src/` is unmodified; `verify_markdown` is a library call only.
  * Diagnostics carry rule id, repo-relative path, and line number, and never
    interpolate document content.
  * `cargo lint` passes with no new allows.
* **Depends on**: G.3.

### G.5 — Red: registry, catalog, and composition contract tests

* **Domain**: tests
* **Files**: `tests/contract/harness_registry_test.rs` (new), `Cargo.toml`
  (`[[test]]` entry `contract_harness_registry`)
* **Work**: Failing tests for: **two synthetic G-owned providers composing
  correctly** (proving the extensibility contract rather than asserting it); a
  duplicate `package_id` failing; a duplicate `rule_id` failing; a registered rule
  producing no evaluation result failing; and a provider file present with no
  catalog entry failing, plus the inverse.
* **Milestone**: Compiles and fails because `harness_contract::registry` does not
  exist.
* **Acceptance criteria**:
  * At least two independent providers are composed in one test.
  * Synthetic providers contain no Package A–F policy content.
  * Every fail-closed condition has a distinct named test.
  * Red output is captured before G.6 begins.
* **Depends on**: G.4.

### G.6 — Green: registry composition and completeness

* **Domain**: code
* **Files**: `tests/helpers/harness_contract/registry.rs` (new),
  `tests/helpers/harness_contract/mod.rs`
* **Work**: Implement the provider catalog keyed by stable `package_id`,
  composition, uniqueness enforcement, one-result-per-rule enforcement, and the
  catalog-versus-provider-directory completeness check.
* **Milestone**: All G.5 tests pass; all three targets green.
* **Acceptance criteria**:
  * An omitted or uncatalogued provider fails the harness.
  * Adding a package requires exactly two edits: a new provider file and one
    catalog entry. This is stated plainly, not claimed as zero shared edits.
  * Rollback is documented as deleting both.
  * `cargo lint` passes with no new allows.
* **Depends on**: G.5.

### G.7 — Coverage manifest mapping

* **Domain**: config
* **Files**: `.cargo/test-coverage-manifest.toml`
* **Work**: Map the five harness directories, `tests/fixtures/harness_contract/`,
  and `tests/helpers/harness_contract/` to the three new targets. Without this,
  `scripts/test-coverage-oracle --mode run` — the change-scoped runner agents are
  directed to use — resolves **zero** required targets for a harness-markdown edit
  and reports PASS: the exact false-PASS defect this package exists to eliminate,
  in the runner agents actually invoke.
* **Milestone**: A representative change in every mapped surface selects the
  correct target(s).
* **Acceptance criteria**:
  * Every mapped surface resolves to a non-empty required-target set.
  * `scripts/test-coverage-oracle --mode completeness` passes.
  * No harness input can resolve to an empty required set.
* **Depends on**: G.6.

### G.8 — CI trigger narrowing and self-guarding assertion

* **Domain**: config
* **Files**: `.github/workflows/ci.yml`,
  `tests/helpers/harness_contract/providers/self_ci.rs` (new),
  catalog entry in `registry.rs`
* **Work**: Delete the `'.github/**/*.md'` entry from **both** the `push` and
  `pull_request` `paths-ignore` blocks. Glob negation is prohibited. Update the
  comment block to record why harness markdown is executable markdown, and to
  record the invariants that make broadening safe: the trigger is `pull_request`
  (not `pull_request_target`), `permissions: contents: read`,
  `persist-credentials: false`, actions SHA-pinned, no `secrets.*` references.
  Register G's own provider asserting that neither trigger block reintroduces a
  broad `.github` markdown ignore and that the two blocks stay aligned — so the
  gap cannot silently reopen. This is G guarding its own CI contract, not a
  Package A–F policy, and it doubles as the worked registration example for G.9.
* **Milestone**: A harness-markdown-only change arms `build`; a `docs/**`-only
  change does not.
* **Acceptance criteria**:
  * Both trigger blocks updated identically; no `!` negation anywhere.
  * The required-status-check contingency note is preserved and re-verified.
  * The self-guarding assertion fails if either block is reverted.
  * No new CI job or command; no secrets, write permissions, or
    `pull_request_target` introduced.
* **Depends on**: G.7.

### G.9 — Documentation and agent discoverability

* **Domain**: docs
* **Files**: `docs/ARCHITECTURE.md`, `AGENTS.md`,
  `.github/instructions/agent-engram.instructions.md` is **not** touched; instead
  add the registration duty to `.github/policies/workflow-policies.md`
* **Work**: Document what the harness proves, the four required failure
  conditions, how to run it, and the two-edit registration procedure using the
  G.8 `self_ci` provider as the worked example. Surface the registration duty
  where agents actually look — `AGENTS.md` development workflow and the workflow
  policies — not only in `docs/ARCHITECTURE.md`, which agents do not read during
  policy work.
* **Milestone**: An agent editing harness markdown discovers the registration duty
  without reading the implementation.
* **Acceptance criteria**:
  * The four failure conditions are documented.
  * The two-edit registration procedure is documented with the concrete `self_ci`
    example.
  * `AGENTS.md` and the workflow policies state the registration duty.
  * No unrelated documentation is modified.
* **Depends on**: G.8.

## Dependency Graph

Strictly linear. Each unit depends only on its immediate predecessor:

```text
G.1 → G.2 → G.3 → G.4 → G.5 → G.6 → G.7 → G.8 → G.9
```

Acyclic by construction. Red/green pairs are G.1/G.2, G.3/G.4, and G.5/G.6.

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | Satisfied. No `unsafe`. Assertion failure is an explicit panic in test context, the idiomatic `cargo test` failure channel. G.2/G.4/G.6 each require `cargo lint` to pass with no new `#![allow(..)]`, so pedantic compliance is verified per unit rather than deferred. |
| II. Test-First Development | Satisfied and now *observable*. Three separate targets give three independent red/green pairs; each red phase must be captured as evidence before its green unit starts. |
| III. Workspace Isolation | Satisfied by design and by test. Canonical-child containment on both sides, refused symlinks and reparse points, rejected absolute and `..` selectors, with named escape tests in G.1. |
| IV. CLI Workspace Containment | Satisfied. Read-only within the working tree; no writes anywhere. |
| V. Structured Observability | Satisfied. Fixed diagnostic schema: stable rule id, repo-relative path, line number; deterministic ordering; all findings aggregated. |
| VI. Single Responsibility | Satisfied. Zero new dependencies. Reuses existing `ignore`, `globset`, `serde_yaml`, and the existing `verify` and `frontmatter` services rather than hand-rolling traversal or YAML extraction. |
| VII. Destructive Command Approval | Not applicable. No destructive command appears in any unit; all changes are additive and revert cleanly. |
| VIII. Explicit Safety Modes | Satisfied. Investigate-first posture applied in hardening: branch protection was re-verified live before accepting G.8's risk classification, and glob negation was prohibited after analysis rather than assumed safe. |
| IX. Git-Friendly Persistence | Not applicable — no persisted state. |
| X. Context Efficiency | Satisfied. Findings name rule, path, and line, and are aggregated so an agent fixes all defects in one pass without re-reading whole documents. |

No principle violations. No justified exceptions required.

## Hardening Signals Assessment

* Public API, schema, or contract change — **yes**. G.8 changes the CI trigger
  contract. Narrowing `paths-ignore` alters which pushes and pull requests run the
  `build` job, which is a repository-wide behavioral contract with a documented
  required-status-check contingency.
* Security, auth, permission, or compliance-sensitive behavior — no.
* Migration, backfill, destructive data/config action, or irreversible step — no.
  All changes are additive and revertible by a single revert commit.
* External integration, operator checkpoint, or external dependency — no. The
  globally installed `autoharness` is explicitly not touched.
* High runtime, rollout, or rollback risk — **partial**. An over-broad
  `paths-ignore` narrowing would run the full Rust build on documentation-only
  pull requests; an under-broad one would leave harness contract violations
  unverified in CI, which is the exact defect G exists to remove.

**Requires plan hardening: yes**

## Runtime Verification and Closure

* **Runtime surface changed**: none in the product. `engram` binary behavior,
  CLI surface, MCP tool surface, and daemon behavior are all unchanged. The only
  behavioral change outside tests is the CI trigger set.
* **Runtime verification**: prove the four required failure conditions by
  observing real non-zero exits — a missing target, a malformed fixture, an empty
  scan, and an unresolvable workspace root must each fail the target. Then prove
  the trigger change by confirming a harness-markdown-only change arms the `build`
  job while a `docs/**`-only change does not.
* **Operational closure**: record the verified non-zero exit evidence, the
  confirmed CI trigger behavior for both the armed and ignored cases, and the
  re-verification that `build` remains a non-required status check. Rollback is a
  single revert of the release unit; no data or config migration is involved.

## Plan Hardening

### Hardening required — why

Yes. The plan carries one contract-change signal and one rollout-risk signal,
both concentrated in **G.8**: narrowing the CI `paths-ignore` set changes which
pushes and pull requests run the `build` job. That is a repository-wide
behavioral contract with a previously documented failure mode, so it must be
specified precisely rather than left to implementation-time judgement.

Units G.1-G.7 and G.9 do not independently require hardening. They are additive,
test-scoped, read-only, and revertible.

### Invariants to preserve

1. **Code-PR coverage is never weakened.** Every change touching `**/*.rs`,
   `Cargo.toml`, `Cargo.lock`, `**/*.toml`, `.github/workflows/**`, `src/**`,
   `crates/**`, `scripts/**`, or `examples/**` must continue to arm the full
   fmt → clippy → test → audit sequence.
2. **No product runtime behavior change.** `src/` is read but never modified.
   `engram::services::verify::verify_markdown` is consumed as a library call.
3. **No new dependency.** `regex`, `walkdir`, and `glob` stay absent.
4. **Fail-closed always.** An empty file set, an unresolvable workspace, a
   missing target, and an unevaluated rule identifier are failures, never passes.
5. **G stays policy-free.** No Package A–F assertion ships in G, and G encodes no
   dependency on the failed Packages B or C.

### Learnings and instructions consulted

* `docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md` — supplies
  the binding rule that a fail-closed oracle must treat an indeterminate or empty
  input as failure, and the warning against shell-backed verification tooling.
  Both are reflected in the Option A decision and in invariant 4.
* `docs/decisions/2026-07-04-ci-build-skip-required-check-spike.md`,
  `docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md`, and
  `docs/closure/2026-07-04-ci-build-skip-adversarial-review.md` — the origin of
  the current `paths-ignore` design, its adversarial-review P1 finding against a
  blanket `**/*.md`, and the required-status-check contingency. All three were
  re-read; the contingency is reconfirmed below.
* `.github/instructions/ci-security.instructions.md` and
  `.github/instructions/workflows.instructions.md` — govern the G.8 workflow edit.

### G.8 specification, tightened

Investigation during hardening produced a fact that materially simplifies G.8 and
removes a fragile implementation path:

> `.github/copilot-instructions.md` is the **only** markdown file under `.github/`
> that is not already inside the five harness directories.

Consequences:

* **Do not attempt glob negation.** An earlier reading of G.8 implied a
  "narrowed pattern set", which invites `!`-negation entries in `paths-ignore`.
  GitHub Actions negation is order-sensitive and easy to get silently wrong, and
  a silently-wrong ignore rule reproduces exactly the under-run defect G exists to
  eliminate. Negation is prohibited for this change.
* **Preferred implementation — remove the entry.** Delete the
  `'.github/**/*.md'` entry from both the `push` and `pull_request` blocks. Since
  every `.github` markdown file is agent-harness contract content, the entire
  surface is executable markdown once G exists. This is the fail-closed choice:
  it can only over-trigger, never under-trigger.
* **Accepted cost.** A documentation-only edit to `.github/copilot-instructions.md`
  will run the full Rust build. That is one file, and over-triggering is the safe
  direction. The rejected alternative — keeping a single explicit
  `'.github/copilot-instructions.md'` ignore entry — was set aside because that
  file is itself an agent contract surface and a plausible future assertion
  target, so ignoring it would re-open the gap for the one file most likely to
  need it.
* **Unchanged entries.** `.backlogit/**`, `docs/**`, `.autoharness/**`, root
  `*.md`, and `scripts/**/*.md` remain ignored exactly as today.

### Required-status-check contingency — reconfirmed

The `ci.yml` comment warns that `paths-ignore` is safe only while `build` is not a
required status check, otherwise documentation-only pull requests would hang on
"Expected — Waiting for status to be reported".

Reconfirmed during hardening: `GET repos/softwaresalt/agent-engram/branches/main/protection`
returns HTTP 404 "Branch not protected". `main` has no branch protection, so
`build` is not a required check and no pull request can hang. G.8 must not alter
this contingency note; it must preserve it and record that it was re-verified.

Because G.8 **reduces** the ignore surface, it strictly decreases the population
of pull requests that skip `build`. It therefore cannot introduce a new hang risk
even if branch protection were enabled later.

### Risky actions

| ProposedAction | Targets | Change kind | ActionRisk | Approval | Rollback | ActionResult |
|---|---|---|---|---|---|---|
| Remove the `'.github/**/*.md'` ignore entry from both trigger blocks | `.github/workflows/ci.yml` | CI trigger contract change | `moderate` | Not required — additive coverage only, strictly reduces skipping, `main` unprotected | Revert the single-file edit | `planned` |
| Add `[[test]]` target and helper module | `Cargo.toml`, `tests/**` | Additive test surface | `low` | Not required | Revert the release unit | `planned` |
| Add harness fixtures | `tests/fixtures/harness_contract/` | Additive test data | `low` | Not required | Delete the fixture directory | `planned` |

No action in this plan is `destructive` or `high`. Nothing here deletes data,
rewrites history, alters product configuration, or touches an external system.
The globally installed `autoharness` is not modified.

### Deepened runtime verification

Environment prechecks, run before verification is accepted:

1. `cargo fmt --all -- --check` — clean.
2. `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic` — clean.
3. Confirm no new entry appeared in `[dependencies]` or `[dev-dependencies]`.

Target scenarios — each must be observed as a **real non-zero exit**, not merely
reasoned about:

| Scenario | How to prove |
|---|---|
| Missing assertion target | Register an assertion against a non-existent document; observe failure naming the resolved absolute path. |
| Malformed input | Point the harness at the malformed fixture; observe failure via `verify_markdown`. |
| Empty file set | Scan a directory containing no documents; observe failure, not a pass. |
| Unresolvable workspace | Force the marker check to fail; observe failure before any assertion is evaluated. |
| Prohibited content present | Register a `DoesNotContain` assertion matching fixture content; observe failure with the offending line number. |
| Determinism | Run the target three times; diagnostic ordering is byte-identical. |

CI trigger verification, both directions:

* **Armed case** — the pull request for this release unit itself changes
  `.github/workflows/ci.yml`, which already arms `build`. To prove the harness
  markdown path specifically, confirm on the merged result that a subsequent
  change touching only a file under one of the five harness directories arms
  `build`.
* **Ignored case** — confirm that a change touching only `docs/**` still skips
  `build`.

Blocked-path handling: if either trigger case cannot be observed before closure,
record it as an open runtime-verification item with the reason. Do **not** mark
G.8 verified on reasoning alone — an unverified trigger change is the precise
failure mode this plan is hardening against.

### Deepened operational closure

* **Healthy signals**: `build` runs on code and harness-markdown changes;
  `build` is skipped on `docs/**`-only changes; the harness target passes on
  `main`; total `cargo dev-test` wall time shows no material regression.
* **Failure signals**: `build` skipped on a harness-markdown-only change (the
  under-run defect has returned); the harness passing against a deliberately
  violating fixture (a false PASS, the most serious possible outcome); harness
  runtime materially increasing suite duration.
* **Rollback trigger**: any observed false PASS, or `build` failing to arm on a
  harness-markdown-only change after merge.
* **Rollback procedure**: revert the release unit's merge commit as a unit. No
  data, schema, or configuration migration is involved, so revert is complete and
  immediate.
* **Owner**: the Ship agent executing this release unit, through post-merge
  operational closure.
* **Validation window**: the first three merged pull requests after this unit
  lands — at least one code change, one harness-markdown change, and one
  documentation-only change — confirming the trigger matrix in both directions.

### Unresolved operator decisions

None block execution. Two planning-level questions from the source document are
now resolved by this hardening:

* **Single target vs. one per domain** — superseded by Correction Round 1. The
  plan now declares **three** targets (`contract_harness_corpus`,
  `contract_harness_assertions`, `contract_harness_registry`) because a single
  shared target made the red phases impossible to observe independently, which
  Principle II requires.
* **Exact `paths-ignore` change** — resolved above: remove the
  `'.github/**/*.md'` entry outright; negation globs are prohibited.

The remaining open question from the source document stands and is intentionally
deferred: no Package A–F assertion ships in G. G delivers the mechanism and its
own self-tests only.

## Plan Review

**Gate decision: FAIL (round 1)** — 9 P1 findings. No P0 findings.

Reviewed at plan revision prior to Correction Round 1. Personas dispatched with
cross-model diversity:

| Persona | Model | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Constitution Reviewer | claude-sonnet-5 | 0 | 2 | 4 | 5 |
| Rust Reviewer | claude-sonnet-5 | 0 | 2 | 4 | 2 |
| Scope Boundary Auditor | claude-sonnet-5 | 0 | 2 | 2 | 5 |
| Learnings Researcher | claude-haiku-4.5 | 0 | 0 | 0 | 0 |
| Architecture Strategist | gpt-5.6-sol | 0 | 2 | 5 | 1 |
| Security Lens Reviewer | grok-4.6 | 0 | 1 | 2 | 2 |
| Agent-Native Parity Reviewer | gemini-3.8-flash | 0 | 0 | 5 | 1 |

Hardening was required and a `## Plan Hardening` section is present. Risky
actions are classified with `ProposedAction` / `ActionRisk` / `ActionResult`, so
the strict-safety gate condition is satisfied. The FAIL is driven entirely by
P1 findings, not by missing hardening.

The architectural decision (Option A) was **affirmed** by every persona that
evaluated it. The Learnings Researcher returned `confidence: high` with zero
contradictions against prior learnings. All P1 findings are bounded, mechanical,
and individually actionable; none invalidates Option A.

### P1 findings

**P1-1 — Red/green phases are not independently observable (Constitution).**
G.1 and G.2 write to the same file, and G.2's tests reference evaluator and
registry symbols not implemented until G.4. A Rust test binary must type-check as
a whole crate before any test runs, so G.3's stated milestone ("passes the G.1
tests") is unreachable: after G.3 the crate still fails to compile on unresolved
G.2/G.4 symbols. Principle II requires each red phase to be *observed* to fail
for the stated reason. **Fix**: split into separate test targets so each red/green
pair compiles and is independently verifiable.

**P1-2 — G.2 exceeds the 2-hour rule (Constitution).** Five distinct test
scenarios against the "fewer than 4 test scenarios" heuristic. **Fix**: split.

**P1-3 — Module resolution model is wrong (Rust).** `tests/helpers/*.rs` files are
not auto-discovered. Every consumer includes them explicitly via
`#[path = "../helpers/<file>.rs"] mod <name>;` — confirmed 41 such declarations
across `tests/`. A plain `mod harness_contract;` in
`tests/contract/harness_contract_test.rs` resolves to `tests/contract/harness_contract.rs`
and fails with "file not found for module" — a red phase failing for the wrong
reason. **Fix**: specify the `#[path]` include explicitly.

**P1-4 — `tests/helpers/mod.rs` is the wrong host (Rust, Architecture).**
Confirmed: `tests/helpers/mod.rs` is itself a declared `[[test]]` target
(`name = "helpers_daemon_harness"`), and it carries a file-level
`#![allow(dead_code)]`. Nesting `harness_contract` inside it would inherit that
allow — silently defeating the fail-closed posture the harness exists to model —
and would drag markdown-contract code into an unrelated daemon-harness binary.
**Fix**: create a sibling file; do not modify `tests/helpers/mod.rs`.

**P1-5 — Speculative assertion kinds (Scope).** `FrontmatterFieldEquals` and
`SectionPresent` appear in none of G's five required failure conditions and no
concrete downstream need is cited. `SectionPresent` is expressible as `Contains`.
This also inflates G.4 past the granularity limit. **Fix**: trim to the kinds G
actually exercises.

**P1-6 — Extensibility contract is asserted but never exercised (Scope).** The
"each package adds its own registry provider" guarantee is the reason downstream
packages can adopt the harness, yet no test composes two or more providers. For
the package whose entire mandate is "prove it, don't assert it", shipping this
unproven is self-contradictory. **Fix**: add a two-provider composition test using
synthetic, policy-free providers owned by G.

**P1-7 — Change-scoped coverage oracle yields a false PASS (Architecture,
Agent-Native).** Confirmed: `.cargo/test-coverage-manifest.toml` exists, and files
outside `src/`/`crates/` map to no targets. An agent editing harness markdown and
running `scripts/test-coverage-oracle --mode run` resolves zero required targets
and reports PASS — precisely the false-PASS defect G exists to eliminate, in the
runner agents are directed to use. **Fix**: map the harness surfaces to the new
target and prove the mapping.

**P1-8 — An omitted provider is undetectable (Architecture).** Comparing
registered rule identifiers against evaluation results catches evaluator gaps and
duplicates, but a provider module omitted from composition contributes no
identifier and passes silently. **Fix**: an explicit provider catalog with stable
package IDs plus a completeness check against conventionally-named registry files.

**P1-9 — Containment is claimed but not designed (Security).** The plan asserts
"every resolved path must remain under the canonicalized repository root" and the
Constitution Check calls Principles III/IV "Strengthened"/"Satisfied", but
symlinks, Windows junctions/reparse points, `..` components, and absolute
selectors are never addressed. `Path::starts_with` is lexical and does not resolve
`..`; `Path::join` with an absolute right-hand side discards the base. A tracked
symlink under `.github/` would be followed by `read_to_string`, reading outside
the workspace — a direct Principle III/IV violation. No G.1 test covers an escape
case. **Fix**: fail-closed link policy, canonical-child checks on both sides, and
named escape tests.

### Notable P2 findings carried into the correction

* Diagnostics must not interpolate file content or matched substrings into CI
  logs (secrets-leak risk on public `pull_request` runs); use rule id +
  repo-relative path + line number (Security P3, Agent-Native P2).
* Unbounded recursive traversal has no depth, count, or file-size limit; a
  pathological tracked tree can hang or OOM CI after G.6 broadens triggers
  (Security P2).
* Multi-line needles in `Contains` / `DoesNotContain` would silently never match
  under per-line search — reintroducing match-absence ambiguity at the matcher
  layer (Rust P2).
* `engram::services::parsing::frontmatter::parse` maps *malformed* frontmatter to
  `metadata: None`, indistinguishable from *absent*; a frontmatter assertion must
  consult the `frontmatter.malformed` finding first (Rust P2).
* Pedantic lints (`missing_panics_doc`, `missing_errors_doc`, `must_use_candidate`)
  apply to test targets under `--all-targets`; the plan must require `cargo lint`
  to pass on new files (Constitution P2, Rust P2).
* The evaluator should aggregate all findings before failing, so an agent can fix
  everything in one pass rather than one-per-compile (Agent-Native P2, Principle X).
* Discoverability: agents read `AGENTS.md` and `.github/instructions/`, not
  `docs/ARCHITECTURE.md`; the registration duty must be surfaced where agents look
  (Agent-Native P2).
* `ignore = "0.4"` and `globset = "0.4"` are **existing** regular dependencies.
  The deliberation's research findings listed only `regex`/`walkdir`/`glob` as
  absent and did not consider them. Principle VI prefers reusing existing
  dependencies over hand-rolled traversal (Constitution P3).
* Sort key must be the UTF-8 repo-relative path string, not `PathBuf`'s
  platform-native `Ord` (Rust P3).
* Repository markers for root validation must be named concretely (Rust P3).
* Constitution Check omits Principles VII and VIII rows (Constitution P3).
* `paths-ignore` alignment between the two trigger blocks should itself be a
  contract assertion, so the gap cannot silently reopen (Architecture P3).

### Correction authorization

All nine P1 findings are bounded and mechanical, with concrete fixes supplied and
no challenge to Option A. Per the operating instruction for this staging session,
**one correction round is authorized**, followed by a confirmation review. If the
confirmation review does not reach PASS, Package G stops as blocked with no
harvest.

## Correction Round 1

Applied in response to the nine P1 findings above. Option A is unchanged; every
correction is mechanical.

| Finding | Correction |
|---|---|
| P1-1 red/green not observable | Split into three `[[test]]` targets giving three independent red/green pairs (G.1/G.2, G.3/G.4, G.5/G.6). Each red phase must be captured as evidence before its green unit begins. |
| P1-2 G.2 exceeded 2-hour rule | Scenarios redistributed across three red units; each stays within the granularity heuristics. |
| P1-3 wrong module resolution model | Every target now declares `#[path = "../helpers/harness_contract/mod.rs"] mod harness_contract;` explicitly, matching the 41 existing `#[path]` declarations in `tests/`. |
| P1-4 `tests/helpers/mod.rs` wrong host | Support code moved to a new `tests/helpers/harness_contract/` subdirectory. `tests/helpers/mod.rs` is not modified, so its `#![allow(dead_code)]` is not inherited and the daemon harness stays uncoupled. |
| P1-5 speculative assertion kinds | Trimmed from six to four. `SectionPresent` dropped (expressible as `Contains`); `FrontmatterFieldEquals` dropped (no concrete downstream need cited). |
| P1-6 extensibility asserted, not proven | G.5 now requires a two-provider composition test using synthetic, policy-free, G-owned providers. |
| P1-7 coverage-oracle false PASS | New unit G.7 maps the five harness directories, the fixtures directory, and the support directory in `.cargo/test-coverage-manifest.toml`, and requires that no harness input resolve to an empty required-target set. |
| P1-8 omitted provider undetectable | Registry gains an explicit provider catalog keyed by stable `package_id`, with a completeness check against the provider directory in both directions. The plan now states plainly that adding a package edits two files; it no longer claims zero shared edits, which Rust's module system cannot deliver. |
| P1-9 containment claimed, not designed | Containment is now specified and tested: canonicalization of both sides, component-wise child check, refused symlinks and reparse points, rejected absolute and `..` selectors, plus six named escape tests in G.1. |

Carried P2/P3 corrections: bounded traversal (`max_depth`, `max_filesize`) with
fail-closed bound diagnostics; whole-document needle search with byte-offset line
derivation so multi-line needles cannot silently miss; frontmatter assertions
gated on the `frontmatter.malformed` finding; a fixed diagnostic schema that never
interpolates document content; aggregated findings for single-pass agent
correction; per-unit `cargo lint` acceptance criteria; UTF-8 repo-relative sort
key; named repository markers (`Cargo.toml` and `.github/agents/`); Constitution
Check rows added for Principles VII and VIII; the dependency graph simplified to a
linear chain; G.9 extended to surface the registration duty in `AGENTS.md` and
`.github/policies/workflow-policies.md` where agents actually look.

**Material discovery during correction**: `ignore = "0.4"` and `globset = "0.4"`
are existing regular dependencies that the original research overlooked. Adopting
`ignore::WalkBuilder` resolves the symlink, traversal-bound, and sort-determinism
findings natively, with no new dependency and less hand-rolled code — a better
outcome for Principle VI than the original `std::fs` recursion.

**Scope discipline**: the one added surface is G's own `self_ci` provider, which
guards the G.8 CI change from silent reversal and doubles as the worked
registration example. It is G-owned, not a Package A–F policy. G still ships no
package policy and encodes no dependency on Packages B or C.

## Plan Review — Confirmation Round 2

**Gate decision: FAIL — Package G is BLOCKED.** The single authorized correction
round is exhausted and the confirmation review did not reach PASS. Per the
operating instruction for this staging session, G stops here: **no harvest, no
backlog IDs, no shipment.**

| Reviewer | Model | Verdict |
|---|---|---|
| Security Lens Reviewer | grok-4.6 | **PASS** — all round-1 findings closed |
| Rust Reviewer | claude-sonnet-5 | **FAIL** — 1 new P1 |
| Architecture Strategist | gpt-5.6-sol | **FAIL** — 1 escalated P1, 1 partially-closed P1 |
| Scope Boundary Auditor | claude-sonnet-5 | **FAIL** — 1 P1 regression |
| Constitution Reviewer | claude-sonnet-5 | **FAIL** — 3 P1 |

### Closed by Correction Round 1

Confirmed closed with evidence by the reviewers who raised them: P1-3 and P1-4
(module resolution and host module), P1-5 (speculative kinds), P1-6
(extensibility now exercised), P1-7 (coverage-oracle mapping), P1-9
(containment design and escape tests), and every carried P2/P3 item including
diagnostics hygiene, traversal bounds, multi-line needles, frontmatter gating,
pedantic acceptance criteria, sort key, named markers, Constitution rows VII/VIII,
and the dependency graph. Security returned a clean PASS.

### Open P1 findings — verified independently

**C-1 — `verify_markdown` rejects 20 live harness files (Architecture, escalated
from round-1 P2).** `engram::services::verify::verify_markdown` unconditionally
emits `template.unresolved` at `Severity::Error` for every line matching `{{…}}`,
and `VerifyReport::conformant` is derived from `findings.is_empty()`. It is an
*ingestion*-conformance service, not a harness-document validator.

Independently verified: **20 of the 92 tracked harness markdown files contain a
same-line `{{…}}` placeholder**, including `.github/agents/_orchestrator.agent.md`,
`.github/agents/_ship.agent.md`, and eleven files under
`.github/agents/subagents/`. Delegating structural validity to `verify_markdown`
as the corrected plan specifies would fail those files on day one. The round-1
review flagged this coupling as P2 on abstraction grounds; the concrete evidence
escalates it to P1. The plan needs a harness-owned validator, or an adapter that
accepts only `frontmatter.malformed` and `body.empty` and discards
`template.unresolved`.

**C-2 — Shared `mod.rs` cannot satisfy the no-new-allow criterion (Rust, new).**
`.cargo/config.toml` sets `rustflags = ["-Dwarnings"]` globally, and each
`tests/*.rs` target compiles as its own self-contained crate where `pub` does not
exempt items from `dead_code`. All three targets include the same catch-all
`harness_contract/mod.rs`, so once G.4 and G.6 add `model`, `evaluator`,
`registry`, and `providers`, the corpus target compiles modules it never calls and
fails `-D warnings` — regressing an already-green target. This directly
contradicts the plan's own "no new `#![allow(..)]`" acceptance criteria in
G.2/G.4/G.6.

Independently verified: `tests/helpers/mod.rs:34` carries `#![allow(dead_code)]`
for exactly this reason. Correction Round 1 avoided inheriting that allow by
moving one directory over — and reproduced the identical structural problem there.
The fix is per-target `#[path]` inclusion of only the files each target exercises,
or promoting the support tree to a workspace-member lib crate.

**C-3 — Granularity regression across all three red units (Scope and Constitution,
concurring).** The correction relocated the round-1 scenario-count overrun from a
green unit onto the red units rather than eliminating it. Counted from the plan
text: **G.1 ≈ 12 scenarios**, G.3 = 7, G.5 = 6, against the NON-NEGOTIABLE
"fewer than 4 test scenarios" heuristic. G.1 is roughly three times the limit and
materially worse than the original single violation. Correction Round 1's own
table asserts "each stays within the granularity heuristics", which is inaccurate.

**C-4 — G.4 cannot make G.3 green (Constitution, new).** G.4's file list omits
`tests/helpers/harness_contract/mod.rs`, but `model.rs` and `evaluator.rs` must be
declared there for `harness_contract::model` and `::evaluator` to resolve. G.6
correctly lists `mod.rs` for the analogous `registry` edit; G.4 does not. As
written, G.4 leaves G.3 red — the same red/green mismatch the correction was meant
to eliminate, relocated to a new seam.

**C-5 — G.8 has no red phase and mixes domains (Constitution, new).** G.8
introduces new logic (`providers/self_ci.rs` plus a registry catalog entry) with
no preceding failing test and no captured red evidence, unlike G.1/G.3/G.5 — a
Principle II violation in newly written material. It also combines a CI workflow
YAML edit with Rust provider authoring and a registry code edit in one unit,
violating Width Isolation.

**C-6 — Provider catalog binding is not proven (Architecture, partially closed).**
Matching a catalog key to a filename does not prove the catalog entry invokes that
module's provider function. A miskeyed entry leaves a provider uncomposed while
both the filename set and the catalog key set still agree. The plan also omits
`providers/mod.rs`, which Rust requires, so the advertised two-edit workflow has no
concrete mechanism binding declaration, identity, and registration.

### Carried P2 findings for a future attempt

* `WalkBuilder::follow_links(false)` stops descent but still *yields* symlink
  entries, which `read_to_string` follows at read time; Windows junctions may
  report as directories and be descended. A per-entry `symlink_metadata` refusal
  before any open is required (Security NEW-1).
* `self_ci` should also assert the invariants that make the trigger broadening
  safe — no `pull_request_target`, no `secrets.`, `contents: read`,
  `persist-credentials: false` — otherwise it guards only the ignore list
  (Security NEW-2).
* The hardening verification table still asks for an absolute path in the
  missing-target diagnostic, contradicting the diagnostics contract
  (Security NEW-3).
* `WalkBuilder` root strategy must be pinned; `hidden(true)` is the default and
  `.github` is a dot-directory. Depth-0 roots bypass the filter, so adding each
  harness directory as its own root is safe, but the plan must say so and set
  `hidden(false)` as defense in depth (Rust P2).
* Directory-selector semantics for `Contains` / `FrontmatterFieldPresent` remain
  undefined — every document, any document, or the corpus? This conflicts with
  exactly-one-result-per-rule (Architecture P2).
* `FrontmatterFieldPresent` is retained without a cited concrete need, the same
  standard that removed its two siblings (Scope P2).
* `self_ci` is policy-shaped content in a nominally policy-free package, and G.9
  edits `.github/policies/workflow-policies.md`, a file inside the harness's own
  asserted surface. Both are defensible but should be explicit constraints
  (Scope P2).
* Provider directory inventory via `read_dir` observes untracked local files and
  has unspecified order; it needs a fail-closed unexpected-file rule and
  `BTreeSet` ordering (Architecture P2).

### Disposition

The architectural decision (Option A) remains affirmed by every reviewer across
both rounds, and the Learnings Researcher confirmed zero contradictions with prior
learnings. The blocking issues are concentrated in two areas that a future attempt
must settle **before** planning resumes:

1. **The validator abstraction.** `verify_markdown` is the wrong reuse target.
   Decide between a harness-owned structural validator and a filtering adapter,
   with the 20 affected live files as the acceptance evidence.
2. **The module-sharing topology.** One catch-all `mod.rs` across three targets is
   incompatible with global `-Dwarnings` and the no-new-allow criterion. Decide
   between per-target `#[path]` inclusion and a workspace-member lib crate, then
   re-derive unit boundaries from that decision.

Unit decomposition should be re-derived after those two decisions, since both
change the file topology that the granularity findings (C-3, C-4, C-5) depend on.
Re-planning G is a fresh Stage operation, not a third correction round.
