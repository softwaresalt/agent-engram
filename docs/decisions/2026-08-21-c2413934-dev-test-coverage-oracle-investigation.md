---
title: C2413934 investigation — canonical cargo dev-test coverage gap
date: 2026-08-21
type: investigation
status: resolved
source_stash_id: C2413934
agent: stage
confidence: high
---

## Problem

The constitution and `AGENTS.md` both state that "All tests MUST pass via
`cargo dev-test` before any code is merged". That statement is currently
vacuous for almost the entire suite.

## Evidence

### E1 — The alias selects six targets out of 208

`.cargo/config.toml`:

```text
dev-test = "test --lib --test hcl_parser_contract_test --test hcl_grammar_abi_test --test hcl_parsing_test --test hcl_routing_test --test hcl_security_test --test hcl_indexing_test"
```

`Cargo.toml` declares **208** `[[test]]` targets. `cargo dev-test` runs the
library unit tests plus **6** HCL-specific integration targets. **202 declared
test targets — every non-HCL contract and integration target — are silently
omitted from the mandated gate.**

Test files on disk: `tests/contract/` 40, `tests/integration/` 113,
`tests/unit/` 54.

### E2 — The narrowing was introduced by HCL work

`git log -- .cargo/config.toml` shows the alias last changed in `d6db8423`
("test: register U1-U5 HCL RED harnesses") and `2b677646` ("test: cover HCL
parser review regressions") — both from the Shipment 117-S HCL parser stream.
The narrowing was a local convenience for that stream that became the global
default gate.

### E3 — The unnarrowed alternatives are the process-explosion problem

`full-test = "test"` and `ci = "test --all-targets --all-features"` build and run
all 208 integration binaries. Each is a separate process, and engram integration
tests stand up real databases, IPC endpoints, and daemon processes. Running the
full matrix on a developer machine is the cost that motivated the narrowing in
the first place. Simply reverting the alias to `cargo test` reintroduces the
original problem.

### E4 — There is no oracle

Nothing in the repository detects that a changed file has no corresponding
target in the `dev-test` selection. A contributor can modify
`src/db/workspace.rs`, run `cargo dev-test`, observe green, and merge without
ever executing `tests/contract/shim_lifecycle_test.rs` or any of the 202 omitted
targets. The failure is silent by construction.

## Root Cause

The `dev-test` alias is a **hardcoded allowlist of six targets** with no
relationship to the change under test. It conflates two separate concerns:

1. *Which targets are relevant to this change?* — currently answered by a
   stale constant.
2. *How many test processes may run concurrently?* — currently answered by
   omitting targets entirely.

## Chosen Direction

**A measurable canonical coverage oracle plus bounded execution**, not a blanket
broadening.

1. **Canonical target manifest.** Declare, in a checked-in manifest, the mapping
   from source surface to the test targets that must cover it.
2. **Coverage oracle.** A check that takes the set of changed files, computes the
   required target set from the manifest, compares it against the set the
   selected command will actually execute, and **fails** when any required target
   is omitted. This is the measurable artifact the stash entry asks for.
3. **Change-scoped selection.** `dev-test` becomes change-aware: it runs the
   required set for the current diff rather than a fixed six.
4. **Bounded execution.** Constrain concurrency explicitly (`--test-threads`,
   `--jobs`) and cap the number of concurrently running test binaries, so the
   process budget is a controlled parameter rather than an emergent consequence
   of the target count.
5. **Unmapped-surface failure.** A source file with no manifest mapping fails the
   oracle. Coverage gaps must be loud.

## Explicit Non-Goals

* Do **not** change Shipment 117-S scope, its HCL targets, or its outcomes.
* Do **not** rewrite or consolidate the 208 test targets in this release unit.
* Do **not** replace `cargo ci`; the exhaustive CI path remains the backstop.

## Measurability

The oracle is measurable because all three quantities are countable and
reportable: required targets for the diff, selected targets, and omitted targets.
The pass condition is `omitted == 0`. The baseline to beat is today's silent
omission of 202 targets.

## Rejected Alternatives

| Alternative | Why rejected |
|---|---|
| Revert `dev-test` to plain `cargo test` | Reintroduces E3 process explosion; the stash entry explicitly forbids ignoring that constraint. |
| Add the missing targets to the alias by hand | Restores the same stale-allowlist failure mode the moment a new target is added. |
| Rely on CI (`cargo ci`) only | Moves the feedback to after push and leaves the constitutional local gate meaningless. |
| Merge all integration tests into one binary | Large refactor of 208 targets; out of scope and would perturb 117-S. |

## Traceability

Source stash `C2413934`. Shipment 118-S, Feature 122-F, review commit `2ef18c0d`.
Constraint reference: Shipment 117-S (HCL parser), unchanged by this work.
