---
type: deliberation
date: 2026-09-13
package: G2
depends_on: G1
source: docs/decisions/2026-09-13-package-g-split-deliberation.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
---

# Package G2 - harness registry and CI/local integration

## Authority and scope

The authority is
`docs/decisions/2026-09-13-package-g-split-deliberation.md`. This deliberation
applies its selected dependency split and T3 topology. It does not reopen either
choice. G2 is the third release unit in `G0 -> G1 -> G2`; it binds the G0
filesystem crate and the G1 assertion crate to this repository's harness corpus.

G2 is test-only and repository-owned. It does not add product runtime behavior,
call `engram`, introduce `verify_markdown`, define evaluator semantics, or
reimplement filesystem behavior. It creates the root-package contract exerciser,
repository seed data, coverage-oracle registration, local lint coverage, and the
minimum existing-workflow trigger correction required by Decision 5a.

The prior v3 review's mechanical findings M-1 through M-5 and advisories A-1
through A-10 are inherited constraints, not material to reproduce. Their source
is `docs/closure/2026-09-13-package-g-v3-plan-review-record.md:149-176`. In
particular, every temporary red `todo!()` has an immediately named green owner,
red units consume no later-unit API, scenarios assert one diagnostic outcome,
and file-count exceptions are stated rather than hidden.

## Problem

The G0 and G1 libraries do not by themselves enforce repository contracts. G2
must make their behavior operational without creating a third implementation
layer:

* resolve the five real harness roots through G0 only
* load G1's typed registry and a minimal repository seed set
* provide a data-only extension point for Packages B and C
* register one root-package `[[test]]` target that existing Cargo and CI commands
  execute
* ensure harness-only Markdown changes trigger CI and select the contract target
* fail closed when the changed-file set is empty or indeterminate
* make coverage-oracle completeness truthful for a test whose inputs are the
  harness corpus and both dev-only crates
* close the canonical local lint gap for workspace members without silently
  widening unrelated CI behavior

The plan is dependent by construction. Neither G0 nor G1 is merged. G2 remains a
draft and is not executable until both prerequisites pass plan review and merge.

## Evidence gathered fresh

All commands in this table were read-only PowerShell commands run from the
repository root on 2026-09-13.

| Evidence | Read-only command and result | Source |
|---|---|---|
| Harness corpus shape | Recursive `Get-ChildItem` over the five roots returned 92 files, all `.md`, with counts agents 23, skills 28, instructions 34, policies 2, prompts 5; maximum root-relative component depth 2; maximum size 98,546 bytes | `.github/{agents,skills,instructions,policies,prompts}`; largest file `.github/agents/_ship.agent.md` |
| T3 topology and ownership | `Select-String` located the selected T3 topology and G2 ownership | `docs/decisions/2026-09-13-package-g-split-deliberation.md:151-221,283-299` |
| Root aliases | `Select-String` confirmed `dev-test = "test --all-targets"`, `lint = "clippy --all-targets --all-features ..."`, and `ci = "test --all-targets --all-features"` | `.cargo/config.toml:20,26-27` |
| Warning policy | `Select-String` confirmed global `rustflags = ["-Dwarnings"]` | `.cargo/config.toml:1-2` |
| CI trigger defect | `Select-String` found `.github/**/*.md` in both trigger blocks | `.github/workflows/ci.yml:30-43` |
| Required-check consequence | The workflow comment says ignored-only PRs would wait forever if `build` became required, and recommends a companion job in that future state | `.github/workflows/ci.yml:20-25` |
| Existing CI commands | CI uses root `cargo clippy ... --all-targets` and `cargo test ... --all-targets`, neither with `--workspace` | `.github/workflows/ci.yml:78-91` |
| Oracle indeterminate behavior | The PowerShell and Bash implementations reject an unresolved base or merge base instead of treating it as an empty diff | `scripts/test-coverage-oracle.ps1:166-199`; `scripts/test-coverage-oracle.sh:147-175` |
| Oracle empty behavior | With a resolved but empty changed set, `select` currently emits zero targets and `STATUS=PASS`; `report` can likewise pass with zero required and omitted targets | `scripts/test-coverage-oracle.ps1:235-271`; `scripts/test-coverage-oracle.sh:213-247` |
| Completeness rule | Only `src/` and `crates/` surfaces contribute target coverage | `scripts/test-coverage-oracle.ps1:203-208`; `scripts/test-coverage-oracle.sh:178-187` |
| Existing crate mapping | The manifest has production `src/` mappings and one workspace-crate mapping for `crates/powerbi-tmdl-parser/` | `.cargo/test-coverage-manifest.toml:29-32,81-139` |
| Existing oracle seam | The unit target already has a cross-platform process helper and scenarios for omitted, unmapped, covered, bounded-run, and completeness outcomes | `tests/unit/dev_test_coverage_oracle_test.rs:23-145`; `Cargo.toml:1073-1077` |
| Workspace lint blast radius | Workspace members are the root package, `powerbi-tmdl-parser`, and `engram-indexer`; G0 and G1 will add two more | `Cargo.toml:1-3`; selected T3 at `docs/decisions/2026-09-13-package-g-split-deliberation.md:189-221` |
| Static lint uncertainty | Both current member crates forbid unsafe code, but neither declares crate-local pedantic linting; `powerbi-tmdl-parser` has 1,993 Rust source lines and `engram-indexer` has 111, so static reading cannot prove a workspace pedantic run is green | `crates/powerbi-tmdl-parser/src/lib.rs:1-9`; `crates/engram-indexer/src/lib.rs:1`; `crates/engram-indexer/src/main.rs:1` |
| Reusable dependencies | Root already declares `serde`, `tempfile`, `toml`, and `globset`; G2 needs no new external dependency and directly adds none | `Cargo.toml:36,59-60,76,123` |

## Question 1 - CI trigger correction

### Option 1A - remove `.github/**/*.md` from both blocks

* Pro: exactly implements Decision 5a and causes harness-only Markdown pull
  requests and pushes to run the existing full `build` job
* Pro: no new workflow, job, action, or CI tool
* Pro: a future harness root under `.github/` is covered without another trigger
  edit
* Con: all `.github` Markdown, not only the five current roots, now incurs the
  full fmt, clippy, test, and audit cost
* Con: if `build` later becomes a required check, other still-ignored doc-only
  pull requests can retain the documented expected-check hazard

### Option 1B - add a companion always-passing job

* Pro: can preserve cheap handling for truly documentation-only changes and
  solve the future required-check waiting state
* Con: it is a new CI mechanism and exceeds G2's permission to use the existing
  workflow only
* Con: it needs its own trustworthy classifier to distinguish harness Markdown
  from inert docs, duplicating selection logic that the coverage oracle owns
* Con: an always-pass branch can mask the exact harness-only change class unless
  conditions are proved exhaustively

### Option 1C - replace the broad pattern with narrow `.github` ignores

* Pro: can preserve skips for known inert `.github` documentation
* Pro: retains one workflow and one job
* Con: requires maintaining a second allow/deny taxonomy as harness layout
  evolves
* Con: a newly added harness root can become silently ignored
* Con: contradicts the authority's explicit instruction to remove the pattern

### Selected option

**Select 1A.** Remove the pattern from both trigger blocks and update the
adjacent explanatory comment so it no longer claims `.github/**/*.md` is
ignored. The full job becoming newly applicable is intentional. The existing
required-check warning remains accurate for the ignore patterns that remain;
G2 records that risk rather than introducing the companion-job design now.

## Question 2 - truthful coverage-oracle mapping

### Option 2A - map the target to an unrelated `src/` surface

* Pro: one small manifest edit satisfies the completeness algorithm
* Con: it falsely claims product runtime source owns a repository harness
  contract
* Con: it causes unrelated runtime edits to select G2 and can miss harness-only
  changes
* Con: explicitly violates the requirement not to invent a `src/` mapping

### Option 2B - map only the G1 crate and rely on test self-coverage

* Pro: `crates/agent-contract-assert/` is a real implementation dependency and
  makes completeness pass
* Pro: the declared test file selects itself when directly changed
* Con: changes to G0 can alter discovery or reads without selecting the G2
  integration target
* Con: changes to harness Markdown or seed data still select nothing unless
  additional non-source surfaces are declared

### Option 2C - map both dependency crates and every live data surface

* Pro: extending the G0 and G1 crate surfaces with the G2 target truthfully
  covers both implementation dependencies and satisfies completeness
* Pro: six data mappings - the five harness roots plus the canonical seed file -
  select the target for every live input change
* Pro: no production `src/` ownership is fabricated
* Con: eight manifest mappings must remain aligned: two crate mappings and six
  repository-data mappings
* Con: G2 cannot finalize the crate entries until G0 and G1 have merged their
  surface records

### Selected option

**Select 2C.** The G2 target is added to both
`crates/agent-contract-fs/` and `crates/agent-contract-assert/` surface target
lists. Separate exact-prefix surfaces map `.github/agents/`, `.github/skills/`,
`.github/instructions/`, `.github/policies/`, `.github/prompts/`, and
`.github/contracts/agent-harness.toml` to the same target. Completeness uses the
crate mappings; change selection uses all eight. This describes the actual
input graph.

## Question 3 - workspace lint coverage

### Option 3A - add `--workspace` to the `lint` alias only

* Pro: closes the gap in the canonical local quality gate, `cargo lint`
* Pro: respects the instruction that `ci.yml` changes are limited to Decision
  5a's trigger correction
* Pro: one alias covers current and future workspace members, including G0/G1
* Con: the existing CI clippy command remains root-package scoped
* Con: first execution may reveal pre-existing pedantic findings in
  `powerbi-tmdl-parser` or `engram-indexer`

### Option 3B - add `--workspace` to the CI clippy step only

* Pro: makes remote lint coverage authoritative
* Con: leaves `cargo lint` unable to reproduce CI locally
* Con: modifies `ci.yml` beyond Decision 5a and is therefore outside G2's
  permitted workflow edit

### Option 3C - add `--workspace` to both local and CI commands

* Pro: exact local/CI parity and complete lint coverage
* Con: modifies the CI clippy step beyond the authorized trigger change
* Con: widens the first-run blast radius to all existing and new members in both
  environments at once

### Selected option

**Select 3A.** Change the alias to
`clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic`.
Do not edit the CI clippy command. Before landing the alias, run that exact
expanded command once after G0 and G1 merge. If existing member crates fail,
stop and create a bounded prerequisite rather than adding suppressions or
folding unrelated fixes into G2. The residual CI/local asymmetry is explicit;
closing it later requires separate authority to edit the CI command.

## Question 4 - fixture-extension contract

### Option 4A - one canonical data-only registry file

Packages append typed entries to
`.github/contracts/agent-harness.toml`. The G2 target embeds that file and hands
its contents to G1's public loader. No Rust registration table exists.

* Pro: Packages B and C extend contracts by data edit only
* Pro: one schema authority - G1 - and one repository registry
* Pro: the existing `.github` trigger correction and an exact oracle surface
  make fixture changes observable
* Con: Packages B and C can contend on one small file
* Con: G1 must expose a stable data loader and serialization contract

### Option 4B - one fragment file per package under a directory

* Pro: isolates package ownership and reduces merge contention
* Con: requires directory discovery, ordering, path handling, and file reads in
  G2 or a new G0 capability
* Con: that logic duplicates or expands the G0 boundary and is not authorized
  by the split decision

### Option 4C - construct entries in Rust

* Pro: compile-time typing and no serialized fixture parser at the G2 layer
* Con: every Package B/C entry requires editing G2's test code
* Con: violates the required extension contract and creates a central code
  registry

### Selected option

**Select 4A.** The canonical file contains only G1 schema data. G2 embeds it at
compile time and passes it unchanged to G1. G2 has no parser, include graph,
filesystem walker, or registration switch. Package B owns IDs prefixed
`package-b.` and Package C owns IDs prefixed `package-c.`; G2 seed IDs use
`g2.seed.`. IDs are globally unique. Every entry names exactly one G0 root from
`agents`, `skills`, `instructions`, `policies`, or `prompts`, a root-relative
G1 path selector, one G1 pattern, one typed G1 assertion form, and one typed G1
scope. Extensions may append entries but may not change schema version, seed
IDs, root semantics, evaluator ordering, or diagnostics.

The seed set is two entries only: one contract for the Stage role-boundary
heading and one for the Ship role-boundary heading. Both target a single known
agent file and require exactly one literal
`## Role Boundary (NON-NEGOTIABLE)` heading. G1, not G2, defines how the typed
form and scope serialize. The G2 green unit writes the fields using G1's merged
canonical spelling. A G1 merge without a public data loader or stable
serialization leaves this draft blocked rather than authorizing a local parser.

## Empty changed-set decision

G2 will distinguish three states explicitly:

| State | Required result |
|---|---|
| Resolved, genuinely empty changed-file set | non-zero exit, `STATUS=FAIL`, `REASON=empty-diff` |
| Non-empty harness change but G2 target absent from `--selected` | non-zero exit, `STATUS=FAIL`, omitted list names `contract_agent_harness_contracts` |
| Base or merge base cannot be resolved | existing non-zero exit 3 and existing indeterminate reason |

Both oracle implementations receive the same explicit-empty behavior. An
explicit `--changed ""` is treated as a provided empty set, not as permission to
fall back to repository discovery. A test fixture with a valid base and no
staged or branch changes proves the genuine Git-empty path. No PASS state is
accepted merely because required and omitted are both zero.

## Dependency on G0 and G1

G2 consumes prerequisite APIs only. Names below are the required public
capabilities; the final symbol spelling is frozen by the reviewed G0/G1 merges.
If a symbol or signature differs, this draft must be reconciled before execution.

### G0 public API consumed

* the public `FileAccess` boundary with `symlink_metadata`, `canonicalize`,
  `read_dir`, and `read_to_string`
* `RealFs`, used as the sole real-filesystem implementation
* the public resolver/read entry point that accepts the workspace root and the
  five allow-listed harness roots
* resolved-file and root identity types returned by that entry point
* the public filesystem/discovery error type, including root-symlink,
  containment, enumeration, and read failures

These capabilities are fixed by
`docs/decisions/2026-09-13-package-g-split-deliberation.md:227-264`. G2 supplies
the workspace root and typed root selection only. It does not join,
canonicalize, enumerate, follow or reject symlinks, normalize paths, or read
corpus files itself.

### G1 public API consumed

* the typed registry and entry types
* the public data loader for G1's canonical serialized registry form
* typed assertion-form and assertion-scope enums
* the public evaluator/runner that accepts G0's resolved corpus plus a loaded
  registry
* typed diagnostics and the public deterministic renderer or report type

G1 owns validation, duplicate detection, selection, matching, assertion
semantics, precedence, ordering, and rendering. G2 checks only PASS versus
non-PASS and preserves G1 diagnostics. It never duplicates evaluator logic.

## Inherited, not re-argued

The following is Decision 6 of the split deliberation, reproduced as the
binding inherited constraint:

1. **No `max_depth` / `max_filesize` / walker filters.** Affirmed 7/7; the
   Security Reviewer explicitly declined to re-request bounds. Evidence S3/S4
   re-confirms both caps would have been inert on every real input. Do not
   reopen.
2. **Plain `std::fs` recursion, not `ignore::WalkBuilder`.** Filter-level silent
   omission must remain structurally unexpressible.
3. **Zero new external dependencies** for G0.
4. **No `verify_markdown`, no `engram` runtime coupling, no product-runtime
   behaviour** in any of G0/G1/G2.
5. **Typed registry**, not stringly-typed assertions.
6. **Test-first with compiling red phases**; ≤3 scenarios per unit.
7. **Two-layer containment** (allow-listed roots + workspace canonical
   containment).

## Accepted decisions

1. Remove `.github/**/*.md` from both CI trigger blocks and update only the
   directly affected comments.
2. Map the G2 target to both prerequisite crate surfaces and all six live data
   surfaces; do not invent a `src/` mapping.
3. Add `--workspace` to the local `lint` alias only; gate it with a bounded
   post-G1 verification and do not suppress new findings.
4. Use one canonical, data-only registry file with namespaced IDs and G1-owned
   schema semantics.
5. Fail closed and distinguish empty, omitted-target, and indeterminate diff
   states.
6. Keep G2 draft-dependent until both G0 and G1 pass review and merge.


