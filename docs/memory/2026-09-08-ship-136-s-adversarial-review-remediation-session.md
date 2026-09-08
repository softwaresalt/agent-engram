---
title: "Ship session — 136-S local adversarial-review remediation (PR #385)"
date: 2026-09-08
shipment_id: "136-S"
feature_id: "142-F"
pr: 385
branch: "feat/136-s-generation-domain-store-atomic-publication-and-database-open"
status: "pr-ready-awaiting-merge-approval"
---

## Session summary

Operator directed a strategy change after 4 authorized hosted-review-fix cycles plus 3
further Copilot passes left 4 Mandatory findings unresolved and non-converging. Resumed
shipment `136-S` / PR #385 at branch
`feat/136-s-generation-domain-store-atomic-publication-and-database-open`, expected HEAD
`c2d8d6f08a75526b232413f0b7168a17b4b8fe97` (verified matching before any action).

## Items completed

1. Mined the complete 11-round / 25-thread Copilot review history for PR #385 (paginated
   GraphQL, all reviews and review threads, resolved and unresolved) — full inventory
   recorded at `docs/closure/2026-09-08-136-s-copilot-review-inventory-and-adversarial-review.md`.
2. Ran a bounded local 3-model adversarial review (`gpt-5.6-sol`, `claude-opus-4.6`,
   `gemini-3.6-flash`) against the full `origin/main...HEAD` diff, seeded with the mined
   issue-family taxonomy so reviewers searched for sibling instances beyond the 4
   then-open threads.
3. Captured durable compound learning at
   `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`
   (recurring issue-family taxonomy + proactive pre-PR checklist).
4. Fixed all 4 previously-unresolved Copilot threads (TDD-first, red test before
   implementation):
   - `PRRT_kwDORJEduc6gFlo-` / stash `2D86F780` — bounded `PublisherLock::acquire`
     (`src/services/generations/publish.rs`).
   - `PRRT_kwDORJEduc6gFv5l` / stash `1C8F1150` — `hash_bounded_reader` destination-path
     attribution (`src/db/cozo_backend/mod.rs`).
   - `PRRT_kwDORJEduc6gFv6O` / stash `B4D1D935` — closure-doc wording
     (`docs/closure/135-S-2026-09-06-post-merge-closure.md`).
   - `PRRT_kwDORJEduc6gF5mI` / stash `3D2B167C` — case-insensitive reserved-name comparison
     (`src/services/generations/store.rs`).
5. Additionally fixed 2 findings the adversarial review confirmed beyond the 4 threads:
   `GenerationId` Windows drive-prefix (`"C:"`) rejection, and a doc-only clarification of
   `PublishError::AtomicReplace`'s committed-but-durability-uncertain semantics.
6. Deferred (P-021) one confirmed HIGH-confidence/MAJOR design gap not reachable in
   production today: stash `9108DB24` (runtime-copy stable path vs. a live
   `OpenedGeneration` handle; pre-integration blocker for future F17/F18 work).
7. Corrected (supplemental entry, not an edit) a stray `\r` JSON-escape corruption in
   stash entry `2D86F780`'s text: stash `6B624CF6`.
8. Full validation: `cargo fmt --all -- --check` (PASS), `cargo clippy --all-targets -- -D
   warnings -D clippy::pedantic` (PASS), the exact CI clippy invocation (PASS), `cargo test
   --all-targets --no-fail-fast` (269 test binaries; 2 pre-existing failures with isolation
   evidence, both unrelated to this diff), `cargo audit` (PASS, no new advisories).
9. Committed (`7d9d4d7ea357a950e87c1348f42b12d43cf9c81e`), associated with shipment `136-S`
   via `backlogit update --commit`, pushed, replied to and resolved all 4 threads citing
   the fixing commit, then committed the resulting backlog commit-tracking metadata
   (`412511cb129023dd1042987f95eec1a8560cda4d`).
10. Waited for CI and a fresh Copilot review pass at each pushed HEAD. Final state at HEAD
    `412511cb`: CI green (`build` 6m18s, `start-launcher-windows` 2m12s),
    `autoharness gate copilot-review 385` reports `SATISFIED: PASS` (25 threads total, 0
    unresolved), `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`.
11. Updated the PR body's Deferred follow-ups table and Local Review Readiness / CI Status
    / Merge sections to the exact final HEAD.

## Items blocked / deferred

* Stash `9108DB24` (P-021, high priority, provisional): runtime-copy stable path vs. live
  handle design gap. Requires a Stage deliberation decision (unique-dir-per-open vs.
  retained lease) before F17/F18 wire in the first production caller. Not reachable today.
* Pre-existing, unrelated test flakes (isolation-confirmed, not a regression from this
  session's work): `t046_s050_daemon_exits_after_idle_timeout_and_restarts` (full-suite-
  parallel timing flake, passes in isolation) and
  `archive_verifier_runs_the_unpacked_native_binary` (reproduces identically at the
  unmodified pre-remediation HEAD `c2d8d6f0`).

## Branch state

* Branch: `feat/136-s-generation-domain-store-atomic-publication-and-database-open`
  (remained on this branch throughout; never checked out `main`).
* HEAD: `412511cb129023dd1042987f95eec1a8560cda4d`.
* Working tree: clean.
* PR #385: `OPEN`, `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`, no merge attempted.

## Decisions with rationale

* Chose to fix `GenerationId`'s Windows drive-prefix gap via an additive `:`-rejection
  check rather than switching to a `Path::components()`-based check (mirroring
  `validate_runtime_generation_id`), because the latter is platform-dependent for
  backslash handling (a pre-existing, separately-tracked inconsistency, stash `96A1197D`)
  and would have silently narrowed the existing, portable `/`/`\\`/`..` rejections.
* Chose a doc-only fix (no new error variant) for the `AtomicReplace`
  committed-but-durability-uncertain finding, per the adversarial review's own
  recommendation, to avoid overengineering a narrow, POSIX-only, rare failure window.
* Deferred the runtime-copy-vs-live-handle finding (stash `9108DB24`) rather than
  attempting an under-scoped fix, because closing it correctly requires a genuine design
  decision (unique-dir-per-open vs. a retained lease) that is not reachable via a targeted
  patch and has no production caller yet.
* Used a synthetic always-failing `Write` implementation (not an OS-level trick) to test
  the `hash_bounded_reader` write-error-attribution fix, since permission bits do not
  re-check on already-open handles and disk-full devices like `/dev/full` are Linux-only
  (this repo's CI also runs Windows).

## Next steps

* **Awaiting explicit operator merge approval** — this is the only remaining gate. Per
  P-009, merge-commit strategy only.
* After merge: run Step 6 post-merge closure (post-merge branch, operational-closure,
  knowledge graduation, compound-refresh evaluation, source-artifact cleanup for
  `142.011-T`–`142.017-T` if not already archived, compact-context).
