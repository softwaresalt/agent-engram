---
title: "136-S compacted memory summary (generation domain, store, atomic publication, database open)"
date: 2026-09-08
shipment_id: "136-S"
feature_id: "142-F"
pr: 385
status: "archived (archived_status: done)"
compacted_from:
  - "docs/memory/2026-09-07-ship-136-s-generation-domain-store-session.md"
  - "docs/memory/2026-09-08-ship-136-s-adversarial-review-remediation-session.md"
  - "docs/memory/2026-09-08-ship-136-s-utf8-fix-and-review-cascade-session.md"
compacted_by: "compact-context (Ship Step 6 item 8, P-020 post-merge closure)"
---

## What was delivered

`136-S` (feature `142-F`) delivered the generation domain and module
(`142.011-T`: `src/services/generations/mod.rs`, `manifest.rs` — strict
single-component generation IDs, checked monotonic revision, sealed
inventory, per-file digests, provenance), the generation store surface
(`142.012-T`: candidate-directory sealing, case-insensitive reserved-name
rejection, path-escape/junction-escape guards), atomic publication
(`142.013-T` + 4 subtasks: `publish.rs` — bounded `PublisherLock`, atomic
manifest replace, orphaned-staging detection, monotonic revision guarding,
concurrent-publisher serialization), the runtime-copy database-open path
(`142.014-T`: `src/db/cozo_backend/mod.rs` — hashed bounded-reader
integrity check, `final_path` UTF-8 validation ordered before any
mutation), and the cheap-clone generation context handle (`142.017-T`).

## Key decisions and rationale

* Fixed `GenerationId`'s Windows drive-prefix (`"C:"`) rejection gap via
  an additive `:`-rejection check rather than switching to a
  `Path::components()`-based check, to avoid narrowing existing portable
  `/`/`\\`/`..` rejections (a pre-existing platform-dependent-comparison
  inconsistency is tracked separately as stash `96A1197D`).
* Chose a doc-only fix (no new error variant) for `PublishError::
  AtomicReplace`'s committed-but-durability-uncertain semantics, per
  adversarial-review recommendation, to avoid overengineering a narrow,
  POSIX-only, rare failure window.
* Moved `open_existing_generation_via_runtime_copy`'s `final_path` UTF-8
  validation to immediately after computing `final_path`, before any
  mutation (`create_dir_all`, lock-file creation, `publish_runtime_copy`)
  — the pre-fix code validated only after those mutations already ran.
* Deferred (not fixed) the runtime-copy-vs-live-handle lifetime design gap
  (stash `9108DB24`) and the `generation_id`-not-bound-to-`ExistingDbLocation`
  identity-binding gap (stash `A67EEA54`), both `requires_deliberation:
  true` and both genuinely requiring a design decision unreachable via a
  targeted patch — no production caller of the affected code exists yet
  (F17/F18 wiring is future scope).
* Ran a bounded 3-model local adversarial review (`gpt-5.6-sol`,
  `claude-opus-4.6`, `gemini-3.6-flash`) after 4 authorized hosted-review-
  fix cycles plus 3 further Copilot passes left 4 Mandatory findings
  unresolved and non-converging — this successfully broke the
  non-convergence and fixed all 4 plus 2 more findings the adversarial
  pass independently confirmed.
* Concluded the Copilot review-response loop after 6 consecutive rounds of
  new findings surfacing after every push (including docs/stash-only
  pushes), per the explicit no-unbounded-loop directive, once the
  authorized fix was fully landed and every surfaced thread was
  replied-to/resolved-or-deferred with full P-021 traceability.

## Failed approaches / non-convergence

* An initial attempt to force the shipment record's canonical `shipped`
  status via `backlogit move --status shipped` (surfaced during a related
  Copilot review round, generalized from the `133-S`/`134-S` precedent) is
  **unconditionally rejected** by the CLI (`ErrShipmentShippedRequiresEnvelope`).
  The only path to canonical `shipped` is the cascade `backlogit shipment
  ship` operation, which is P-015-forbidden here because `142-F` is a
  shared partial-covering root. `archived_status: done` is the deliberate,
  correct terminal status for this shipment family (see post-merge
  closure record for the actual safe-close command log).
* 4 rounds of hosted Copilot review plus 3 further passes left 4 Mandatory
  findings unresolved and non-converging on their own; only after a
  strategy change (bounded local adversarial review) did the loop
  terminate productively.

## Outcomes

* PR #385 merged as a merge commit, merge SHA
  `7632f8c03b23a5b2da064ca027ed584865cfc74b`, under explicit operator
  approval ("PR 385: merge approved"), all last-mile gates re-verified
  at the exact approved HEAD `fd97e8004ab01595aa0f55cfcbf59313e4f1cdf9`
  immediately before merge.
* Full local build/test evidence green at merge: `cargo fmt`, both clippy
  invocations, `cargo test --all-targets --no-fail-fast` (269 binaries,
  only pre-existing unrelated isolation-confirmed flakes), `cargo audit`.
* Post-merge: `cargo build --release` green (no regression, unlike the
  `134-S` precedent's release-build break), 94/94 targeted
  contract/unit/integration/lib tests green, CLI/MCP smoke checks green.
  Verdict: `PASS WITH FOLLOW-UP` (runtime verification) /
  `READY_WITH_CONDITIONS` (releasability) — condition is the forward-
  looking `9108DB24`/`A67EEA54` design decision, not a current defect.
* Shipment `136-S` manually safe-closed (`archived_status: done`)
  following the `133-S`/`134-S`/`135-S` precedent; `142-F` verified
  unchanged (`status: active`), remaining open for later `142-F`-covering
  shipments (`137-S`-`142-S`).

## Deferred follow-ups (P-021, carried forward for Stage triage)

`9CB60992` (medium), `96A1197D` (low), `341497BC` (medium), `F2A07647`
(low), `9108DB24` (high, `requires_deliberation: true`), `6B624CF6` (low),
`F0760EF2` (low), `959D1448` (medium, `requires_deliberation: true`),
`8D97190A` (medium, superseded by `D1D94CF4` — do not action directly),
`D1D94CF4` (low), `259D0E38` (medium), `A67EEA54` (high,
`requires_deliberation: true`).

## Full evidence trail

* PR #385: `https://github.com/softwaresalt/agent-engram/pull/385`
* `docs/closure/2026-09-08-136-s-copilot-review-inventory-and-adversarial-review.md`
* `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`
* `docs/closure/136-S-2026-09-08-runtime-verification.md`
* `docs/closure/136-S-2026-09-08-post-merge-closure.md`
* `.backlogit/reconcile/136-S-pre-20260908T141854Z.md`,
  `.backlogit/reconcile/136-S-post-20260908T142130Z.md`
* Verbose session originals archived at
  `docs/archive/memory/2026-09/2026-09-07-ship-136-s-generation-domain-store-session.md`,
  `docs/archive/memory/2026-09/2026-09-08-ship-136-s-adversarial-review-remediation-session.md`,
  `docs/archive/memory/2026-09/2026-09-08-ship-136-s-utf8-fix-and-review-cascade-session.md`
