---
type: session-memory
date: 2026-08-22
agent: ship
shipment: 123-S
feature: 127-F
stash: DE460A88
batch: dark-factory-20260820-870b1aff-568b257c-c2413934-de460a88 (order 4/4)
---

## Summary

Recovery run that finished shipment 123-S (independent agent-visible MCP catalog
oracle) end-to-end after an interrupted prior attempt. The interrupted run had
created an isolated worktree, committed a RED harness (3 `test(shim)` commits),
left an uncommitted backlog claim, and written two untracked guard scripts — but
never pushed, opened a PR, merged, or wrote closure. No duplicate work was
created; the clean partial work was reused.

## Recovered starting state

- origin/main baseline `ae481b64` (122-S closure PR #356) — authoritative.
- Worktree `.worktrees/ship-123-s-mcp-catalog-oracle-20260822` on
  `feat/123-s-mcp-catalog-oracle` at `b400c3ef` (3 committed RED test commits).
- Committed: `tests/contract/mcp_catalog_oracle_test.rs`,
  `tests/helpers/mcp_catalog_capture.rs`,
  `tests/fixtures/mcp_tool_catalog.expected.json`, `Cargo.toml` [[test]].
  Committed backlog status was still `queued` (claim not committed).
- Uncommitted: backlog claim (123-S/127-F/127.001-008-T -> active) and untracked
  `scripts/check-oracle-independence.{ps1,sh}`.
- Predecessor terminal proof: 120-S/121-S archived, 122-S shipped, all
  merge-backed (PRs #349/#350, #353/#354, #355/#356).

## Work completed

- Verified RED evidence (committed skeleton) and GREEN (7->9 oracle tests).
- U5: added the missing in-test independence assertion (tokens assembled from
  fragments to avoid self-trip); fixed a guard false-positive (fixture prose
  legitimately names `tools_catalog.rs`) by scoping the token scan to the two
  `.rs` sources; fixed a ps1 single-line `Get-Content` StrictMode crash.
- U7: documented fixture maintenance procedure; corrected stale 18->21 tool count.
- U8: closure record with induced-drift and guard-violation evidence.
- Wired the independence guard into CI (`build` job step) — copilot review.
- Hardened schema comparison: malformed `required` / `properties` now surface as
  differences instead of normalizing away — copilot review + regression tests.
- Merged implementation PR #357 (merge commit `37636f51`).
- Shipped 123-S; archived 127-F and 127.001-008-T as done.

## Decisions and rationale

- Forbidden-token scan targets code (`.rs`), not the JSON fixture: the fixture is
  data whose independence is enforced by the regeneration scan + human header;
  its `_policy` legitimately references the source contract path.
- Deferred nonblocking P2/P3 to 129-F (guard-robustness: line-scoped
  regeneration bypass, bare `>` verb, hardcoded scan paths, duplicate-name
  collapse) rather than a 4th review cycle (hard 3-cycle limit).

## Review cycles

3 (hard max): standard (Rust Reviewer, 0 P0/P1) + two Copilot cycles (CI
enforcement; malformed required/properties). The 4th Copilot finding
(duplicate-name collapse) was deferred to 129-F with rationale, not fixed.

## Environment notes

- Engram CLI degraded in the linked worktree ("not a Git repository root"),
  retried once -> TOOL_DEGRADED; used targeted file reads + Git history.
- `backlogit sync` fails on 19 pre-existing malformed archive files (029.* /
  030.005-C) unrelated to 123-S -> INDEX_SYNC_WARN, not touched (out of scope,
  also dirty in root).
- `start-launcher-windows` CI check is a flaky timing test (8s budget vs 9.1s on
  a slow runner); passed on re-run and on subsequent commits.
- Dirty root `C:\Source\GitHub\engram` never touched; all work in the isolated
  worktree.

## Next steps

- Close this closure shipment/PR; run compact-context (batch complete).
- Address 129-F guard-hardening follow-ups when scheduled.
