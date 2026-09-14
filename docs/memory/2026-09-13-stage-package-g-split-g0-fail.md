# Stage session memory — Package G decomposition into G0/G1/G2

* **Date**: 2026-09-13
* **Branch**: `chore/checkpoint-resolution-ordering-restage`
* **HEAD at session start**: `93370734` (re-fetched, confirmed)
* **Agent**: Stage
* **Mode**: DEGRADED — backlogit MCP tools unavailable in this session; CLI
  fallback `C:\Tools\backlogit.exe` v1.10.1 used. `backlogit sync` succeeded
  (1366 artifacts indexed).

## Outcome in one line

The **split was validated**; **G0's own plan FAILED review** at attempt 1 on two
architecture-level P1 findings, so **nothing was harvested and no IDs were
allocated**.

## What was produced

| Artifact | Path |
|---|---|
| Split deliberation (deep, topology options) | `docs/decisions/2026-09-13-package-g-split-deliberation.md` |
| Program decision (combined G permanently superseded) | `docs/decisions/2026-09-13-package-g-split-program-decision.md` |
| G0 deliberation | `docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation.md` |
| G0 plan (+ `## Plan Hardening`) | `docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan.md` |
| G1 deliberation | `docs/decisions/2026-09-13-package-g1-contract-assertion-engine-deliberation.md` |
| G1 plan (draft-dependent) | `docs/exec-plans/2026-09-13-package-g1-contract-assertion-engine-plan.md` |
| G2 deliberation | `docs/decisions/2026-09-13-package-g2-harness-registry-ci-deliberation.md` |
| G2 plan (draft-dependent) | `docs/exec-plans/2026-09-13-package-g2-harness-registry-ci-plan.md` |
| G0 attempt-1 review record | `docs/closure/2026-09-13-package-g0-attempt-1-plan-review-record.md` |

## Decisions made

1. **Split boundary**: by dependency layer — G0 filesystem seam → G1 assertion
   engine → G2 registry/CI. Rejected: fourth mega-plan, split-by-test-tier,
   two-way split.
2. **Topology (T3)**: one dev-only workspace member library crate per package
   (`crates/agent-contract-fs`, `crates/agent-contract-assert`), each exercised
   by a root-package `[[test]]` target, each mapped by its own
   `[[surface]]`. Chosen because `-Dwarnings` makes `dead_code` a hard error and
   `#[path]`-shared test modules structurally force `allow(dead_code)` (11 such
   suppressions exist today), while a library crate's reachable `pub` items are
   never dead. Also satisfies the coverage oracle's `src/`-or-`crates/`-only
   completeness rule, and needs no CI change because CI runs root-package
   targets.
3. **`exactly_one` settled** (G1): `n=0 ⇒ MissingRequired`, `n=1 ⇒ pass`,
   `n>1 ⇒ AmbiguousMultiplicity`, as arms of one `match`, so emitting both is
   unrepresentable.
4. **Program DAG updated**: `G0→G1→G2`, and `G2→B`, `G2→C` (previously `G→B`,
   `G→C`). G0 is the new in-degree-0 program root.

## Correction made to my own work mid-session

The split deliberation originally asserted "no `crates/` surface exists yet"
(S10). That was **false** — produced by a truncated grep;
`crates/powerbi-tmdl-parser/` already exists at
`.cargo/test-coverage-manifest.toml:137-139`. Caught by the G1 planner agent,
verified, and corrected in place as S10/S10a rather than silently edited. The
correction *strengthened* the topology argument.

## Review outcome — G0 attempt 1

Panel: 7 personas / 7 distinct models. **3 FAIL / 4 PASS.**

**Terminal (architecture-level P1, correction budget never opened):**

* **A** — `FsError` has no variant for a *failing* `symlink_metadata` or
  `canonicalize`; two of four seam methods have no failure channel. Raised
  independently by Rust (`gpt-5.6-sol`) and Correctness (`gpt-5.6-terra`).
* **B** — corpus identity undefined; duplicate/nested allow-listed roots emit a
  multiset, which would make G1's `exactly_one` report false multiplicity.
  Raised by Correctness.

**Contested and downgraded**: workspace-root lstat (Correctness P1 → P3) —
Architecture (adjudicator) and Security both ruled it a trusted anchor, a
documentation gap not a fail-open.

**Affirmed, do not re-derive**: the split itself; v3 findings A and B both
CLOSED; the four-method seam; the `FsCall` call-log ordering proof; the `FakeFs`
canonicalization map; the no-bounds decision; the Constitution Check mapping.

16 mechanical findings (M-1..M-16) and 13 advisories (A-1..A-13) are recorded in
the review record for the next attempt to inherit.

## Explicit non-actions

No source, test, or config file touched. No `Cargo.toml`, `.cargo/**`,
`tests/**`, `crates/**`, or `.github/workflows/**` change. No backlog item, no
ID, no shipment, no PR. PR #396 untouched. `143.*` still abandoned. Packages A–F
untouched. No stash entry archived. G v1/v2/v3 deliberations, plans, and the v2
and v3 review records read as evidence and **not modified**. G1 and G2 plans not
reviewed; preserved as dependent drafts with `review_verdict: not-reviewed`.

## Next step for a future session

Re-plan **G0 only** (attempt 2). The nine numbered items under "What the next G0
attempt must settle first" in the review record are the entry point. Do **not**
reopen the split, the seam method set, or the bounds decision. G1/G2 drafts stay
as-is until G0 passes.

Pre-existing dirty state left untouched: ` M .backlogit/stash.jsonl` and an
untracked `.backlogit/checkpoints/checkpoint-20260914-045836.json`, both present
at session start.
