# Deliberation — cozo 0.7 → 0.8+ major bump feasibility (clear lz4_flex RUSTSEC-2026-0041)

- **Date:** 2026-07-29
- **Cycle:** Stage cycle 3
- **Deliberation:** 017-D
- **Source stash:** 99AFF44B (task/low)
- **Disposition:** DEFER — deliberation + investigate-first spike shape; NOT
  scheduled into a shipment this cycle.

## Problem frame

102-F/093-S remediated 9 of 10 baseline cargo-audit vulnerabilities, leaving
exactly **one accepted-with-rationale** advisory: **RUSTSEC-2026-0041**
(`lz4_flex 0.10.0` — decompressing invalid data can leak uninitialized memory).

Dependency chain (from `docs/decisions/2026-07-28-cargo-audit-advisory-triage.md`):

```
engram → cozo 0.7.6 → swapvec 0.3.0 → lz4_flex 0.10.0
```

`swapvec` is a **non-optional** cozo dependency (not feature-gated, so it cannot
be dropped the way the `requests → minreq` HTTP path was in 102-F). `swapvec 0.3.0`
pins `lz4_flex ^0.10`; the fix (`≥0.11.6`) is a `0.x` breaking bump the range
rejects. There is **no in-range fix at cozo 0.7.6** (confirmed: `Cargo.toml:31`
pins `cozo = { version = "0.7", … }`). Clearing the advisory requires either an
upstream cozo/swapvec release using `lz4_flex ≥0.11.6`, **or** a **cozo 0.8+
major bump** with graph-backend blast radius (`storage-sqlite`,
`storage-sqlite-src`, `graph-algo` API surface — the exact features engram
enables at `Cargo.toml:31`).

Exposure (already assessed in the triage doc): `swapvec` is cozo's internal
on-disk spill for large query intermediates; the compressed bytes are produced
and consumed by swapvec itself (trusted round-trip), **not** attacker-controlled
input. Real-world exposure for engram's embedded, local usage is low. CI runs
`cargo audit` with `continue-on-error: true` (non-blocking).

## Options

- **Option A — take the cozo 0.8+ major bump now.** Clears the advisory
  directly. BUT: unknown API delta across `storage-sqlite`/`graph-algo`; engram's
  Datalog scripts (`cozo_queries.rs`, thousands of lines of CozoScript),
  schema bootstrap, and the bundled-SQLite storage path all ride on cozo's public
  surface. Blast radius is large and **unscoped** — cannot be honestly bounded to
  a single ≤2h task, and a mis-scoped plan would violate the 2-hour rule and
  width-isolation (schema + query + storage all at once).
- **Option B — wait for an upstream in-range fix.** Track `swapvec` / `lz4_flex`
  and cozo point releases; take a patched `lz4_flex 0.10.x` (if backported) or a
  cozo 0.7.x that re-pins swapvec — zero blast radius, clears the advisory for
  free. Contingent on upstream; no engram work until it lands.
- **Option C — hold at accepted-with-rationale (status quo).** The advisory is
  already documented and accepted (low exposure, trusted round-trip, non-blocking
  CI). Zero work, zero risk; the one residual advisory stays visible in `cargo
  audit` output.

## Chosen direction

**DEFER (Option C now, gated toward Option B; Option A only after a scoped
spike).** Rationale:

1. No urgency — the advisory is accepted-with-rationale (low real exposure,
   non-blocking CI), so there is no correctness or security pressure to force a
   risky major bump into a low-priority slot.
2. Option A's blast radius is **unproven**. Before any implementation plan, a
   **spike** must scope the cozo 0.8 API delta against engram's usage:
   - enumerate engram's cozo public-API touchpoints (`db::cozo_backend`,
     `cozo_queries.rs` script runners, schema bootstrap, storage/db-open path);
   - build a throwaway branch bumping `cozo` to the latest 0.8+ and capture the
     compile-error surface + any CozoScript/behavioural changes (relation
     syntax, `graph-algo` signatures, SQLite storage-src changes);
   - confirm whether the 0.8+ line actually re-pins `swapvec`/`lz4_flex` to a
     patched version (the whole point — verify the bump CLEARS RUSTSEC-2026-0041,
     not just moves it);
   - estimate the true task decomposition (likely multi-task: manifest bump →
     script/API adaptation → storage migration validation → recall/gate
     re-green) and whether each sub-unit fits ≤2h.
   - **Spike is Ship/runtime-executed** (needs `cargo build` against the new
     cozo) — out of Stage's role scope; Stage records the investigation shape
     here and defers the hands-on branch.
3. Prefer Option B opportunistically: if an upstream in-range fix lands first, it
   supersedes this entirely.

**Not scheduled into a shipment this cycle.** Revisit triggers: (a) cozo/swapvec
publishes with `lz4_flex ≥0.11.6`; (b) a patched `lz4_flex 0.10.x` backport; or
(c) an independent reason to move to cozo 0.8+ (perf/feature), at which point the
advisory clears as a side effect and the spike above scopes the migration.

## Open questions (for the spike)

1. Does the current cozo 0.8+ release actually re-pin `swapvec`/`lz4_flex` to a
   patched line? (If not, Option A does not even clear the advisory.)
2. How large is the CozoScript/relation-API delta 0.7.6 → 0.8+ for engram's
   query surface?
3. Does the bundled-SQLite storage format change (cold-restart / on-disk DB
   compatibility — cf. `cozo_cold_restart_test`)? Any migration needed for
   existing `.engram` DBs?
4. Can the migration be decomposed into ≤2h width-isolated tasks, or is it an
   irreducible large change requiring a dedicated multi-shipment epic?

## Notes

- Governed by the 102-F triage decision (accepted-with-rationale) and the
  102.001-T guardrail ("Do NOT pull in major-version breaks under this
  low-priority task — defer those as separate items").
- Layers on nothing; independent. Safe to defer indefinitely.
- Stash 99AFF44B remains **active** (deliberation-linked), consistent with the
  015-D / 016-D deferred-at-deliberation handling.

## 2026-08-10 Resolution — consolidated with 27F691AE

The critical intake `27F691AE` re-opened this decision with current upstream evidence. The earlier premise that a Cozo 0.8+ release might be available is false as of 2026-08-10: crates.io and Cozo `main` both remain at `cozo 0.7.6`, whose non-optional dependency is `swapvec ^0.3.0`. The newest published `swapvec` is 0.4.2, but both 0.4.2 and its current `main` still require `lz4_flex ^0.10.0`; therefore neither a Cozo point update nor a swapvec release clears `RUSTSEC-2026-0041`. A direct Cargo override to `lz4_flex 0.11.6` cannot satisfy swapvec's `^0.10.0` range.

The exposure rationale remains bounded but does not constitute remediation. Cozo uses `swapvec::SwapVec::default()` for temporary query collection; swapvec defaults compression to `None`. If LZ4 is selected, swapvec calls `decompress_size_prepended` into a fresh vector, and lz4_flex 0.10.0 enables `safe-decode` by default. These facts substantially reduce practical exposure, but Cargo still resolves the affected crate and `cargo audit` correctly reports the advisory.

**Resolved direction: SPIKE, not implementation.** Cozo upgrade is unavailable; wholesale Cozo removal/replacement is disproportionate; containment alone does not clear the advisory; and an incompatible direct override is invalid. The narrow candidate is a reviewed swapvec-compatible patch (for example, a pinned fork or vendored 0.3-compatible package that changes only the lz4_flex requirement), but source compatibility, provenance, lock behavior, cross-platform compilation, and Cozo runtime/data compatibility require hands-on proof. A hardened, security-reviewed, time-boxed spike plan is `docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md`. No production remediation plan is authorized until that spike returns a high- or medium-confidence executable recommendation and proves that `cargo audit` no longer reports the advisory.

This resolution supersedes the earlier indefinite defer and consolidates deliberation `017-D` with stash `27F691AE` into one security decision stream.