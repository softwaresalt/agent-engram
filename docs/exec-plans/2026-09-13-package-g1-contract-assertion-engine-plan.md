---
type: exec-plan
date: 2026-09-13
package: G1
depends_on: G0
source: docs/decisions/2026-09-13-package-g1-contract-assertion-engine-deliberation.md
program: docs/decisions/2026-09-13-package-g-split-program-decision.md
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
requires_plan_hardening: yes
plan_status: draft-dependent
review_verdict: not-reviewed
review_blocked_by: "Prerequisite G0 FAILED plan-review at attempt 1 (docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md). Per the program decision gate policy item 2, G1 is not reviewed and not harvested while its prerequisite authority is absent. This draft is PRESERVED unmodified as a dependent draft."
harvested: false
prerequisite: G0 must PASS plan-review and merge before this plan is executable
---

# Package G1 — typed contract assertion engine (implementation plan)

## Source Document

`docs/decisions/2026-09-13-package-g1-contract-assertion-engine-deliberation.md`

Upstream authority (inherited, not re-litigated):
`docs/decisions/2026-09-13-package-g-split-deliberation.md`.

Prior evidence (read, not re-litigated):
`docs/closure/2026-09-13-package-g-v3-plan-review-record.md`.

## Objective

Deliver a `publish = false`, dev-only workspace member crate
`crates/agent-contract-assert/` that evaluates typed `required` / `prohibited`
/ `exactly_one` assertions from a declarative TOML registry over a resolved file
corpus supplied by G0, and emits stable, deterministically ordered, structured
diagnostics. All testing lives in a new root-package `[[test]]` target
`tests/contract/agent_contract_assert_test.rs`. The crate re-implements no path,
discovery, or read logic (that is G0's), owns no filesystem error codes, and
adds zero new external dependencies.

Packages A-F extend this engine later by adding **registry rows**, not evaluator
code. Repository-specific seed content is **G2's**, not G1's.

## Verified Preconditions

Every row verified this session against `HEAD = 93370734` with read-only
commands.

| # | Precondition | Observed | Consequence |
|---|---|---|---|
| P1 | 267 `[[test]]` targets, all explicit `name` + `path`; `autotests` key absent | `Cargo.toml` grep | G1's exerciser needs an explicit `[[test]]` block; no stray target is auto-created |
| P2 | Global `rustflags = ["-Dwarnings"]` | `.cargo/config.toml` | `dead_code` is a **hard error**; no `allow(dead_code)` is permitted; pub lib items do not trip it |
| P3 | Aliases: `dev-test = "test --all-targets"`, `lint = "clippy --all-targets --all-features -- -D warnings -D clippy::pedantic"`, `fmt-check = "fmt --all -- --check"`, `ci` | `.cargo/config.toml` | The exerciser (a root `[[test]]` target) runs under `dev-test`/`ci` with **no** `--workspace` needed |
| P4 | `serde` (derive), `serde_json`, `thiserror`, `toml 0.8`, `serde_yaml 0.9` already declared | `Cargo.toml:36,37,44,60,77` | G1 reuses `serde`+`toml`+`thiserror`; **no new** dependency; `serde_yaml` avoided as archived/legacy |
| P5 | A `crates/` coverage surface **already exists** (`crates/powerbi-tmdl-parser/`, generic globs) | `.cargo/test-coverage-manifest.toml:137-139` | The split's S10 ("no `crates/` surface yet") is **stale**; adding a G1 `crates/` surface is conventional, not novel |
| P6 | Completeness maps targets only against `src/`+`crates/` surface globs; unmapped target => FAIL | `scripts/test-coverage-oracle.ps1:203-208` | Target name `agent_contract_assert_test` does **not** match `contract_*`, so a new surface is **required** or completeness FAILS |
| P7 | Completeness module-coverage checks only top-level `src/` entries | `scripts/test-coverage-oracle.ps1:209-221` | The new crate directory needs **no** module entry; only the target needs a surface glob |
| P8 | Neither `crates/agent-contract-fs` (G0) nor `crates/agent-contract-assert` (G1) exists | `Test-Path` = False | G1 is genuinely new; G0 is a **draft** dependency |
| P9 | Edition 2024, MSRV 1.85, `publish = false` precedent (`engram-indexer`) | `Cargo.toml`, `crates/engram-indexer` | The `publish = false` member-crate pattern is established here |
| P10 | Program-decision file is **absent** at HEAD | `Test-Path` = False for `docs/decisions/2026-09-13-package-g-split-program-decision.md` | Frontmatter `program:` points at a file the split references but which is not yet present; recorded as a risk |

## Architecture

### Crate and target topology

```text
crates/agent-contract-assert/            # G1 crate: publish = false, dev-only
  Cargo.toml                             # depends on serde, toml, thiserror, agent-contract-fs (G0)
  src/lib.rs                             # ALL public API (single lib file)
tests/contract/agent_contract_assert_test.rs   # G1 exerciser: [[test]] in root Cargo.toml
```

Root `Cargo.toml` gains: `crates/agent-contract-assert` in `[workspace]
members`; `agent-contract-assert = { path = "crates/agent-contract-assert" }`
in `[dev-dependencies]`; and a `[[test]] name = "agent_contract_assert_test",
path = "tests/contract/agent_contract_assert_test.rs"` block.
`.cargo/test-coverage-manifest.toml` gains a `[[surface]] path =
"crates/agent-contract-assert/"` with `targets = ["agent_contract_assert_test"]`.

### Dead-code discipline (P2)

The crate exposes its whole surface as `pub` from the lib root. Under
`-Dwarnings`, `dead_code` does **not** fire on `pub` items reachable from a
library crate root, so **no `allow(dead_code)` appears anywhere**. Every public
item is additionally exercised by a named `#[test]` in the exerciser, so the
surface is not merely public but demonstrably called.

### Module / type surface (every item exercised by a named caller in the exerciser)

| Item | Introduced by | Implemented by | Exercised by |
|---|---|---|---|
| `ContractRegistry`, `ContractEntry`, `AssertionForm` (`Required`/`Prohibited`/`ExactlyOne`), `RegistryError::Malformed`, `parse_registry` | G1.1 | G1.2 | G1.1 tests |
| `validate`, `RegistryError::DuplicateId` | G1.3 | G1.4 | G1.3 tests |
| `ResolvedCorpus` (trait), `CorpusFile<'a>`, `Diagnostic`, `DiagnosticCode::MissingRequired`, `evaluate` | G1.5 | G1.6 | G1.5 tests |
| `DiagnosticCode::ProhibitedPresent` (+ `Prohibited` arm) | G1.7 | G1.8 | G1.7 tests |
| `DiagnosticCode::AmbiguousMultiplicity` (+ `ExactlyOne` arm) | G1.9 | G1.10 | G1.9 tests |
| `run`, `render` (collect + sort + stable output) | G1.11 | G1.12 | G1.11 tests |
| `impl ResolvedCorpus for agent_contract_fs::<CorpusType>` (G0 adapter) | G1.13 | G1.14 | G1.13 tests |

`FakeCorpus` (the tiny synthetic fixture implementing `ResolvedCorpus`) is
defined **in the exerciser**, not in the crate, so no test-only type leaks into
the crate's public surface.

### Red-phase mechanism (NON-NEGOTIABLE)

* **API-introducing `[R]` units** add the minimal `pub` signatures they need
  with `todo!()` bodies, plus the `#[test]`s that call them. `todo!()` has the
  never type and coerces to any return type, so the workspace **compiles** at
  every commit; the tests fail at **runtime** on the `todo!()` panic. The paired
  `[G]` unit replaces the bodies. No `[R]` unit leaves the workspace
  non-compiling; no `[G]` unit exists without a preceding `[R]`.
* Where `evaluate` must be exhaustive over `AssertionForm` before all arms are
  implemented, the unimplemented arms are `todo!()` arms — reachable only by the
  later units' tests, so earlier units' tests never hit them.

## Assertion Model

G1 evaluates the three forms over the **whole resolved corpus** (set scope):

| `form` | Pass condition | Failure diagnostic |
|---|---|---|
| `required` | pattern matches in >=1 file of the corpus | `MissingRequired` when 0 matches |
| `prohibited` | pattern matches in 0 files of the corpus | `ProhibitedPresent` when >=1 match |
| `exactly_one` | pattern matches in exactly 1 file (n=1) | `MissingRequired` (n=0) **or** `AmbiguousMultiplicity` (n>1) |

**Deliberate expressive boundary.** G1 implements **set-scope** evaluation only.
Per-file (`each_file`) granularity, conditional, cross-file, ordering, and
numeric-threshold assertions are **deferred, plan-level extensions**, not silent
omissions and not row-adds. A future package that needs them makes a plan-level
change; it must not add an evaluator arm silently. This mirrors the split's
inherited "typed registry, not stringly-typed" discipline and keeps G1 minimal.

**`exactly_one` is Decision 4, implemented exactly.** Let *n* be the match
count. The arm is a single `match n { 0 => MissingRequired, 1 => pass, _ =>
AmbiguousMultiplicity }`. Because the two failure codes are arms of **one**
`match` on *n*, emitting both for one assertion is **unrepresentable**, not
merely forbidden. Ordering across different assertions is `(assertion id, then
code)` (both total orders); see Failure Semantics.

## Failure Semantics

G1's taxonomy is **registry/assertion semantics only**. Filesystem and discovery
failures are G0's taxonomy; G1 receives only a successfully resolved corpus
(`Ok`) and never invents or re-classifies a filesystem error code.

| Code | When it fires | Owning stage | Proven by |
|---|---|---|---|
| `MalformedRegistry` | registry text fails to parse, or omits/mistypes a required field (`form`, `pattern`, `id`, `root`) | parse (`parse_registry`) | G1.1 `[R]` -> G1.2 `[G]` |
| `DuplicateAssertionId` | two entries share an `id` | validate (`validate`) | G1.3 `[R]` -> G1.4 `[G]` |
| `MissingRequired` | `required` (or `exactly_one` with n=0) matches in 0 files; also the empty-corpus case | evaluate (`Required`/`ExactlyOne` arm) | G1.5/G1.6 (required, incl. empty corpus); G1.9/G1.10 (n=0) |
| `ProhibitedPresent` | `prohibited` matches in >=1 file | evaluate (`Prohibited` arm) | G1.7 `[R]` -> G1.8 `[G]` |
| `AmbiguousMultiplicity` | `exactly_one` matches in n>1 files | evaluate (`ExactlyOne` arm) | G1.9 `[R]` -> G1.10 `[G]` |

**Precedence (total, mutually exclusive by construction)**: `MalformedRegistry`
(load) precedes `DuplicateAssertionId` (validate) precedes the assertion
outcomes (`MissingRequired` / `ProhibitedPresent` / `AmbiguousMultiplicity`).
An entry has exactly one `form`, so its three assertion outcomes are mutually
exclusive; `exactly_one`'s two failure codes are arms of one `match` on *n*.

**Empty corpus vs failed-to-resolve.** A G0 **resolution failure** is an `Err`
that never reaches `evaluate` — the caller (G2) passes it through or wraps it;
G1 does not re-classify it and defines no code for it. An **empty corpus** is an
`Ok` corpus with zero files that *does* reach `evaluate`, where `required` /
`exactly_one` map n=0 to `MissingRequired` and `prohibited` trivially passes.
The two are type-distinct (`Err` vs `Ok(empty)`).

**Diagnostic content policy.** `Diagnostic.detail` carries assertion metadata
only — `id`, form, pattern, match count. It **never** contains raw file
content. `rel_path` (when present in a diagnostic) is workspace-relative with
`/` separators, as delivered by G0; G1 never emits an absolute host path.

**Ordering.** Diagnostics are collected then sorted on the total key
`(assertion_id, code_ordinal, rel_path)`. `assertion_id` uniqueness is
guaranteed by `DuplicateAssertionId` validation, so `assertion_id` alone totally
orders diagnostics in the set-scoped model; `code_ordinal` and `rel_path` are
present for robustness. String comparison is byte-lexicographic and
locale-independent; `rel_path` is `/`-normalised by G0, so ordering is
byte-identical on Windows and Unix.

## Work Units

`[R]` = compiling red harness unit (`todo!()` bodies; tests fail at runtime).
`[G]` = implementation greening exactly its paired `[R]`. **7 `[R]`/`[G]`
pairs = 14 units.** Each unit is <=3 scenarios. **13 of 14 units touch <=3
files; G1.1 (bootstrap/registration) touches 5 and is a justified exception**
(see G1.1). No unit's success criteria reference a type introduced by a later
unit. There is **no** ownerless placeholder test — every failing test is greened
by its paired `[G]` unit.

### G1.1 `[R]` — Register the crate + target + surface; introduce registry parse

* **Files owned (5 — bootstrap exception)**: `crates/agent-contract-assert/Cargo.toml`;
  `crates/agent-contract-assert/src/lib.rs`; root `Cargo.toml` (members +
  dev-dep + `[[test]]` block); `tests/contract/agent_contract_assert_test.rs`;
  `.cargo/test-coverage-manifest.toml` (new `[[surface]]`).
* **What it does**: creates the `publish = false` crate depending on
  `serde`+`toml`+`thiserror` and a path-dep on `agent-contract-fs` (G0). Adds
  `pub` types `ContractRegistry`, `ContractEntry`, `AssertionForm`
  (`Required`/`Prohibited`/`ExactlyOne`), `RegistryError` (with `Malformed`),
  and `parse_registry(&str) -> Result<ContractRegistry, RegistryError>` with a
  `todo!()` body. Registers the target `agent_contract_assert_test` and the
  surface mapping it. The exerciser contains the parse `#[test]`s only (no
  placeholder).
* **Success criteria**:
  1. `cargo test --test agent_contract_assert_test` **compiles**; the parse
     tests fail at runtime on `todo!()`.
  2. A valid TOML registry declaring all three forms round-trips `id`, `root`,
     `pattern`, `form` (asserted once the body is greened in G1.2).
  3. `scripts/test-coverage-oracle.ps1 --mode completeness` reports
     `STATUS=PASS` with `TARGET_COUNT=268` and `UNMAPPED_TARGETS_COUNT=0`
     (the new surface maps the new target, P6).
* **Note**: this unit exceeds the <=3-files guideline (5 files) because
  registration is inherently multi-file (crate manifest, crate root, root
  manifest, exerciser, coverage manifest). Estimated ~15 minutes; a justified
  exception, not a general pattern.

### G1.2 `[G]` — Implement registry parse

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: implements `parse_registry` via `serde` derive over
  `toml 0.8`. `form` and `pattern` are non-`Option` fields with no
  `#[serde(default)]`, so omission is a deserialization error mapped to
  `RegistryError::Malformed`.
* **Success criteria**:
  1. Valid registry (all three forms) parses and round-trips its fields.
  2. Syntactically malformed TOML yields `Malformed`.
  3. A registry entry omitting `form` yields `Malformed`.

### G1.3 `[R]` — Duplicate-id validation tests

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs` (adds
  `validate` + `RegistryError::DuplicateId`, `todo!()` body),
  `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: introduces `validate(&ContractRegistry) -> Result<(),
  RegistryError>` and the `DuplicateId` variant.
* **Success criteria**:
  1. A registry with all-unique ids validates `Ok`.
  2. Two entries sharing an `id` yield `DuplicateId` naming that id.
  3. Determinism: a registry `[a, b, a, c, b]` reports the **first** duplicate
     encountered in declaration order (`a`).

### G1.4 `[G]` — Implement duplicate-id validation

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: scans entries in **declaration order** (the `Vec` preserves
  TOML source order), tracking seen ids in a `BTreeSet`; reports the first id
  whose second occurrence is seen. No `HashMap`/`HashSet` iteration is used for
  reporting.
* **Success criteria**:
  1. Unique-id registry returns `Ok(())`.
  2. First-declared collision is the reported id, stable across runs.
  3. Reporting order is independent of id string ordering (declaration order
     decides, not lexical order).

### G1.5 `[R]` — Corpus trait + `required` evaluation tests

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs`,
  `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: introduces `trait ResolvedCorpus`, `struct CorpusFile<'a>`,
  `struct Diagnostic`, `enum DiagnosticCode` (starting with `MissingRequired`),
  and `evaluate(&ContractRegistry, &impl ResolvedCorpus) -> Vec<Diagnostic>`
  (`todo!()` body). Defines `FakeCorpus` in the exerciser.
* **Success criteria**:
  1. A `required` assertion whose pattern appears in some file yields **no**
     diagnostic.
  2. A `required` assertion whose pattern appears in **no** file yields one
     `MissingRequired` diagnostic naming the assertion id.
  3. An **empty** corpus (zero files) with a `required` assertion yields one
     `MissingRequired` diagnostic.

### G1.6 `[G]` — Implement `required` (set) evaluation

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: implements `evaluate` for `AssertionForm::Required`;
  `Prohibited` and `ExactlyOne` arms remain `todo!()` (reachable only by later
  units' tests, keeping the workspace compiling).
* **Success criteria**:
  1. Present-in-some-file -> no diagnostic.
  2. Absent-from-all -> `MissingRequired`.
  3. Empty corpus -> `MissingRequired`.

### G1.7 `[R]` — `prohibited` evaluation tests

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs` (adds
  `DiagnosticCode::ProhibitedPresent`),
  `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: adds the `ProhibitedPresent` code (a `pub` enum variant, so
  no dead-code issue) and prohibited `#[test]`s that exercise the still-`todo!()`
  `Prohibited` arm (red at runtime).
* **Success criteria**:
  1. Prohibited pattern absent from all files -> no diagnostic.
  2. Prohibited pattern present in one file -> one `ProhibitedPresent`.
  3. Prohibited pattern present in multiple files -> exactly one
     `ProhibitedPresent` naming the assertion (set scope emits one diagnostic).

### G1.8 `[G]` — Implement `prohibited` (set) evaluation

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: implements the `Prohibited` arm of `evaluate`.
* **Success criteria**:
  1. Absent -> no diagnostic.
  2. Present-once -> `ProhibitedPresent`.
  3. Present-many -> exactly one `ProhibitedPresent`.

### G1.9 `[R]` — `exactly_one` evaluation tests (Decision 4)

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs` (adds
  `DiagnosticCode::AmbiguousMultiplicity`),
  `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: adds the `AmbiguousMultiplicity` code and three separate
  `#[test]`s (one asserted outcome each) exercising the still-`todo!()`
  `ExactlyOne` arm.
* **Success criteria**:
  1. `exactly_one` with n=1 -> pass (no diagnostic).
  2. `exactly_one` with n=0 -> `MissingRequired` **only**.
  3. `exactly_one` with n>1 -> `AmbiguousMultiplicity` **only**.

### G1.10 `[G]` — Implement `exactly_one` as a single `match` on n

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: implements the `ExactlyOne` arm as
  `match n { 0 => MissingRequired, 1 => pass, _ => AmbiguousMultiplicity }` —
  the one `match` that makes both-emission unrepresentable.
* **Success criteria**:
  1. n=1 -> pass.
  2. n=0 -> `MissingRequired` only.
  3. n>1 -> `AmbiguousMultiplicity` only.

### G1.11 `[R]` — Deterministic ordering + rendering tests

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs` (adds
  `run(&ContractRegistry, &impl ResolvedCorpus) -> Vec<Diagnostic>` and
  `render(&[Diagnostic]) -> String`, `todo!()` bodies),
  `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: `run` validates then evaluates all entries and returns
  diagnostics sorted on `(assertion_id, code_ordinal, rel_path)`; `render`
  produces a stable structured string.
* **Success criteria**:
  1. Three failing assertions declared in order `C, A, B` yield diagnostics
     ordered `A, B, C` by id, regardless of declaration order.
  2. The order is independent of corpus file iteration order (two `FakeCorpus`
     values with different internal orders yield identical diagnostic order).
  3. `render` output is byte-identical across repeated runs and contains no
     absolute path (paths are `/`-normalised, content-free).

### G1.12 `[G]` — Implement ordering + rendering

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: implements `run` (collect + explicit sort on the total key)
  and `render` (stable field order, `/`-separated relative paths, metadata-only
  detail).
* **Success criteria**:
  1. Declaration-order-independent id ordering.
  2. Corpus-order-independent ordering.
  3. Byte-stable, absolute-path-free rendered output.

### G1.13 `[R]` — G0 corpus adapter test

* **Files owned (2)**: `crates/agent-contract-assert/src/lib.rs` (adds
  `impl ResolvedCorpus for agent_contract_fs::<CorpusType>`, `todo!()`-free but
  provisional), `tests/contract/agent_contract_assert_test.rs`.
* **What it does**: adapts G0's resolved corpus to G1's `ResolvedCorpus` trait
  and drives one end-to-end assertion through it, using **G0's `FakeFs`** (or a
  G0 corpus constructor) to build a small deterministic corpus — **no**
  repository `.github` content is read.
* **Success criteria**:
  1. A G0-produced synthetic corpus, adapted through the impl, yields the same
     diagnostics as the equivalent `FakeCorpus` input.
  2. The adapter exposes files in G0's `/`-normalised order.
  3. The adapter compiles against G0's public corpus type (this is the sole
     unit coupled to G0's concrete API).
* **G0-dependency note**: success criteria 1-3 reference **draft** G0 API item
  names; if G0's merged API differs, only this unit and G1.1's `Cargo.toml`
  dependency line need revision.

### G1.14 `[G]` — Finalise the adapter

* **Files owned (1)**: `crates/agent-contract-assert/src/lib.rs`.
* **What it does**: completes the adapter body to iterate G0's corpus as
  `CorpusFile` views (borrowed, zero re-implementation of read logic).
* **Success criteria**:
  1. Adapter and `FakeCorpus` produce identical diagnostics.
  2. Order matches G0's normalised order.
  3. Full exerciser is green under `cargo test --test agent_contract_assert_test`.

## Dependency on G0

G1 consumes exactly these **G0 public API items** (names provisional; G0's plan
owns them and they MUST be reconciled when G0 merges):

1. The crate `agent-contract-fs` as a `[dependencies]` path dependency of the
   G1 crate.
2. G0's resolved-corpus type (provisionally `agent_contract_fs::FileSet` /
   `ResolvedCorpus`): an **ordered** iteration of files, each exposing a
   **workspace-relative `/`-normalised path** and **UTF-8 content**. Adapted to
   G1's `ResolvedCorpus` trait in G1.13/G1.14.
3. G0's resolve error type: consumed **only** as an opaque `Err` passed through
   by the caller (G2); G1 never matches on it and defines no code for it.
4. G0's deterministic test double (provisionally `agent_contract_fs::FakeFs`) or
   a corpus constructor, for the G1.13 adapter test only.

The trait abstraction confines this coupling to G1.1's `Cargo.toml` and the
G1.13/G1.14 adapter units; the parse, validate, evaluate, ordering, and render
units are decoupled and test against G1's own `FakeCorpus`.

## Verification

Run at the final state (after G1.14). Each `[R]` unit compiles but leaves its
own tests failing until its paired `[G]`; only the final state is fully green.

```text
cargo dev-test
cargo test --test agent_contract_assert_test
cargo lint
cargo fmt-check
pwsh scripts/test-coverage-oracle.ps1 --mode completeness
```

Pass conditions: `cargo dev-test` and `cargo test --test
agent_contract_assert_test` green; `cargo lint` and `cargo fmt-check` clean
(no `allow(dead_code)`, pedantic-clean); the oracle reports `STATUS=PASS`,
`TARGET_COUNT=268`, `UNMAPPED_TARGETS_COUNT=0`.

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | G0 is unmerged; its corpus/error type names are draft | Trait abstraction (deliberation ii-b) confines G0 coupling to G1.1's dep line and G1.13/G1.14; only those revise if G0's API changes |
| R2 | The program-decision file (`...-split-program-decision.md`) is absent at HEAD (P10) | Frontmatter records it as the split names it; execution should confirm it exists (or is created by the split's own harvest) before G1 ships |
| R3 | Target name `agent_contract_assert_test` is unmatched by existing `contract_*` globs (P6) | G1.1 adds the required `[[surface]]`; G1.1 SC3 asserts completeness `STATUS=PASS` |
| R4 | Reviewers may expect `each_file` scope from the inherited v3 model | Documented as a deliberate, plan-level deferred extension, not a silent omission; keeps units bounded and dead-code-free |
| R5 | Non-deterministic diagnostic order across platforms | Explicit sort on `(assertion_id, code_ordinal, rel_path)`; byte-lexicographic, `/`-normalised paths; no hash-order reporting (see Plan Hardening) |

**Unresolved risk carried to execution**: R2 — the `program:` frontmatter path
points at a file absent at HEAD. The split deliberation references it, but it is
not present; whoever executes G1 must confirm the program decision exists before
treating this plan as program-anchored.

## Plan Hardening

### G0 API changes before G1 executes

G1's evaluator, diagnostics, ordering, and every assertion test are written
against G1's own `ResolvedCorpus` trait and a G1-owned `FakeCorpus`. The only
code coupled to G0's concrete API is (a) the `Cargo.toml` path-dependency line
in G1.1 and (b) the adapter impl in G1.13/G1.14. If G0's merged corpus type is
renamed or reshaped, **only those two sites change**; no assertion logic and no
assertion test is touched. The `Dependency on G0` list is the exhaustive
reconciliation checklist.

### Empty corpus vs corpus that failed to resolve

These are **type-distinct** and never conflated: G0's resolution returns
`Result<Corpus, G0Error>`. A **failure** is `Err` and never reaches G1's
`evaluate` — the caller (G2) passes it through or wraps it, and G1 defines **no**
filesystem/resolution code and performs **no** re-classification. An **empty
corpus** is `Ok` with zero files, a valid value that reaches `evaluate`, where
`required`/`exactly_one` map n=0 to `MissingRequired` and `prohibited` passes.
G1.5 SC3 tests the empty-`Ok` path explicitly; the `Err` path is out of G1's
surface by construction.

### Duplicate-id detection is deterministic

Detection scans entries in **declaration order** — the `Vec<ContractEntry>`
preserves TOML source order via serde — tracking seen ids in a `BTreeSet` and
reporting the first id whose second occurrence is encountered. No `HashMap` /
`HashSet` iteration order participates in the reported result. G1.3 SC3 pins the
`[a, b, a, c, b]` case to `a`, and G1.4 SC3 asserts the result is independent of
lexical id ordering, so the determinism is proven by test, not asserted.

### Diagnostic ordering is stable across platforms

`run` collects all diagnostics then sorts on the total key
`(assertion_id, code_ordinal, rel_path)`. `assertion_id` uniqueness is
guaranteed by `DuplicateAssertionId` validation, so it alone totally orders the
set-scoped diagnostics; `code_ordinal` (an explicit fixed enum ordinal, never a
name hash) and `rel_path` are robustness tie-breaks. `str` comparison in Rust is
byte-lexicographic and locale-independent, and `rel_path` is `/`-normalised by
G0 (split Decision 3, step 6), so the sort and the rendered output are
**byte-identical on Windows and Unix**. G1.11 SC1/SC2 prove declaration-order
and corpus-order independence; SC3 proves byte-stable, absolute-path-free
output.

## Constitution Check

Mapped against the real principles I-XI in
`.github/instructions/constitution.instructions.md`, plus Development Workflow
item 5 and the Task Granularity 2-hour rule.

| Principle | Applies | How G1 satisfies it (or why N/A) |
|---|---|---|
| I. Safety-First Rust | Yes | Crate is `#![forbid(unsafe_code)]`, edition 2024/MSRV 1.85; all fallible paths return `Result<_, RegistryError>` and propagate with `?`; no `unwrap`/`expect` in library code; pedantic-clean under `cargo lint` |
| II. Test-First Development | Yes | Every unit is `[R]` (compiling, `todo!()`, failing) before its `[G]`; all tests in `tests/contract/agent_contract_assert_test.rs` |
| III. Workspace Isolation and Security Boundaries | Yes (by delegation) | G1 performs **no** filesystem or path operations; all containment/path safety is G0's and is passed through, never re-classified; registry parse is pure |
| IV. CLI Workspace Containment | Yes | All new files live under the repo tree (`crates/agent-contract-assert/`, `tests/`, root `Cargo.toml`, `.cargo/`); nothing is created outside cwd |
| V. Structured Observability | Yes | Diagnostics are the observability surface: typed `DiagnosticCode`, metadata-only `detail`, deterministically ordered, stably rendered |
| VI. Single Responsibility (dependency minimality) | Yes | **Zero new** external dependencies; reuses only already-declared `serde`, `toml`, `thiserror` (P4), each justified; plain-Rust registry option was deliberated and rejected because it makes `MalformedRegistry` unrepresentable at runtime |
| VII. Destructive Command Approval | N/A | G1 execution creates files within the repo and runs read-only build/test/lint; no destructive terminal command (delete, force-overwrite, history rewrite, data drop) is involved |
| VIII. Explicit Safety Modes for Elevated Risk | N/A | Low blast radius; no destructive or production-impacting work. Posture is freeze-scope to the declared G1 crate + exerciser + root `Cargo.toml` + coverage manifest; investigate-first already discharged via the deliberation's fresh evidence |
| IX. Git-Friendly Persistence | Yes | Registry is TOML (human-readable, comment-bearing, mergeable); these artifacts are markdown + YAML frontmatter |
| X. Agent Context Efficiency | Yes | Diagnostics carry minimal structured metadata (id/form/pattern/count), **never** raw file content; output is targeted, not a file dump |
| XI. Merge Commit History Preservation | Yes (at ship) | G1 ships as its own PR merged with a merge commit (no squash/rebase), per the split's three-shipment plan |
| Dev-Workflow item 5 — No dead code | Yes | `pub` lib-root items do not trip `dead_code` under `-Dwarnings`; **no** `allow(dead_code)` anywhere; every public item is exercised by a named exerciser `#[test]` |
| Task Granularity — 2-hour rule | Yes | 14 units, each single-domain (Rust), <=3 scenarios; 13 of 14 touch <=3 files; G1.1 (5 files, ~15 min) is a stated bootstrap exception |
