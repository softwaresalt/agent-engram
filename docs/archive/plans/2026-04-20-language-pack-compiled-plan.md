# Implementation Plan: Tree-sitter Parser Support for Swift, Kotlin, C, and C++

**Source**: [`docs/decisions/2026-04-20-group-b-language-coverage-deliberation.md`](../decisions/2026-04-20-group-b-language-coverage-deliberation.md)
**Selected option**: B — split Group B; ship compiled-language pack now, defer SQL+Markdown
**Stash IDs**: `0523404D` (covering); `D715B3EE` and `47F34E2C` deferred

## Problem Frame

Extend the tree-sitter parsing service in `src/services/parsing/` to support four
additional source languages — Swift, Kotlin, C, and C++ — by replicating the existing
per-language module pattern established for Rust, Python, TypeScript, JavaScript, Go,
and C#.

The change is additive: new submodules, new `Language` enum variants, new dispatcher
arms, and new file-extension mappings. No changes to the `ExtractedSymbol` /
`ExtractedEdge` IR. No changes to the storage or embedding layers.

Affected modules (existing, will be edited):
* `src/services/parsing.rs` — `Language` enum, `as_str`, `TryFrom<&str>`, mod decls, `parse_source` dispatch
* `src/services/code_graph.rs:1102-1117` — `language_from_path()` extension match
* `Cargo.toml` — four new `tree-sitter-{swift,kotlin,c,cpp}` deps
* `tests/unit/parsing_test.rs` — language-coverage tests

New modules (will be created):
* `src/services/parsing/swift.rs`
* `src/services/parsing/kotlin.rs`
* `src/services/parsing/c.rs`
* `src/services/parsing/cpp.rs`

## Requirements Trace

| Requirement (from deliberation) | Implementation Action |
|---|---|
| Add Swift parser | Unit S1 (research/spike) → S2 (parser submodule) → S3 (wiring + tests) |
| Add Kotlin parser | Unit K1 (research/spike) → K2 (parser submodule) → K3 (wiring + tests) |
| Add C parser | Unit C1 (parser submodule) → C2 (wiring + tests) |
| Add C++ parser | Unit P1 (parser submodule) → P2 (wiring + tests) |
| Maintain ABI 14 constraint | Pin all new crates to ABI-14-compatible versions (see Decisions) |
| No IR or storage changes | Reuse existing `ExtractedSymbol::{Function,Class,Interface}` variants exactly |
| Match existing test discipline | Each parser module has integration test in `tests/unit/parsing_test.rs` proving symbol+edge extraction on a representative source |

## Implementation Units

The work decomposes into 13 units across 4 sub-epics + 1 shared-infra sub-epic. Each
unit obeys the 2-hour rule, single skill domain, and produces an atomic, verifiable
outcome.

### Sub-epic SI: Shared Infrastructure (1 unit)

#### SI-1: Wire 4 new languages into Language enum and dispatcher

**Scope**: Single edit to `src/services/parsing.rs` adding four enum variants
(`Swift`, `Kotlin`, `C`, `Cpp`), corresponding `as_str()` arms, `TryFrom<&str>` arms,
4 new `mod {lang};` declarations, and 4 new `Language::* => {lang}::parse_*_source(...)`
arms in `parse_source()`. Edit `src/services/code_graph.rs:1102-1117` adding
`"swift" => "swift"`, `"kt" | "kts" => "kotlin"`, `"c" | "h" => "c"`,
`"cpp" | "cc" | "cxx" | "hpp" | "hxx" | "h++" => "cpp"`.

**Files**: 2 (`src/services/parsing.rs`, `src/services/code_graph.rs`)
**Tests**: parsing.rs already has `Language` round-trip coverage; one new test asserting `TryFrom<&str>` for the 4 new identifiers + `as_str` round-trip.
**Posture**: test-first
**Notes**: Submodules at this stage are **no-op stubs** returning `Ok(ParseResult { symbols: vec![], edges: vec![] })` so the crate compiles AND files of these languages can be safely indexed (silently producing no symbols) between SI-1 landing and the per-language sub-epics completing. Real implementations replace the no-op body in A-2/B-2/C-1/D-1.

### Sub-epic A: Swift parser (3 units)

#### A-1: Verify tree-sitter-swift ABI 14 compatibility (spike)

**Scope**: Create a throwaway integration test that imports `tree_sitter_swift::LANGUAGE`
and calls `Parser::set_language()` against the runtime tree-sitter 0.24. If it errors
with "Incompatible language version", document the failure, search older crate
versions (`cargo info tree-sitter-swift@0.6`, `0.5`, etc.) until one binds successfully,
and pin that version in `Cargo.toml`. If no version is ABI-14 compatible, halt this
sub-epic and surface to operator.

**Files**: 1 throwaway test file (deleted at end of unit), `Cargo.toml` (pin choice)
**Tests**: smoke test passes binding
**Posture**: spike (time-box: 2h)
**Acceptance**: `cargo test --test parsing_test test_swift_grammar_loads` passes (or sub-epic halted with documented finding)

#### A-2: Implement `src/services/parsing/swift.rs`

**Scope**: New file mirroring `csharp.rs` shape. Walks tree, extracts:
* Functions / methods → `ExtractedSymbol::Function`
* Classes / structs / actors → `ExtractedSymbol::Class`
* Protocols → `ExtractedSymbol::Interface`
* Calls inside function bodies → `ExtractedEdge::Calls`
* Top-level definitions → `ExtractedEdge::Defines`
* `import` statements → `ExtractedEdge::Imports`

Tree-sitter node names: function → `function_declaration`, class → `class_declaration`,
struct → `protocol_declaration` (verify against grammar). Validate node names against
the alex-pinkus/tree-sitter-swift grammar repo before writing matchers.

**Files**: 1 new (`src/services/parsing/swift.rs`)
**Tests**: deferred to A-3
**Posture**: characterization-first (write parser, verify against representative Swift sample)
**Acceptance**: parser compiles; `parse_swift_source(VALID_SWIFT_SOURCE)` returns `Ok(_)` without panic; produces ≥1 symbol

#### A-3: Integration test for Swift parser

**Scope**: New test in `tests/unit/parsing_test.rs` named `test_swift_parsing` covering
a representative source (function, class, protocol, import, internal call). Asserts
specific symbol count, specific edge count, and at least one symbol of each variant.

**Files**: 1 (`tests/unit/parsing_test.rs`)
**Tests**: 1 test scenario
**Posture**: test-first (test written from spec, then verified after A-2 implementation)
**Acceptance**: `cargo test --test parsing_test test_swift` green

### Sub-epic B: Kotlin parser (3 units)

#### B-1: Verify tree-sitter-kotlin ABI 14 compatibility (spike)

Same shape as A-1. Verify `tree-sitter-kotlin@0.3.8` against tree-sitter 0.24
runtime. If incompatible, walk older versions until ABI 14 found, or halt sub-epic.

**Posture**: spike (time-box: 2h)

#### B-2: Implement `src/services/parsing/kotlin.rs`

Same shape as A-2. Kotlin node kinds:
* `function_declaration` → Function
* `class_declaration` → Class (handle `data class`, `sealed class`)
* `interface_declaration` → Interface
* `import_header` → Imports edge
* call expressions inside bodies → Calls

**Posture**: characterization-first

#### B-3: Integration test for Kotlin parser

Same shape as A-3. New `test_kotlin_parsing` test scenario.

**Posture**: test-first

### Sub-epic C: C parser (2 units — no spike, ABI 0.23.4 confirmed available)

#### C-1: Implement `src/services/parsing/c.rs`

**Scope**: New file using `tree-sitter-c@0.23.4`. C does NOT have classes or
interfaces. Map:
* `function_definition` → Function
* `struct_specifier` (with name) → Class
* `#include` preprocessor directive → Imports edge
* call expressions in function bodies → Calls
* No Interface variant emitted

**Files**: 1 new
**Posture**: characterization-first
**Acceptance**: parses representative C source without panic; emits Function + Class symbols

#### C-2: Integration test for C parser

Same shape as A-3.

### Sub-epic D: C++ parser (2 units — ABI 0.23.4 confirmed available)

#### D-1: Implement `src/services/parsing/cpp.rs`

**Scope**: New file using `tree-sitter-cpp@0.23.4`. Map:
* `function_definition` (incl. methods inside `class_specifier`) → Function
* `class_specifier` → Class
* `struct_specifier` (with name) → Class
* `#include` → Imports edge
* call expressions → Calls
* No Interface variant (C++ has no first-class interfaces; abstract classes treated as Class)

Skip template-instantiation nodes; extract only top-level declarations and
in-class methods to keep graph noise low. Document this scope decision in the
module doc comment.

**Files**: 1 new
**Posture**: characterization-first
**Acceptance**: parses representative C++ source including a class with methods; emits Function + Class symbols; no panic on templates

#### D-2: Integration test for C++ parser

Same shape as A-3. Includes a class with two methods + free function + `#include`.

## Dependency Graph

```text
SI-1 ──┬──> A-1 ──> A-2 ──> A-3
       ├──> B-1 ──> B-2 ──> B-3
       ├──> C-1 ──> C-2
       └──> D-1 ──> D-2
```

* SI-1 is the only blocking unit — all language work depends on it
* Within each sub-epic the units are sequential
* The 4 sub-epics are independent of each other and can execute in parallel after SI-1
* Total critical-path length: 4 units (SI-1 → A-1 → A-2 → A-3, the longest sub-epic)

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| **One enum variant `Cpp` (not `CPlusPlus`)** | Matches Rust naming conventions and existing `Tsx` brevity precedent |
| **Tier 0 ABI spike for Swift and Kotlin only** | C/C++ confirmed at `0.23.4` matching existing dep line; Swift/Kotlin use independent versioning and are unverified |
| **C struct → Class variant** (not a new IR variant) | Preserves IR surface; avoids storage-layer churn during CozoDB migration. Loses some semantic precision (struct ≠ class) but acceptable for code-graph use cases |
| **Skip C++ template instantiations** | Templates produce noisy graph entries; pattern matches C# generic-handling decision in `csharp.rs` |
| **No Interface variant for C/C++** | C has no interfaces; C++ has no first-class interfaces. Forcing a mapping (e.g., abstract-class-as-Interface) requires AST-level analysis beyond top-level node kinds and adds 4–6h per language |
| **Stub submodules in SI-1** | Allows SI-1 to land independently, unblocking parallel sub-epic work, and keeps the `Language` enum extension reviewable as one focused PR-prep change |

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `tree-sitter-swift@0.7.1` emits ABI 15+ | Medium | Drops Swift from this release | A-1 spike walks back through older versions; fall back to git-pin or skip Swift if no ABI 14 version exists |
| `tree-sitter-kotlin@0.3.8` emits ABI 15+ | Medium | Drops Kotlin from this release | Same as Swift |
| C++ template/macro AST produces unexpected node kinds | Low | Parser panics on edge-case input | D-1 acceptance includes "no panic on templates"; characterization test includes a templated example |
| Swift/Kotlin grammar repos use different node-kind names than expected | Medium | A-2/B-2 needs revision | Validate node names against the grammar repo's `node-types.json` BEFORE writing matchers |
| Kotlin grammar coverage of newer language features is incomplete (sealed classes, coroutines) | Low | Some symbols missed | Acceptable; document in module doc comment; can be expanded later |
| `tree-sitter-c@0.23.4` and `tree-sitter-cpp@0.23.4` both define `LANGUAGE` const at top level | Very low | Compile-time symbol conflict | Standard pattern; `tree_sitter_c::LANGUAGE` and `tree_sitter_cpp::LANGUAGE` are namespace-isolated |

## Plan Hardening Signals

* public API, schema, or contract change: **absent** — additive enum variants only; no behavioral break for existing callers
* security, auth, permission, or compliance-sensitive behavior: **absent** — parsers are pure functions over input strings
* migration, backfill, destructive data/config action, or irreversible step: **absent** — no data migration; no irreversible action
* external integration, operator checkpoint, or external dependency: **absent** — only crates.io grammar deps, all standard
* high runtime, rollout, or rollback risk: **absent** — additive language enum; rollback = revert PR

**Requires plan hardening: no**

The work is purely additive and replicates an established pattern (026-F). Two
spike units (A-1, B-1) handle the only meaningful unknowns. No production behavior
changes for existing languages.

## Runtime Verification and Closure

| Aspect | Detail |
|---|---|
| Runtime surface changed | None directly. Code-graph indexing now accepts 4 new file extensions; existing workspaces will start indexing matching files on next sync |
| Runtime verification | Manual: index a small repo containing a Swift, Kotlin, C, and C++ file each; verify symbols appear in `list_symbols`/`unified_search` results without errors |
| Operational closure | No monitoring or rollback artifact required (additive change, no destructive path). PR description should call out the four new extensions so users with mixed-language repos understand new files will be indexed |
| Validation window | None required — additive change with no degradation surface |

## Verification

* `cargo fmt --all -- --check` passes
* `cargo clippy -- -D warnings -D clippy::pedantic` passes (zero new warnings)
* `cargo test` passes including 4 new language tests
* `cargo build --release` succeeds (proves grammar crates link)

## Estimated Effort

* SI-1: 1.5h
* Sub-epic A (Swift, with spike): 5h (spike 2h + parser 2h + test 1h)
* Sub-epic B (Kotlin, with spike): 5h
* Sub-epic C (C, no spike): 3h (parser 2h + test 1h)
* Sub-epic D (C++, no spike): 3.5h (parser 2.5h + test 1h)

**Total: ~18h serial; ~6.5h critical path** with parallel sub-epic execution after SI-1 (1.5h SI-1 + longest sub-epic Swift at 5h)

If both Swift and Kotlin spikes fail, scope shrinks to C+C++ only (~8h, 4 tasks).


## Plan Review

**Reviewer**: Consolidated inline review (Constitution + Rust + Scope Boundary + Learnings personas)
**Date**: 2026-04-20
**Hardening required**: no — confirmed by reviewer (all five hardening signals genuinely absent for additive parser pattern)

### Gate Decision: **ADVISORY**

The plan is fundamentally sound — it faithfully replicates the established 026-F pattern, respects the ABI 14 constraint, declares hardening signals correctly (all absent), and stays within the deliberation-confirmed scope (no SQL, no Markdown, no IR extension). One P1 finding requires plan adjustment before harvest; remaining findings are advisory.

### Findings

#### P1 — Intermediate breakage risk in SI-1

**Issue**: SI-1 lands `language_from_path()` edits that map `.swift`, `.kt`, `.c`, `.cpp` to language identifiers, AND lands stub submodules that return `Err("not implemented")`. After SI-1 merges and before sub-epics A/B/C/D land, any user workspace containing those file types would have indexing fail on those files (dispatcher calls stub → returns Err → CodeGraph::ParseFailed).

The agent-engram repo itself contains no Swift/Kotlin/C/C++ files so CI won't catch this, but downstream users with mixed-language workspaces would see new indexing errors after pulling main between SI-1 and the language sub-epics completing.

**Recommendation**: Adjust SI-1 to make stubs return `Ok(ParseResult { symbols: vec![], edges: vec![] })` (silent no-op) instead of `Err`. Each language sub-epic's parser-implementation unit (A-2, B-2, C-1, D-1) replaces the no-op with the real parser. This keeps SI-1 safely landable in isolation and removes the inter-unit ordering hazard.

Alternatively: split SI-1 into two parts — (a) Language enum + dispatcher arms only, (b) `language_from_path` extension mapping deferred and folded into each sub-epic's final unit. This is cleaner architecturally but adds 4 micro-edits; the no-op-stub approach is simpler.

**Required action**: Update SI-1 scope (no-op stubs, not Err stubs) before harvest.

#### P2 — Node-kind validation method underspecified

**Issue**: A-2 and B-2 say "Validate node names against the grammar repo before writing matchers" but don't specify how. The grammar's `node-types.json` is the canonical source.

**Recommendation**: Each parser-implementation unit's acceptance criteria should explicitly include "node kinds verified against `tree-sitter-{lang}/src/node-types.json` and listed in module doc comment." This makes the validation auditable.

#### P2 — C parser missing call-edge handling for function pointers

**Issue**: C-1 lists `call_expression` for Calls edges but doesn't mention function-pointer call sites (`(*fn_ptr)(args)` or `fn_ptr(args)`). C codebases use these heavily.

**Recommendation**: C-1 should explicitly state whether function-pointer calls are in or out of scope. Recommend OUT (matches conservative approach used by other languages); document in module doc comment.

#### P3 — Effort estimate doesn't show parallel critical path

**Issue**: 18h total assumes serial execution. With SI-1 done, sub-epics A/B/C/D are independent and parallelizable. Critical path is actually SI-1 (1.5h) + longest sub-epic (Swift, 5h) = 6.5h if executed in parallel.

**Recommendation**: Note critical-path estimate (6.5h parallel) alongside total (18h serial) so Ship can plan accordingly.

#### P3 — Missing CHANGELOG / user-facing note

**Issue**: PR will add 4 new auto-indexed file extensions; users with mixed-lang workspaces should know.

**Recommendation**: Add a "PR description must mention new extensions" note to operational closure section.

### Constitution Mapping

| Principle | Status |
|---|---|
| I. Safety-First Rust | ✓ No unsafe; all error paths use Result/`?` |
| II. Test-First Development | ✓ Each parser has dedicated test unit (A-3/B-3/C-2/D-2); A-1/B-1 spikes time-boxed |
| III. Workspace Isolation | N/A (pure functions over input strings) |
| IV. CLI Containment | N/A |
| V. Structured Observability | N/A (parsers don't emit traces; existing surrounding code-graph instrumentation covers this) |
| VI. Single Responsibility | ✓ Four narrow grammar deps justified by feature scope |
| VII. Destructive Approval | N/A |
| VIII. Safety Modes | N/A (no hardening signals) |
| IX. Git-Friendly Persistence | N/A |
| X. Context Efficiency | ✓ Parsers produce IR, not raw content |

### Coverage of Existing Patterns

* Faithfully follows 026-F pattern (`csharp.rs` / `rust.rs` exemplars)
* ABI 14 constraint correctly cited from compound learning `tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md`
* C++ template skipping decision matches C# generic-handling precedent
* Spike-then-implement for unverified grammars (Swift/Kotlin) is the correct pattern given the ABI risk

### Required Plan Adjustment Before Harvest

Update SI-1 scope to specify **no-op stubs** (`Ok(ParseResult { symbols: vec![], edges: vec![] })`) rather than Err stubs. This is a small wording change to the plan; no structural change to the unit hierarchy. Once that adjustment is made, the plan is harvest-ready.

### Decision

**ADVISORY**: Make the P1 adjustment (no-op stubs in SI-1), then proceed to harvest. P2/P3 findings can be incorporated into individual task acceptance criteria during harvest or addressed during implementation.

<!-- plan-review-attempt: 2 -->

## Plan Review (Attempt 2)

**Reviewer**: Consolidated inline review (re-issue after P1 adjustment)
**Date**: 2026-04-20

### Gate Decision: **PASS**

P1 finding addressed: SI-1 stubs now specified as no-op success returns. P3 critical-path note added. P2 findings (node-kind validation method, function-pointer scope, CHANGELOG note) deferred to be folded into individual task acceptance criteria during harvest — they do not block the gate.

Plan is harvest-ready.
