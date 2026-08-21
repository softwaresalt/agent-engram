---
title: "Completed feature memory compaction: 094-F, 096-F Python call-edge / namespace resolution"
type: compacted-memory
date: 2026-08-21
status: complete
sources:
  - docs/archive/memory/2026-07-20/094-F-python-call-edges-closure-memory.md
  - docs/archive/memory/2026-07-23/096-F-python-namespace-resolution-memory.md
  - docs/archive/memory/2026-07-27/096-f-merge-closure-memory.md
  - docs/archive/memory/2026-07-27/096-f-review-closure-memory.md
  - docs/archive/memory/2026-07-27/stage-stash-triage-ff7de872-fe8b3b2d-memory.md
---

# Completed feature memory compaction

## 094-F — Python bare-call code-graph edges (pilot, shipment 089-S)

PR #277 (merge `5f18b79`) was the pilot for per-language call-graph rollout:
before this, only Rust emitted `Calls` edges. Shipped bare Python call
extraction (`foo()`, body-scoped, builtin blocklist) and a
language-scoped cross-file singleton resolver so Python calls cannot
mis-bind to same-named Rust definitions. Fail-closed on attribute/method
calls, subscripts, and nested-function calls (v1 scope). Split two
follow-ups rather than over-scoping: bug **FF7DE872** (same-file
same-name shadowing — `find_function_id` first-match) and feature
**FE8B3B2D** (module-namespace-qualified resolution — later became 096-F).
Existing `.py` files need `engram sync --force`/`index --force` to acquire
the new edges (plain sync/index/`--full` hash-skip unchanged files).

## 096-F — Python module-namespace-qualified call resolution (shipment 091-S)

PR #288 (merge `c1f34ae3`) closed the module-namespace recall gap left by
094-F. The Stage plan-review gate ran **9 consolidation cycles** before
harvest converged (task count stable at 10 throughout) — the deliberation
repeatedly tightened a single **prove-or-fail-closed invariant**: a
same-file `def` stays on the fast direct-edge path only when it is
provably the sole binding across every modeled scope AND has no
non-import rebind form (assignment, `class`, `del`, `match`-case, `for`,
`with…as`, `except…as`, walrus, parameter target); otherwise the call
routes to staging for canonical resolution. The final resolution rule is
**last-binding-wins, call-site-effective and order-aware**: resolve to the
last provable binding preceding the call; anything ambiguous, a competing
duplicate import, or an unprovable rebind fails closed (no edge) rather
than guessing — the legacy name-only fallback is preserved only for
`NoModuleContext`/`UnsupportedImportForm`, never for competing/shadowed/
duplicate bindings. Two Copilot review cycles post-harvest found and fixed
real P0/P1 false-edge gaps (competing same-name imports; callee-module
export rebind via `module_export_rebound`; nested-function/class-body
`global` rebind via `collect_nested_dynamic_rebinds`) — cycle 3 was clean,
satisfying the circuit breaker. All 13 tasks (096.001–096.013-T) shipped
test-first; `cargo dev-test` 459 lib tests + all integration/contract
binaries green. Follow-up hardening deferred to queued feature 099-F
(099.001–099.007-T), none blocking or false-edge.

## FF7DE872 — Same-file same-name shadowing fail-closed (feature 100-F, shipment 092-S, queued)

Stage triage (2026-07-27) confirmed FE8B3B2D was already fully shipped as
096-F/091-S (stash archived, no further action) and routed the independent
FF7DE872 bug through the full pipeline: deliberation 014-D chose Option A
(fail-closed on same-file same-name ambiguity, language-agnostic, additive
helper) over Option B (last-wins, rejected as unsound for Rust's
inline-module-per-file pattern). Produced feature 100-F (U1 RED harness →
U2 guarded minting sites → U3 cross-language acceptance) as shipment 092-S,
queued for Ship to execute.

## Preserved, not compacted

At the source-checkpoint time (2026-07-27), 099-F (Python-canonical
hardening follow-ups) and 100-F/092-S (same-file shadowing fix) were
queued, not yet shipped. Both have since shipped and archived — 100-F/
092-S on 2026-07-28, 099-F via shipment 098-S — and are tracked as
completed backlog history, not active follow-ups.
