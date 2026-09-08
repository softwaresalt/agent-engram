---
title: "Proactive pre-PR checklist derived from 11 rounds of Copilot review on PR #385 (136-S)"
description: "Recurring hosted-review issue families for generation-store/database-publication work, and the concrete review/test heuristics that would catch them before a PR is opened."
problem_type: "review-non-convergence"
category: "best-practices"
component: "services::generations (store/publish/context), db::cozo_backend runtime-copy open"
root_cause: "Each hosted-review round fixed only the findings named in that round instead of searching for sibling instances of the same defect family, so structurally identical defects (containment gaps, unbounded locks, misattributed errors, stale readiness evidence) resurfaced repeatedly across 11 review rounds without converging."
resolution_type: "workaround"
severity: "medium"
message: "P-018 UNRESOLVED_THREADS non-convergence after 4 hosted review-fix cycles"
file_path: "src/services/generations/{store,publish,mod}.rs, src/db/cozo_backend/mod.rs"
citations:
  - "PR #385 (softwaresalt/agent-engram)"
  - "commits 3541356b, 1571bada, b7f260b2, c2d8d6f0"
  - "PR #385 review threads PRRT_kwDORJEduc6gFlo-, PRRT_kwDORJEduc6gFv5l, PRRT_kwDORJEduc6gFv6O, PRRT_kwDORJEduc6gF5mI"
  - ".backlogit/stash.jsonl entries 2D86F780, 1C8F1150, B4D1D935, 3D2B167C, 9CB60992, 96A1197D, 341497BC, F2A07647, 9108DB24, 6B624CF6"
  - "local adversarial review, PR #385, 2026-09-07/08 (3-model panel: gpt-5.6-sol, claude-opus-4.6, gemini-3.6-flash)"
tags:
  - "copilot-review"
  - "review-non-convergence"
  - "path-containment"
  - "bounded-locks"
  - "error-attribution"
  - "readiness-evidence-staleness"
  - "generation-store"
---

## Problem

PR #385 (shipment `136-S`, generation domain/store/atomic-publication/database-open)
went through **11 rounds** of GitHub Copilot hosted review across 25 review
threads before a local adversarial-review pass was used to break the cycle. A
4th operator-authorized review-fix cycle (commit `3541356b`) closed the
then-current finding set, but three subsequent pushes (`1571bada`, `b7f260b2`,
and a final HEAD `c2d8d6f0`) each triggered a fresh Copilot pass that
surfaced further Mandatory findings — some genuinely new defects, others
"suppressed" (Copilot's own dedup for unchanged code) sibling variants of
already-fixed defect families. The reactive fix-one-round-then-push loop was
not converging: every push re-armed review against the *entire* diff, so a
narrowly-scoped fix always left an opening for the next round to find a
structurally similar issue the previous round's fix did not generalize to.

## Root Cause

Two compounding causes:

1. **Fixes were scoped to the exact reported line, not the defect family.**
   E.g. round 1 restricted `ExistingDbLocation` construction to a
   store-sealed path; round 2 found the *same* unsealed-construction defect
   again because the fix didn't yet canonicalize against a trusted root;
   round 3 found it a *third* time because the root itself wasn't required
   to be a directory. Three rounds to close one family because each fix
   addressed only the literal reported symptom.
2. **Readiness/checkpoint bookkeeping updates counted as new pushes**,
   re-arming hosted review against the full diff even when the *only* change
   was a PR-body or memory-checkpoint edit — turning bookkeeping hygiene
   into its own source of additional review rounds (4 of the 11 rounds were
   triggered by doc/PR-body-only pushes, not code changes).

## Recurring issue families (evidenced across all 25 threads + 2 "suppressed" passes)

1. **Path containment vs. filesystem/platform semantics.** Sealed-target
   minting had to be progressively hardened: restrict construction →
   canonicalize against a trusted root → require the root be a directory →
   reject `Component::Prefix` (Windows drive letters like `C:`) in addition
   to `..`/`/`/`\\`. Each hardening step closed one bypass but not the
   platform-dependent sibling case.
2. **Reserved/exclusive namespace ownership under case-folding.** A
   candidate directory could alias store-owned infrastructure names
   (`active.json`, `.publisher.lock`) — first via an unreserved name
   entirely, then (after reservation was added) via ASCII case variants on
   case-insensitive filesystems (`ACTIVE.JSON`), because the initial fix
   compared names byte-exact instead of case-insensitively.
3. **Lock acquisition boundedness asymmetry.** A new lock primitive
   (`PublisherLock`) was implemented fully unbounded while an existing,
   already-reviewed bounded pattern (`try_write()` polling under a 30 s
   deadline, `src/db/cozo_backend/mod.rs`) sat in the very same PR. The new
   primitive was never checked against the established pattern before
   review.
4. **Precise error attribution.** A shared helper (`hash_bounded_reader`)
   serving both a read-only digest path and a copy-with-write path
   hard-coded the *read* path into every error, including the one branch
   (`write_all`) whose failure is actually on the *destination* being
   written.
5. **Runtime database copy integrity.** `fs::copy` (unbounded, no
   revalidation) was replaced with a bounded, digest-verified copy — but the
   deeper issue (a stable, reused runtime-copy path being replaced/cleaned
   while an older, still-alive handle may depend on it, and a direct
   contradiction with the task's own archived "nothing is deleted"
   acceptance criteria) was never resolved, only pushed later by a "no
   production caller yet" scope carve-out.
6. **Readiness/checkpoint metadata staleness.** The PR body's "Local Review
   Readiness" block and a memory checkpoint repeatedly named a stale
   `Reviewed HEAD` after a subsequent push — because every push (even a
   docs-only one) advances `headRefOid` and re-arms Copilot, but the
   readiness bookkeeping update lagged by one round each time.
7. **Deferred-follow-up bookkeeping omissions.** New stash IDs captured by
   one round were not immediately reflected in the PR's own
   "Deferred follow-ups" table, so the operator-visible record briefly
   under-reported outstanding follow-ups.
8. **Happy-path test bias.** Tests proved the bounded-copy and reserved-name
   *positive* cases but did not initially cover: Windows drive-prefix IDs,
   case-variant reserved names, a concurrent/live-old-handle reopen of the
   same generation, or a synthetic destination-write failure — exactly the
   gaps that let several of the above escape review for multiple rounds.

## Resolution (this pass)

Ran a bounded local 3-model adversarial review (`gpt-5.6-sol`,
`claude-opus-4.6`, `gemini-3.6-flash`) against the full `origin/main...HEAD`
diff, seeded with the taxonomy above so reviewers searched for sibling
instances instead of only the four then-open threads. Confirmed and fixed,
each with a red-then-green regression test:

- Bounded `PublisherLock::acquire` with the same `try_write()`-polling +
  30 s-deadline + 50 ms-poll pattern already proven in
  `src/db/cozo_backend/mod.rs` (`src/services/generations/publish.rs`).
- Case-insensitive (ASCII) comparison for `RESERVED_ROOT_NAMES`
  (`src/services/generations/store.rs`).
- Threaded the actual destination path through `hash_bounded_reader` so a
  write failure is attributed to the staging file, not the source being
  read (`src/db/cozo_backend/mod.rs`).
- Rejected `:` in `GenerationId` so a Windows drive-prefix (`C:`) can never
  be mistaken for a single safe path component
  (`src/services/generations/mod.rs`) — confirmed not currently
  exploitable (every live path-join site is independently gated by a
  stricter check) but fixed as defense-in-depth on the type's own
  invariant.
- Reworded the carried-forward 135-S closure doc's overstated "no file
  modified" claim to the accurate, narrower disposition-preservation claim.
- Clarified (doc-only) that `PublishError::AtomicReplace` can represent a
  committed-but-durability-uncertain outcome when `rename` succeeds but the
  follow-up parent-directory `fsync` fails, and what a caller should do
  (re-check current revision before retrying) — deliberately not a new
  error variant, to avoid overengineering a narrow POSIX-only window.
- Captured the deeper runtime-copy-vs-live-handle design gap (family 5) as
  a P-021 deferred-scope stash entry (`9108DB24`) rather than attempting an
  under-scoped fix, since it needs a real design decision
  (unique-directory-per-open vs. a retained lease) not reachable via a
  targeted patch, and has no production caller yet.
- Captured a stray `\r` JSON-escape corruption discovered in an existing
  stash entry (`2D86F780`) as a supplemental correction entry (`6B624CF6`)
  rather than editing the original in place (single-write invariant).

## Prevention — proactive pre-PR checklist for generation-store/publication work

Before opening or pushing to a PR touching store/publish/runtime-copy code:

1. **Grep for the established pattern before inventing a new one.** If the
   PR adds a new lock, cache, or bounded-retry primitive, first check
   whether an analogous primitive already exists elsewhere in the same PR
   or crate (e.g. `try_write()`-polling in `cozo_backend/mod.rs`) and mirror
   it explicitly, rather than reviewing the new primitive in isolation.
2. **When a containment/validation check is added, ask "what OS/platform
   variant of this same string would bypass it?"** — case folding
   (Windows/macOS case-insensitive filesystems), platform-dependent
   `Path::components()` parsing (backslash is a separator only on Windows;
   drive prefixes are `Component::Prefix` only on Windows), and
   symlink/junction aliasing are the three variants that recurred here.
3. **When a shared helper serves both a read-only and a read+write path**
   (e.g. `hash_bounded_reader`/`digest_bounded` vs.
   `copy_bounded_with_digest`), verify every error branch names the
   operation's actual failing path, not a single path parameter reused
   across both call shapes.
4. **Before sealing/replacing a file at a path that can be reused across
   calls**, ask whether an earlier caller could still hold that path open,
   and cross-check any "nothing is deleted"/retention claims in the task's
   own acceptance criteria against what the implementation actually does.
5. **Treat every push as a full re-review, including docs-only pushes.**
   Batch PR-body/readiness-block/checkpoint corrections into the *same*
   commit as the code fix they describe wherever possible, instead of a
   separate trailing push, to avoid spending a review round on bookkeeping
   alone.
6. **Add negative/platform/concurrency tests alongside the positive case**
   for every new validation rule: a case-variant, a platform-specific
   syntax variant (drive letters, UNC paths), and — for anything holding a
   handle across an "open" API — a test that reopens or republishes while
   the first handle is still alive.
7. **Ship should continuously assess Copilot issue patterns for compound
   promotion.** After any run of 3+ related hosted-review fix cycles on the
   same PR, or when a local adversarial review confirms 3+ findings in one
   of the families above, capture/update this document rather than letting
   the pattern re-derive itself from scratch on the next generation-store
   PR.

## Related learnings

- `docs/compound/best-practices/scope-multi-symptom-evidence-corrections-2026-08-10.md`
  — the same "narrow the correction to the exact characterized claim"
  discipline applies to family 6/7 above (readiness/follow-up bookkeeping).
- Stash `9CB60992` / `341497BC` (`open_existing_generation_via_runtime_copy`
  async-wrapping and runtime-root containment) and `9108DB24` (this pass) are
  three related, still-deferred facets of the same F09 runtime-copy
  component; a future F17/F18 integration task should read all three
  together rather than one at a time.
