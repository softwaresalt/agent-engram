---
type: deliberation
date: 2026-09-13
package: G1
depends_on: G0
source: docs/decisions/2026-09-13-package-g-split-deliberation.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
---

# Package G1 — typed contract assertion engine (deliberation)

## Authority and scope

This deliberation is subordinate to, and **inherits**, the accepted split
authority `docs/decisions/2026-09-13-package-g-split-deliberation.md`. It does
**not** reopen that split. In particular it does not re-argue the split boundary
(Decision 1, Option D1-3), the code topology (Decision 2, Option T3), the G0
seam contract (Decision 3), the `exactly_one` precedence (Decision 4), the G2
problem set (Decision 5), the inherited constraints (Decision 6), or the cost
accounting (Decision 7). Those are settled upstream.

G1 is the **second of three** units (G0 -> G1 -> G2). It **depends on G0** and
is planned as **dependent**: G0 is not merged, so the accompanying G1 plan is a
**draft** that only becomes executable if G0 passes plan-review and merges
first.

**In scope for this deliberation**: the three open questions the split
delegated to the per-package plan for G1 — (i) registry representation, (ii) how
G1 consumes G0's resolved corpus, (iii) diagnostic ordering strategy — plus the
precise statement of which G0 public API items G1 consumes.

**Out of scope**: Packages A-F, G0's internal design, G2's registration and
fixture authoring, and any re-litigation of the split's decisions.

## Problem

The split (Decision 1) localises three failure taxonomies into three units. G1
owns exactly one of them: **registry and assertion semantics**. G1 must evaluate
typed `required` / `prohibited` / `exactly_one` assertions over a resolved file
corpus supplied by G0, and emit stable, deterministically ordered, structured
diagnostics. It must do this while:

* reusing G0's resolved corpus and **re-implementing no path, discovery, or read
  logic**;
* owning **no filesystem error codes** — resolution and read failures are G0's
  taxonomy and are passed through or wrapped, never re-classified;
* adding **zero new external dependencies** (Constitution VI, applied to G1 by
  Decision 6 item 3);
* shipping as a `publish = false` dev-only member crate that is **lint-clean
  under `-Dwarnings`** (no `allow(dead_code)` anywhere);
* keeping the registry **generic and extensible** — no Package-A/B/C/D/E policy
  text, no repository-specific seed content (that belongs to G2).

Three combined-Package-G attempts failed; the split exists so a defect in one
taxonomy can no longer fail the others. G1's job is to be a self-contained,
independently reviewable evaluator that a defect in G0's seam or G2's wiring
cannot contaminate, and vice versa.

## Evidence gathered this session (fresh, not inherited)

All rows verified this session against `HEAD = 93370734` with read-only
commands.

| # | Evidence | Value | Source |
|---|---|---|---|
| E1 | HEAD / branch | `933707347cf4a7a47f843a34928d30a9a5e3b6d2` on `chore/checkpoint-resolution-ordering-restage` | `git rev-parse` |
| E2 | Serialization crates already declared in root `Cargo.toml` | `serde = { version = "1", features = ["derive"] }` (L36), `serde_json = "1"` (L37), `thiserror = "1"` (L44), `toml = "0.8"` (L60), `serde_yaml = "0.9"` (L77) | `Cargo.toml:36,37,44,60,77` |
| E3 | `autotests` key | **absent** (0 occurrences) | `Cargo.toml` grep |
| E4 | Registered `[[test]]` targets | **267** explicit `name` + `path` blocks | `Cargo.toml` grep |
| E5 | Targets using the `contract_` name prefix | **63** | `Cargo.toml` grep |
| E6 | Global rustflags | `rustflags = ["-Dwarnings"]` under `[build]` | `.cargo/config.toml` |
| E7 | Cargo aliases | `dev-test = "test --all-targets"`, `lint = "clippy --all-targets --all-features -- -D warnings -D clippy::pedantic"`, `fmt-check = "fmt --all -- --check"`, `ci = "test --all-targets --all-features"` | `.cargo/config.toml` |
| E8 | Workspace members | `[".", "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml:2` |
| E9 | **A `crates/` coverage surface already exists** | `path = "crates/powerbi-tmdl-parser/"` with `targets = ["contract_*", "integration_*", "unit_*", "cold_*", "helpers_*"]` | `.cargo/test-coverage-manifest.toml:137-139` |
| E10 | Completeness target-coverage rule | targets are mapped **only** against surfaces whose path starts with `src/` or `crates/`; an unmapped `[[test]]` target => `STATUS=FAIL` | `scripts/test-coverage-oracle.ps1:203-208` |
| E11 | Completeness module-coverage rule | checks **only** top-level entries under `src/`; `crates/` member directories are **not** module-checked | `scripts/test-coverage-oracle.ps1:209-221` |
| E12 | `crates/engram-indexer` has **no** coverage surface | absent from manifest | `.cargo/test-coverage-manifest.toml` grep |
| E13 | Neither `crates/agent-contract-fs` (G0) nor `crates/agent-contract-assert` (G1) exists yet | `Test-Path` = False for both | filesystem |
| E14 | Program-decision file referenced by the split | `docs/decisions/2026-09-13-package-g-split-program-decision.md` is **absent** at HEAD | `Test-Path` = False |

### What E9 + E10 mean (correcting a stale inherited claim)

The split deliberation's evidence S10 states "every surface is `src/...`; **no
`crates/` surface exists yet**". At `HEAD = 93370734` that is **stale**: a
`crates/powerbi-tmdl-parser/` surface now exists (E9) and it uses the generic
glob set `contract_*, integration_*, unit_*, cold_*, helpers_*`. The
consequence for G1 is precise and must not be overstated:

* Completeness **target coverage** (E10) fails only for a `[[test]]` target that
  no `src/` or `crates/` surface glob matches. A target named with the
  `contract_` prefix would already be absorbed by the existing generic
  `contract_*` globs.
* This plan names G1's exerciser target **`agent_contract_assert_test`** (so the
  operator-specified verification command `cargo test --test
  agent_contract_assert_test` is exact). That name does **not** match
  `contract_*`, so it is **genuinely unmapped** until a surface maps it, and
  `--mode completeness` would therefore FAIL without the new surface.
* A new `[[surface]]` `path = "crates/agent-contract-assert/"` with `targets =
  ["agent_contract_assert_test"]` is therefore **required**, and it is also the
  **truthful** mapping: a change to the G1 crate really should run the G1
  exerciser (this drives `--mode select` / `report` / `run` correctly, not just
  completeness).

E11 confirms the new crate directory itself needs no module-coverage entry; only
the target needs a surface glob. E12 confirms an unmapped member crate
(`engram-indexer`) is a pre-existing state, not a completeness blocker, because
completeness maps *targets*, not *crate directories*.

## Question (i) — registry representation

The registry is the declarative input G1 evaluates. A **required, tested**
failure mode is "malformed registry", so the representation must make a
malformed input a **runtime, observable** condition.

### Option i-a — construct the registry in plain Rust (no serialization dep)

Build `ContractRegistry` values directly from Rust code (literals / a builder),
with no `serde`, `toml`, or `serde_json`.

* Pro: the strictest possible reading of Constitution VI / Decision 6 item 3 —
  zero serialization dependency, not merely zero *new* dependency.
* Pro: no parser to test, no parse-error surface.
* **Con (decisive)**: a malformed registry becomes **unrepresentable at
  runtime** — you cannot construct an ill-typed Rust value; it is a compile
  error, not a value G1 can be handed. The functional scope **requires**
  `MalformedRegistry` to be modelled and tested, so plain Rust would delete a
  mandated tested failure mode. To recover it we would have to invent a
  string-parsed side channel anyway, i.e. re-introduce a serialization step.
* Con: a Rust-literal registry is not a **git-friendly, human-editable** data
  artifact (Constitution IX); Packages A-F extend the registry as **data**, and
  editing Rust is a code change, not a row-add.

### Option i-b — TOML + `serde` derive (reuse declared `toml 0.8` + `serde 1`)

`parse_registry(&str) -> Result<ContractRegistry, RegistryError>`; a
`#[derive(Deserialize)]` model with non-`Option` `form` / `pattern` fields so
omission is a deserialization error mapped to `MalformedRegistry`.

* Pro: makes "malformed" a natural, testable runtime condition (syntax error, or
  a missing/unknown field).
* Pro: reuses crates **already declared** in the root workspace (E2) — zero new
  dependency; Decision 6 item 3 satisfied.
* Pro: TOML is human-readable, comment-bearing, and merge-friendly
  (Constitution IX); Packages A-F extend it as data.
* Pro: matches the split's own Open-Questions note ("reusing the already-declared
  root `toml 0.8`").
* Con: adds `serde` + `toml` to G1's crate dependency graph (already in the
  workspace, so not a *new* dependency, but still a justified surface).

### Option i-c — JSON + `serde_json`

Same shape as i-b but with `serde_json` (E2, L37).

* Pro: reuses a declared crate; models malformed naturally.
* Con: JSON is **less git-friendly** than TOML (no comments, noisier diffs), a
  worse home for a registry that A-F will hand-edit (Constitution IX).
* Con: no advantage over TOML here; TOML is the established config idiom in this
  workspace.

`serde_yaml 0.9` is present (E2, L77) but is treated as **archived/legacy** in
this workspace; YAML is deliberately not considered — TOML gives the same
serde-derive ergonomics with a supported, git-friendly format.

### Selected (i): i-b — TOML + `serde` derive

TOML makes the mandated `MalformedRegistry` failure a real, testable runtime
condition; reuses only already-declared crates (zero new dependency); and keeps
the registry a human-editable, mergeable data artifact that A-F extend by adding
rows. `thiserror 1` (E2, L44) is reused for the small `RegistryError` enum's
`Display` (idiomatic, already declared, boilerplate-reducing); `serde_json` is
**not** taken. Each reused crate is justified in the plan's Constitution Check.

## Question (ii) — how G1 consumes G0's resolved corpus

The split topology **fixes** that G1 depends on `crates/agent-contract-fs`
(G0) for the resolved corpus and must not re-implement path/discovery/read
logic. The open question is the **coupling shape** of that dependency.

### Option ii-a — evaluate directly over G0's concrete corpus type

`evaluate(&ContractRegistry, &agent_contract_fs::<CorpusType>) -> Vec<Diagnostic>`.

* Pro: no G1-local abstraction; one obvious type.
* Pro: impossible to drift from G0's real corpus shape.
* **Con (decisive for a draft dependency)**: G0's public corpus type is a
  **draft** (G0 is unmerged, E13). Binding G1's central `evaluate` signature —
  and therefore **every** assertion test — directly to a not-yet-reviewed
  concrete type maximises the blast radius of any G0 API change: a rename or a
  field change forces edits across all evaluator units and their tests.
* Con: G1's own tests would then need G0's real corpus builder to construct
  inputs, coupling G1's unit tests to G0's resolve/read entry points rather than
  to a tiny synthetic fixture G1 controls.

### Option ii-b — a G1-local trait abstraction (`ResolvedCorpus`) with a thin G0 adapter

G1 defines `trait ResolvedCorpus { fn files(&self) -> impl
Iterator<Item = CorpusFile<'_>>; }` and `struct CorpusFile<'a> { rel_path:
&'a str, content: &'a str }`. `evaluate` is generic over `C: ResolvedCorpus`.
G1 still declares the path dependency on G0 (honouring the fixed topology) and
provides **one** adapter `impl ResolvedCorpus for agent_contract_fs::<CorpusType>`.
G1's own assertion tests use an in-exerciser `FakeCorpus` implementing the
trait.

* Pro: G0 coupling is **confined** to two places — G1's `Cargo.toml` dependency
  line and the single adapter unit — so a G0 API change touches only the
  adapter, never the evaluator or its tests (directly addresses the plan's
  named G0-API-change risk).
* Pro: G1's assertion logic is exercised against a **tiny synthetic fixture**
  (`FakeCorpus`) that G1 owns, exactly as the scope permits, with no repository
  content and no G0 resolve run required for the bulk of testing.
* Pro: `CorpusFile` is a **borrowed view** over already-resolved data — it
  re-implements **no** path/discovery/read logic; G0 still owns all of that.
* Con: a small amount of abstraction (one trait, one view struct, one adapter)
  versus i-a's directness.
* Con: the view must faithfully mirror what G0 exposes; if G0 delivers paths and
  content separately, the adapter — not G1's logic — absorbs that shape.

### Selected (ii): ii-b — G1-local `ResolvedCorpus` trait with a thin G0 adapter

The decoupling is worth its small cost precisely because G0 is a draft. It
confines G0 API churn to the adapter, lets G1's evaluator and every assertion
test run against a G1-owned synthetic fixture, and still honours the fixed
"G1 depends on G0" topology (the Cargo path-dependency and the adapter impl are
retained). This also yields the crisp **empty-vs-failed** distinction below.

**Empty corpus vs failed-to-resolve (falls out of ii-b).** G0's resolution
returns `Result<Corpus, G0Error>`. A **resolution failure** is `Err` — it never
reaches G1's `evaluate` (the caller, G2, passes it through or wraps it without
G1 re-classification, per the no-new-filesystem-codes rule). An **empty corpus**
is `Ok` with zero files — a valid value that *does* reach G1, where `required`
and `exactly_one` assertions map n=0 to `MissingRequired` and `prohibited`
assertions trivially pass. The two are **type-distinct** (`Err` vs `Ok(empty)`),
so G1 never has to guess which occurred.

## Question (iii) — diagnostic ordering strategy

Decision 4 settles that diagnostics **across different assertions** are ordered
by `(assertion id, then code)`, both total orders. The plan-level question is
*how* that order is produced.

### Option iii-a — emit in evaluation order, no explicit sort

Rely on iterating the registry in declaration order and the corpus in G0's
order, emitting diagnostics as they arise.

* Pro: no sort step.
* **Con (decisive)**: it makes stable output an **implicit** consequence of two
  iteration orders rather than an explicit contract. It also contradicts
  Decision 4, which mandates an **explicit** `(assertion id, then code)` order,
  not "whatever order evaluation happened to produce". Any future change to
  evaluation order would silently change diagnostic order.

### Option iii-b — collect then sort on an explicit total key

Collect all diagnostics, then sort on the total key `(assertion_id, code)`,
extended with a third key `rel_path` to break ties among any per-file
diagnostics that share `(assertion_id, code)`.

* Pro: implements Decision 4's mandated order **explicitly**; independent of
  evaluation/iteration order.
* Pro: `assertion_id` uniqueness is **guaranteed** by G1's own
  `DuplicateAssertionId` validation, so `(assertion_id)` alone already totally
  orders diagnostics across assertions; `code` and `rel_path` are present for
  robustness and forward-compatibility but are never the deciding key in G1's
  set-scoped model.
* Pro: string comparison is byte-lexicographic and locale-independent in Rust;
  `rel_path` is `/`-normalised by G0 (split Decision 3, step 6), so the sort is
  **byte-identical on Windows and Unix**. `DiagnosticCode` is ordered by an
  explicit fixed ordinal, never by name hashing.
* Con: buffers all diagnostics before emitting (trivial for this corpus size).

### Selected (iii): iii-b — collect then sort on `(assertion_id, code, rel_path)`

This is the only option consistent with Decision 4's explicit-order mandate and
the cross-platform-stability requirement. The third key `rel_path` is a
**completion** of Decision 4 (a tie-break for any future per-file diagnostics),
not a re-argument of it; in G1's current set-scoped model each failing assertion
emits at most one diagnostic, so `assertion_id` alone decides.

## Inherited, not re-argued (reproducing split Decision 6)

The following are carried forward from the split deliberation's Decision 6 and
are **not open for re-derivation** in this plan or its review. They are restated
here so G1's reviewers inherit them rather than reopening them:

1. **No `max_depth` / `max_filesize` / walker filters.** Affirmed 7/7 upstream;
   a filesystem concern owned by G0, inert on G1 (G1 does no walking).
2. **Plain `std::fs` recursion, not `ignore::WalkBuilder`.** G0's concern;
   filter-level silent omission must remain structurally unexpressible.
3. **Zero new external dependencies.** Applied to G1: G1 adds **no** new
   external crate; it reuses only already-declared `serde` / `toml` / `thiserror`
   (E2), each justified.
4. **No `verify_markdown`, no `engram` runtime coupling, no product-runtime
   behaviour** in G0/G1/G2. G1 is a pure data-driven evaluator.
5. **Typed registry**, not stringly-typed assertions. G1's `AssertionForm` /
   `DiagnosticCode` are enums, not strings.
6. **Test-first with compiling red phases**; <=3 scenarios per unit. Every G1
   unit is an `[R]` (compiling, `todo!()` bodies) or `[G]` unit.
7. **Two-layer containment** (allow-listed roots + workspace canonical
   containment). G0's concern; G1 performs no path operations and re-classifies
   no containment outcome.

Additionally inherited and settled: **Decision 4** (the `exactly_one`
precedence — n=0 => `MissingRequired` only; n=1 => pass; n>1 =>
`AmbiguousMultiplicity` only; the two are arms of one `match` on n, so emitting
both is unrepresentable) is implemented **exactly as written**, not re-derived.

## Dependency on G0

G1 consumes the following **G0 public API items**. G0's plan owns their exact
names and signatures; the names below are **provisional** and MUST be reconciled
when G0 merges. This list is the complete G0 surface G1 touches — the trait
abstraction (ii-b) confines it to G1's `Cargo.toml` and the adapter unit.

1. **The crate `agent-contract-fs`** (`crates/agent-contract-fs`) as a
   `[dependencies]` path dependency of the G1 crate.
2. **G0's resolved-corpus type** (provisionally `agent_contract_fs::FileSet` or
   `ResolvedCorpus`) — the successful output of G0's resolve/read layer. G1
   needs from it, and only from it: an **ordered** iteration of files, each
   exposing a **workspace-relative, `/`-normalised path** (`&str`/`&Path`) and
   **UTF-8 content** (`&str`). G1 adapts this to its own `ResolvedCorpus` trait.
3. **G0's resolve error type** (provisionally `agent_contract_fs::ResolveError`
   or similar) — consumed **only** as an opaque `Err` that the caller (G2)
   passes through; G1 never matches on it and never re-classifies it.
4. **For the adapter test only**: G0's deterministic test double (provisionally
   `agent_contract_fs::FakeFs`) plus G0's resolve entry point, to build a small
   synthetic corpus without touching repository files. If G0 instead exposes a
   direct corpus constructor, the adapter test uses that; either way no real
   `.github` content is read by G1.

Everything else G1 needs — the registry schema, parsing, validation, the
evaluator, diagnostics, ordering, and rendering — is **owned by G1** and defined
against G1's own trait and a G1-owned synthetic fixture.

## Open questions deliberately left to the plan

* Exact work-unit boundaries and `[R]`/`[G]` pairing.
* The concrete synthetic fixture contents used by G1's own tests (tiny,
  non-repository-specific).
* Reconciliation of the provisional G0 type names once G0 merges.
