---
type: session-memory
date: 2026-07-16
agent: ship
session: "2c95481b — 084-S durable staged_call ship + closure"
topic: "084-S / 089-F merged; post-merge closure"
---

# Session memory — 084-S durable staged_call provenance shipped

## Outcome

Shipment **084-S** (feature **089-F**, durable `staged_call` provenance via JSONL) merged to
`main` as merge commit **a0962f6** via **PR #253** (P-009 merge-commit). Closes the durability
gap where staged cross-file calls were lost across daemon dehydrate/rehydrate.

## Tasks completed

* `089.001-T` — export `staged_call` to `staged_calls.jsonl` on dehydration (deterministic, idempotent).
* `089.002-T` — rehydrate `staged_call` from JSONL on restart (idempotent, legacy-tolerant).
* `089.003-T` — restart integration test: rehydrated staging resolves via the post-pass, matching a full re-index oracle with no false edges.

## Files touched

Modified in the merged diff:

* `src/services/dehydration.rs` — `SCHEMA_VERSION` 5.0.0 -> 5.1.0; `serialize_staged_calls_jsonl`; staged export with reliable stale-sidecar removal (propagates non-NotFound errors).
* `src/services/hydration.rs` — generation-gated staged sidecar load; version allowlist accepts 5.1.0 / 5.0.0 / 3.0.0; `try_exists` errors propagated; `line_preview` char-boundary helper.
* `src/db/cozo_queries.rs` — `list_staged_calls_full()` returning `StagedCallRecord` (incl. `created_at`).
* Three new integration tests: `staged_call_dehydration_test.rs`, `staged_call_rehydration_test.rs`, `staged_call_restart_resolution_test.rs`.
* `Cargo.toml` — new `[[test]]` targets registered.
* `.backlogit/queue/{089.001-T,089.002-T,089.003-T}.md` -> `.backlogit/archive/` — the three task files were marked done and archived during the build (commit `8ad8124`, part of PR #253 / merge `a0962f6`).

Verified unchanged (listed for context, not part of the diff):

* `src/db/cozo_backend/schema.rs` — the `staged_call` relation stays at 4 columns.

## Decisions and rationale

* **Scope lock to the existing 4 columns** (`caller_id`, `callee_name`, `source_file`, `created_at`). Marker fields (`is_method`, `is_qualified`, `provenance`) are deferred to 088-S Unit B (`091.011-T`, blocks-on 089-F). Adding inert columns now would ship dead schema and front-run the 088-S adversarial gate. The JSONL format is forward-compatible (`#[serde(default)]`, tolerant of missing/extra keys) so 088-S B1 can add markers cleanly.
* **SCHEMA_VERSION 5.0.0 -> 5.1.0, generation-gated sidecar.** Nodes/edges format is unchanged, so 5.0.0 is grandfathered as valid input. The sidecar is trusted only when `.engram/.version == 5.1.0`. Fail-closed: an old 5.0.0 daemon rejects a 5.1.0 snapshot, preventing mixed-generation writes that would drop staged rows and later surface as false edges.

## Failed approaches

None material. One iteration during Copilot fixes: the restart test first compared raw `(from,to)` UUID pairs, which never match across independent index runs (per-run UUID node IDs). Corrected to compare stable `name@file` endpoint identities via `all_functions()`.

## Open questions

* 088-S Unit B (`091.011-T`) will add marker columns; must extend the accepted-staged-version condition and the JSONL schema forward-compatibly at that time.

## Next steps

* Queue order: **088-S** -> **083-S** -> **085-S** -> **086-S**.
* Operator caveat carried into closure: existing 5.0.0 workspaces need `engram update` or `engram reinstall` to bump `.version` to 5.1.0 before durable staging activates (`engram install` returns `AlreadyInstalled` on an existing workspace and does not restamp `.version`); binary-only upgrade hydrates but leaves staging dormant (safe, no false edges).
* Candidate hardening chore: flaky `t030_003_markdown_heading_and_code_block_indexed_via_ipc` under CI parallel/resource load.
