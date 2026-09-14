---
type: exec-plan
date: 2026-09-13
package: G2
depends_on: G1
source: docs/decisions/2026-09-13-package-g2-harness-registry-ci-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
requires_plan_hardening: yes
plan_status: draft-dependent
review_verdict: not-reviewed
review_blocked_by: "Prerequisite chain broken: G0 FAILED plan-review at attempt 1 (docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md), so G1 was also not reviewed. Per the program decision gate policy item 2, G2 is not reviewed and not harvested while its prerequisite authority is absent. This draft is PRESERVED unmodified as a dependent draft."
harvested: false
prerequisite: G0 and G1 must both PASS plan-review and merge before this plan is executable
---

# Package G2 - harness registry and CI/local integration plan

## Source Document

`docs/decisions/2026-09-13-package-g2-harness-registry-ci-deliberation.md`

The controlling program authority is
`docs/decisions/2026-09-13-package-g-split-deliberation.md`, as linked by the
G2 deliberation. This plan applies its Decisions 2, 5, and 6 without reopening
them.

> [!IMPORTANT]
> G2 is the third release unit in `G0 -> G1 -> G2`. Neither prerequisite is
> merged as of this plan. This plan is `draft-dependent` and MUST NOT execute
> until both G0 and G1 pass plan review and merge. After those merges, reconcile
> the precise public symbol names in the dependency gate below before the first
> red unit; a behavior or ownership mismatch blocks execution.

## Objective

Bind G0's real filesystem boundary and G1's typed registry/evaluator to the
repository's five harness roots, load a minimal repository registry that
Packages B and C can extend without Rust edits, and expose the result as one
root-package contract target exercised by existing local and CI test commands.

Completion means:

* `contract_agent_harness_contracts` is an explicit root `[[test]]` target at
  `tests/contract/agent_harness_contracts_test.rs`
* the target resolves and reads the live harness corpus only through G0, then
  loads and evaluates contracts only through G1
* Packages B and C append namespaced entries to one data fixture without editing
  G2 Rust code
* Cargo aliases and the existing CI test command select the target through
  `--all-targets`
* `.github/**/*.md` is removed from both `paths-ignore` blocks
* coverage-oracle completeness and change selection truthfully map both crates,
  all five roots, and the seed fixture
* empty, omitted-target, and indeterminate changed-file states cannot collapse
  into the same PASS
* `cargo lint` covers every workspace member through `--workspace`

No application source, product runtime, new external dependency, new workflow,
new CI tool, evaluator logic, or filesystem logic belongs to G2.

## Verified Preconditions

The following evidence was personally rechecked with read-only PowerShell
commands before drafting.

| Precondition | Verified result | Evidence |
|---|---|---|
| Split and topology | T3 is selected: G0 and G1 are dev-only crates and G2 is a root-package exerciser | `docs/decisions/2026-09-13-package-g-split-deliberation.md:151-221` |
| G2 ownership | Trigger correctness, empty changed-set behavior, lint gap, and extension contract belong to G2 | `docs/decisions/2026-09-13-package-g-split-deliberation.md:283-299` |
| Corpus | 92 files, all `.md`; agents 23, skills 28, instructions 34, policies 2, prompts 5; max root-relative component depth 2; max file size 98,546 bytes | Recursive enumeration of `.github/{agents,skills,instructions,policies,prompts}`; largest file `.github/agents/_ship.agent.md` |
| Warning floor | `[build] rustflags = ["-Dwarnings"]`; dead code is a hard error | `.cargo/config.toml:1-2` |
| Local selection | `dev-test` and `ci` both use `test --all-targets`; current `lint` lacks `--workspace` | `.cargo/config.toml:20,26-27` |
| Root target convention | Root tests use explicit adjacent `name` and `path`; the oracle harness is registered this way | `Cargo.toml:139-156,1073-1081` |
| Existing CI test | `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` | `.github/workflows/ci.yml:84-91` |
| Trigger defect | `.github/**/*.md` occurs in both push and pull-request ignore lists | `.github/workflows/ci.yml:30-43` |
| Required-check warning | The workflow already documents why ignored-only PRs become stuck if `build` becomes required | `.github/workflows/ci.yml:20-25` |
| Oracle fail-closed base | Missing base ref or merge base exits non-zero as indeterminate | `scripts/test-coverage-oracle.ps1:166-199`; `scripts/test-coverage-oracle.sh:147-175` |
| Oracle empty gap | A resolved empty set can reach `select` or `report` with zero required/omitted and PASS | `scripts/test-coverage-oracle.ps1:235-271`; `scripts/test-coverage-oracle.sh:213-247` |
| Completeness scope | Only `src/` and `crates/` mappings cover declared targets for completeness | `scripts/test-coverage-oracle.ps1:203-208`; `scripts/test-coverage-oracle.sh:178-187` |
| Existing manifest | It includes `src/` surfaces and `crates/powerbi-tmdl-parser/`, but G0/G1 do not exist yet | `.cargo/test-coverage-manifest.toml:29-32,81-139` |
| Oracle test seam | `run_oracle` already invokes the platform-specific script and extracts structured fields | `tests/unit/dev_test_coverage_oracle_test.rs:23-64` |
| Dependency availability | `serde`, `tempfile`, `toml`, and `globset` already exist in the root dependency graph | `Cargo.toml:36,59-60,76,123` |
| Review inheritance | M-1 through M-5 and A-1 through A-10 are recorded for inheritance | docs/closure/2026-09-13-package-g-v3-plan-review-record.md:149-176 |
| Program artifact availability | The required program path is not present at this HEAD; the frontmatter reference is retained for the prerequisite program artifact | Read-only Test-Path docs/decisions/2026-09-13-package-g-split-program-decision.md returned false |

The corpus counts are planning evidence, not fixed acceptance ceilings. G2 tests
root identity, extension, and evaluation outcomes rather than freezing the
current file count or size.

## Architecture

### Dependency direction

```text
.github harness Markdown --+
                           +-> G0 resolve/read API -> G1 registry/evaluator API
workspace root ------------+                              |
                                                          v
.github/contracts/agent-harness.toml -- G1 loader --> root [[test]] result

Cargo.toml + .cargo/config.toml + coverage manifest + ci.yml
                          +-> registration and invocation guarantees
```

The root target is orchestration only:

1. Obtain the workspace root from `CARGO_MANIFEST_DIR`.
2. Ask G0's merged public entry point to resolve and read exactly the five
   allow-listed roots through `RealFs`.
3. Embed `.github/contracts/agent-harness.toml` with `include_str!`; this is a
   compile-time data fixture, not corpus path discovery or runtime file I/O.
4. Pass the fixture bytes unchanged to G1's public loader.
5. Pass G0's resolved corpus and G1's loaded registry to G1's public evaluator.
6. Fail the test on a G0 error, G1 load error, or any G1 non-PASS report, while
   preserving the typed/rendered diagnostic.

G2 MUST NOT join root paths, enumerate directories, canonicalize, inspect
symlinks, normalize corpus paths, read corpus files, parse registry fields,
match patterns, evaluate forms/scopes, order diagnostics, or render diagnostic
content. Those operations remain in G0 or G1.

### Prerequisite API gate

Before G2.1, compare the merged crates against this capability list:

| Crate | Public capability G2 consumes | Blocking mismatch |
|---|---|---|
| `agent-contract-fs` | `FileAccess` methods `symlink_metadata`, `canonicalize`, `read_dir`, `read_to_string` | Any method bypassed or unavailable through the public resolver |
| `agent-contract-fs` | `RealFs` | G2 would need direct `std::fs` corpus access |
| `agent-contract-fs` | public resolve/read entry point for the five typed roots | G2 would need path, discovery, or read logic |
| `agent-contract-fs` | resolved corpus/root identity and typed errors | G2 could not pass data or preserve failures without adaptation logic |
| `agent-contract-assert` | typed registry/entry, form, and scope types | G2 would become stringly typed |
| `agent-contract-assert` | canonical data loader from `&str` or bytes | Packages B/C could not extend a data fixture without code edits |
| `agent-contract-assert` | evaluator/runner over G0 corpus plus loaded registry | G2 would duplicate evaluator logic |
| `agent-contract-assert` | typed diagnostic and deterministic report/renderer | G2 would invent failure semantics |

Exact symbol spelling is inherited from the merged APIs. Capability changes are
not repaired with adapters in G2. A behavioral mismatch blocks and returns the
plan for reconciliation.

### Dependencies

G2 adds no external dependency and no version change. The root already consumes
G0 and G1 as path dev-dependencies after their prerequisite merges. The target
uses the standard library plus those public crates. The oracle unit may reuse
root-declared `tempfile` to create an isolated Git repository under
`target/`, keeping all writes inside the workspace, for the
indeterminate/empty-state proof. It does not add a dependency or a production
consumer.

`#![forbid(unsafe_code)]` and `#![warn(clippy::pedantic)]` remain explicit on the
new test target. No `allow(dead_code)` is permitted. There is no bootstrap
placeholder: every red `todo!()` is introduced in a named `[R]` unit and removed
by its immediately following `[G]` owner.

## Registration and Fixture-Extension Contract

This section is the contract Packages B and C consume.

### Target registration

The root `Cargo.toml` receives exactly:

```toml
[[test]]
name = "contract_agent_harness_contracts"
path = "tests/contract/agent_harness_contracts_test.rs"
```

The name is the coverage-oracle identity. The path is the sole G2 Rust
exerciser. Neither Package B nor Package C adds another harness runner.

### Canonical registry file

The sole repository registry is:

```text
.github/contracts/agent-harness.toml
```

Its serialized keys and enum spellings are exactly G1's merged canonical schema;
G2 defines no aliases, defaults, coercions, or secondary parser. The logical
fields required for every entry are:

| Field | Contract |
|---|---|
| schema version | one file-level value equal to the G1-supported version |
| `id` | globally unique, stable, lowercase package namespace plus descriptive suffix |
| root | exactly one typed G0 root: `agents`, `skills`, `instructions`, `policies`, or `prompts` |
| path selector | root-relative selector interpreted by G1; never absolute and never `..` |
| pattern | one G1 pattern value; G2 does not reinterpret it |
| form | one typed G1 assertion form |
| scope | one typed G1 assertion scope |

### Seed entries

The initial fixture has two entries and no speculative package contracts:

| ID | Root and selector | Pattern | Form/scope intent |
|---|---|---|---|
| `g2.seed.stage-role-boundary` | `agents`, `_stage.agent.md` | literal heading `## Role Boundary (NON-NEGOTIABLE)` | exactly one occurrence in the selected set |
| `g2.seed.ship-role-boundary` | `agents`, `_ship.agent.md` | literal heading `## Role Boundary (NON-NEGOTIABLE)` | exactly one occurrence in the selected set |

The implementation spells form and scope using G1's merged enum serialization.
If G1 cannot express the stated literal/selector semantics, the prerequisite API
gate blocks; G2 does not add an evaluator mode.

### Package extension rules

* Package B appends entries with IDs under `package-b.`
* Package C appends entries with IDs under `package-c.`
* extensions edit only the canonical TOML file for registration; no edit to
  `agent_harness_contracts_test.rs`, Cargo target declarations, G0, or G1
* entries use existing G1 fields and variants only; a schema change is separate
  G1 work
* seed IDs and meanings are immutable to extension packages
* duplicate IDs, malformed data, disallowed roots, invalid selectors, or
  unknown typed values fail through G1
* evaluation and diagnostic order remain G1-defined and deterministic
* an extension is accepted only after the direct G2 target, oracle select for
  the fixture path, `cargo dev-test`, and `cargo lint` pass

The fixture is embedded as one compile-time input. No fragment discovery or
include order exists. This keeps extension data-only and prevents G2 from
recreating G0 path logic.

## CI Integration

### Exact trigger edit proposed, not applied by this plan

The current push fragment is:

```yaml
  push:
    branches:
      - main
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '.autoharness/**'
      - '*.md'                # root README/CHANGELOG/AGENTS.md
      - '.github/**/*.md'     # instructions/prompts/agents
      - 'scripts/**/*.md'     # scripts/metrics/README.md
```

It becomes:

```yaml
  push:
    branches:
      - main
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '.autoharness/**'
      - '*.md'                # root README/CHANGELOG/AGENTS.md
      - 'scripts/**/*.md'     # scripts/metrics/README.md
```

The current pull-request fragment is:

```yaml
  pull_request:
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '.autoharness/**'
      - '*.md'                # root README/CHANGELOG/AGENTS.md
      - '.github/**/*.md'     # instructions/prompts/agents
      - 'scripts/**/*.md'     # scripts/metrics/README.md
```

It becomes:

```yaml
  pull_request:
    paths-ignore:
      - '.backlogit/**'
      - 'docs/**'
      - '.autoharness/**'
      - '*.md'                # root README/CHANGELOG/AGENTS.md
      - 'scripts/**/*.md'     # scripts/metrics/README.md
```

The adjacent comment currently says `.github/**/*.md` is scoped ignored
Markdown. Replace only that claim with text explaining that `.github` Markdown
is executable harness-contract input and therefore re-arms the full job. Retain
the required-check contingency: ignored-only PRs still need a companion
same-name check if `build` becomes required.

No job, action, workflow, conditional, or CI command changes. In particular, the
CI test line remains:

```yaml
run: cargo test --no-default-features --features cozo-backend,embeddings --all-targets
```

Because the exerciser is a root-package explicit `[[test]]`, Cargo's existing
`--all-targets` selects it.

### Local lint edit

The current alias is:

```toml
lint = "clippy --all-targets --all-features -- -D warnings -D clippy::pedantic"
```

It becomes:

```toml
lint = "clippy --workspace --all-targets --all-features -- -D warnings -D clippy::pedantic"
```

The CI clippy line is deliberately unchanged because workflow edits are limited
to Decision 5a. The canonical local `cargo lint` gate now checks the root,
`powerbi-tmdl-parser`, `engram-indexer`, G0, and G1.

## Coverage-Oracle Registration

The manifest must express both code ownership and live-data selection.

1. Extend the merged `crates/agent-contract-fs/` surface with
   `contract_agent_harness_contracts`.
2. Extend the merged `crates/agent-contract-assert/` surface with
   `contract_agent_harness_contracts`.
3. Add exact surfaces for the five harness roots and the canonical registry
   fixture, each selecting only `contract_agent_harness_contracts`.

Normative shape, merged with the G0/G1 target lists rather than duplicating a
surface path:

```toml
[[surface]]
path = "crates/agent-contract-fs/"
targets = ["contract_agent_contract_fs", "contract_agent_harness_contracts"]

[[surface]]
path = "crates/agent-contract-assert/"
targets = ["contract_agent_contract_assert", "contract_agent_harness_contracts"]

[[surface]]
path = ".github/agents/"
targets = ["contract_agent_harness_contracts"]

[[surface]]
path = ".github/skills/"
targets = ["contract_agent_harness_contracts"]

[[surface]]
path = ".github/instructions/"
targets = ["contract_agent_harness_contracts"]

[[surface]]
path = ".github/policies/"
targets = ["contract_agent_harness_contracts"]

[[surface]]
path = ".github/prompts/"
targets = ["contract_agent_harness_contracts"]

[[surface]]
path = ".github/contracts/agent-harness.toml"
targets = ["contract_agent_harness_contracts"]
```

The two crate mappings make completeness truthful because the target directly
consumes both crates. The six non-source mappings make changed-input selection
truthful. They do not count toward completeness under the current algorithm,
which is why a data-only mapping alone is insufficient. No `src/` surface is
added.

The exact prerequisite target names shown above must be reconciled to the merged
G0/G1 `[[test]]` names. The G2 target name is fixed.

## Failure Semantics

| Failure mode | When it fires | Required observable result | Proving unit |
|---|---|---|---|
| Target unregistered | Cargo lacks the explicit name/path block | exact-target child invocation exits non-zero | G2.1/G2.2 |
| Target present but not invoked | sentinel test is not run by direct target command | invocation proof does not observe the sentinel success | G2.1/G2.2 |
| Local alias drift | `dev-test` or `ci` no longer includes `--all-targets` | registration contract test fails | G2.1/G2.2 |
| CI invocation drift | CI test command no longer contains `--all-targets` | registration contract test fails | G2.1/G2.2 |
| Registry malformed or duplicated | G1 rejects the canonical seed/extension data | target fails with G1's one typed load diagnostic | G2.3/G2.4 |
| G0 corpus failure | root symlink, containment, enumeration, or read fails | target fails with the preserved G0/G1 diagnostic; no partial evaluation | G2.3/G2.4 |
| Contract violation | a loaded assertion is non-PASS | target fails with G1's single diagnostic outcome for that assertion | G2.3/G2.4 |
| Package extension requires code registration | a synthetic namespaced entry is appended in data only | extension test fails if G2 Rust must be edited or a registration table is consulted | G2.3/G2.4 |
| Completeness mapping missing | declared G2 target matches no `src/` or `crates/` surface | `--mode completeness` exits non-zero and names the unmapped target | G2.5/G2.6 |
| Harness root change unselected | one of the five corpus roots changes | `--mode select` lacks the G2 target and the test fails | G2.5/G2.6 |
| Registry fixture change unselected | canonical TOML changes | `--mode select` lacks the G2 target and the test fails | G2.5/G2.6 |
| Genuinely empty diff | base resolves and staged/branch/untracked set is empty, or explicit changed input is present but empty | non-zero, `STATUS=FAIL`, `REASON=empty-diff` | G2.7/G2.8 |
| Contract target omitted | non-empty harness change is reported with a selected set excluding G2 | non-zero, `STATUS=FAIL`, omitted list contains `contract_agent_harness_contracts` | G2.7/G2.8 |
| Indeterminate diff | base ref or merge base cannot resolve | existing exit 3 and specific indeterminate reason | G2.7/G2.8 |
| Push harness Markdown ignored | push block still ignores `.github/**/*.md` | push trigger contract fails | G2.9/G2.10 |
| PR harness Markdown ignored | pull-request block still ignores `.github/**/*.md` | PR trigger contract fails | G2.9/G2.10 |
| Workspace member omitted from lint | alias lacks `--workspace` | lint configuration contract fails | G2.9/G2.10 |

Empty diff and indeterminate diff are mutually exclusive states. A non-empty
diff whose required target is absent is an omission, not an empty diff. Each
test scenario asserts one of these outcomes only.

## Work Units

`[R]` means a compiling red harness with failing `todo!()` test bodies. `[G]`
means the immediately paired implementation that removes every `todo!()` from
its red owner. There are **5 `[R]`/`[G]` pairs, 10 work units total**. Each unit
has at most three scenarios. No red success criterion names a G2 API introduced
by a later pair; G0 and G1 are merged prerequisites, not later G2 units.

All units touch at most three files except G2.2. **G2.2 is an explicit four-file
registration exception**: Cargo target registration, its target file, the
existing meta-test red body, and the minimum crate-surface coverage mapping must
land atomically. Omitting the manifest edit would make the existing completeness
test fail immediately after the new target is declared. This is the precise
registration exception anticipated by the task-granularity rule; it is not a
universal file-count waiver.

### G2.1 `[R]` - Target registration and invocation harness

* **Files owned**: `tests/unit/dev_test_coverage_oracle_test.rs`
* **Description**: Add three named tests with compiling `todo!()` bodies. They
  specify direct execution of the future target sentinel, the exact root Cargo
  name/path registration, and continued `--all-targets` selection by both local
  aliases and the CI test command.
* **Scenarios**: 3

Success criteria:

1. The unit target compiles and the three new tests fail only at their named
   `todo!()` bodies.
2. Scenario 1 defines one outcome: invoking
   `cargo test --test contract_agent_harness_contracts registration_sentinel -- --exact`
   must execute one passing sentinel.
3. Scenarios 2 and 3 independently define the Cargo registration/local-alias
   contract and the existing CI invocation contract.

### G2.2 `[G]` - Register and execute the root target

* **Files owned**: `Cargo.toml`,
  `tests/contract/agent_harness_contracts_test.rs`,
  `tests/unit/dev_test_coverage_oracle_test.rs`,
  `.cargo/test-coverage-manifest.toml`
* **Description**: Add the explicit target block and target root with crate lint
  attributes plus `registration_sentinel`. Replace G2.1 todos with executable
  assertions. Add the G2 target to the merged G1 crate surface as the minimum
  truthful completeness mapping; G2.6 adds the rest of the input graph.
* **Scenarios**: the 3 scenarios owned by G2.1

Success criteria:

1. The exact-target child command runs `registration_sentinel` and exits zero,
   proving actual registration and invocation rather than file presence.
2. Cargo contains exactly one matching target with the required name/path, and
   `dev-test` plus `ci` retain `--all-targets`.
3. The CI test command remains exactly the existing feature set plus
   `--all-targets`; no workflow edit occurs in this unit.
4. `--mode completeness` remains green at the end of the unit because the new
   target is already mapped to the real G1 crate surface.

### G2.3 `[R]` - Seed registry, live binding, and extension harness

* **Files owned**: `.github/contracts/agent-harness.toml`,
  `tests/contract/agent_harness_contracts_test.rs`
* **Description**: Create the two-entry seed fixture using G1's merged canonical
  serialization. Add three compiling `todo!()` tests for load, live execution,
  and data-only extension.
* **Scenarios**: 3

Success criteria:

1. The seed fixture contains only the two documented `g2.seed.*` entries using
   G1's merged canonical serialized shape; loader acceptance is proved in G2.4.
2. The target compiles against G0 and G1 public APIs and fails only at the three
   owned `todo!()` bodies.
3. No new module, parser, path helper, filesystem helper, or dependency is
   introduced.

### G2.4 `[G]` - Bind G0 and G1 to the repository corpus

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs`
* **Description**: Replace all G2.3 todos. Embed the seed fixture, resolve/read
  the five roots through `RealFs` and G0's public runner, and load/evaluate
  through G1. Exercise an appended synthetic namespaced entry through the same
  loader to prove data-only extension.
* **Scenarios**: the 3 scenarios owned by G2.3

Success criteria:

1. Seed loading succeeds and returns exactly the two stable G2 IDs; malformed or
   duplicate behavior remains G1-owned.
2. Live evaluation visits the five typed roots and reports PASS with no
   diagnostic; the test contains no direct `std::fs`, walker, canonicalization,
   symlink, normalization, parser, matcher, or renderer logic.
3. Appending one synthetic `package-b.*` or `package-c.*` entry to fixture data
   is accepted and evaluated without changing a Rust registration table,
   proving the extension mechanism rather than future package content.

### G2.5 `[R]` - Coverage mapping harness

* **Files owned**: `tests/unit/dev_test_coverage_oracle_test.rs`
* **Description**: Add three compiling `todo!()` scenarios for completeness,
  a representative harness-root change, and the canonical fixture change.
* **Scenarios**: 3

Success criteria:

1. The completeness scenario requires zero unmapped targets and explicitly
   confirms the G2 target is in the mapped target set.
2. A change under `.github/agents/` has one asserted outcome: select includes
   `contract_agent_harness_contracts`.
3. A change to `.github/contracts/agent-harness.toml` has the same single
   selection outcome in its own test.

### G2.6 `[G]` - Declare the truthful input graph

* **Files owned**: `.cargo/test-coverage-manifest.toml`,
  `tests/unit/dev_test_coverage_oracle_test.rs`
* **Description**: Replace G2.5 todos. Extend both merged crate surface target
  lists and add exact mappings for five corpus roots plus the registry fixture.
* **Scenarios**: the 3 scenarios owned by G2.5

Success criteria:

1. `--mode completeness` reports no unmapped target or module with no invented
   `src/` mapping.
2. Changes under each of the five harness roots select the G2 target; the named
   representative scenario is backed by a table over the five paths without
   combining different diagnostic outcomes.
3. A registry fixture change selects the G2 target, while an unrelated
   `docs/` path does not acquire a false source mapping.

### G2.7 `[R]` - Empty, omitted, and indeterminate diff harness

* **Files owned**: `tests/unit/dev_test_coverage_oracle_test.rs`
* **Description**: Add three compiling `todo!()` scenarios. Reuse its
  cross-platform script runner and root-declared `tempfile` with `TempDir::new_in`
  under repository `target/` for isolated Git states.
* **Scenarios**: 3

Success criteria:

1. A table-driven empty-set scenario covers both a repository with a valid base
   and no staged, branch, untracked, or unstaged changes and explicit
   `--changed ""`; both inputs assert only `FAIL/empty-diff`.
2. A non-empty harness change with a selected set that omits G2 expects only an
   omission failure naming `contract_agent_harness_contracts`.
3. An unresolved base expects only the existing indeterminate failure and exit
   code 3.

### G2.8 `[G]` - Fail closed on an empty changed set

* **Files owned**: `scripts/test-coverage-oracle.ps1`,
  `scripts/test-coverage-oracle.sh`,
  `tests/unit/dev_test_coverage_oracle_test.rs`
* **Description**: Replace G2.7 todos and implement parity. Track whether
  `--changed` was supplied separately from its string value. After either
  explicit input or successful Git discovery, reject a zero-length changed set
  before `select`, `report`, or `run` can emit PASS. Leave completeness
  independent of a diff. Preserve existing indeterminate reasons and exit 3.
* **Scenarios**: the 3 scenarios owned by G2.7

Success criteria:

1. Both scripts return non-zero, `STATUS=FAIL`, and `REASON=empty-diff` for a
   resolved empty set; neither prints a PASS line.
2. The non-empty omitted-target case still reaches ordinary resolution and
   names the G2 target as omitted, proving it was not misclassified as empty.
3. Missing base and merge-base states keep their current specific reason and
   exit 3, proving indeterminate was not collapsed into empty.

### G2.9 `[R]` - CI trigger and workspace lint harness

* **Files owned**: `tests/contract/agent_harness_contracts_test.rs`
* **Description**: Add three compiling `todo!()` scenarios: push trigger,
  pull-request trigger, and canonical lint alias.
* **Scenarios**: 3

Success criteria:

1. The push scenario expects `.github/**/*.md` to be absent from only the push
   `paths-ignore` list.
2. The pull-request scenario expects the same single outcome for its own block.
3. The lint scenario expects the alias token sequence to contain `--workspace`,
   `--all-targets`, `--all-features`, `-D warnings`, and
   `-D clippy::pedantic`.

### G2.10 `[G]` - Re-arm harness CI and close the local lint gap

* **Files owned**: `.github/workflows/ci.yml`, `.cargo/config.toml`,
  `tests/contract/agent_harness_contracts_test.rs`
* **Description**: First run the expanded workspace clippy command as a bounded
  preflight. If it fails in a pre-existing member, stop and create a separate
  prerequisite; do not suppress or absorb unrelated fixes. If green, replace
  G2.9 todos, remove the two ignore lines, correct the adjacent comment, and add
  `--workspace` to the local lint alias. Make no other workflow change.
* **Scenarios**: the 3 scenarios owned by G2.9

Success criteria:

1. Push and pull-request blocks no longer ignore `.github/**/*.md`; all other
   ignore entries are byte-for-byte unchanged.
2. The comment identifies `.github` harness Markdown as executable contract
   input and retains the required-check contingency.
3. `cargo lint` expands to the selected workspace command and passes across all
   members; there is no `allow(dead_code)` or other new suppression.
4. The CI clippy/test commands and every other job remain unchanged.

## Verification

Run in this order after G2.10. These are implementation-time commands; none were
run while drafting this plan.

### Targeted red/green checks

```powershell
cargo test --test unit_dev_test_coverage_oracle
cargo test --test contract_agent_harness_contracts
cargo test --test contract_agent_harness_contracts registration_sentinel -- --exact
```

### Coverage-oracle checks on Windows

```powershell
pwsh scripts/test-coverage-oracle.ps1 --mode completeness
pwsh scripts/test-coverage-oracle.ps1 --mode select --changed .github/agents/_ship.agent.md
pwsh scripts/test-coverage-oracle.ps1 --mode select --changed .github/contracts/agent-harness.toml
```

The two select commands must include
`TARGET=contract_agent_harness_contracts`. Run the Bash mirror on Linux/macOS:

```bash
bash scripts/test-coverage-oracle.sh --mode completeness
bash scripts/test-coverage-oracle.sh --mode select --changed .github/agents/_ship.agent.md
bash scripts/test-coverage-oracle.sh --mode select --changed .github/contracts/agent-harness.toml
```

### Existing local and CI-equivalent gates

```powershell
cargo fmt-check
cargo lint
cargo dev-test
cargo ci
cargo test --no-default-features --features cozo-backend,embeddings --all-targets
cargo audit
```

`cargo lint` must visibly expand with `--workspace`. `cargo dev-test`, `cargo ci`,
and the CI-equivalent command must run the
`contract_agent_harness_contracts` binary. The final diff check must confirm
that `ci.yml` changed only the two ignore lines and their directly affected
comment text.

## Risks

| Risk | Impact | Mitigation or stop condition |
|---|---|---|
| Required split program decision is absent | Program sequencing cannot be verified against its declared artifact | keep the required frontmatter path; block execution until the program artifact exists and agrees with G0 -> G1 -> G2 |
| G0 public API shifts before merge | G2 could be forced to duplicate path/read behavior | prerequisite API gate; block rather than adapt locally |
| G1 chooses Rust-only registry construction | data-only Package B/C extension becomes impossible | require merged public data loader; block and reconcile G1 if absent |
| Current member crates fail workspace pedantic lint | G2.10 cannot close the alias gap without scope creep | bounded preflight; separate prerequisite, no suppressions |
| Removing the broad ignore increases CI load | all `.github` Markdown runs the full job | accepted cost; measure after merge; revert only through a new decision that preserves harness execution |
| `build` later becomes a required check | still-ignored doc-only PRs can wait for a missing check | retain existing contingency and create companion-job work only when status policy changes |
| Canonical fixture becomes a merge hotspot | B/C edits can conflict | stable namespaces, append-only entries, small file, merge commits preserve history |
| Corpus grows beyond current measurements | snapshot assertions become brittle | counts are evidence only; tests enforce roots, extension, and diagnostics, not maxima |
| Oracle implementations drift | Windows and Linux disagree on empty behavior | same three scenarios run through the platform helper; verify both scripts explicitly |
| Data surface omitted when a new harness root appears | changes can evade target selection | extension contract requires manifest mapping review for any new root; current five are exact |

## Plan Hardening

### H1 - Newly gated `.github` documentation

Removing `.github/**/*.md` means all `.github` Markdown PRs now run the full
fmt, workspace-local lint when run by developers, root CI clippy, test, and audit
sequence. This can expose unrelated failures and consume more CI time. That is
an accepted consequence for the current broad pattern because the five harness
roots are executable contract inputs and GitHub trigger filters cannot express
G1 semantics. G2 does not add an always-pass job. The existing warning remains:
if `build` becomes a required status check, ignored-only PRs can wait forever and
must receive a separately designed companion check. Rollback is restoring the
two lines, but that also restores the known false-negative and therefore
requires a replacement trigger mechanism first.

### H2 - Empty staged-file false green is proved, not assumed

The red test creates an isolated repository under `target/` with a resolvable
base and no staged, unstaged, branch, or untracked delta. One table-driven
scenario sends both that ordinary no-`--changed` path and explicit
`--changed ""` through the same asserted `FAIL/empty-diff` outcome; the explicit
form therefore cannot silently switch to auto-discovery. The omitted-target
scenario supplies a real harness path and excludes the G2 target, so it
must name that target rather than report empty. The indeterminate scenario uses
an unresolved base and preserves exit 3. These three independent outcomes
prevent a zero-required/zero-omitted PASS from masquerading as coverage.

### H3 - Stable Package B/C extension

The extension point is one canonical data file, one G1 schema version, and
stable ID namespaces. G2 has no registration list in Rust and no fragment
ordering. A synthetic appended entry proves that the loader and evaluator see
new data without code changes. Packages B and C may add entries only; they
cannot alter seed IDs, schema, root semantics, form/scope semantics, ordering,
or diagnostics. Any need for a new field or assertion form returns to G1 as a
separate reviewed change.

### H4 - G0/G1 API movement

This plan names capabilities rather than inventing adapters against unmerged
symbols. At execution start, the prerequisite gate records the exact merged
symbol mapping. A spelling-only difference is reconciled in the plan before the
red unit. A semantic difference - missing `RealFs`, direct corpus read required,
no data loader, incompatible corpus type, or absent typed diagnostics - blocks
G2. No compatibility wrapper may duplicate G0/G1 logic.

### H5 - Workspace lint blast radius

Adding `--workspace` to `cargo lint` brings `powerbi-tmdl-parser`,
`engram-indexer`, G0, and G1 under pedantic lint together with all targets and
features. The two existing crates have not been proved green under this exact
command; static inspection cannot settle Clippy findings. G2.10 therefore starts
with one bounded preflight. A failure is not repaired with `allow`, not hidden by
an exclusion, and not folded into G2 source edits. It becomes a separate
prerequisite. The CI clippy step remains root-scoped because this package may
edit `ci.yml` only for Decision 5a; the residual asymmetry is documented rather
than concealed.

### H6 - Registration atomicity and dead code

G2.2 is the sole four-file exception. Target registration and one truthful
crate mapping are atomic because the checked-in completeness test would fail if
the target existed unmapped. The target has a real sentinel and then real G0/G1
consumers; it has no placeholder module or dormant helper. Global
`-Dwarnings` remains active, and no `allow(dead_code)` is permitted. Every red
`todo!()` is removed by its adjacent green unit.

### H7 - Scope freeze

Implementation is frozen to:

* `Cargo.toml`
* `.cargo/config.toml`
* `.cargo/test-coverage-manifest.toml`
* `.github/workflows/ci.yml`
* `.github/contracts/agent-harness.toml`
* `scripts/test-coverage-oracle.ps1`
* `scripts/test-coverage-oracle.sh`
* `tests/contract/agent_harness_contracts_test.rs`
* `tests/unit/dev_test_coverage_oracle_test.rs`

No `src/` file, G0/G1 crate source, workflow other than `ci.yml`, dependency
version, branch topology, or product runtime surface may change. A required
change outside this list blocks and returns to planning.

## Constitution Check

| Principle or rule | Applies | Status | Plan evidence |
|---|---|---|---|
| I Safety-First Rust | Yes | Compliant | Test target forbids unsafe code; errors fail closed through G0/G1; global `-Dwarnings` remains active |
| II Test-First Development | Yes | Compliant | 5 compiling `[R]`/`[G]` pairs; every red todo has its immediately following owner; at most 3 scenarios per unit |
| III Workspace Isolation and Security Boundaries | Yes | Compliant | G2 supplies only the workspace root and typed roots; all containment, symlink rejection, and reads stay in G0 |
| IV CLI Workspace Containment | Yes | Compliant | Planned writes are restricted to the nine listed repository paths; isolated test repositories use `TempDir::new_in` under repository `target/` and never escape the workspace |
| V Structured Observability | Yes | Compliant | Typed G0/G1 diagnostics are preserved; oracle states expose `STATUS` and `REASON`; verification records target invocation and gate outcomes |
| VI Single Responsibility | Yes | Compliant | This principle's dependency-minimality requirement is met: zero new external dependencies; only existing path crates, standard library, and existing `tempfile` are used |
| VII Destructive Command Approval | N/A | N/A - no destructive action | Plan requires no deletion, overwrite of unrelated files, history rewrite, data drop, or system package change |
| VIII Explicit Safety Modes for Elevated Risk | Yes | Compliant | Freeze-scope applies to H7; investigate-first bounded lint preflight applies before widening the alias |
| IX Git-Friendly Persistence | Yes | Compliant | Registry and planning state are deterministic TOML/Markdown; IDs and ordering are stable and mergeable |
| X Agent Context Efficiency | Yes | Compliant | One canonical registry, G0/G1 reuse, exact oracle surfaces, and targeted verification avoid duplicate logic and broad scans |
| XI Merge Commit History Preservation | Yes | Compliant | G2 is a separate release unit and must merge by merge commit only after prerequisite and review gates; squash/rebase are forbidden |
| Development Workflow item 5 - No dead code | Yes | Compliant | No placeholder module, no unowned bootstrap test, no `allow(dead_code)`; all red todos are removed by named green owners |
| Quality Gates | Yes | Compliant | Verification runs fmt, workspace lint, dev-test, CI alias, CI-equivalent feature test, both oracle modes, and audit in order |
| Task Granularity - 2-hour rule | Yes | Compliant with one declared exception | 10 atomic units, each at most 3 scenarios; all touch at most 3 files except the justified four-file G2.2 registration atom |

## Exit Criteria

G2 is ready for plan review only when this draft's G0/G1 prerequisite API gate
has been reconciled against merged code. G2 implementation is complete only when
all five pairs are green, the exact verification sequence passes, the final diff
respects H7, and review reports zero unresolved P0/P1 findings. Until G0 and G1
both pass review and merge, no G2 work unit is executable.






