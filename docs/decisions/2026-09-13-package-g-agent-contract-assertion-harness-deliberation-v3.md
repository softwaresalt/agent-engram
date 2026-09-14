---
type: deliberation
date: 2026-09-13
package: G
version: v3
supersedes: docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md
status: accepted
branch: chore/checkpoint-resolution-ordering-restage
head: fde75189
---

# Package G v3 — agent-contract assertion harness (deliberation)

## Authority and scope

This is a **fresh planning authority** for Package G only. The v2 attempt
(`docs/closure/2026-09-13-package-g-v2-plan-review-record.md`) closed **all
twenty round-1 findings** and failed the confirmation round on exactly two
correction-induced verification gaps, B1 and B2. This deliberation re-derives
only those two decisions from fresh repository evidence and carries the
affirmed v2 architecture forward unchanged.

**Explicitly out of scope and not re-litigated**: the affirmed architecture
(enumerated below), Packages A–F, PR #396, and `143.*`.

## Problem

Packages A–F will author machine-checkable contracts over the agent harness
corpus. They need a data-driven assertion runner that can state "this contract
text must appear", "this text must never appear", and "this text must appear
exactly once" over a fixed set of harness markdown files, and fail CI loudly
and deterministically when a contract is violated.

The runner's single most important property is **fail-closed**: no
configuration, no traversal option, and no error path may cause a violated
contract to report PASS.

## Fresh evidence (gathered this session, not inherited)

| # | Evidence | Value | Source |
|---|---|---|---|
| E1 | Harness corpus size | 92 files, **all** `.md` | recursive enumeration of the five roots |
| E2 | Maximum relative depth | **2** | same enumeration |
| E3 | Maximum file size | **98,546 bytes** (`.github/agents/_ship.agent.md`) | same enumeration |
| E4 | Second largest | 83,566 bytes (`.github/policies/workflow-policies.md`) | same enumeration |
| E5 | `[[test]]` targets | **267**, all explicit `name` + `path` | `Cargo.toml` |
| E6 | `autotests` key | **absent**; 0 root `tests/*.rs`; 0 `tests/*/main.rs` | `Cargo.toml`, filesystem |
| E7 | `contract_`-prefixed targets | 63 | `Cargo.toml` |
| E8 | Crate lints | `#![forbid(unsafe_code)]` at `src/lib.rs:10`, `#![warn(clippy::pedantic)]` at `:11` | `src/lib.rs` |
| E9 | Edition / MSRV | 2024 / 1.85 | `Cargo.toml:15-16` |
| E10 | `.github/**/*.md` in `paths-ignore` | present in **both** `push` and `pull_request` blocks | `.github/workflows/ci.yml` |
| E11 | Coverage oracle modes | `report｜select｜completeness｜run` | `scripts/test-coverage-oracle.ps1:19` |
| E12 | `allow(dead_code)` in `tests/` | **11 occurrences**, 6 files | recursive grep |
| E13 | `#[path = ...]` in `tests/` | **46 occurrences** | recursive grep |
| E14 | `mod.rs` files under `tests/` | exactly one (`tests/helpers/mod.rs`), consumed via `#[path]` | filesystem |
| E15 | `WalkBuilder` usage in `src/` | **zero** live call sites (one historical comment at `services/code_graph.rs:4353`) | recursive grep |
| E16 | Regular deps available to test targets | `tempfile` (`Cargo.toml:59`) imported at `tests/helpers/mod.rs:43` | both files |
| E17 | Candidate deps already declared | `toml 0.8` (:60), `ignore 0.4` (:75), `globset 0.4` (:76), `serde_yaml 0.9` (:77) | `Cargo.toml` |

**E12/E13 correct advisory A-1 from the v2 review.** The v2 deliberation claimed
`allow(dead_code)` appears "exactly once" and that there is "one `#[path]`
precedent". Both claims were wrong by an order of magnitude. The corrected
figures are recorded above and used below. This deliberation supersedes those
claims; the v2 deliberation is marked superseded rather than edited.

## Decision B1 — no silent walker omissions

### The v2 defect

v2 configured `WalkBuilder::max_depth(Some(8))` and
`max_filesize(Some(262_144))`. The `ignore` crate treats both as **filters**: a
matching file is omitted from the walk, not reported. Because F4 fires only on a
*wholly empty* resolution, a single in-scope file exceeding a bound leaves the
resolved set silently, and a `prohibited` assertion over that set returns PASS.
This is a fail-open regression in a harness whose core property is fail-closed.

### Options considered

**Option B1-1 — keep the bounds, convert omission into a reported failure.**
Enumerate without filters, then validate depth and size as an explicit
post-discovery pass that emits a deterministic, file-naming failure.

* Pro: retains an explicit resource budget; omission is structurally impossible.
* Con: the budget is not justified by any evidence (E2, E3). It adds a policy
  surface, two registry-or-constant knobs, and a failure code that no real input
  can trigger. Dead policy is a maintenance liability and a future fail-open
  temptation.

**Option B1-2 — remove the bounds entirely.**
Rely on the fixed five-root allow-list plus workspace containment. Enumerate
every regular file exhaustively with no filtering option of any kind.

* Pro: the class of defect becomes **structurally impossible** — there is no
  filter to misconfigure. Simplest correct thing. Directly satisfies the
  operator constraint "do not add speculative security controls that create
  fail-open behavior."
* Con: an unbounded walk over a hostile tree could be slow.
* Con assessment: the roots are **five compile-time constants** inside a
  test-only target, symlinks are rejected rather than followed, and the corpus
  is 92 markdown files at depth ≤2 (E1, E2). The worst realistic outcome of
  removing the bounds is a slower test, never a false PASS. The worst realistic
  outcome of keeping them is a false PASS.

**Option B1-3 — document the fail-open behaviour as an accepted residual.**
Rejected outright. It trades the harness's one non-negotiable property for a
bound that E2/E3 show never binds.

### Decision: **Option B1-2 — remove the bounds.**

Evidence E2 (max depth 2 vs. a cap of 8) and E3 (max 98,546 bytes vs. a cap of
262,144) show both v2 caps were **inert on every real input**. They could never
have fired in the present corpus, so they bought nothing; they could only have
fired in the future, on the largest and most contract-dense file in the
repository — `_ship.agent.md` is 98,546 bytes today and grows with every
P-021 clause — where firing would have silently deleted exactly the file the
harness most needs to inspect. The cap was therefore not merely useless but
anti-correlated with safety.

Consequences:

1. Enumeration is **exhaustive and non-filtering**. Every regular file under
   each allowed root enters the candidate set.
2. Glob matching (`globset`) is applied to the *candidate set*, and a file
   excluded by a glob is excluded **by the contract's own declared pattern** —
   an intentional, registry-visible narrowing, not a traversal-level omission.
3. F4 (empty set) is evaluated **only after** exhaustive enumeration.
4. No `max_depth`, no `max_filesize`, no size/depth policy code, no
   corresponding failure code.

### Corollary decision: plain `std::fs` recursion, not `ignore::WalkBuilder`

`ignore` is a dependency of this crate (E17) but has **zero live call sites**
(E15). Using it here requires explicitly disabling five separate default
behaviours (`.gitignore`, global ignore, `.ignore`, hidden-file skipping,
parent-directory ignore inheritance) — every one of which is a *silent* omission
by default, and each of which is a distinct opportunity to reintroduce B1.
v2 needed a dedicated work-unit pair (its G.6/G.7) for nothing but this
disabling.

A caller-driven `std::fs::read_dir` recursion has **no filtering options at
all**. B1 becomes unexpressible rather than merely avoided. It also composes
naturally with the injected filesystem boundary that Decision B2 requires,
which `WalkBuilder` — owning its own recursion — does not.

`globset` is retained for pattern matching. No dependency is added or removed;
`ignore` simply remains unused by this target, exactly as it is unused by `src/`
today.

## Decision B2 — reachable verification by layer

### The v2 defect

v2's end-to-end unit promised a table-driven proof of F1–F8 through
`run_registry`, but two codes are unreachable through that entry point:

* **F5** — the runner validates the registry *before* resolving, so an escaping
  or non-allow-listed root is rejected as `RootNotAllowed` → **F3** and never
  reaches the resolver.
* **F6** — v2's only unreadable mechanism deleted a throwaway file after
  resolution; driven through the runner the walk simply never yielded it,
  producing **F4**.

The acceptance criterion as written was unsatisfiable.

### Options considered

**Option B2-1 — add a fault-injection seam to the public runner.**
Make `run_registry` accept an injectable filesystem so every code is reachable
end to end.

* Pro: one uniform proof surface.
* Con: it makes the *public* entry point fault-injectable purely to serve tests,
  and the "end-to-end" proof then runs against a fake filesystem — which is not
  end-to-end in any meaningful sense. It papers over the layering question
  rather than answering it.

**Option B2-2 — honest layering: each code is proven at the layer that owns it.**
The end-to-end suite asserts only the codes reachable through the public runner.
Internal contracts are proven by layer-level tests against an injected boundary.
A traceability table maps every code to its legitimate test level.

* Pro: the public runner stays honest and un-instrumented. Each proof runs at
  the level where the behaviour actually lives. Prevalidation precedence becomes
  a *tested property* rather than an obstacle.
* Con: requires an explicit, documented reachability matrix so no reader
  mistakes the scoped end-to-end claim for a total one.

### Decision: **Option B2-2 — honest layering.**

Do not require an end-to-end scenario for a code that runner prevalidation
necessarily maps to an earlier code. Ownership is assigned as follows.

| Code | Meaning | Owning layer | Test level | Reachable via public `run_registry`? |
|---|---|---|---|---|
| F3 | Malformed registry: parse error, missing `scope`, root outside the allow-list | `registry` | registry unit | **Yes** |
| F7 | Duplicate assertion `id` | `registry` | registry unit | **Yes** |
| F5 | Resolution-phase failure: containment violation, or directory-listing / metadata access failure | `resolve` | resolver unit (injected boundary) | **No** — see note |
| F4 | Valid contained root resolving to zero files | `resolve` | resolver unit **and** runner integration | **Yes** |
| F6 | Read-phase failure: an *admitted* set member cannot be read | `read` | read-layer unit (injected boundary) | **No** — see note |
| F1 | Required contract missing | `assert` | assert unit **and** runner integration | **Yes** |
| F2 | Prohibited contract present | `assert` | assert unit **and** runner integration | **Yes** |
| F8 | Ambiguous multiplicity (`exactly_one` matched >1) | `assert` | assert unit **and** runner integration | **Yes** |

**Note on F5.** The registry allow-list is a strict subset of the contained
roots, so a registry that passes validation can never present an escaping root
to the resolver. F5 is therefore unreachable through `run_registry` **by
design** — that is prevalidation working correctly, not a coverage gap. F5 is
owned by direct `validate_root` / `resolve_file_set` unit tests. The runner
integration suite proves the *complement*: that a disallowed root surfaces as
F3, i.e. that prevalidation precedence holds.

**Note on F6.** F6 is a genuine runtime contract — an admitted member that
cannot be read must fail, never be skipped. The real filesystem cannot produce
it deterministically (it is a race by construction, and Windows file-locking
semantics make the v2 throwaway-file trick nondeterministic). It is therefore
proven at the read layer against the injected boundary and **deliberately not
asserted end to end**. The plan states this rather than claiming otherwise.

**F6 is retained, not merged.** It is distinguishable from F5 on an objective
axis: F5 means the candidate never entered the resolved set; F6 means it entered
and then could not be read. Both are reachable and separately observable through
the injected boundary, so neither is dead code. Precedence `F5 → F4 → F6` is
consistent with this reading: a resolution failure aborts before emptiness is
knowable, and an unreadable member presupposes a non-empty set.

**The runner must not falsely claim F6.** The runner resolves once and reads the
admitted set; it does not re-walk. A member deleted between discovery and read
therefore maps to **F6**, and that is the documented public contract. The runner
integration tests assert F4 only for the case that genuinely produces it — an
allowed root whose declared glob matches nothing.

### Injected boundary — `FileAccess`

A two-method trait defined in **test-only harness code** (the whole target is
test-only; nothing is added to `src/`):

* `list_dir(&Path) -> io::Result<Vec<Entry>>` — one directory level, with each
  entry's `symlink_metadata` file type.
* `read_to_string(&Path) -> io::Result<String>`.

Two implementations: `RealFs` (production path, used by `run_registry`) and
`FakeFs` (a programmed in-memory tree that can return a specific `io::Error` for
a specific path). No platform tricks, no permission manipulation, no throwaway
files, no `unsafe`. Deterministic on Windows, Linux, and macOS alike.

This is **not** a runner seam: `run_registry` takes no injection parameter and
always uses `RealFs`. The boundary is a parameter of the `resolve` and `read`
layers, which unit tests call directly.

## Decision B3 — scenario-counting convention (settled before drafting)

v2's review recorded a split verdict on unit sizing because "scenario" was never
defined. It is defined here, objectively, so sizing is not re-argued per
reviewer.

> **SC.** One scenario = one `#[test]` function. A work unit MUST declare
> **≤3** `#[test]` functions. A `#[test]` MAY be table-driven only when every
> row drives the same public entry point at the same pipeline stage and exercises
> **one** `form`×`scope` combination — its positive and negative cases together
> count as one scenario. Rows spanning different `form`×`scope` combinations,
> different pipeline stages, or different `DiagnosticCode` outcomes are
> **distinct scenarios** and MUST be separate `#[test]` functions.

SC ratifies the v2 Constitution reviewer's ruling (positive/negative of one form
is one scenario, not cap-evasion) while forbidding the packing the v2 Rust
reviewer objected to (eight F-codes in one table). It is mechanically checkable
by counting `#[test]` attributes and reading each test's assertion targets.

Applying SC to v2's units forces the end-to-end unit to split and raises the
unit count. That is the intended consequence: more, smaller units, each
genuinely task-sized.

## Decision B4 — red-phase wording narrowed (advisory A-2)

The compiling-`todo!()` red-phase mechanism is affirmed and retained, but its
statement is narrowed to the units it actually describes:

* **API-introducing `[R]` units** add minimal signatures with `todo!()` bodies
  plus the tests that call them. The target compiles; tests fail at runtime on
  the `todo!()` panic; `dead_code` cannot fire because every declared item has a
  caller in the same unit.
* **Data/config-asserting `[R]` units** (seed registry, CI workflow, coverage
  manifest) introduce no API. Their red phase is a compiling test that fails
  because the asserted repository state does not yet exist. The paired `[G]`
  unit creates that state.

Both are genuine failing-test-first red phases. Neither requires
`allow(dead_code)` — and note E12 shows the repository uses `allow(dead_code)`
11 times in `tests/`, so the v2 claim of a single precedent was wrong; the
convention here remains **zero** uses in this target regardless.

## Affirmed v2 decisions carried forward unchanged

These were affirmed by all seven reviewers across both v2 rounds and are
inherited as settled:

1. **Option T-A topology** — one explicitly registered `[[test]]` target named
   `contract_agent_harness_contracts` with a dedicated submodule subtree.
   Re-verified this session: E6 shows `autotests` is absent and no
   `tests/*.rs` or `tests/*/main.rs` exists, so a subtree cannot produce a stray
   cargo target. E7 confirms the `contract_` name prefix convention.
2. **Zero `verify_markdown` / `engram` coupling.** The target imports nothing
   from the library crate.
3. **Zero new dependencies.** `globset`, `toml`, `serde` are regular
   `[dependencies]` (E17) and link into test targets, proved by E16.
4. **Typed registry** with the **`AssertionScope` 3×2 form×scope matrix**;
   `scope` is required with no default.
5. **Compiling `todo!()` red phases** (as narrowed by B4).
6. **Failure precedence `F3 → F7 → F5 → F4 → F6 → F1/F2/F8`** and the
   four-tuple diagnostic sort key `(entry_order, normalized_path, code,
   match_ordinal)`. Unchanged — B2 required no precedence change, only an
   explicit layer-ownership statement.
7. **Two-layer containment**: the data-layer `root` allow-list (F3) and the
   resolver's canonicalized containment check (F5) remain orthogonal.
8. **CI `--mode select` contract** and the removal of `'.github/**/*.md'` from
   both `paths-ignore` blocks (E10 confirms both are still present).
9. **Diagnostic content policy**: workspace-relative `/`-separated paths;
   `detail` carries assertion metadata only, never file content.

## Non-goals

* No change to `src/`. No production code of any kind.
* No `verify_markdown` integration, now or later.
* No dependency addition, removal, or version change.
* No contract authoring for Packages A–F — this delivers the runner plus a
  minimal seed registry that proves the runner works against the live corpus.
* No conditional, cross-file-consistency, ordering, or numeric-threshold
  assertion forms. The matrix is deliberately 3×2; extending it is a plan-level
  change, recorded explicitly so a future author does not silently add an
  evaluator arm (v2 advisory A-5).

## Outcome

Proceed to planning with B1 resolved by **removal of speculative bounds**, B2
resolved by **honest layer ownership with a published reachability matrix**, B3
resolved by **convention SC**, and B4 resolved by **narrowed red-phase wording**.
