# Adversarial Review — 074-S NotReady `--direct` scope fix

- **Date:** 2026-07-05
- **Change under review:** commit `2daf605` (LOCAL, not pushed) on branch `074-notready-scope-fix`
- **Title:** `fix(errors): scope NotReady --direct hint to daemon-startup path`
- **Scope:** 3 files, +66 / −1 — `src/errors/mod.rs`, `src/errors/codes.rs`, `src/shim/lifecycle.rs`
- **Plan:** `docs/exec-plans/2026-07-05-072-001-notready-scope-fix-plan.md`
- **Provenance:** 072.001-T / 074-S; PR #207 Copilot review
- **Mode:** read-only (no code modified by this review)

## Reviewer panel (tier-diverse, cross-provider)

| Reviewer | Tier | Model | Independent verdict |
|---|---|---|---|
| Reviewer-A | Tier 1 (fast) | `gemini-3.5-flash` | APPROVE (0 findings) |
| Reviewer-B | Tier 2 (standard) | `gpt-5.4` | BLOCK (2 MAJOR) |
| Reviewer-C | Tier 3 (frontier) | `claude-opus-4.8` | APPROVE-WITH-FIXES (2 MINOR) |

Two independent ground-truth agents were also run by the assembler to verify the
reviewers' crux claims against live source (repo-wide grep + daemon lock/Ready ordering).

---

## Consensus verdict: **APPROVE** (merge-ready) — with mandatory follow-up backlog items

The commit **correctly and completely accomplishes its declared, plan-reviewed scope**:
it splits the misleading `--direct` hint off the respawn shutdown-wait path into a new
`DaemonError::ShutdownTimeout` variant (wire code `8010`), leaving the startup-path
`NotReady` (`8006`) contract frozen. The change is additive, compiler-exhaustive,
regression-free, and satisfies every item in the plan's Definition of Done.

**No defect was flagged by all three reviewers** (no HIGH-confidence blocker). The single
cross-model signal (flagged by 2 of 3) is a **real but pre-existing, out-of-scope** issue
that this commit neither introduced nor was scoped to fix — it belongs in a follow-up, not
as a merge blocker.

**Why Reviewer-B's BLOCK is overridden:** Its two MAJOR findings target the *startup*
`poll_until_ready` path and the `wait_for_daemon_exit` race semantics. Both behaviors
**predate this commit** (the `--direct` hint on `NotReady` was introduced by shipment 073-S;
the race loop is unchanged). This commit does not touch `poll_until_ready` — and the plan
(`§Startup call site … Keep as-is.`) deliberately, with prior plan-review sign-off, left it
alone. A correctly-scoped, additive, regression-free fix should not be blocked for adjacent
pre-existing issues. The frontier reviewer (Reviewer-C) reached the same scoping conclusion
("pre-existing … out of the stated scope, so it does not block"). These issues are captured
below as high-value follow-ups.

---

## Verified-good checklist (assembler-confirmed against live source)

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | Every shutdown-wait/respawn timeout now returns `ShutdownTimeout`; no site missed | ✅ | Only one such site exists: `src/shim/lifecycle.rs:393` (`wait_for_daemon_exit` deadline branch). Repo-wide grep of `ShutdownTimeout` shows it as the sole prod construction. |
| 1b | Startup path still returns `NotReady` (hint correct there) | ✅ | `src/shim/lifecycle.rs:457` (`poll_until_ready`) unchanged — sole remaining `NotReady` producer. |
| 2 | `to_response` `Daemon(inner)` match exhaustive, no wildcard (compiler-forced) | ✅ | `src/errors/mod.rs:492-511` — arms `SpawnFailed` / `NotReady` / `ShutdownTimeout`, no `_ =>`. |
| 2b | No caller `match`/`if let`/`matches!` on `DaemonError::NotReady` outside `to_response` | ✅ | Repo-wide grep: only *constructions* elsewhere (`SpawnFailed` in `lifecycle.rs`, `tools/doctor.rs:221/231`). `?`-propagation callers (`respawn_daemon`, `ensure_daemon_running_inner`) need no special handling. |
| 3 | `8010` unused elsewhere; fits monotonic 8xxx range | ✅ | `src/errors/codes.rs:67` sole definition; used only at `mod.rs:506` (arm) + test. `8009 = WATCHER_INIT_FAILED` → `8010` is next free. |
| 3b | `NotReady` wire contract frozen (`8006` / `DaemonNotReady` / `{timeout_ms}`) | ✅ | `mod.rs:499-504`; test `not_ready_wire_contract_unchanged` pins literal `8006`. |
| 4 | thiserror brace-safety — only `{timeout_ms}`, renders brace-free | ✅ | `mod.rs:170-173`; test `shutdown_timeout_message_omits_direct` asserts `!contains('{') && !contains('}')`. Em-dash renders fine. |
| 5 | Message guidance accurate — stuck daemon holds the lock in this path | ✅ | Ground-truth: daemon acquires `DaemonLock` at `src/daemon/mod.rs:105` **before** Ready; in a shutdown-wait timeout the old daemon is still running (endpoint reachable OR pid alive) and holds the lock. "Stop the running engram daemon process, then retry" is correct/actionable. |
| 6 | Tests prove both message + wire contracts for both variants; non-tautological | ✅ (with LOW-conf gap, see F2) | `shutdown_timeout_message_omits_direct`, `shutdown_timeout_wire_contract`, `not_ready_*` all present with literal `8006`/`8010` pins. |
| 7 | Additive only; 3 files; `forbid(unsafe_code)` intact; no `unwrap`/`expect`/`panic`/`unsafe` added; no schema/protocol change | ✅ | `#![forbid(unsafe_code)]` at `src/lib.rs:10`; grep of the 3 files: no `unsafe`. Diff adds no `unwrap`/`expect`/`panic`. Purely additive taxonomy split. |

---

## Findings by confidence tier

### Consensus (HIGH — flagged by all 3 reviewers)
**None.** No defect was independently identified by every reviewer. (This is itself a
positive merge-readiness signal; the verified-good checklist above is the consensus-positive.)

### Majority (MEDIUM — flagged by 2 of 3 reviewers)

#### F1 — Startup/respawn `NotReady` `--direct` hint remains misleading when a freshly-spawned daemon acquires the lock but hangs before Ready
- **Confidence:** MEDIUM (Reviewer-B `respawn-ready-timeout-still-suggests-direct` @ `lifecycle.rs:318`; Reviewer-C `semantic-boundary-imperfect` @ `lifecycle.rs:457` — same root cause)
- **Severity (most-conservative of the two):** MAJOR
- **Status for THIS PR:** **out of scope / pre-existing → does NOT block.** Introduced by 073-S; `poll_until_ready` is untouched by `2daf605` and was deliberately kept as-is per plan.
- **Verified REAL by assembler:** `src/daemon/mod.rs:105` acquires `DaemonLock` *before* the daemon reports Ready (health returns `"starting"` until hydration completes — `ipc_server.rs:300-308`). Plausible trigger: slow/hung workspace hydration on a large repo. Sequence: daemon acquires lock → hydration stalls → `poll_until_ready` times out (30 s) → returns `NotReady` with the `--direct` hint → `engram index --direct` then fails at `DaemonLock::acquire` → `LockError::AlreadyHeld` (`src/cli/direct.rs:73-79`). This is the *same misleading-hint class* the 074-S line of work aims to eliminate — but on the startup path this commit intentionally did not touch.
- **Recommended follow-up (not a blocker):** In `poll_until_ready`'s `NotReady` branch, gate the `--direct` hint on actual lock/PID state, or introduce a distinct "daemon started but never became Ready (still holds lock)" variant analogous to `ShutdownTimeout`.

### Unique (LOW — flagged by exactly 1 reviewer)

#### F2 — Behavior-changing line has no call-site test (regression would go undetected)
- **Confidence:** LOW (Reviewer-C `call-site-coverage-gap` @ `lifecycle.rs:393`)
- **Severity:** MINOR
- **Status:** **plan-sanctioned deferral → advisory only.** Both new tests construct
  `DaemonError::ShutdownTimeout` in isolation; none drive `wait_for_daemon_exit`. Reverting
  `lifecycle.rs:393` to `NotReady` would leave all tests green. However, the plan
  (`§4`, lines 156-161) explicitly marks the lifecycle-level test **"Optional (nice-to-have,
  may defer) … Ship may skip the lifecycle test if the harness is disproportionate."** The
  message + wire unit tests are the plan's primary gate for this LOW-blast change. **Not a
  plan deviation.**
- **Recommended follow-up:** Add a lifecycle test that drives `wait_for_daemon_exit` to its
  deadline (unreachable endpoint + live `pid_hint`, short `SHUTDOWN_WAIT_TIMEOUT_MS`) and
  asserts the error maps to `DAEMON_SHUTDOWN_TIMEOUT`/`8010`, pinning the call site.

#### F3 — `wait_for_daemon_exit` may misclassify a concurrent-replacement race
- **Confidence:** LOW (Reviewer-B `shutdown-timeout-misclassifies-concurrent-replacement` @ `lifecycle.rs:393`)
- **Severity:** MAJOR (reviewer's rating); assessed **LOW practical impact**
- **Status:** **pre-existing edge case → does NOT block.** If a concurrent shim kills the old
  PID and binds a replacement before this loop observes an unreachable endpoint, the deadline
  branch could fire while a *replacement* daemon (not the old one) holds the lock. This race
  is unchanged by the commit; pre-`2daf605` it returned `NotReady` (arguably more harmful).
  The new message ("a daemon is running and holds the lock — stop it, then retry") remains
  roughly accurate and strictly less harmful than the old `--direct` steer even in the race.
- **Recommended follow-up (low priority):** If the old PID is gone but the endpoint responds,
  probe/reuse the replacement daemon (mirroring `poll_until_ready`'s "concurrent shim won the
  spawn race" recovery) instead of timing out.

---

## Remediation plan (sorted by priority = confidence × severity)

| # | Finding | Confidence | Severity | Priority | Action class | Blocks merge? |
|---|---|---|---|---|---|---|
| F1 | Startup/respawn `NotReady` `--direct` hint misleading when spawned daemon holds lock but hangs | MEDIUM (2) | MAJOR (3) | **6** | `manual` (design decision) — follow-up | No (out of scope / pre-existing) |
| F3 | `wait_for_daemon_exit` concurrent-replacement misclassification | LOW (1) | MAJOR (3) | **3** | `advisory` — follow-up | No (pre-existing) |
| F2 | No call-site test for the retargeted `ShutdownTimeout` branch | LOW (1) | MINOR (2) | **2** | `advisory` — follow-up (plan-sanctioned deferral) | No |

**Net:** zero required changes to `2daf605` before merge. All three items are follow-ups.

---

## Backlog work items (P0/P1 findings)

```yaml
type: bug
title: "poll_until_ready NotReady --direct hint misleading when spawned daemon holds lock but hangs"
description: >
  On the daemon STARTUP path, poll_until_ready returns DaemonError::NotReady whose
  message points operators at `engram index --direct` / ENGRAM_DIRECT=1. But the
  daemon acquires the workspace DaemonLock (src/daemon/mod.rs:105) BEFORE it reports
  Ready. If a freshly-spawned (or respawned) daemon acquires the lock and then hangs
  during hydration (e.g. slow/large-workspace hydration), poll_until_ready times out
  and emits NotReady with the --direct hint; `engram index --direct` then fails at
  DaemonLock::acquire with LockError::AlreadyHeld (src/cli/direct.rs:73-79). This is
  the same misleading-hint class that 074-S fixed for the shutdown-wait path, but on
  the startup path (deliberately left untouched by commit 2daf605 per plan).
file: "src/shim/lifecycle.rs"
line: 457
severity: "MAJOR"
confidence: "MEDIUM"
priority: "P1"
fix: >
  Gate the --direct hint in poll_until_ready's NotReady branch on actual lock/PID
  state, or add a distinct "daemon started but never became Ready (still holds lock)"
  variant (analogous to ShutdownTimeout) whose message omits --direct.
linked_review: "docs/closure/2026-07-05-notready-scope-fix-adversarial-review.md"
related: ["073-S", "074-S", "PR #207"]
```

### Deferred (P2/P3 — noted, not filed unless operator requests)
- **F3 (P3):** Recover from concurrent-replacement race in `wait_for_daemon_exit`
  (`src/shim/lifecycle.rs:393`) — low practical impact, pre-existing.
- **F2 (P3):** Add lifecycle call-site test asserting `wait_for_daemon_exit` emits
  `ShutdownTimeout`/`8010` (`src/shim/lifecycle.rs:393`) — plan-sanctioned optional.

---

## Bottom line

`2daf605` is **APPROVE / merge-ready** as a correctly-scoped, additive, regression-free,
plan-compliant fix. Ship the PR. Separately, **file backlog item F1 (P1)** so the
misleading-`--direct` class is fully closed on the startup path as well — the adversarial
panel's most valuable, cross-model signal, verified real against live source, but outside
this commit's deliberate scope.
