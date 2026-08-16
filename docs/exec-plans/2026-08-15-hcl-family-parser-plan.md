---
title: "Shared HCL parser for Terraform-family files"
date: "2026-08-15"
status: "reviewed"
source: "docs/decisions/2026-08-15-hcl-family-parser-deliberation.md"
spike: "docs/decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md"
stash_id: "4BC7A6DE"
deliberation_id: "018-D"
feature_id: "121-F"
review_id: "121.001-R"
shipment_id: "117-S"
plan_review: "passed"
adversarial_review: "passed"
---

# Shared HCL Parser for Terraform-Family Files

## Problem Frame

Engram's code graph cannot discover or parse HCL/Terraform-family files. Add one canonical `hcl` language service for `.hcl`, `.tf`, and `.tfvars`, using the existing enum dispatcher, graph persistence, traversal, and local-daemon sync paths. Reliability and dependency integrity outrank breadth: v1 is syntactic and fail-closed, not a Terraform evaluator.

The source decision is `docs/decisions/2026-08-15-hcl-family-parser-deliberation.md`; the read-only compatibility evidence is `docs/decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md`.

## Scope and Invariants

### In Scope

- Exact registry dependency `tree-sitter-hcl = "=1.1.0"` with lock checksum verification.
- One `Language::Hcl` parser and canonical `hcl` identity for all three extensions.
- Top-level block and attribute symbols plus syntactic traversal references.
- Startup discovery/default enablement and live-sync routing.
- Contract, unit, and integration RED harnesses before production dependencies.
- Configuration documentation, local-daemon runtime verification, and operational closure.

### Out of Scope

- HCL1, JSON-form HCL, `.tf.json`, Terraform plans/state, evaluation, provider schemas, type inference, module download, cross-workspace target resolution, and schema/database migration.
- Separate Terraform parser variants, plugin/trait framework redesign, vendored grammar code, Git dependencies, and unsafe language-handle conversion.

### Protected Invariants

- `#![forbid(unsafe_code)]` remains effective for Engram; no workspace `unsafe` block is added.
- Every filesystem path remains workspace-contained and `.gitignore` aware.
- Existing language identities and routing remain byte-for-byte compatible.
- Unknown or malformed HCL cannot crash the daemon or create fabricated resolved targets.
- All three extensions share one grammar and one canonical language identity.
- Existing unresolved-reference self-loop plus `target_hint` behavior remains additive; no schema migration.

## Requirements Trace

| ID | Requirement | Planned units |
|---|---|---|
| R1 | Detect `.hcl`, `.tf`, `.tfvars` as canonical `hcl` | U2, U3, U7, U8 |
| R2 | Register one composable HCL parser | U2, U4, U5 |
| R3 | Extract top-level block and attribute symbols | U1, U2, U3, U5 |
| R4 | Extract normalized traversal references | U1, U2, U3, U6 |
| R5 | Include files in startup traversal and default config | U3, U7 |
| R6 | Route created/modified files through live sync | U3, U8 |
| R7 | Prove Rust/tree-sitter 0.25 compatibility | U2, U4, U5 |
| R8 | Contract, unit, integration test-first delivery | U1, U2, U3, all production units |
| R9 | Preserve containment and fail-closed behavior | U3, U7, U8, U10 |
| R10 | Document support and operational expectations | U9, U10 |
| R11 | One cohesive release, tasks under two hours and single-domain | U1-U10 |

## Public Extraction Contract

### Canonical Language

`language_from_path` returns `hcl` for all three case-sensitive repository extensions. `Language::try_from("hcl")`, `Language::as_str()`, and `parse_source` use one `Language::Hcl`. `hcl` is added to default supported languages. No `terraform` language alias is stored.

### Symbols

For each top-level `block`, join its header identifier/string segments with dots and emit one `ExtractedSymbol::Class` plus `Defines`: `resource.aws_instance.web`, `data.aws_ami.ubuntu`, `module.vpc`, `variable.region`, `output.endpoint`, `provider.aws`, `terraform`, or equivalent generic HCL block headers. For each top-level `attribute`, emit a class symbol named by its key; this gives `.tfvars` first-class symbols. Bodies, hashes, line ranges, and token counts follow existing structural-symbol conventions. Nested attributes are not separate v1 symbols.

### References

Within attribute and block-body expressions, normalize traversal chains from `variable_expr` plus `get_attr` segments into one dotted target, including `var.region`, `local.name`, `module.vpc.id`, `data.aws_ami.ubuntu.id`, and `aws_vpc.main.id`. Preserve only syntactic evidence; do not evaluate indexes, functions, conditionals, providers, or modules. Deduplicate identical `(source context, target)` references deterministically. Persist through existing `ExtractedEdge::References`; unresolved targets retain a file self-loop and `target_hint`.

Malformed files may yield partial syntax only when declaration/traversal nodes are unambiguous; otherwise return no fabricated symbols/edges. Grammar initialization or parser failure returns `EngramError` and is recorded per-file without terminating the daemon.

## Implementation Units

### U1 — Contract RED Harness

- Domain: contract tests only.
- Files: `tests/contract/hcl_parser_contract_test.rs`.
- Estimate: 60-90 minutes; at most three scenarios.
- Posture: test-first; compile against current public MCP/IPC surfaces and fail behaviorally before implementation.
- Scenarios: (1) `list_symbols` reports canonical HCL declarations from `.tf`, `.tfvars`, `.hcl`; (2) `map_code` exposes expected reference target hints without false resolved targets; (3) valid mixed-extension indexing returns no daemon/tool error.
- Exit: harness compiles and fails with explicit missing-HCL expectations.

### U2 — Unit RED Harness

- Domain: unit tests only.
- Files: `tests/unit/hcl_parsing_test.rs`, `tests/unit/retrieval_eval_language_gate_test.rs`.
- Estimate: 90-120 minutes; at most three scenario groups.
- Posture: test-first; obtain language through `Language::try_from("hcl")` so the harness compiles before the variant exists.
- Scenarios: (1) canonical language and extension aliases; (2) block/top-level-attribute symbol names and metadata; (3) normalized/deduplicated traversals plus malformed-input fail-closed behavior.
- Exit: compile succeeds and expectations fail before production changes.

### U3 — Integration RED Harness

- Domain: integration tests only.
- Files: `tests/integration/hcl_indexing_test.rs`.
- Estimate: 90-120 minutes; three scenarios.
- Posture: test-first with isolated temporary workspaces and bounded waits.
- Scenarios: (1) startup discovery indexes one file of each extension through one `hcl` identity; (2) created/modified extension events route to reindex and explicit sync updates symbols without duplicates; (3) real local daemon IPC lists/maps HCL symbols and survives malformed HCL.
- Exit: harness compiles and fails for missing routing/parser support, never from an unbounded wait.

### U4 — Exact Dependency and Provenance Gate

- Domain: dependency metadata only.
- Files: `Cargo.toml`, `Cargo.lock`.
- Estimate: 30-60 minutes.
- Depends on: U1, U2, U3.
- Change: exact-pin `tree-sitter-hcl = "=1.1.0"`; verify lock checksum `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`, Apache-2.0, `tree-sitter-language 0.1`, and one `tree-sitter 0.25.x` runtime.
- Exit: dependency diff is limited to the expected grammar/build graph; no Git/path dependency and no workspace source change.

### U5 — Shared Parser Registration and Declaration Symbols

- Domain: Rust parser implementation only.
- Files: `src/services/parsing/hcl.rs`, `src/services/parsing.rs`.
- Estimate: 90-120 minutes; fewer than five new functions.
- Depends on: U4 and RED U1-U3.
- Change: register `Language::Hcl`, initialize `tree_sitter_hcl::LANGUAGE.into()`, parse through one module, and extract top-level block/attribute structural symbols with `Defines`.
- Exit: U2 declaration tests turn green; no traversal-reference implementation beyond test scaffolding.

### U6 — HCL Traversal Reference Extraction

- Domain: Rust parser extraction only.
- Files: `src/services/parsing/hcl.rs`.
- Estimate: 90-120 minutes; fewer than five functions and three scenario groups.
- Depends on: U5.
- Change: walk expression traversals, build conservative dotted targets, associate source context, deduplicate deterministically, and emit `References`.
- Exit: U1/U2 reference expectations turn green with no evaluation or schema inference.

### U7 — Canonical Detection, Discovery, and Default Enablement

- Domain: startup routing/config code only.
- Files: `src/services/code_graph.rs`, `src/models/config.rs`.
- Estimate: 60-90 minutes.
- Depends on: U5 and RED U2/U3.
- Change: map the three extensions to `hcl`, include `hcl` in default supported languages, and preserve retrieval-eval delegation to the canonical mapper.
- Exit: discovery/default startup portions of U2/U3 turn green; no unrelated language defaults change.

### U8 — Live-Sync HCL Routing

- Domain: daemon event routing only.
- Files: `src/daemon/debounce.rs`.
- Estimate: 45-75 minutes.
- Depends on: U7 and RED U3.
- Change: classify created/modified `.hcl`, `.tf`, `.tfvars` as `ReindexFile` by delegating HCL-family recognition to the canonical language mapper rather than duplicating three extension literals; preserve delete/rename semantics.
- Exit: live-sync U3 scenario is green and all existing markdown/source routing remains unchanged.

### U9 — User-Facing Configuration Documentation

- Domain: documentation only.
- Files: `docs/configuration.md`, `README.md` only if it contains the canonical supported-language inventory.
- Estimate: 45-60 minutes.
- Depends on: U7, U8.
- Change: document `hcl` default support, extension aliases, syntactic-only graph semantics, and explicit non-goals.
- Exit: docs match runtime vocabulary and contain no promise of Terraform evaluation.

### U10 — Local-Daemon Runtime Verification and Closure

- Domain: runtime operations and closure documentation only.
- Files: `docs/closure/{date}-hcl-family-parser-operational-closure.md`.
- Estimate: 60-90 minutes.
- Depends on: U1-U9.
- Change: Ship runs targeted then full quality gates, exercises cold-start index, live modification plus sync, list/map queries, malformed-file resilience, restart idempotence, and records evidence.
- Exit: closure names healthy/failure signals, owner, 30-minute local observation window, rollback trigger, and revert procedure.

## Dependency Graph

```text
U1 contract RED ─┐
U2 unit RED ─────┼──> U4 exact dependency ──> U5 parser/symbols ──> U6 references
U3 integration RED┘                              └──> U7 detection/default ──> U8 live sync
                                                                 U7/U8 ──> U9 docs
U1-U9 ─────────────────────────────────────────────────────────────────> U10 closure
```

Production work must not begin until U1-U3 compile and fail for their intended missing-HCL assertions. U4 is the first production/dependency mutation. U5-U8 may proceed only in dependency order.

## Decisions and Rationale

1. One `hcl` identity: extension aliases represent one syntax family and duplicate parsers would drift.
2. Direct enum dispatch: follows the shipped architecture; no speculative parser trait.
3. Registry exact pin: reproducible checksum and compatible bridge outweigh unreleased upstream 1.2.0.
4. Crates.io artifact is authority: the historical `v1.1.0` Git tag is not source-equivalent to the published crate; this mismatch is documented, not hidden.
5. Default enablement: the requested feature should work for Terraform-family projects without a silent config opt-in.
6. Structural class symbols: reuse current graph schema and avoid migration; block headers and top-level attributes are stable syntactic declarations.
7. Syntactic traversals only: reliability requires not pretending to evaluate Terraform.
8. Existing reference persistence: additive target hints already support unresolved schema objects and require no database change.

## Risks and Caveats

- Supply-chain/provenance mismatch between Git tag and registry artifact: exact pin/checksum, owner/license review, lock diff, audit, and no Git substitution.
- ABI mismatch only discoverable at runtime: U2/U5 grammar-load test blocks implementation completion.
- Generated dependency FFI contains conventional upstream unsafe internals: no Engram unsafe code; existing crate boundary and audit policy apply.
- HCL traversal AST edge cases may overclaim: conservative normalization, malformed fixtures, no evaluation, and dedup tests.
- Default indexing adds workload in repositories with Terraform: existing file-size, ignore, hash-skip, concurrency, and per-file error controls remain; monitor counts/duration.
- Live-sync drift: route through canonical HCL detection and test startup/live parity.
- Existing `References` comments mention SQL: update only comments needed for generic semantics; do not redesign persistence.

## Plan Hardening Signals

| Signal | Present | Rationale |
|---|---|---|
| Public API, schema, or contract change | Yes | Adds a canonical language and graph extraction contract; no DB schema change. |
| Security, auth, permission, or compliance-sensitive behavior | Yes | Adds an external generated C/Rust grammar dependency and new parsed input surface. |
| Migration, backfill, destructive, or irreversible action | No | Existing files reindex additively; rollback is release revert and reindex. |
| External integration/operator checkpoint/dependency | Yes | crates.io dependency provenance and runtime ABI must be gated. |
| High runtime, rollout, or rollback risk | Yes | Local daemon startup/live-sync behavior and default workload change. |

**Requires plan hardening: yes**

## Runtime Verification and Closure

### Prechecks

- Lockfile resolves exactly `tree-sitter-hcl 1.1.0`, official checksum, one `tree-sitter 0.25.x`, and existing `tree-sitter-language` bridge.
- No workspace `unsafe` added; no path/Git dependency; all fixtures live in isolated temporary workspaces.
- U1-U3 RED evidence exists before U4; targeted tests pass before full gates.

### Runtime Scenarios

1. Cold-start a local daemon on an isolated workspace containing valid `.tf`, `.tfvars`, and `.hcl`; verify one canonical `hcl` file identity per file and expected block/attribute symbols.
2. Query `list_symbols` and `map_code`; verify normalized target hints and no fabricated cross-file resolution.
3. Modify a `.tf` file; verify live routing marks reindex, explicit sync updates changed symbols, and a restart produces no duplicates/stale symbols.
4. Add malformed HCL; verify a bounded per-file error/empty conservative result without daemon or IPC failure.

### Monitoring and Rollback

Because this is a local MCP daemon without a hosted dashboard, closure records a manual checklist: daemon health, index result `errors`, HCL files parsed/skipped, symbol/reference counts, sync duration against the same fixture baseline, and IPC success. Owner is the Ship operator for a 30-minute post-merge local observation window.

Rollback triggers: any daemon crash/IPC error from valid HCL; any of the three extensions not routed as `hcl`; symbol/reference counts diverge from deterministic fixtures after restart; workspace escape; unsafe shim; unexpected second tree-sitter runtime; or material indexing regression confirmed against the same fixture. Rollback is a merge-commit revert of the release followed by forced code-graph reindex; no data migration reversal is needed.

## Plan Hardening

Hardening is required because this release changes a default runtime parsing surface, introduces a generated native grammar dependency, expands the language contract, and affects startup plus live-sync routing. No schema migration or destructive operation is planned. Reinforcing context: `.github/instructions/strict-safety.instructions.md`, `.github/instructions/release-observability.instructions.md`, `.github/instructions/constitution.instructions.md`, `docs/archive/plans/2026-04-15-026-f-decided-plan.md`, and the compatibility spike.

### Hardened Dependency Gate

The only approved package choice is crates.io `tree-sitter-hcl = "=1.1.0"` with archive checksum `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`. Ship must inspect the lock diff before building. The resolved graph must contain one `tree-sitter 0.25.x`; the HCL crate must depend normally on `tree-sitter-language 0.1`; license must remain Apache-2.0. A changed checksum, yanked release, Git/path source, extra runtime tree-sitter, or source-provenance substitution blocks the shipment.

The published/tag mismatch is accepted only as documented registry provenance risk; it is not permission to build from the historical tag. Any alternate grammar/version requires a new read-only spike, operator decision, plan amendment, and fresh adversarial review. No unsafe conversion, transmute, copied FFI shim, or vendored generated source is an allowed remediation.

### Risky Actions

#### ProposedAction PA-1 — Add Exact External Grammar Dependency

- `summary`: Add checksum-locked `tree-sitter-hcl 1.1.0` from crates.io.
- `targets`: `Cargo.toml`, `Cargo.lock`, dependency build graph.
- `change_kind`: External dependency and generated native parser integration.
- `rollback`: Revert the release commit, restore the prior lockfile, and force code-graph reindex.
- `approval_required`: Yes for any package/version/source other than the operator-requested, reviewed exact pin; the reviewed pin is pre-authorized by this request.
- `ActionRisk`: high.
- `ActionResult`: planned.

#### ProposedAction PA-2 — Enable HCL by Default

- `summary`: Add canonical `hcl` to default languages and discover three extension aliases.
- `targets`: startup file discovery, local indexing workload, graph contents.
- `change_kind`: Runtime default/contract change.
- `rollback`: Revert default/routing changes and force reindex; no schema rollback.
- `approval_required`: No additional checkpoint; scope is explicitly requested.
- `ActionRisk`: moderate.
- `ActionResult`: planned.

#### ProposedAction PA-3 — Route HCL Live Events

- `summary`: Reindex created/modified `.hcl`, `.tf`, and `.tfvars` through canonical HCL detection.
- `targets`: daemon event adapter and explicit-sync pending set.
- `change_kind`: Local daemon runtime routing.
- `rollback`: Revert adapter change; deleted/renamed semantics remain unchanged.
- `approval_required`: No additional checkpoint; behavior is test-gated and explicitly requested.
- `ActionRisk`: moderate.
- `ActionResult`: planned.

### Protected Failure Behavior

- Grammar load failure is a typed per-file parse error; daemon startup and IPC stay available.
- Malformed HCL does not panic, execute expressions, access the network, or escape the workspace.
- Unsupported/ambiguous traversal shapes are skipped rather than guessed.
- Live-sync classification does not broaden unrelated languages or change delete/rename handling.
- Repeated index/sync is idempotent: no duplicate files, symbols, or references.

### Reinforced Verification Gates

1. **RED gate**: U1-U3 compile before any Cargo/source mutation and fail only on missing HCL behavior.
2. **Dependency gate**: exact version/checksum/license/source and single runtime tree-sitter are reviewed before grammar execution.
3. **ABI gate**: a targeted grammar-load/representative parse test must turn green before any extraction task is accepted.
4. **Extraction gate**: exact fixture symbols and traversal target hints match the documented contract; malformed/unsupported shapes fail closed.
5. **Routing parity gate**: startup discovery and created/modified live events recognize all three aliases through canonical `hcl`; existing languages and markdown retain prior outcomes.
6. **Containment gate**: only workspace-contained, ignore-eligible files are indexed.
7. **Runtime gate**: cold start, list/map, modified-file sync, restart idempotence, and malformed-file resilience pass on a local daemon with bounded waits.
8. **Full gate**: Ship runs format, clippy/pedantic, all tests, and audit in repository order. Stage does not run them.

### Operational Closure Detail

Healthy signals are: daemon health green; zero errors for valid fixtures; exactly three HCL files discovered; expected deterministic declaration/reference fixture counts; successful list/map IPC; changed symbol visible after sync; and identical graph counts after restart. Failure signals are any crash, IPC error, valid-file parse error, alias misclassification, unexpected target resolution, duplicate/stale node, workspace escape, checksum/source drift, or unsafe workspace code.

The pre-deploy audit confirms no migration, no feature flag, exact rollback procedure, dependency provenance, monitoring checklist, and all gates. Post-merge observation is 30 minutes on the local daemon, owned by Ship/operator. The closure record must state `healthy`, `degraded`, or `rolled back`; silence is not success. Any failure signal is an immediate rollback trigger.

### Unresolved Blocking Decisions

None for the reviewed exact pin and v1 syntactic scope. If the dependency or ABI gate fails, the safe outcome is `blocked` and return to Stage; Ship may not guess a replacement or widen scope.

## Plan Review Remediation — Cycle 1

This section resolves the initial standard multi-persona findings and is authoritative wherever it narrows or reslices the earlier U1-U10 draft. Harvest must use the revised U1-U14 units below. No original requirement is dropped.

### Tightened Extraction and Error Contract

V1 accepts only plain header segments: an HCL block symbol is emitted when the block type is an `identifier` and every label is a plain `string_lit` containing literal text only. Template/interpolation labels, dynamic labels, malformed headers, nested attributes, indexed traversals, splats, and traversal fragments with non-identifier segments are skipped. Top-level attributes require a plain identifier key.

A reference is emitted only for a contiguous `variable_expr` root followed by zero or more plain `get_attr` identifier segments. The root plus at least one attribute is required. `index`, legacy index, splat, template, function-call, and dynamically computed path forms are not normalized in v1. Identical `(source context, target)` pairs are deduplicated in deterministic encounter order. Fixed malformed fixtures assert the exact conservative partial-or-empty result; the implementation may not fabricate around `ERROR` nodes.

Grammar initialization/ABI failure and source syntax are distinct. `Parser::set_language(&tree_sitter_hcl::LANGUAGE.into())` uses only the crate's safe exported handle; no manual extern, raw handle, pointer shim, transmute, unsafe allowance, copied FFI, or shared parser cache is allowed. A grammar initialization error is a service defect that fails the ABI gate and blocks HCL enablement. Each parse owns a local parser. Malformed source is bounded per-file input: it may yield only allowlisted unambiguous nodes, records a typed file error when current index semantics support one, and never terminates daemon/IPC. Async clients observe indexing errors through the existing index/sync result and logs; list/map never invent results or a new client-specific error envelope.

### Single Canonical Routing Authority

Add `Language::from_path` (or equivalently named pure helper) in `src/services/parsing.rs` as the sole extension-to-canonical-language authority. It maps normalized extension aliases `.hcl`, `.tf`, and `.tfvars` to `Language::Hcl`. `code_graph::language_from_path` delegates to it while preserving raw-extension fallback for unknown files; startup discovery, ordinary/forced explicit sync, retrieval-eval's existing delegation, and `daemon::debounce` consume the same authority. No second HCL extension list or `terraform` token is allowed. Existing extension case behavior is preserved and locked by characterization tests rather than changed implicitly.

All exhaustive `Language` matches are updated directly. HCL reuses the current ignore-aware discovery, file-size guards, hash skip, scheduler, per-file error, persistence, and explicit-sync paths end-to-end. Live events are already workspace-relative and filtered by the watcher; HCL classification occurs only after those existing containment/exclusion controls. Tests include an ignored/out-of-workspace alias case without prescribing a new canonicalization branch that could change existing languages.

### Graph Semantics and Client Parity

`ExtractedSymbol::Class` is used only as the existing structural declaration carrier; documentation calls HCL entries structural symbols, never OO classes. The implementation must verify no HCL-specific consumer assumes OO semantics. Reference persistence keeps exactly one outcome per extracted reference: resolve to an actual existing class ID through the existing query, otherwise create only the file self-loop with `target_hint`; never emit both. Reindex remains idempotent.

Agent/MCP, IPC, and CLI wrappers share the existing backend/serializer. The canonical returned/stored token is only `hcl`; `.tf`, `.tfvars`, `.hcl` are file aliases and `terraform` is not an accepted language token. Contract/runtime checks compare shared response fields and error behavior without adding a new filter API. Startup, explicit sync, and live reindex must produce the same file identity, symbol count, reference count, and duplicate behavior.

### Security and Resource Gates

- Record crates.io URL, version, official SHA-256, lock diff, license, owners, resolved dependency tree, and attestation that no Git/path/vendored source was used.
- Block on checksum/yank/source drift, any new relevant advisory, a second runtime tree-sitter, or dependency deviation outside the reviewed HCL path.
- Reuse existing pre-read and post-read file-size guards. Add bounded fixtures for oversize rejection, deeply nested/malformed syntax, dynamic/template/index/splat skip rules, and symlink/reparse or ignored-path containment.
- Verify the parser receives an in-memory string only and has no network, subprocess, module/provider download, environment expansion, or external-file path. Review rejects any such API call/import; runtime closure attests zero side effects.
- Diff gate rejects new unsafe allowances, raw pointer/FFI shims, transmutes, vendored generated code, or weakening `forbid(unsafe_code)`.
- Rollback restores manifest/lock, performs a clean rebuild through normal Ship workflow, force-reindexes the same fixture workspace, and verifies pre-change graph expectations.

### Revised Implementation Units (Authoritative)

#### U1 — MCP/IPC Contract RED Harness

- Domain/files: contract tests only; `tests/contract/hcl_parser_contract_test.rs`.
- Estimate/scenarios: 60-90 minutes; (1) canonical `hcl` declarations returned for one compact mixed-extension fixture; (2) map response carries unresolved traversal target hints and no fabricated resolution; (3) index/sync error fields are consistent and bounded.
- Exit: compiles and fails only because HCL behavior is absent.

#### U2 — Parser Unit RED Harness

- Domain/files: unit tests only; `tests/unit/hcl_parsing_test.rs`.
- Estimate/scenarios: 90 minutes; (1) language/ABI entry obtained through `TryFrom`; (2) allowlisted block/top-level-attribute symbols; (3) plain traversal normalization, deterministic dedup, and dynamic/malformed skip behavior.
- Exit: compiles without direct dependency references and fails behaviorally.

#### U3 — Canonical Routing RED Harness

- Domain/files: unit tests only; `tests/unit/hcl_routing_test.rs`.
- Estimate/scenarios: 60-90 minutes; (1) three aliases produce only `hcl` with existing case behavior; (2) startup discovery and explicit sync share identity/default gating; (3) created/modified live routing delegates to the same classifier while delete/rename, ignored, and outside aliases retain existing safe outcomes.
- Exit: compiles and fails on missing canonical routing.

#### U4 — Security/Resource RED Harness

- Domain/files: unit tests only; `tests/unit/hcl_security_test.rs`.
- Estimate/scenarios: 60-90 minutes; (1) oversize HCL is rejected by existing guards; (2) deep/malformed/dynamic fixtures remain bounded and conservative; (3) parser boundary has no filesystem/network/subprocess/module-provider side effect and no unsafe workaround.
- Exit: compiles and exposes missing security behavior without executing external processes or network calls.

#### U5 — Local-Daemon Integration RED Harness

- Domain/files: integration tests only; `tests/integration/hcl_indexing_test.rs`.
- Estimate/scenarios: 90-120 minutes; (1) cold-start daemon indexes one file per alias under one `hcl` identity and serves list/map; (2) modified file plus explicit sync updates counts without duplicates; (3) restart/malformed fixture preserves daemon health with bounded polling.
- Exit: compiles and fails on absent HCL support, not timing ambiguity.

#### U6 — Exact Dependency Mutation

- Domain/files: dependency files only; `Cargo.toml`, `Cargo.lock`.
- Estimate: 30-45 minutes. Depends on U1-U5 RED evidence.
- Change/exit: exact registry pin and expected lock graph only. Provenance/audit evidence is verified here and carried to U14, but no docs/source are changed in this unit.

#### U7 — Canonical Language Identity and Grammar Registration

- Domain/files: Rust parser core only; `src/services/parsing.rs`, `src/services/parsing/hcl.rs`.
- Estimate: 75-105 minutes. Depends on U6.
- Change: `Language::Hcl`, all exhaustive conversions/dispatch, sole path classifier, local parser initialization through safe `LANGUAGE.into()`, and empty parse result before extraction.
- Exit: U2 ABI/language and U3 alias tests pass; extraction tests remain RED.

#### U8 — Top-Level HCL Declaration Extraction

- Domain/files: Rust parser implementation only; `src/services/parsing/hcl.rs`.
- Estimate: 75-105 minutes. Depends on U7.
- Change/exit: one shared tree walk/helper set emits allowlisted structural block/attribute symbols and `Defines`; corresponding U2/U1 assertions pass.

#### U9 — Conservative Traversal Reference Extraction

- Domain/files: Rust parser implementation only; `src/services/parsing/hcl.rs`.
- Estimate: 75-105 minutes. Depends on U8.
- Change/exit: extend the same walk/helpers with allowlisted dotted traversals, source context, deterministic dedup, and one existing persistence outcome; reference tests pass.

#### U10 — Startup Discovery and Default Enablement

- Domain/files: startup/config Rust only; `src/services/code_graph.rs`, `src/models/config.rs`.
- Estimate: 60-90 minutes. Depends on U7 and U3.
- Change/exit: delegate canonical mapping, add only `hcl` default, preserve raw unknown fallback and existing pipeline; startup/explicit-sync tests pass.

#### U11 — Live-Sync Routing Parity

- Domain/files: daemon routing Rust only; `src/daemon/debounce.rs`.
- Estimate: 45-75 minutes. Depends on U10 and U3.
- Change/exit: consume canonical path classifier for HCL ReindexFile routing after existing watcher filters; no duplicate extension literals; live parity tests pass.

#### U12 — User-Facing HCL Documentation

- Domain/files: docs only; `docs/configuration.md` and `README.md` only when its language inventory applies.
- Estimate: 45-60 minutes. Depends on U9-U11.
- Change/exit: canonical `hcl`, aliases, structural/syntactic semantics, unsupported forms, no `terraform` token, and no evaluation promise are accurate.

#### U13 — Local-Daemon Runtime Verification Evidence

- Domain/files: runtime verification only; `docs/closure/{date}-hcl-family-parser-runtime-verification.md` is the evidence output.
- Estimate: 60-90 minutes. Depends on U1-U12.
- Exit: targeted/full gates and all hardened runtime scenarios pass or shipment becomes blocked; no implementation remediation is performed inside this unit.

#### U14 — Operational Closure Record

- Domain/files: closure documentation only; `docs/closure/{date}-hcl-family-parser-operational-closure.md`.
- Estimate: 30-45 minutes. Depends on U13 passing evidence.
- Exit: provenance attestation, healthy/failure signals, owner/window, audit, rollback trigger/runbook, client parity, and final `healthy|degraded|rolled back` disposition are complete. Failed evidence remains blocked and returns to Stage/Ship planning; closure does not mutate code.

### Revised Dependency Graph

```text
U1 contract RED ─┐
U2 parser RED ───┤
U3 routing RED ──┼──> U6 dependency ──> U7 identity/ABI ──> U8 symbols ──> U9 refs
U4 security RED ─┤                              └──> U10 startup/default ──> U11 live
U5 daemon RED ───┘
U9 + U10 + U11 ──> U12 docs
U1-U12 ──────────> U13 runtime evidence ──> U14 closure
```

No merge/release is allowed unless RED evidence precedes U6, targeted tests pass, full repository gates pass, U13 passes, and U14 records healthy closure. Any failure sets the shipment/task blocked. Implementation fixes occur only in their owning U7-U11 task or are returned for a newly planned unit; runtime/closure tasks verify and document only.

## Plan Review

### Gate Decision

**PASS after remediation cycle 1.** Plan hardening was required and is present. Seven standard personas reviewed the plan independently: Constitution, Rust, Scope Boundary, Learnings, Architecture, Agent-Native Parity, and Security. The initial gate was FAIL because P1 findings remained; the authoritative `Plan Review Remediation — Cycle 1` resliced the work and tightened contracts. A combined multi-persona re-review returned `GATE PASS` with no remaining P0/P1 finding.

### Merged Findings and Decisions

| Finding group | Initial severity | Disposition |
|---|---|---|
| AST declaration/traversal rules could fabricate dynamic forms | P1 | Fixed: explicit node/segment allowlists and skip rules; deterministic malformed/dedup assertions. |
| Grammar ABI failure was conflated with malformed input | P1 | Fixed: ABI/init is a blocking service defect; malformed source is bounded per-file input. |
| Canonical identity and alias ownership were spread across modules | P1 | Fixed: U7 owns one `Language::from_path` authority; startup, explicit sync, retrieval delegation, and live routing consume it. |
| RED harnesses and runtime/closure work were too wide | P1 | Fixed: test surfaces split into U1-U5; runtime evidence U13 and closure U14 are separate single-domain units. |
| Dependency provenance/native parser resource risks lacked blocking evidence | P1 | Fixed: exact artifact attestation, advisory/source graph block, bounded oversize/deep/malformed tests, no-side-effect/unsafe diff gates. |
| Containment and live routing were not explicit enough | P1 | Fixed: existing watcher/traversal containment and ignore controls precede canonical classification; alias escape/ignored regression coverage added. |
| Agent/MCP, IPC, CLI error/filter parity was ambiguous | P1 | Fixed: no new filter API; only stored/returned `hcl`; shared backend response/error semantics and runtime parity checks. |
| Graph `Class` semantics and reference collisions were underspecified | P1/P2 | Fixed: structural-only documentation; existing single resolved-or-self-loop outcome; no competing edges; idempotence gate. |
| Safe tree-sitter API/parser ownership was implicit | P2 | Fixed: local parser, safe exported `LANGUAGE.into()`, no raw/unsafe/cache shim. |
| Retrieval-eval and provenance work risked scope creep | P2 | Fixed: no dedicated retrieval feature/test; only existing canonical delegation; U6 limited to manifest/lock mutation while evidence flows to closure. |
| Docs could precede extractor behavior or expose alternate tokens | P2/P3 | Fixed: U12 depends on U9-U11 and uses only canonical `hcl` plus extension aliases. |
| Rollback/full-gate failure handling was incomplete | P1/P2 | Fixed: explicit hard release stop, restore/clean rebuild/reindex parity runbook, and remediation returns to owning/new unit. |

### Remaining Advisory Dispositions

No implementation follow-up is required before harvest. Medium/low observations were either incorporated above or rejected as unnecessary API/framework expansion. In particular, a new parser trait, new client filter API, new persistence schema, and bespoke HCL scheduler were rejected to preserve simplicity and scope.

### Runtime Verification and Closure Assessment

U13/U14 now separate execution evidence from closure documentation. The release is blocked unless RED precedes production mutation, exact dependency/ABI/security gates pass, targeted and full repository gates pass, local daemon startup/sync/restart/client-parity scenarios pass, and closure records a healthy disposition.

### Review Statistics

- Review cycles: 1 fix cycle.
- Initial persona reports: 7.
- Initial raw findings: 37; merged into 12 actionable groups.
- Remaining P0/P1: 0.
- Final gate: PASS.

## Adversarial Review Remediation — Cycle 1

Three independent Copilot CLI reviewers used `gpt-5.4`, `claude-opus-4.6`, and `gemini-3.1-pro-preview`. This section resolves their consensus and is authoritative over earlier unit numbering. The final harvest uses U1-U16 below.

### Consensus Remediation

**HIGH P1 — RED/dependency order (all three reviewers): fixed.** U2 no longer claims to load the grammar ABI before the dependency exists. U1-U5 remain dependency-agnostic RED harnesses and precede U6. U7 is a separate test-only ABI/Engram-registration harness after U6 makes the crate importable and before source implementation U8. Its single test first proves the direct safe grammar handle loads, then fails on missing Engram `hcl` registration, preserving one RED test outcome.

**MEDIUM P1/P2 — structural symbol/reference collisions (majority): fixed.** HCL symbols are namespace-prefixed: `hcl.block.<header-segments>` and `hcl.attribute.<key>`. IDs remain file-scoped through existing persistence. HCL references never call the current global/name-first `resolve_reference_target` in v1; code-graph persistence creates only the file self-loop plus normalized `target_hint`. This prevents HCL symbols from capturing SQL references and prevents HCL hints from binding to unrelated classes. A future HCL-scoped unambiguous resolver requires a separate plan.

**MEDIUM P1/P2 — provenance/tag mismatch (majority): fixed/accepted with explicit exception.** The operator-authorized registry selection is the exact crates.io artifact, not the historical tag. Evidence now includes official archive SHA-256 `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`, exact published manifests/binding, owner/license metadata, dependency endpoint, and exact published
ode-types.json` SHA-256 `d86638c95d20335b960abb62f6758ab53f78fd0efbe4b6669473b5a20dfd1fb5`. The tag mismatch remains a documented provenance exception and blocks substitution, not the reviewed checksum-addressed release.

**LOW P0 — alleged tree-sitter version mismatch (single reviewer): rejected as factually contradicted.** Official crates.io dependency metadata and the matching downloaded archive show normal `tree-sitter-language = 0.1`, dev `tree-sitter = 0.25.3`, and no runtime dependency on tree-sitter 0.20. U6/U7 still fail closed if Cargo or ABI evidence differs.

**LOW P1 — AST evidence unresolved (single reviewer): resolved by evidence.** Stage safely streamed and parsed only the exact published archive's data; no grammar code ran. The allowlisted nodes exist in the artifact with the node-types hash above. U7-U10 runtime tests remain mandatory because static AST metadata does not prove parse behavior.

### Further Advisory Dispositions

- Parser output uses `Vec` encounter order plus a membership set (or an order-preserving equivalent); unordered map/set iteration may not determine output order.
- U8 begins with a read-only audit of class serialization/consumers; docs explicitly expose structural class-kind compatibility without OO claims. No schema expansion is introduced.
- U3 adds one assertion that retrieval-eval's existing delegated `language_of` returns `hcl`; no retrieval feature is added.
- U15 performs two bounded edit/sync cycles and one restart, checking stable counts and errors; no open-ended soak or arbitrary latency SLO.
- Rollback does not purge a database destructively. `code_file.language` is stored as a string, not a deserialized enum. In an isolated rollback rehearsal, the reverted binary's forced discovery/reconciliation must remove now-unsupported HCL graph records; if not, rollback is blocked for operator-approved cleanup.
- A runtime failure uses the configured blocked-return/handoff workflow. Neither Stage nor Ship auto-archives a failed shipment or crosses role boundaries.
- U15/U16 remain explicit Ship-owned operational tasks because the operator requires runtime verification and closure; they are single-domain and not implementation units.

### Final Authoritative Implementation Units

#### U1 — MCP/IPC Contract RED Harness
Test-only `tests/contract/hcl_parser_contract_test.rs`; 60-90 minutes; compact mixed-extension list/map plus bounded index-result errors. Compiles without HCL crate/variant references and fails on absent behavior.

#### U2 — Parser Behavior RED Harness
Test-only `tests/unit/hcl_parsing_test.rs`; 60-90 minutes; obtain language by `Language::try_from("hcl")`, then assert namespaced allowlisted symbols and conservative traversal/dedup behavior. No direct `tree_sitter_hcl` import or ABI claim.

#### U3 — Canonical Routing RED Harness
Test-only `tests/unit/hcl_routing_test.rs`; 60-90 minutes; alias/case characterization, startup/explicit-sync/default identity, live routing parity, and one existing retrieval-language delegation assertion.

#### U4 — Security/Resource RED Harness
Test-only `tests/unit/hcl_security_test.rs`; 60-90 minutes; existing oversize guards, bounded deep/malformed/dynamic skip behavior, ignored/alias containment, and pure in-memory no-side-effect boundary.

#### U5 — Local-Daemon Integration RED Harness
Test-only `tests/integration/hcl_indexing_test.rs`; 90-120 minutes; cold start/list-map, modified file plus explicit sync, and bounded restart/malformed health.

#### U6 — Exact Dependency Mutation
Dependency-only `Cargo.toml`, `Cargo.lock`; 30-45 minutes; depends on U1-U5 RED. Exact registry pin/checksum/license/source and expected tree-sitter-language graph only.

#### U7 — Grammar ABI and Engram Registration RED Harness
Test-only `tests/unit/hcl_grammar_abi_test.rs`; 30-45 minutes; depends on U6. One test safely loads `tree_sitter_hcl::LANGUAGE.into()` then expects Engram `Language::try_from("hcl")`/dispatcher registration, so the overall test is RED before U8. No production source change.

#### U8 — Canonical Language Identity and Grammar Registration
Rust parser core `src/services/parsing.rs`, `src/services/parsing/hcl.rs`; 75-105 minutes; depends on U7 RED. Add `Language::Hcl`, exhaustive conversions, sole path classifier, local safe parser, and empty extraction. U7 and identity subset turn green.

#### U9 — Namespaced Top-Level Declaration Extraction
Rust parser `src/services/parsing/hcl.rs`; 75-105 minutes; depends on U8. Read-only class-consumer audit then allowlisted `hcl.block.*`/`hcl.attribute.*` structural symbols and Defines.

#### U10 — Conservative Traversal Extraction
Rust parser `src/services/parsing/hcl.rs`; 75-105 minutes; depends on U9. Plain traversal target hints only, source context, stable order-preserving dedup.

#### U11 — HCL Reference Persistence Guard
Rust graph persistence `src/services/code_graph.rs`; 45-75 minutes; depends on U10. For `Language::Hcl`, bypass global name resolution and create only file self-loop plus target hint; assert no SQL/HCL cross-collision and idempotence.

#### U12 — Startup Discovery and Default Enablement
Startup/config Rust `src/services/code_graph.rs`, `src/models/config.rs`; 60-90 minutes; depends on U11/U3. Delegate canonical mapping, add only `hcl` default, preserve unknown fallback and existing pipeline.

#### U13 — Live-Sync Routing Parity
Daemon routing Rust `src/daemon/debounce.rs`; 45-75 minutes; depends on U12/U3. Consume sole classifier after existing filters; preserve delete/rename and unrelated routing.

#### U14 — User-Facing HCL Documentation
Docs `docs/configuration.md` and applicable README inventory; 45-60 minutes; depends on U9-U13. Canonical `hcl`, aliases, namespaced structural symbols, hint-only references, skip forms, and non-goals.

#### U15 — Local-Daemon Runtime Verification Evidence
Runtime verification with evidence `docs/closure/{date}-hcl-family-parser-runtime-verification.md`; 60-90 minutes; depends on U1-U14. Targeted/full gates, clients, two edit/sync cycles, restart, stable counts, resource/containment/side-effect checks, and isolated rollback reconciliation rehearsal. Any failure blocks; no code repair here.

#### U16 — Operational Closure Record
Closure docs `docs/closure/{date}-hcl-family-parser-operational-closure.md`; 30-45 minutes; depends on passing U15. Provenance exception/attestation, healthy/failure signals, owner/window, audit, rollback/blocked handoff, and disposition.

### Final Dependency Graph

```text
U1-U5 dependency-agnostic RED ──> U6 exact dependency ──> U7 ABI/registration RED
U7 ──> U8 identity/registration ──> U9 symbols ──> U10 traversals ──> U11 persistence guard
U11 + U3 ──> U12 startup/default ──> U13 live routing
U9-U13 ──> U14 docs
U1-U14 ──> U15 runtime evidence ──> U16 closure
```

### Adversarial Re-review Disposition

The first re-review produced two PASS verdicts and one single-reviewer `ENABLE-BEFORE-GUARD` P1 observation (LOW consensus confidence). It was valid and fixed: U12 now depends on U11 as well as the routing harness, and U13 remains downstream of U12, so no startup/default/live enablement can precede the HCL reference-persistence guard. The single-reviewer U8 sizing P2 is accepted because its enum conversions, sole classifier, safe parser initialization, and empty dispatcher are one tightly coupled parser-registration concern under 105 minutes. The U3 dependency notation means U12 turns the pre-existing RED routing harness green; U3 is not production code.

## Final Review Gate

Standard multi-persona re-review cycle 2 returned PASS after adversarial amendments. Final independent adversarial review at the current authoritative DAG returned PASS from all three models with zero P0/P1 findings. The plan is ready for harvest.
