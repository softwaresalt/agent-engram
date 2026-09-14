---
type: deliberation
date: 2026-09-13
package: G-split
depth: deep
supersedes: >-
  docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md,
  docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md,
  docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: 93370734
---

# Package G split — three independently reviewable release units (deliberation)

## Authority and scope

This is a **fresh planning authority for the decomposition of Package G only**.
It does not reopen the combined Package G. The combined package is permanently
superseded by the program decision that accompanies this deliberation
(`docs/decisions/2026-09-13-package-g-split-program-decision.md`).

**Explicitly out of scope**: Packages A–F, PR #396, `143.*`, and any
re-litigation of decisions this deliberation marks INHERITED below.

## Problem

Three consecutive planning attempts at the combined Package G failed review
(v1, v2 `docs/closure/2026-09-13-package-g-v2-plan-review-record.md`, v3
`docs/closure/2026-09-13-package-g-v3-plan-review-record.md`). The failures did
not repeat a single defect; each attempt failed at a different layer:

| Attempt | Terminal cause | Layer |
|---|---|---|
| v1 | broad round-1 findings across the whole surface | all |
| v2 | B1 filter-level silent omission; B2 fault-injection seam | filesystem + layering |
| v3 | A root-symlink fail-open; B canonicalization not injectable | **filesystem seam only** |

The v3 record is decisive evidence for decomposition. Its own adjudication says
findings A and B are **one structural problem** — "the two-method `FileAccess`
seam is under-provisioned for what the `resolve` layer asks of it". Every other
layer of v3 passed: the registry model, the typed assertion evaluator, the
diagnostic renderer, scope discipline (0 P0/P1/P2 from the Scope Boundary
Auditor), the no-speculative-limits decision (7/7), and every inherited v2
constraint (7/7).

**The combined package therefore forces a whole-package FAIL on a defect
localized to one layer.** Three times, a sound assertion engine and a sound CI
integration were blocked by a filesystem seam that is roughly 15% of the plan.
That is a packaging defect, not an engineering one. The mega-plan is the failure
mode.

## Evidence gathered this session (fresh, not inherited)

| # | Evidence | Value | Source |
|---|---|---|---|
| S1 | Harness corpus | **92 files, all `.md`** | fresh recursive enumeration of the five roots |
| S2 | Per-root counts | agents 23, skills 28, instructions 34, policies 2, prompts 5 | same |
| S3 | Max relative depth | **2** | same |
| S4 | Max file size | **98,546 bytes** | same |
| S5 | Global rustflags | `rustflags = ["-Dwarnings"]` under `[build]` | `.cargo/config.toml` |
| S6 | `allow(dead_code)` in `tests/` | **11** occurrences | recursive grep |
| S7 | Workspace members | `["." , "crates/powerbi-tmdl-parser", "crates/engram-indexer"]` | `Cargo.toml:1-3` |
| S8 | Member-crate precedent | `crates/engram-indexer` — `publish = false`, own `Cargo.toml`, `src/lib.rs` + `src/main.rs` | `crates/engram-indexer/Cargo.toml` |
| S9 | Coverage-oracle completeness rule | target coverage computed **against `src/` and `crates/` surfaces only**; unmapped `[[test]]` target ⇒ FAIL | `scripts/test-coverage-oracle.ps1:203-208` |
| S10 | Existing manifest surfaces | mostly `src/...`, plus **one existing `crates/` surface**: `path = "crates/powerbi-tmdl-parser/"` with `targets = ["contract_*", "integration_*", "unit_*", "cold_*", "helpers_*"]` | `.cargo/test-coverage-manifest.toml:137-139` |
| S10a | Correction | An earlier draft of this deliberation asserted "no `crates/` surface exists yet". That was **false** — produced by a truncated grep. The claim is corrected here rather than silently edited away: a `crates/` surface precedent **does** exist, which *strengthens* Option T3 below (it is conventional, not novel), and it means the new surfaces G0/G1 add are a **second and third instance of an established pattern**, not the first. Note also that the existing surface's globs (`contract_*`, `integration_*`, ...) would **not** match a target named `agent_contract_fs_test`, so a new surface is still required. | this session |
| S11 | CI test command | `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` — **no `--workspace`** | `.github/workflows/ci.yml` |
| S12 | CI `paths-ignore` | `.github/**/*.md` present in **both** `push` and `pull_request` blocks | same |
| S13 | Cargo aliases | `dev-test`, `full-test`, `lint`, `fmt-check`, `ci` | `.cargo/config.toml` |
| S14 | Oracle modes | `report｜select｜completeness｜run` | `scripts/test-coverage-oracle.ps1:19` |

### What S5 + S6 mean

`-Dwarnings` is set globally in `[build] rustflags`, so `dead_code` is a **hard
compile error**, not a warning, in every compiled unit. The eleven
`allow(dead_code)` sites in `tests/` are not sloppiness — they are the
**structurally forced consequence** of sharing a helper module across multiple
test binaries via `#[path]`: each binary compiles the whole module but uses only
part of it, so every unused item is a build-breaking error unless suppressed.

This is the mechanism behind the operator's constraint "if a test-only module
cannot ship independently without dead code, choose an owned dev-only
crate/tool topology that can". It is now measured, not asserted.

### What S9 + S10 + S11 mean

Two independent constraints pull in opposite directions:

* **S9/S10**: a newly registered `[[test]]` target that is mapped by no `src/`
  or `crates/` surface makes `--mode completeness` FAIL. A pure-`tests/`
  topology therefore has no truthful surface to map to — mapping contract tests
  to, say, `src/services/` would be a false statement about what they exercise.
  The one existing `crates/` surface (S10) maps only `contract_*`-style globs,
  which will not match the new target names, so the new targets genuinely need
  their own surfaces under either topology.
* **S11**: CI and `cargo dev-test` run the **root package only**. A workspace
  member crate's own `#[cfg(test)]` unit tests would not run in CI without
  adding `--workspace`.

Any topology must satisfy both. This is the pivot of the topology decision below.

## Decision 1 — the split boundary

### Options considered

**Option D1-1 — keep one package, fourth attempt.**
Re-plan the combined package with a provisioned seam.

* Pro: one review, one shipment, no new program structure.
* Con: the operator has explicitly foreclosed this ("Three G planning attempts
  are exhausted... do not reopen it"), and the evidence supports the
  foreclosure: three attempts, three different terminal layers, the same
  whole-package blast radius each time. A fourth mega-plan has an unbounded
  review surface (v3 was 35 units, ~40 KB) and re-exposes affirmed layers to
  fresh reviewers with fresh opinions. **Rejected.**

**Option D1-2 — split by test tier (unit / integration / contract).**

* Pro: matches the three-tier test architecture in Constitution II.
* Con: the v3 defect is not tier-shaped. The seam defect spans the unit tier and
  the contract tier simultaneously; splitting by tier would put finding A in one
  package and finding B in another when adjudication says they are one problem.
  **Rejected.**

**Option D1-3 — split by dependency layer: filesystem seam → assertion engine →
harness registry/CI.** *(SELECTED)*

* The cut lines fall exactly where the v3 review drew them. Finding A and
  finding B are both wholly inside the filesystem seam; they land together in
  G0, satisfying the adjudication that they must be resolved together.
* Each unit has a genuinely different failure taxonomy: G0 owns
  filesystem/discovery failures; G1 owns registry/assertion semantics; G2 owns
  registration and invocation. No taxonomy straddles a boundary.
* Each unit is independently valuable. G0 is deterministic contained path
  discovery and reading — useful to any future repository tooling with no
  assertion concept at all. G1 is a generic data-driven assertion evaluator over
  a corpus — useful with any corpus provider. G2 binds them to this repository's
  actual harness roots and CI.
* Each unit is independently reviewable at a size a panel can hold: roughly
  12/10/8 work units instead of 35.
* **Failure containment is the point**: a defect in the seam can no longer block
  a sound evaluator, and a defect in CI wiring can no longer block a sound seam.

**Option D1-4 — split into two (seam+engine, then CI).**

* Pro: fewer shipments.
* Con: re-merges the exact boundary that failed three times. The seam is where
  all three attempts died; giving it its own gate is the entire lesson.
  **Rejected.**

### Selected: D1-3 — three units, G0 → G1 → G2

## Decision 2 — code topology

This is the decision the operator flagged as load-bearing: the chosen topology
must let **G0 ship independently and remain lint-clean** under S5.

### Option T1 — modules under `tests/`, shared via `#[path]`

The v3 approach: one registered `[[test]]` target whose root file declares
`#[path]` modules.

* Pro: 46 existing `#[path]` precedents; target runs automatically under
  `cargo dev-test` and CI (S11) with no workflow change.
* Con 1 — **dead code is unavoidable at the split boundary**. G0 must ship
  before G1 exists. Under T1, G0's modules are compiled into a test binary whose
  only consumer is G0's own tests. That is survivable for G0 alone, but the
  moment G1 adds a second binary that consumes only part of G0's surface, S5
  forces `allow(dead_code)` — reproducing the eleven existing suppressions. The
  operator's constraint is explicit that this disqualifies the topology.
* Con 2 — **S9 has no truthful answer**. Three new `[[test]]` targets mapped by
  no `src/` or `crates/` surface fail `--mode completeness`; the only repair is
  a false surface mapping.
* Con 3 — G0 is not independently *usable*; it is only independently *compiled*.
  Nothing outside a test binary can consume it.
* **Rejected.**

### Option T2 — one shared test-support crate for all three packages

A single `crates/agent-contract-harness` containing seam + engine + registry.

* Pro: one crate, one surface, one dependency edge.
* Con: it re-creates the mega-package at the code level. G1 and G2 would edit
  the same crate that G0 ships, so "G0 ships independently" becomes false the
  moment G1 lands — the three shipments would contend on one manifest and one
  lib root, and a G1 regression would be a G0-crate regression. It also
  reintroduces the v3 seam-sufficiency question, because one crate's public API
  must simultaneously satisfy three layers' needs before any of them is
  reviewed. **Rejected.**

### Option T3 — one dev-only workspace member crate per package, exercised by root-package `[[test]]` targets *(SELECTED)*

```text
crates/agent-contract-fs/      # G0 — publish = false, std-only library
  Cargo.toml
  src/lib.rs                   # FileAccess, RealFs, FakeFs, resolve, errors
crates/agent-contract-assert/  # G1 — publish = false, depends on agent-contract-fs
  Cargo.toml
  src/lib.rs                   # registry model, evaluator, diagnostics
tests/contract/agent_contract_fs_test.rs        # G0 exerciser  [[test]] in root Cargo.toml
tests/contract/agent_contract_assert_test.rs    # G1 exerciser  [[test]] in root Cargo.toml
tests/contract/agent_harness_contracts_test.rs  # G2 exerciser  [[test]] in root Cargo.toml
```

Each crate is a **`[dev-dependencies]` path dependency of the root package**, so
it is compiled only for test targets and never linked into `cargo build
--release`.

How T3 satisfies every constraint:

| Constraint | How T3 satisfies it |
|---|---|
| **No dead code (S5, Dev-Workflow item 5)** | `dead_code` does not fire on `pub` items reachable from a library crate root. The crates expose their whole surface as `pub`; there is nothing to suppress. The operator's "no dead-code suppression" requirement is met *structurally*, not by discipline. |
| **Coverage-oracle completeness (S9/S10)** | A new `crates/agent-contract-fs/` surface maps the G0 target; a `crates/agent-contract-assert/` surface maps the G1 target. The mapping is **truthful** — changing the crate really should run that target. `crates/powerbi-tmdl-parser/` is an existing precedent for a `crates/` surface (S10), so this is an established pattern, not a workaround. |
| **CI runs it without a workflow change (S11)** | The **exercisers are root-package `[[test]]` targets**, so `cargo dev-test`, `cargo ci`, and the CI `cargo test --all-targets` pick them up automatically. No `--workspace` is required, because the crates carry **no internal `#[cfg(test)]` tests** — all testing lives in the root-package targets that consume the public API. |
| **G0 ships independently** | `crates/agent-contract-fs` + its exerciser + its manifest surface is a complete, green, self-contained change. G1 adds a *new* crate and a *new* target; it does not edit G0's crate. |
| **Independently useful** | The G0 crate is a real library with a real API. Any future repository tool can depend on it. |
| **Dependency minimality (Constitution VI)** | G0 is **std-only** — zero new external dependencies. G1 depends only on G0 plus already-declared `serde`/`toml` from the root workspace if a serialized registry is chosen; see Decision 4. |
| **Precedent (S7/S8)** | `crates/engram-indexer` already establishes the `publish = false` member-crate pattern in this workspace. T3 is conventional here, not novel. |

Residual cost, stated honestly: T3 adds two `Cargo.toml` files and two workspace
members, and `cargo clippy --all-targets` at the root lints the root package's
targets but not the dependency crates' own lints. G2 owns closing that (see
Decision 5). This is a **known, owned gap**, not an unexamined one.

### Selected: T3

## Decision 3 — the G0 seam contract (resolves v3 findings A and B together)

v3's adjudication requires the seam to be provisioned for everything the resolve
layer asks of it. The seam is therefore defined as **exactly the operations the
resolver performs, and no more**:

| Method | Why the resolver needs it | Which v3 finding it closes |
|---|---|---|
| `symlink_metadata(path) -> Result<FileKind>` | lstat the **root itself** before any canonicalization or traversal | **A** — root-symlink fail-open |
| `canonicalize(path) -> Result<PathBuf>` | containment proof, routed through the boundary so `FakeFs` can inject a lexically-contained / canonically-escaping path with **no on-disk symlink and no Windows elevation** | **B** — Windows non-determinism |
| `read_dir(path) -> Result<Vec<Entry>>` | non-filtering enumeration; each `Entry` carries name + **lstat-derived** `FileKind` so a symlink entry is rejected **before** it is followed | A (entry arm) |
| `read_to_string(path) -> Result<String>` | the read layer; also the only place a disappear-between-discovery-and-read fault can occur | — |

**Four methods, no more.** No `len`/size accessor: size is not consulted by any
resolver decision because there is **no max-filesize policy** (Decision 6). Per
the operator's "metadata/type/size if used" and Constitution VI, an unused
accessor is speculative surface and is omitted.

**Resolver ordering (normative):**

1. Lexically join each allow-listed relative root to the workspace root.
2. `symlink_metadata` the joined root. `Symlink` ⇒ reject. **This happens before
   step 3.** This is the ordering change v3 lacked.
3. `canonicalize` the joined root **through the seam**; component-wise
   `Path::starts_with` against the canonicalized workspace root; escape ⇒ reject.
4. Recurse with `read_dir`. Any entry whose lstat-derived kind is `Symlink` ⇒
   reject, never follow, never silently skip.
5. No depth filter, no size filter, no ignore-file filter, no hidden-file filter.
6. Sort the resolved set by workspace-relative path with separators normalized
   to `/`, so ordering is identical on Windows and Unix.

**Why steps 2 and 3 cannot be reordered**: `canonicalize` follows symlinks. Any
containment check performed on a canonicalized root has already lost the
information that the root *was* a symlink. Rejecting at step 2 is the only
ordering in which the root-symlink case is observable.

## Decision 4 — G1 `exactly_one` precedence (settled here, not deferred)

v3 mechanical finding M-4 recorded that `exactly_one` can yield two different
diagnostic codes for one scenario and that the plan's scenario convention
forbade it — an unresolved contradiction. It is settled now:

> For an `exactly_one` assertion, let *n* be the number of matches.
> *n* = 0 ⇒ **`MissingRequired`** and nothing else.
> *n* = 1 ⇒ pass.
> *n* > 1 ⇒ **`AmbiguousMultiplicity`** and nothing else.
>
> The two codes are **mutually exclusive by construction**: they are arms of one
> `match` on *n*, so emitting both for the same assertion is unrepresentable, not
> merely forbidden. Deterministic precedence is therefore vacuous at the
> assertion level, and the only remaining ordering question — the order of
> diagnostics across *different* assertions — is settled by sorting on
> (assertion id, then code), both of which are total orders.

This removes the contradiction at the type level rather than by convention, so
no scenario-counting rule has to police it.

## Decision 5 — G2's two honest problems

**5a — `.github/**/*.md` is in `paths-ignore` (S12).** A pull request that
changes only harness markdown does not trigger CI at all. An assertion harness
that never runs on the exact change class it exists to police is decorative. G2
must remove `.github/**/*.md` from both `paths-ignore` blocks. This uses the
existing workflow; it adds no CI tool.

**5b — an empty changed-file set must not read as green.** The oracle already
fails closed on an *indeterminate* diff
(`scripts/test-coverage-oracle.ps1:171-173`), but "genuinely empty" and
"contract target not selected" must also be distinguished. G2 owns a red/green
pair proving that an empty staged-file set does not report PASS for the contract
target.

G2 also owns the `--workspace` lint gap noted under T3, and the
registration/fixture-extension contract that Packages B and C will use.

## Decision 6 — inherited, not re-argued

These are carried forward from the v3 review's "what v3 got right" section and
are **not open for re-derivation** in any G0/G1/G2 plan or review:

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

## Decision 7 — what the split costs

Stated for honesty, because a split is not free:

* Three review gates instead of one. Mitigated: each gate is ~1/3 the surface,
  and a FAIL no longer destroys two sound layers.
* Three shipments instead of one, so three PRs and three merges. Constitution XI
  (merge commits, no squash/rebase) applies to each; three merge commits is
  *better* bisect granularity, not worse.
* G1 cannot start until G0 is merged. Accepted: that serialization is exactly
  the failure containment being purchased.
* Packages B and C now depend on **G2**, not on a combined G. The program DAG is
  updated accordingly in the program decision.

## Open questions deliberately left to the per-package plans

* Exact work-unit boundaries and red/green pairing — owned by each plan.
* The seed registry's concrete entries — G1 defines the schema, G2 authors the
  repository-specific entries.
* Whether the G1 registry is deserialized from TOML (reusing the already-declared
  root `toml 0.8`) or constructed in Rust — G1's plan decides, constrained by
  Decision 6 item 3 applied to G1: no *new* dependency.
