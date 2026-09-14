---
title: "Package G — executable agent-contract assertion harness"
description: "Choose the repository-owned surface for an executable harness that proves agent, prompt, instruction, and policy contract assertions"
topic: "Executable harness for validating agent/prompt/instruction/policy contract assertions"
depth: "standard"
decision_status: "superseded"
superseded_by: "docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md"
promoted_to: "plan"
linked_artifacts:
  - "docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md"
tags:
  - "verification"
  - "contract-tests"
  - "harness"
  - "ci"
---

## Problem Frame

The decomposed write-boundary program (Packages A–F) repeatedly produced
**policy-only** release units: changes whose entire value is a contract asserted
over tracked harness markdown under `.github/agents/`, `.github/skills/`,
`.github/instructions/`, `.github/policies/`, and `.github/prompts/`. Those
packages cannot prove their own acceptance criteria, because the repository owns
no executable that fails when a required contract is absent.

Evidence gathered during program deliberation and confirmed in this session:

* `autoharness verify-workspace` requires an explicit workspace path and is a
  **globally installed external tool**, not a repository-owned surface. It is out
  of bounds for modification and cannot be relied on as the program's gate.
* `scripts/pre-commit-markdownlint.ps1` exits `0` when the staged markdown set is
  empty (`if (-not $StagedMd) { exit 0 }`), and also exits `0` with a warning when
  `markdownlint` is not installed. An empty or unavailable input set is reported
  as success.
* Shell `Select-String` / `grep` match *absence* does not fail a script by
  itself; proving "this contract is present" and "this prohibited pattern is
  absent" both require explicit, inverted exit-code handling that no current
  repository script performs for harness markdown.
* No test in `tests/` reads anything under `.github/agents/`, `.github/skills/`,
  `.github/instructions/`, `.github/policies/`, or `.github/prompts/`. Only
  `.github/workflows/*.yml` is read, by ad-hoc `String::contains` assertions.

The consequence is structural: a policy package can be written, reviewed, and
merged while its stated contract silently fails to hold. Packages B and C failed
independently, and Package A remains planned but unreviewed, partly because
there is no mechanism to demonstrate the assertions they claim.

### Who cares and why

The Stage and Ship pipeline agents are the consumers. They need a gate that
returns non-zero when a declared harness contract is violated, so that
policy-only work becomes verifiable rather than assertional.

### Constraints

* Single-domain, roughly 2-hour tasks; one independently shippable release unit.
* Test-first: harness tests and fixtures precede implementation.
* Only tracked, repository-owned surfaces. No modification of globally installed
  `autoharness` or any external package.
* No product runtime behavior change — this is verification infrastructure.
* No `Select-String` / `grep` success ambiguity. Non-zero exit when an expected
  contract is absent **or** a prohibited contract is present.
* Explicit workspace/path containment; no writes outside the working tree.
* Deterministic output, usable in CI and locally.
* Later packages (B, C, A, D, E) must be able to register assertions and fixtures
  without a monolithic bespoke script and without weakening type safety.
* Reuse existing cargo test targets, scripts, fixtures, and CI commands. No new
  dependency without concrete evidence.

### Success criteria

The harness fails, with a non-zero exit and a deterministic diagnostic, on each
of: a missing required assertion, a malformed input document, an empty resolved
file set, and invocation without a resolvable workspace root.

### Explicitly out of scope

Package G ships the **mechanism only**. It bakes in no policy belonging to
Packages A–F. It changes no product runtime behavior, creates no PR, and encodes
no dependency on the failed Packages B or C.

## Research Findings

Gathered from the live repository at commit `fe8040e4`.

### Existing repository-file contract-test idiom

The repository already has a well-established pattern for tests that assert over
tracked repository files. Repo root is resolved at compile time via
`env!("CARGO_MANIFEST_DIR")`, and the file is read with an explicit panic on
failure:

```rust
fn repository_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}
```

This idiom appears in `tests/contract/supervisor_release_artifact_test.rs`,
`tests/contract/supervisor_install_exclusion_test.rs`, and
`tests/integration/release_archive_smoke_workflow_test.rs`, all of which read
`.github/workflows/*.yml`. The assertions themselves are ad-hoc
`workflow.contains("…")` chains — the right mechanism, without structure or
reuse.

### Cargo target and CI wiring

* Test targets are declared explicitly as `[[test]]` entries in `Cargo.toml`
  (name + path), one per file.
* `.cargo/config.toml` aliases are native: `dev-test = "test --all-targets"`,
  `ci = "test --all-targets --all-features"`,
  `lint = "clippy --all-targets --all-features -- -D warnings -D clippy::pedantic"`,
  `fmt-check = "fmt --all -- --check"`. `audit` is **not** an alias.
* CI's `build` job runs `cargo test --no-default-features --features
  cozo-backend,embeddings --all-targets`, so a new `[[test]]` target is picked up
  automatically with no further workflow wiring.

### The CI trigger gap

`.github/workflows/ci.yml` lists `.github/**/*.md` under `paths-ignore` for both
`push` and `pull_request`. The existing comment is explicit about the governing
principle:

> Markdown is SCOPED, not blanket: only doc markdown is ignored … EXECUTABLE
> markdown re-arms CI on purpose — notably `tests/**/*.md`, i.e. the verify
> fixtures … A blanket `**/*.md` would under-run and let fixture breakage merge
> silently.

Once Package G exists, harness markdown under `.github/{agents,skills,instructions,policies,prompts}/`
becomes **executable markdown** — a direct input to a test target — and the
current ignore entry would let a contract violation merge without ever running
the harness. Narrowing that entry is therefore a required part of G, and it is
precisely the precedent the file already documents for `tests/**/*.md`.

### Reusable library surface

`src/services/verify.rs` exposes `pub fn verify_markdown(rel_path, content) ->
Result<VerifyReport, EngramError>` with a structured finding model
(`VerifyFinding { rule, message, line, severity }`) and a `conformant` flag. It
is publicly reachable as `engram::services::verify` (`pub mod services` in
`src/lib.rs`, `pub mod verify` in `src/services/mod.rs`), so integration tests
can call it directly. It already detects present-but-malformed frontmatter,
empty bodies, and unresolved `{{…}}` template variables — three of the
malformed-input cases G must fail on — with no change to the shipped binary.

### Dependency surface

`serde`, `serde_json`, `serde_yaml = "0.9"`, and `tempfile = "3"` are regular
dependencies, available to tests. Dev-dependencies are `proptest`, `tokio-test`,
`serial_test`, and a pinned `rayon`. **`regex`, `walkdir`, and `glob` are
absent**, so the harness must use `std::fs` traversal and plain string matching —
consistent with every existing repository-file contract test.

### Prior learning

`docs/compound/2026-08-22-cargo-dev-test-alias-must-stay-native.md` is directly
on point and supplies two binding rules:

* Do not convert a pervasively-referenced, zero-setup cargo alias into a
  script-backed external subcommand. Ship extra tooling as separate commands.
* "A coverage oracle that reads a diff must fail closed when the diff is
  indeterminate … treating an unknown diff as empty reports a false PASS."

The second rule is the exact defect Package G exists to eliminate, generalized
from diffs to file sets. That learning also established
`scripts/test-coverage-oracle.{sh,ps1} --mode completeness`, which fails when any
declared target or `src/` module is unmapped — a working precedent for a
fail-closed registry that cannot drift.

## Options Evaluated

### Option A: Rust contract-test target with a typed assertion registry

Add a shared helper module plus one or more `[[test]]` contract targets under
`tests/contract/`. The helper exposes a typed assertion model and a fail-closed
scanner; each later package registers its assertions in its own small registry
module, which the harness composes.

* **Pros**: Matches the dominant existing idiom exactly. Zero new dependencies.
  Compile-time type safety for the registry — a malformed assertion is a build
  error, not a silent skip. Non-zero exit is inherent to `cargo test`. Picked up
  automatically by `cargo dev-test`, `cargo ci`, and the CI `build` job. Reuses
  `engram::services::verify` for malformed-input detection without touching the
  product binary. Repo-root containment via `env!("CARGO_MANIFEST_DIR")` is
  compile-time and cannot be redirected by a caller's cwd. Cross-platform with no
  shell involved, avoiding the LF/Bash-4 fragility called out in prior learnings.
* **Cons**: Requires narrowing the CI `paths-ignore` entry so harness markdown
  re-arms CI. Requires a `[[test]]` entry per target. Contributors must write
  Rust to add assertions.
* **Effort**: Medium.
* **Fit**: Strong. Satisfies every stated constraint.

### Option B: Extend `engram verify` with harness-contract rules

Add harness-specific rules to `src/services/verify.rs` and a directory mode to
the `engram verify` CLI subcommand.

* **Pros**: Reuses an existing finding/severity model and its exit-code mapping.
  Fixture precedent already exists at `tests/fixtures/verify/`.
* **Cons**: **Directly violates the "no product runtime behavior change"
  constraint** — `engram verify` is a shipped CLI subcommand, and new rules would
  change the behavior of the released binary for every consumer. `verify_markdown`
  is also per-file and pure: it has no concept of a file set, a required-file
  registry, or an "expected document is absent" failure, so the substantive work
  would be new `src/` surface rather than reuse. Largest blast radius of the three.
* **Effort**: High.
* **Fit**: Poor. Fails a hard constraint.

### Option C: Repository script pair, mirroring `test-coverage-oracle`

Add `scripts/harness-contract-check.{ps1,sh}` invoked by CI and locally.

* **Pros**: Precedent exists (`test-coverage-oracle`, `check-oracle-independence`).
  No Rust knowledge needed to add an assertion.
* **Cons**: Dual-language maintenance of identical logic. Assertion matching would
  be `Select-String` / `grep`, which is the **exact success-ambiguity the
  requirements forbid**. No type safety — a typo in a registry entry silently
  matches nothing and passes. Not picked up by `cargo test`, so it needs separate
  CI wiring and is easy to omit locally. Prior learnings document real
  cross-platform fragility for this class of script (LF endings, Bash 4
  associative arrays).
* **Effort**: Medium.
* **Fit**: Weak. Reintroduces the defect being fixed.

## Trade-off Comparison

| Criterion | A: Rust contract target | B: Extend `engram verify` | C: Script pair |
|---|---|---|---|
| Product runtime change | None | **Yes — violates constraint** | None |
| New dependencies | None | None | None |
| Match-absence ambiguity | Eliminated (typed, inverted) | Eliminated | **Present (`grep`/`Select-String`)** |
| Type safety of registry | Compile-time | Compile-time | **None** |
| Empty-file-set failure | Explicit invariant | Not modeled | Must be hand-written per script |
| Workspace containment | Compile-time repo root | Caller-supplied path | Caller-supplied path |
| CI pickup | Automatic (`--all-targets`) | Automatic | Requires explicit wiring |
| Cross-platform risk | Low (no shell) | Low | **Medium (LF, Bash 4)** |
| Alignment with existing idiom | Strong | Partial | Moderate |
| Blast radius | Tests only | **Shipped binary** | Scripts + CI |

## Decision

**Adopt Option A**: a Rust contract-test harness with a typed assertion registry,
plus a scoped narrowing of the CI `paths-ignore` entry.

Rationale:

1. It is the only option that satisfies every hard constraint simultaneously —
   no product runtime change, no new dependency, no match-absence ambiguity, and
   compile-time type safety.
2. It reuses the repository's dominant, already-proven idiom for asserting over
   tracked files, rather than introducing a fourth verification style.
3. Failure semantics come for free and cannot be bypassed: a failed assertion is
   a failed test, which is a non-zero `cargo test` exit.
4. It reuses `engram::services::verify` as a **library call** for malformed-input
   detection, achieving reuse without changing the shipped binary.

### Failure-semantics contract

The harness must fail, deterministically and with a non-zero exit, on all four
required conditions. These are the acceptance criteria for G itself:

| Condition | Required behavior |
|---|---|
| Missing assertion target | A registered assertion naming a document that does not exist fails with the resolved absolute path in the diagnostic. |
| Malformed input | A document whose frontmatter is present but unparseable, or whose body is empty, fails via `engram::services::verify::verify_markdown`. |
| Empty file set | A registered scan resolving to zero documents fails. An empty set is never a pass. |
| Unresolvable workspace | If the compile-time repo root does not canonicalize, or lacks expected repository markers, the harness fails before evaluating any assertion. |
| Prohibited contract present | A negative assertion whose forbidden content is found fails, with the offending line number. |

### Extensibility contract

Later packages register assertions without editing a monolith and without
weakening type safety:

* A typed assertion model — a `ContractAssertion` record carrying a target
  selector, an assertion kind, and a stable rule identifier — with the kind
  expressed as an enum (document-exists, contains, does-not-contain,
  frontmatter-field-present, frontmatter-field-equals, section-present). A new
  assertion is a value, not a new code path.
* Each later package adds **its own registry module file** exposing a single
  function returning its assertions. The harness composes the registries; no
  package edits another package's file, and no bespoke per-package script exists.
* Registry composition is fail-closed in the same spirit as
  `test-coverage-oracle --mode completeness`: a registered rule identifier that
  resolves to no evaluation is itself a failure, so a typo cannot silently pass.
* Fixtures follow the established `tests/fixtures/` convention, resolved relative
  to the compile-time repo root — matching `tests/fixtures/verify/`.

### CI integration

Narrow the `paths-ignore` entry `'.github/**/*.md'` so that harness markdown
under `.github/agents/`, `.github/skills/`, `.github/instructions/`,
`.github/policies/`, and `.github/prompts/` re-arms CI, while genuinely
non-executable `.github` documentation stays ignored. This applies the file's own
documented "executable markdown re-arms CI on purpose" principle to the new test
inputs. No new CI job or command is added; `cargo test --all-targets` already
runs the target.

## Rejected Alternatives

* **Option B** was rejected because extending `engram verify` changes the shipped
  binary's behavior, violating the explicit "no product runtime behavior change"
  constraint. Its reuse advantage is largely illusory: `verify_markdown` is
  per-file and pure, so file-set, required-file, and registry semantics would all
  be new `src/` surface. The same reuse benefit is obtained in Option A by calling
  the function as a library.
* **Option C** was rejected because its matching layer is exactly the
  `Select-String` / `grep` success ambiguity the requirements prohibit, it offers
  no type safety for the registry, and prior compound learnings document concrete
  cross-platform fragility for this class of dual-language script.
* **Modifying `autoharness`** was never in scope: it is a globally installed
  external tool, explicitly out of bounds.

## Unresolved Questions

* Whether the harness should be a single `[[test]]` target or one target per
  harness domain. Deferred to planning; the registry design supports either, and
  a single target is the smaller first step.
* The exact narrowed `paths-ignore` pattern set. The intent is settled; the
  literal glob list is a planning detail that must be verified against the
  `build`-is-not-a-required-check contingency already documented in `ci.yml`.
* Whether any Package A–F assertion should ship as a worked example. Current
  position: **no** — G ships the mechanism plus self-tests only, keeping package
  policies out of G and preserving independence from failed Packages B and C.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Narrowing `paths-ignore` causes doc-only PRs to run the full Rust build more often. | Narrow precisely to the five harness directories, leaving `docs/**`, root `*.md`, and other `.github` docs ignored. The `ci.yml` comment confirms `build` is not a required status check, so no PR can hang. |
| The harness becomes a monolith as packages accumulate assertions. | Per-package registry modules composed by the harness; no shared file is edited to add an assertion. |
| A registered assertion silently matches nothing. | Fail-closed registry evaluation — an unevaluated rule identifier is a failure, mirroring `test-coverage-oracle --mode completeness`. |
| Scanning 92 harness markdown files slows the test suite. | Pure file reads with no I/O beyond `std::fs`; deterministic sorted traversal. Measure during implementation and split targets only if needed. |
| Temptation to add `regex`/`walkdir` for convenience. | Explicit constraint: plain string matching and `std::fs` traversal only, matching every existing repository-file contract test. Any dependency addition requires concrete evidence and re-review. |
| Contributors must write Rust to add an assertion. | Accepted. The registry entry is a declarative value; type safety is the point, and it is the cost of eliminating match-absence ambiguity. |

## Post-Review Addendum (2026-09-13)

The implementation plan derived from this decision —
`docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md`
— reached **BLOCKED** after a full plan review, one authorized correction round,
and a confirmation review. Package G was not harvested; no backlog IDs were
allocated and no shipment was assembled.

**The decision itself stands.** Option A was affirmed by every reviewer across
both rounds — Constitution, Rust, Scope, Architecture, Security, Agent-Native
Parity, and Learnings. The rejection of Option B (extending the shipped
`engram verify` CLI) and Option C (a PowerShell/bash script pair) was not
challenged. What failed was the *implementation design*, in two specific places.

### Correction to the Research Findings

The "Dependency surface" section above states that `regex`, `walkdir`, and `glob`
are absent and concludes that traversal must be hand-rolled with `std::fs`. That
conclusion was wrong. **`ignore = "0.4"` and `globset = "0.4"` are existing
regular dependencies.** `ignore::WalkBuilder` provides `follow_links(false)`,
`max_depth`, `max_filesize`, and `sort_by_file_path` natively. A future attempt
should reuse it rather than writing recursive traversal, per Principle VI.

Caveat discovered in review: `WalkBuilder`'s `hidden(true)` default would prune a
dot-directory discovered as a child, and `.github` is a dot-directory. Depth-0
roots bypass that filter, so each harness directory must be added as its own root,
with `hidden(false)` set explicitly as defense in depth.

### Correction to the Reusable Library Surface

The "Reusable library surface" section above proposes reusing
`engram::services::verify::verify_markdown` for malformed-input detection. Review
established this is the wrong abstraction. `verify_markdown` is an *ingestion*
conformance service: it unconditionally emits `template.unresolved` at
`Severity::Error` for any line containing `{{…}}`, and `conformant` is derived from
`findings.is_empty()`.

**20 of the 92 tracked harness markdown files contain a same-line `{{…}}`
placeholder** — including `.github/agents/_orchestrator.agent.md`,
`.github/agents/_ship.agent.md`, and eleven subagent definitions. Those are
legitimate agent contracts, not defects. Delegating structural validity to
`verify_markdown` would reject them on day one.

A future attempt must either own a harness-specific structural validator, or
interpose an adapter that accepts only `frontmatter.malformed` and `body.empty`
and discards `template.unresolved`. The 20 affected files are the acceptance
evidence for whichever path is chosen.

### Second Blocking Issue — Module Topology

`.cargo/config.toml` sets `rustflags = ["-Dwarnings"]` globally, and each
`tests/*.rs` target compiles as its own crate where `pub` does not exempt items
from `dead_code`. A single shared support `mod.rs` included by multiple targets
therefore cannot compile without `#![allow(dead_code)]` — which is precisely why
`tests/helpers/mod.rs:34` already carries that allow. A future attempt must choose
between per-target `#[path]` inclusion of only the files each target exercises, or
promoting the support tree to a workspace-member lib crate, and then derive unit
boundaries from that choice.

### Guidance for the Next Attempt

Settle the validator abstraction and the module topology **before** decomposing
into units. Both decisions change the file topology, and the granularity findings
recorded in the plan's confirmation review depend on it. Re-planning G is a fresh
Stage operation, not a continuation of this one.
