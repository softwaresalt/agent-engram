---
type: plan-review-record
date: 2026-09-13
package: G
plan: docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v2.md
source: docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v2.md
verdict: FAIL
rounds: 2
harvested: false
backlog_ids: none
shipment: none
branch: chore/checkpoint-resolution-ordering-restage
---

# Package G v2 — plan-review record

**Verdict: FAIL.** Correction budget (one correction + one confirmation) exhausted.
No harvest. No backlog IDs allocated. No shipment assembled. No PR.

## Gate rule applied

The operator authorized: *"If P0/P1 are bounded mechanical issues, allow one
correction + confirmation; otherwise stop. PASS requires zero P0/P1; P2-only
ADVISORY is not auto-harvested."*

Round 1 findings were bounded and mechanical, so one correction round was
authorized and executed. The confirmation round returned two FAIL verdicts
carrying **new** P1 findings — two of which were *introduced by the correction
itself*. The budget is spent; the gate is terminal.

## Reviewer panel — cross-model diversity

| Persona | Model | R1 | R2 (confirmation) |
|---|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | FAIL (4 P1, 2 P2) | **FAIL** (2 P1, 2 P2) |
| Architecture Strategist | `claude-opus-4.8` | PASS (2 P2, 2 P3) | **PASS** (1 P2, 3 P3) |
| Scope Boundary Auditor | `gemini-3.8-flash` | PASS (2 P3) | not re-run (R1 PASS, no design change in its scope) |
| Constitution Reviewer | `grok-4.6` | FAIL (1 P0, 2 P1, 2 P2, 3 P3) | **PASS** (0 P0/P1) |
| Security Reviewer | `claude-sonnet-5` | PASS (1 P2, 3 P3) | not re-run (R1 PASS) |
| Correctness Reviewer | `gpt-5.6-terra` | FAIL (9 P1, 1 P2) | **FAIL** (2 P1, 1 P2) |
| Maintainability Reviewer | `claude-opus-4.7` | PASS (1 P2, 4 P3) | not re-run (R1 PASS) |

Seven personas across seven distinct models in round 1; four re-run in the
confirmation round (the three R1-FAIL personas plus Architecture, whose P2 drove
a material design change).

## Required explicit checks — round 2 consensus

| # | Check | Consensus |
|---|---|---|
| 1 | No `verify_markdown` coupling | **PASS** (4/4) |
| 2 | Lint-clean modules | **PASS** (4/4) |
| 3 | All file ownership complete | **PASS** (4/4) |
| 4 | Test-first dependency graph | **PASS** (4/4, one P2 wording caveat) |
| 5 | CI red/green | **PASS** (4/4) |
| 6 | Empty-set / path containment | **FAIL** (2 FAIL, 1 PASS, 1 PASS-with-P2) |
| 7 | Deterministic diagnostics | **PASS** (4/4) |
| 8 | No dependency additions | **PASS** (4/4) |
| 9 | Unit sizing | **Split** (1 FAIL, 3 PASS) |

## Round-1 findings — all resolved

Every round-1 blocking finding was confirmed resolved by the reviewer that
raised it.

| R1 finding | Raised by | R2 status |
|---|---|---|
| Missing `#![forbid(unsafe_code)]` on the test crate | Rust | RESOLVED |
| Coverage-manifest edit had no red predecessor | Rust, Correctness | RESOLVED (G.18/G.19) |
| Containment checked only after the walk | Rust, Correctness | RESOLVED (pre-walk root validation) |
| `allow(dead_code)` / `#[path]` precedent misstated | Rust | RESOLVED in plan |
| Missing `tests/fixtures/agent_harness/` surface | Rust | RESOLVED |
| End-to-end wiring unit was green-only | Constitution (P0), Rust, Correctness | RESOLVED (G.14 `[R]` / G.15 `[G]`) |
| `[R]` units were compile-fail, not compiling-but-failing | Constitution | RESOLVED (red-phase mechanism) |
| `cargo audit` omitted from the gate sequence | Constitution | RESOLVED (G.21) |
| Constitution Check incomplete / overstated | Constitution | RESOLVED (full I–XI mapping) |
| Unreadable-file mechanism unspecified | Constitution | RESOLVED (in-workspace throwaway) |
| `.gitignore` / ignore sources not disabled | Correctness | RESOLVED (G.6/G.7) |
| F1–F8 precedence undefined | Correctness | RESOLVED (`F3→F7→F5→F4→F6→F1/F2/F8`) |
| `exactly_one` counting domain undefined | Correctness, Architecture | RESOLVED (`AssertionScope`) |
| Diagnostic order not total; platform-dependent paths | Correctness | RESOLVED (4-tuple key + `normalize_path`) |
| Registry TOML referenced before creation | Correctness | RESOLVED (chain reordered) |
| Only F1 proven end to end | Correctness | **PARTIAL** — see B1 |
| `--mode report` cannot prove target requirement | Correctness | RESOLVED (`--mode select`) |
| Exact count pinning brittle | Maintainability, Correctness | RESOLVED (floors) |
| Per-file universal contracts would need evaluator code | Architecture | RESOLVED (3×2 form×scope matrix) |
| "Independent branches" overstated | Architecture | RESOLVED (single total chain) |

## Blocking findings — round 2

### B1 — Resource bounds silently drop files (fail-open) — HIGH consensus

* **Raised by**: Rust (P1), Correctness (P1), Architecture (P2 / NF-1) — 3 of 4.
* **Introduced by**: the correction round itself, in response to the round-1
  Security P2 requesting concrete resource bounds.
* **Defect**: `WalkBuilder::max_depth(Some(8))` and
  `max_filesize(Some(262_144))` cause `ignore` to **silently omit** matching
  files from the walk rather than erroring. F4 fires only on a *wholly empty*
  resolution, and G.14's per-directory floors catch only wholesale loss. A single
  in-scope harness file exceeding the cap therefore leaves the resolved set
  silently, and a `prohibited` assertion over that set returns PASS.
* **Why it matters**: this reintroduces precisely the silent-skip false-PASS
  vector that H4 claims to defend "four ways", and it is a **fail-open**
  regression in a harness whose core security property is fail-closed.
* **Required fix**: treat an in-scope file that violates a bound as a hard
  reported failure (an F6-class bounds error) rather than a walk filter — or
  drop the walker-level bounds and enforce the budget after resolution, so
  omission is never silent. Architecture's alternative (documenting it as an
  accepted residual in H6) is viable only if the fail-open behaviour is stated
  explicitly, which the plan currently does not do.

### B2 — F5 and F6 remain unprovable end to end — single reviewer, verified

* **Raised by**: Correctness (P1).
* **Defect**: G.14 scenario 1 promises a table-driven end-to-end proof across
  F1–F8 through `run_registry`, but the pipeline makes two of them unreachable:
  * **F5**: `run_registry` validates the registry *before* resolving, so an
    escaping or non-allow-listed root is rejected as `RootNotAllowed` → **F3**
    and never reaches the resolver. F5 is provable only through the direct
    `validate_root` API (G.4), not through the runner.
  * **F6**: the only specified unreadable mechanism deletes the throwaway after
    resolution. Driven through the runner, the walk simply never yields the
    deleted file, producing **F4** instead of F6. No runner seam or deterministic
    post-resolution/pre-read hook is specified.
* **Why it matters**: the correction was made specifically to close round-1
  finding G-09 ("only F1 proven end to end"). It closes it for F1–F4, F7, F8 but
  not for F5 and F6, so the stated acceptance criterion cannot be met as written.
* **Required fix**: either introduce an explicit runner seam allowing fault
  injection between resolve and read, or restate G.14 scenario 1 to cover the
  codes reachable through `run_registry` and prove F5/F6 at their own boundaries,
  with the plan saying so plainly rather than claiming full end-to-end coverage.

### B3 — Unit sizing: scenario packing — SPLIT, not consensus

* **Rust (P1)**: several units stay under four only by packing independent cases
  — G.8 (seven outcomes under three labels), G.10, G.12 (F1–F8 mappings plus
  precedence plus determinism), G.14 (eight F-code cases as one table-driven
  scenario), G.20 scenario 3 (three distinct checks).
* **Counter-position**: Constitution explicitly ruled that "positive/negative of
  one form in one scenario is a single scenario, not cap-evasion", and both
  Constitution and Architecture independently counted every unit at ≤3 and
  passed check 9. Correctness raised only a **P2** (G.15 inherits three scenarios
  plus a separately demonstrated extensibility case = four).
* **Disposition**: recorded as a **MEDIUM-confidence** finding, not consensus-
  blocking on its own. It is listed here because a future attempt must settle the
  counting convention explicitly rather than leave it to reviewer interpretation.

## Advisory findings carried forward (not blocking, do not auto-harvest)

| ID | Severity | Raised by | Summary |
|---|---|---|---|
| A-1 | P2 | Rust | The **source deliberation** still carries the uncorrected R3/R4 claims (`allow(dead_code)` appears "exactly once"; one `#[path]` precedent). The plan's R3a/R4 are correct; the deliberation must be corrected or marked stale. |
| A-2 | P2 | Rust | The red-phase mechanism is stated universally but does not hold literally for G.1 (placeholder), G.10 (extends an existing `evaluate` arm), and G.16/G.18 (fail against existing repository data, add no new API). Narrow the wording to units that introduce new APIs. |
| A-3 | P2 | Correctness | G.15 inherits G.14's three scenarios plus a fourth extensibility-acceptance demonstration. |
| A-4 | P3 | Constitution | Constitution Check cites "H1–H7"; Plan Hardening now contains H1–H8. Stale cross-reference. |
| A-5 | P3 | Architecture (NF-2) | The assertion matrix does not enumerate shapes it *cannot* express — conditional (`if X then Y`), cross-file consistency, ordering, numeric thresholds beyond 0/1/≥1. These are legitimate deferrals, but the boundary is implicit; an A–F author hitting the wall could silently add an evaluator arm. |
| A-6 | P3 | Architecture (NF-3) | `prohibited`+`set` and `prohibited`+`each_file` have identical accept/reject predicates and differ only in diagnostic granularity. The matrix prose reads as two distinct predicates. |
| A-7 | P3 | Architecture (NF-4) | The five-directory `root` allow-list means any future contract over a sixth `.github` path is a plan-level change, not a row-add — a latent tension with the "rows not code" claim. |
| A-8 | P3 | Security (R1) | `serde_yaml 0.9` is upstream-archived; G.16 adds a new consumer. Inherited, not introduced. CODEOWNERS for the registry recorded as an out-of-scope follow-up. |

## What is affirmed and must not be re-litigated

All seven reviewers across both rounds affirmed the core architecture. A future
attempt inherits these as settled:

1. **Option T-A** — one explicitly registered `[[test]]` target named
   `contract_agent_harness_contracts` with a dedicated submodule tree. Confirmed
   lint-clean, `dead_code`-free, and incapable of producing a stray cargo target.
2. **Zero `verify_markdown` / `engram` coupling** — unanimous PASS, both rounds.
3. **Zero new dependencies** — unanimous PASS. `ignore`, `globset`, `serde_yaml`,
   `toml`, `serde` are regular `[dependencies]` and link into test targets
   (proved by `tests/helpers/mod.rs:39-42` importing `tempfile`, a regular dep at
   `Cargo.toml:59`).
4. **The `AssertionScope` 3×2 matrix** — Architecture validated it against the
   live corpus (`_orchestrator.agent.md`, `_ship.agent.md` frontmatter tiers;
   `## Role Boundary (NON-NEGOTIABLE)` singularity) and confirmed every sampled
   contract shape is expressible as data.
5. **The red-phase mechanism** — compiling `todo!()` signatures plus their calling
   tests. Confirmed sound: `todo!()` has the never type and coerces to any return
   type; an item called by a test in the same unit is not dead code. Resolves the
   Principle II vs. "commit must be buildable" tension.
6. **The failure precedence `F3 → F7 → F5 → F4 → F6 → F1/F2/F8`** and the
   four-tuple diagnostic sort key.
7. **CI integration** — `--all-targets` already executes a registered target; the
   only edit needed is removing `'.github/**/*.md'` from both `paths-ignore`
   blocks (verified present at `ci.yml` lines 34 and 41).
8. **Two-layer containment** — data-layer `root` allow-list (F3) plus resolver
   canonicalized containment (F5) are orthogonal, not redundant.

## What a future attempt must settle first

1. **Resolve B1**: decide whether walker-level resource bounds are used at all.
   If they are, omission must be reported, never silent. This is the one place
   where the security hardening and the fail-closed invariant are in direct
   conflict, and the plan currently resolves it in favour of fail-open.
2. **Resolve B2**: either specify a runner fault-injection seam, or scope the
   end-to-end claim honestly to the codes reachable through `run_registry`.
3. **Settle the scenario-counting convention (B3)** before drafting units, so
   sizing is not re-argued per reviewer.
4. **Correct or stale-mark the v2 deliberation's R3/R4** (A-1).

## Explicit non-actions

No implementation code written. No backlog item created — **no IDs allocated**.
No shipment assembled. No PR created; PR #396 untouched. `143.*` left abandoned.
Packages A–F not modified or restaged. No stash entry archived. No `src/` change.
No dependency change. No change to `.github/workflows/ci.yml` or
`.cargo/test-coverage-manifest.toml` — the plan proposes those edits; none were
applied.
