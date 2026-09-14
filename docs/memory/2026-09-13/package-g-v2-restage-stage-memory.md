---
type: session-memory
agent: stage
date: 2026-09-13
session: stage-package-g-v2-restage-20260913
branch: chore/checkpoint-resolution-ordering-restage
outcome: blocked
harvested: false
backlog_ids: none
shipment: none
---

# Stage session — Package G v2 (fresh planning authority)

## Outcome

**BLOCKED at plan review round 2.** No harvest, no backlog IDs, no shipment, no PR.

## Preflight

* Branch `chore/checkpoint-resolution-ordering-restage`, re-fetched, base
  `5020d1c4`, single worktree (`git worktree list` = 1).
* Tool gate: backlogit MCP surface unavailable in this session's tool set →
  `DEGRADED_MODE: backlogit (CLI fallback)`. `backlogit 1.10.1` CLI probed OK.
* `backlogit sync` → `INDEX_SYNC_OK` (1366 artifacts).
* Checkpoints: 24 total — 18 resolved, 6 abandoned, **0 active**, 0 quarantined,
  0 needing quarantine → ZERO-CANDIDATE NORMAL STARTUP. No recovery, no anomalies.

## Artifacts produced

| Path | Status |
|---|---|
| `docs/decisions/…-deliberation-v2.md` | `decided-plan-blocked` (supersedes v1) |
| `docs/exec-plans/…-plan-v2.md` | `blocked` (plan + hardening + R1 corrections) |
| `docs/closure/2026-09-13-package-g-v2-plan-review-record.md` | FAIL record, both rounds |

v1 deliberation and v1 plan were marked `superseded` via frontmatter only. Their
bodies and both v1 review rounds are byte-unchanged — review history preserved.

## Research corrections made this session

Re-derived every repository fact; several v1/v2-R1 claims were wrong.

* **Regular `[dependencies]` link into integration test targets.** Proved by
  `tests/helpers/mod.rs:39-42` importing `tempfile::TempDir`, where
  `tempfile = "3"` is a regular dep at `Cargo.toml:59`. So `ignore` (L75),
  `globset` (L76), `serde_yaml` (L77), `toml` (L60), `serde` (L31) are all usable
  with zero new dependencies.
* **267 `[[test]]` targets, all explicit `name` + `path`.** No `autotests` key,
  no root `tests/*.rs`. Auto-discovery (which covers only `tests/*.rs` and
  `tests/*/main.rs`) contributes nothing, so a `tests/contract/<dir>/` subtree
  cannot create a stray target.
* **No in-repo precedent for a subdirectory submodule tree.** The `pub mod f1;`
  in `tests/integration/canonical_call_resolution_test.rs:247-260` that v1
  planning cited is **fixture text inside a raw string** passed to `write_file`,
  not a module declaration. The real idiom is `#[path]`-included helpers across
  three files.
* **Test crates do not inherit `src/lib.rs`'s crate attributes** — including
  `#![forbid(unsafe_code)]` (L9) and the 20-entry allow list. A new test target
  faces raw pedantic and must carry its own `forbid`.
* **`allow(dead_code)` is broader than believed**: crate-level only in
  `tests/helpers/mod.rs:34`, but item-level in 6 more test files.
* **`clippy::unwrap_used`/`expect_used` are configured nowhere** (contradicting
  the AGENTS.md summary); contract tests use `.unwrap()` 55 times.
* **CI already executes any registered target** via `--all-targets`
  (`ci.yml:82-92`). The only gap is `paths-ignore: '.github/**/*.md'` on both
  `on.push` (L34) and `on.pull_request` (L41), which makes harness-only PRs skip
  CI entirely.
* **Coverage oracle baseline** `STATUS=PASS` (267 targets, 13 modules, 0 unmapped).
  Surfaces match by target-**name** glob and every `src/` surface lists
  `"contract_*"`, so a `contract_`-prefixed target auto-maps. `--mode report`
  prints counts only; **`--mode select` prints `TARGET=<name>`** — only `select`
  can prove a target is required for a diff.
* Harness inventory: agents 23, skills 28, instructions 34, policies 2, prompts 5
  = 92.

## Decisions established (affirmed by all 7 reviewers, both rounds — do not re-litigate)

1. **Option T-A**: one `[[test]]` target `contract_agent_harness_contracts` at
   `tests/contract/agent_harness_contracts_test.rs` with submodules under
   `tests/contract/agent_harness/`. Single consuming crate → every item
   reachable → no `allow(dead_code)`.
2. **Zero `verify_markdown` / `engram` coupling.** Harness owns its validator.
3. **Zero new dependencies.**
4. **`AssertionScope` 3×2 matrix** (`required`/`prohibited`/`exactly_one` ×
   `set`/`each_file`) — added in the correction round to answer Architecture's
   finding that per-file universal contracts ("every agent declares a model
   tier") were inexpressible and would have forced a `ForAll` evaluator variant,
   breaking the "A–F add rows, not code" criterion. Architecture validated the
   matrix against the live corpus and confirmed every sampled shape is data-only.
5. **Red-phase mechanism**: every `[R]` unit adds `todo!()` signatures **plus**
   the tests calling them, so the crate compiles at every commit and tests fail
   at runtime. Resolves the Principle II vs. "commit must be buildable" tension.
   Confirmed sound (`todo!()` has the never type; a test-called item is not dead).
6. **Failure precedence** `F3 → F7 → F5 → F4 → F6 → F1/F2/F8`; diagnostic sort
   key `(entry_order, normalized_path, code, match_ordinal)`.
7. **Two-layer containment**: data-layer `root` allow-list (F3) + resolver
   canonicalized containment (F5) — orthogonal, not redundant.

## Review history

**Round 1** — 7 personas, 7 distinct models (`gpt-5.6-sol`, `claude-opus-4.8`,
`gemini-3.8-flash`, `grok-4.6`, `claude-sonnet-5`, `gpt-5.6-terra`,
`claude-opus-4.7`): 4 PASS, 3 FAIL. 1 P0, ~16 P1. All bounded and mechanical →
one correction round authorized per operator instruction. 19 corrections applied;
unit count 15 → 21.

**Round 2 (confirmation)** — 4 personas re-run (3 R1-FAIL + Architecture):
Constitution **PASS** (all 8 of its findings resolved, 0 P0/P1); Architecture
**PASS** (A1, A2 resolved); Rust **FAIL**; Correctness **FAIL**.

**Every round-1 finding was confirmed resolved.** The FAIL comes from two **new**
P1 defects introduced by the correction itself.

## The two blocking issues a future attempt must settle FIRST

1. **B1 — Resource bounds are fail-open (3 of 4 reviewers).** The `max_depth(8)` /
   `max_filesize(262_144)` bounds added in response to the round-1 Security P2
   cause `ignore::WalkBuilder` to **silently omit** files rather than error. F4
   fires only on a wholly empty resolution and G.14's floors catch only wholesale
   loss, so one oversized in-scope harness file leaves the set silently and a
   `prohibited` assertion over it returns PASS. This reintroduces the exact
   silent-skip vector H4 claims to defend four ways, in a harness whose core
   property is fail-closed. **Fix**: report bound violations as a hard failure, or
   enforce the budget after resolution — never as a walk filter.
2. **B2 — F5 and F6 are unreachable end to end.** G.14 promises a table-driven
   F1–F8 proof through `run_registry`, but the runner validates the registry
   before resolving, so an escaping root is `RootNotAllowed`/F3 and never reaches
   the resolver (F5 unreachable); and the F6 throwaway-deletion mechanism, driven
   through the runner, yields F4 because the walk simply never sees the file.
   **Fix**: add a runner fault-injection seam, or scope the end-to-end claim
   honestly and prove F5/F6 at their own boundaries.

Also open (non-consensus / advisory): scenario-counting convention disputed
(Rust P1 vs Constitution+Architecture PASS — settle it before drafting units);
red-phase wording is stated universally but does not hold for G.1/G.10/G.16/G.18;
G.15 has a fourth inherited scenario; assertion matrix does not enumerate shapes
it cannot express (conditional, cross-file, ordering, numeric thresholds);
`prohibited`+`set` and `prohibited`+`each_file` have identical pass/fail
predicates; the five-directory allow-list makes a sixth `.github` path a
plan-level change.

## Gate rule applied

Operator: *"If P0/P1 are bounded mechanical issues, allow one correction +
confirmation; otherwise stop. PASS requires zero P0/P1."* One correction round was
executed; the confirmation round returned FAIL with new P1s. **Budget exhausted →
terminal FAIL, no harvest.** This was not escalated to a third round.

## Explicit non-actions

No implementation code written. No backlog item created — **no IDs allocated**,
`143.*` left abandoned. No shipment assembled. No stash entry archived. No PR;
PR #396 untouched. Packages A–F not modified or restaged. No `src/` change, no
dependency change, no edit to `.github/workflows/ci.yml` or
`.cargo/test-coverage-manifest.toml`.

## Next step

Re-plan Package G as a **fourth fresh Stage operation** after B1 and B2 are
settled, inheriting the seven affirmed decisions above. Not a third correction
round on this plan.
