---
type: deliberation
date: 2026-09-13
slug: package-g-agent-contract-assertion-harness-v2
depth: standard
status: decided
supersedes: docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md
package: G
program: write-boundary / harness-contract enforcement
branch: chore/checkpoint-resolution-ordering-restage
---

# Package G v2 — Executable agent-contract assertion harness

## Supersession notice

This artifact **supersedes** `2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md`
(v1, status `decided-plan-blocked`). The v1 artifact and its two review rounds are
retained unmodified as evidence. Nothing in v1 is overwritten.

v1 was blocked at plan review after a correction round on two structural issues:
the validator abstraction and the module topology. The operator has since
established those two decisions as **inputs**, not open questions. This v2
deliberation confirms them against freshly re-verified repository facts and
settles the one genuinely open sub-decision they leave: **which module topology**
satisfies the lint gate while keeping every type and function exercised.

v1's Option A/B/C comparison (Rust `[[test]]` target vs. extending `engram verify`
vs. a PowerShell/bash script pair) is **not reopened**. It was affirmed by seven
reviewers across two rounds. This deliberation inherits that outcome.

## Problem Frame

### What

The repository's agent harness — `.github/agents`, `.github/skills`,
`.github/instructions`, `.github/policies`, `.github/prompts` — is 92 markdown
files carrying load-bearing behavioral contracts. Those contracts are enforced
today only by human reading and by agents choosing to comply. There is no
executable assertion that a required contract is present, that a prohibited
contract is absent, or that a contract is unambiguously singular.

Package G builds that executable harness. It is the **root prerequisite** for
decomposed Packages A–F: those packages add *data* (registry entries, fixtures),
not evaluator code.

### Who cares and why

* **The operator**, who currently has no mechanical signal that a harness edit
  silently deleted or duplicated a load-bearing clause.
* **Ship and Stage**, whose gates cite clauses that can drift out from under them.
* **Packages A–F**, which are blocked on a generic assertion substrate.

### Constraints (all re-verified this session — see Research Findings)

| # | Constraint | Source |
|---|---|---|
| C1 | No product runtime behavior change | Operator scope statement |
| C2 | No new dependency unless evidence proves necessary | Operator decision 3 |
| C3 | Lint-clean under `-D warnings -D clippy::pedantic`, no broad `allow(dead_code)` | `.cargo/config.toml`, operator decision 2 |
| C4 | Every task `<4` scenarios, ~2h width | Operator decision 4; constitution |
| C5 | Every implementation unit preceded by a failing test unit; CI wiring included | Operator decision 5 |
| C6 | Any unit adding a root module owns its module files in dependency-safe order | Operator decision 6 |
| C7 | Workspace containment; read-only evaluation | Constitution III/IV |
| C8 | Must not reuse `verify_markdown` | Operator decision 1 |

### Success criteria

1. `cargo dev-test` runs the harness assertions locally with zero setup.
2. CI runs them on any PR that touches harness markdown.
3. A deleted required clause, an added prohibited clause, a malformed registry,
   an empty resolved file set, an out-of-cwd path, an unreadable file, a duplicate
   assertion ID, and an ambiguous multiplicity each produce a **nonzero** result
   with a deterministic, structured diagnostic.
4. Packages A–F extend it by adding registry rows, not evaluator code.

### Explicitly out of scope

Any `src/` change. Any MCP tool. Any authoring of A–F assertion data. Any PR.
Any change to Packages A–F. `verify_markdown` modification.

## Research Findings (freshly verified 2026-09-13, post-v1)

Every item below was re-derived this session. Items marked **[CORRECTS v1]**
contradict or materially sharpen a v1 claim.

### R1 — Cargo test-target registration is 100% explicit

`Cargo.toml` declares **267** `[[test]]` targets, every one with an explicit
`name` and `path` into `tests/contract/`, `tests/integration/`, or `tests/unit/`.
There is **no `autotests` key** and **no root-level `tests/*.rs` file**. Cargo
auto-discovery covers only `tests/*.rs` and `tests/*/main.rs`; neither exists, so
auto-discovery contributes nothing and cannot collide with a new subdirectory.

**Implication**: a new target requires an explicit `[[test]]` block. Placing
submodule files under `tests/contract/<dir>/` cannot accidentally create a second
target.

### R2 — Regular `[dependencies]` are available to test targets **[CORRECTS v1]**

v1 treated the dependency question as unsettled. It is settled empirically:
`tests/helpers/mod.rs` imports `tempfile::TempDir`, and `tempfile = "3"` is a
**regular** `[dependencies]` entry (`Cargo.toml:59`), not a dev-dependency.
Cargo links both `[dependencies]` and `[dev-dependencies]` into test targets.

Therefore the following are directly usable from a `[[test]]` target with
**zero new dependencies**, satisfying C2:

| Crate | Cargo.toml line | Use in Package G |
|---|---|---|
| `ignore = "0.4"` | 75 | Bounded, deterministic file walk |
| `globset = "0.4"` | 76 | Registry glob patterns |
| `serde_yaml = "0.9"` | 77 | Frontmatter + `ci.yml` parsing |
| `toml = "0.8"` | 60 | Registry file format (if TOML chosen) |
| `serde` / `serde_json` | `[dependencies]` | Typed registry deserialization |

`ignore::WalkBuilder` supplies `follow_links(false)`, `max_depth`,
`max_filesize`, and `sort_by_file_path` — it resolves the symlink, bounds, and
determinism requirements natively. **Caveat (carried forward from v1, still
true)**: `hidden(true)` is the `WalkBuilder` default and `.github` is a
dot-directory. A depth-0 root bypasses the hidden filter, but nested traversal
does not, so each harness directory must be added as its own root *and*
`hidden(false)` set explicitly.

### R3 — Lint surface for test crates is stricter than for the library **[CORRECTS v1]**

`.cargo/config.toml` sets `[build] rustflags = ["-Dwarnings"]` globally.
`cargo lint` is `clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`;
CI clippy is the same with `--no-default-features --features cozo-backend,embeddings`.
Both include `--all-targets`, so a new test target **is** linted at pedantic.

Critically: `src/lib.rs` carries a 20-entry crate-level allow list
(`too_many_lines`, `wildcard_imports`, `doc_markdown`, `needless_raw_string_hashes`,
`module_name_repetitions`, …). **Test targets are separate crates; that allow list
does not apply to them.** A new test target faces raw pedantic.

**Verified precedent for handling this**: existing test targets carry *narrow,
per-target* crate attributes — `#![allow(clippy::doc_markdown)]` in 21 files and
`#![allow(clippy::needless_raw_string_hashes)]` in 15 files. This is an
established, reviewable pattern and is **not** a broad suppression.

**`allow(dead_code)` is different.** It appears exactly once, in
`tests/helpers/mod.rs:34`, and only because that file is `#[path]`-included into
many targets, each of which uses a different subset. That justification does not
transfer to a single-target design.

`clippy::unwrap_used` / `expect_used` are **not** configured anywhere in
`Cargo.toml` or `src/lib.rs` **[CORRECTS the AGENTS.md summary]**; existing
contract tests use `.unwrap()` 55 times. Test code may use `unwrap()`.

### R4 — Multi-file test topology has exactly one precedent **[CORRECTS v1]**

The only cross-file idiom in `tests/` is
`#[path = "../helpers/mod.rs"] mod helpers;` — used to share three helper files
*across directories*. There is **no existing subdirectory submodule tree** under
`tests/`. (v1 planning cited `pub mod f1;` in
`tests/integration/canonical_call_resolution_test.rs` as a precedent; that string
is **fixture content inside a raw-string literal**, not a real module
declaration. The v1 topology reasoning built on it is void.)

Plain `mod agent_harness;` from a root file at
`tests/contract/agent_harness_contracts_test.rs` resolves to
`tests/contract/agent_harness/mod.rs`. This is standard Rust and, per R1, cannot
create a stray cargo target.

### R5 — CI already executes any registered target; the gap is the trigger **[SHARPENS v1]**

The CI test step is
`cargo test --no-default-features --features cozo-backend,embeddings --all-targets`.
Because of `--all-targets`, **a newly registered `[[test]]` target runs in CI
automatically with no workflow edit.**

The actual gap is the trigger. `.github/workflows/ci.yml` declares, on **both**
`push` and `pull_request`:

```yaml
paths-ignore:
  - '.backlogit/**'
  - 'docs/**'
  - '.autoharness/**'
  - '*.md'
  - '.github/**/*.md'     # <-- the gap
  - 'scripts/**/*.md'
```

`paths-ignore` uses all-match semantics, so a PR touching **only** harness
markdown skips CI entirely — the harness validator would never run on precisely
the change class it exists to validate. Removing the `.github/**/*.md` entry from
both blocks is the necessary and sufficient CI change.

Supporting facts carried from v1 and still accurate: `main` has no branch
protection (`GET .../branches/main/protection` → 404), so `build` is not a
required status check and narrowing `paths-ignore` cannot hang a PR. Only one
`.github` markdown file sits outside the five harness directories
(`.github/copilot-instructions.md`), so deleting the entry outright is the
fail-closed choice and glob negation is unnecessary.

### R6 — Coverage-oracle interaction is a real, bounded obligation

Baseline run this session:

```text
MODE=completeness  TARGET_COUNT=267  MODULE_COUNT=13
UNMAPPED_TARGETS_COUNT=0  UNMAPPED_MODULES_COUNT=0  STATUS=PASS
```

`--mode completeness` fails when a declared `[[test]]` target is unmapped by a
`src/`/`crates/` surface. Surfaces map by **target-name glob**, and every `src/`
surface lists `"contract_*"`. Therefore **a target named `contract_*` is
automatically mapped and completeness stays PASS with no manifest edit.**

Separately, the manifest documents that "a changed file outside `src/` that
matches no surface contributes no required targets (docs, `.github`, and similar
are not source surfaces)." So change-scoped `--mode run` currently selects
*nothing* for a harness-markdown edit — a false-PASS in the runner agents are
told to use. The manifest **does** accept non-`src` surfaces (existing precedent:
`path = ".cargo/config.toml"`, `path = "scripts/test-coverage-oracle"`), so
adding harness surfaces mapped to the new target closes this cleanly.

### R7 — Harness surface inventory

| Directory | `.md` files |
|---|---|
| `.github/agents` | 23 |
| `.github/skills` | 28 |
| `.github/instructions` | 34 |
| `.github/policies` | 2 |
| `.github/prompts` | 5 |
| **Total** | **92** |

Consistent with the v1 finding that **20 of 92** contain same-line `{{…}}`.

### R8 — `verify_markdown` remains disqualified (operator decision 1, confirmed)

`engram::services::verify::verify_markdown` is an *ingestion* conformance
service. It emits `template.unresolved` at `Severity::Error` for any `{{…}}`
line and computes `conformant = findings.is_empty()`. Against R7, it would
reject 20 legitimate harness contracts on day one. Package G needs a
harness-owned validator whose only semantics are its own typed contract
assertions. **Not reopened.**

## The one open sub-decision: module topology

Decisions 1 and 2 fix *that* Package G owns its validator and *that* the topology
must be lint-clean with every item exercised. They do not fix *which* topology.
R3 and R4 make this a genuine choice with real trade-offs.

### Option T-A — One `[[test]]` target, root file + dedicated submodule directory

`tests/contract/agent_harness_contracts_test.rs` (root, registered as
`contract_agent_harness_contracts`) declaring `mod agent_harness;`, resolving to
`tests/contract/agent_harness/mod.rs`, which declares narrowly scoped siblings
(`registry`, `resolve`, `assert`, `report`).

* **Dead code**: a single consuming crate means every `pub` item is reachable
  from one root. If every type and function is exercised, `dead_code` never
  fires and **no `allow(dead_code)` is needed** — satisfying C3 and operator
  decision 2 exactly.
* **Pedantic**: narrow per-target `#![allow(clippy::…)]` on the root only, using
  the R3 precedent, if a specific lint proves unavoidable.
* **C6**: each unit that adds a submodule must, in the same unit, add both the
  file and its `pub mod` line in `mod.rs` — a mechanically checkable obligation.
* **Cost**: introduces a topology with no in-repo precedent (R4).
* **R6**: name begins `contract_` → completeness stays PASS with no manifest edit.

### Option T-B — One `[[test]]` target, single monolithic file

* **Pros**: exactly matches the dominant in-repo idiom; zero topology risk.
* **Cons**: a validator with registry parsing, path resolution, walking,
  assertion evaluation, and diagnostic rendering lands at roughly 600–900 lines
  in one file. That collides with pedantic `too_many_lines` (not allowed for test
  crates per R3) and makes C4's `<4`-scenario unit boundaries nearly impossible
  to express as separable work — every unit edits the same file.

### Option T-C — Workspace-member lib crate (`crates/harness-contract/`)

* **Pros**: strongest module hygiene; `pub` items genuinely exported.
* **Cons**: a new workspace member is a **product-surface** change adjacent to
  C1, requires `[workspace] members` edit plus its own `Cargo.toml`, lint config,
  and coverage-manifest surface (R6 completeness checks `crates/` surfaces too),
  and pulls Package G well past its scope. Disproportionate for a repository-file
  validator.

### Comparison

| Criterion | T-A | T-B | T-C |
|---|---|---|---|
| No `allow(dead_code)` (C3) | ✅ single crate, all reachable | ✅ | ⚠️ needs `pub` discipline |
| Pedantic-clean (R3) | ✅ narrow root allows | ❌ `too_many_lines` risk | ✅ |
| `<4`-scenario unit split (C4) | ✅ per-submodule units | ❌ all units share one file | ✅ |
| C6 file ownership provable | ✅ file + `pub mod` line | n/a | ✅ |
| No new dependency (C2) | ✅ | ✅ | ⚠️ new manifest |
| Scope discipline (C1) | ✅ tests only | ✅ | ❌ new workspace member |
| In-repo precedent (R4) | ⚠️ novel but standard Rust | ✅ | ⚠️ novel |
| Coverage completeness (R6) | ✅ auto via `contract_*` | ✅ | ⚠️ new `crates/` surface |

## Decision

**Adopt Option T-A.** One explicitly registered `[[test]]` target named
`contract_agent_harness_contracts` at
`tests/contract/agent_harness_contracts_test.rs`, with narrowly scoped submodules
under `tests/contract/agent_harness/`.

Rationale: T-A is the only option that simultaneously satisfies the no-broad-
`allow(dead_code)` requirement, the pedantic lint gate without the library's
allow list, the `<4`-scenario unit granularity, and the no-scope-creep boundary.
Its single weakness — no in-repo precedent — is a novelty cost, not a
correctness cost: the construct is ordinary Rust, and R1 proves it cannot produce
a stray cargo target.

### Validator ownership (confirms operator decision 1)

Package G owns its validator end to end. It does **not** call, wrap, adapt, or
filter `verify_markdown`, and takes no dependency on
`engram::services::verify`. Its only semantics are its own typed contract
assertions. The sole `engram`-crate coupling permitted is none at all; the target
uses `ignore`, `globset`, `serde`/`serde_yaml` directly.

### Failure-semantics contract

Each of the following is an assertion failure producing a nonzero result:

| # | Condition | Semantics |
|---|---|---|
| F1 | Expected contract missing | `required` assertion resolved 0 matches |
| F2 | Prohibited contract present | `prohibited` assertion resolved ≥1 match |
| F3 | Malformed registry | Deserialization or schema-validation error |
| F4 | Empty resolved file set | A registry entry's glob resolved to zero files |
| F5 | Invalid or out-of-cwd path | Path escapes workspace root after canonicalization |
| F6 | Unreadable file | I/O error on a resolved file |
| F7 | Duplicate assertion ID | Two registry entries share an `id` |
| F8 | Ambiguous multiplicity | `exactly_one` assertion resolved ≥2 matches |

F4 and F5 are **fail-closed**: silence is never success. F1–F8 are each a
distinct diagnostic code, never collapsed into a generic "failed".

### Extensibility contract

The registry is data. Packages A–F add rows. A new assertion for a new harness
surface requires **zero** evaluator code: a registry entry names a file-set glob,
an assertion form, a pattern, and a multiplicity. Evaluator code changes only if
a genuinely new *assertion form* is required, which A–F are not expected to need.

### Determinism contract

`ignore::WalkBuilder::sort_by_file_path` for file order; registry entries
evaluated in declared order; diagnostics emitted in `(entry_order, file_path)`
order. Two runs over an unchanged tree produce byte-identical diagnostics.

### Containment contract

Read-only. No writes of any kind. Every resolved path is canonicalized and
asserted to be within the workspace root before open; `follow_links(false)`
prevents symlink escape.

### CI integration (confirms operator decision 5)

Two coupled facts from R5: the target runs in CI automatically via
`--all-targets`, and harness-only PRs never reach CI because of
`paths-ignore: '.github/**/*.md'`. The CI unit therefore removes that entry from
**both** the `push` and `pull_request` blocks.

Its **red phase** is repository-native and self-contained: a contract assertion
that parses `.github/workflows/ci.yml` with `serde_yaml` and fails while
`.github/**/*.md` is present in either `paths-ignore` list. That test fails
before the workflow edit and passes after — a genuine red/green pair with no
green-only step. R6's manifest surfaces are owned by the same unit, since both
concern "does this target actually get selected for a harness change".

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Reuse `verify_markdown` (v1 open option) | R8 / operator decision 1 — rejects 20 of 92 files |
| Extend `engram verify` (v1 Option B) | Violates C1, no product runtime change |
| PowerShell/bash script pair (v1 Option C) | `Select-String`/`grep` match-absence ambiguity; no type safety |
| Monolithic single file (T-B) | Blocks C4 unit split; pedantic `too_many_lines` |
| Workspace-member crate (T-C) | Disproportionate; adjacent to C1 |
| Three targets sharing one `mod.rs` (v1 plan) | Root cause of the v1 lint block — forces `allow(dead_code)` |
| Add a new dependency | C2 — R2 proves `ignore`/`globset`/`serde_yaml`/`toml` suffice |

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| `.github` is hidden; `WalkBuilder` skips it | Add each harness dir as its own root **and** set `hidden(false)` explicitly; a dedicated assertion proves the resolved set is non-empty (F4) |
| Novel submodule topology (R4) | Standard Rust; R1 proves no stray target; first unit is a compile-only red harness that proves the topology before any logic lands |
| Pedantic lints unforeseen in test crate (R3) | Narrow per-target `#![allow(clippy::…)]` using the 21-file/15-file precedent; broad suppression is a review-blocking finding |
| An item ends up unexercised → `dead_code` | Every unit's acceptance criteria require its items be exercised by that unit's tests; no `allow(dead_code)` anywhere |
| Removing `paths-ignore` slows doc-only PRs | `main` unprotected, `build` not required; only `.github/copilot-instructions.md` is affected outside the five dirs |
| Coverage oracle false-PASS persists | Manifest surfaces owned by the CI unit; verified by re-running `--mode completeness` |

## Unresolved questions (non-blocking)

1. Registry file format — TOML (`toml` dep, matches `.cargo/test-coverage-manifest.toml`
   house style) vs. inline Rust consts. Resolved at plan time; both satisfy C2.
2. Whether Package B–F rows live in one registry file or per-package files.
   Deferred to those packages; the registry loader must accept either.

## Outcome

`promote_to: plan`. Proceed to `impl-plan` with this artifact as source, then
`plan-harden`, then full `plan-review`.
