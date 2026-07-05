---
title: "Adversarial multi-model review — 073-S / 065.004-T NotReady --direct hint"
type: closure
date: 2026-07-04
slug: notready-direct-hint-adversarial-review
subject_commit: c73553ca559f1ffba25e570283cb207f56543549
subject_branch: 073-notready-direct-hint
task: 065.004-T
feature: 065-F
shipment: 073-S
scope: src/errors/mod.rs
reviewers: 3
review_models:
  - reviewer-a2: claude-haiku-4.5 (Tier 1)
  - reviewer-b: gpt-5.4 (Tier 2)
  - reviewer-c: claude-opus-4.8 (Tier 3)
verdict: APPROVE
gate_blocking: false
---

# Adversarial Multi-Model Review — 073-S / 065.004-T NotReady `--direct` hint

- **Date:** 2026-07-04
- **Change:** commit `c73553c` on branch `073-notready-direct-hint` (local, NOT pushed)
- **Scope:** single file `src/errors/mod.rs`, +36/−1
- **Plan:** `docs/exec-plans/2026-07-04-065-004-notready-direct-hint-plan.md`
- **Verdict:** ✅ **APPROVE** (2 advisory MINOR test-hardening observations, non-blocking)

---

## What changed

`DaemonError::NotReady`'s `#[error(...)]` string was augmented (append-only; prefix
unchanged):

```
FROM: "Daemon failed to reach Ready state within {timeout_ms}ms"
TO:   "Daemon failed to reach Ready state within {timeout_ms}ms; if startup keeps
       timing out, run 'engram index --direct' (or set ENGRAM_DIRECT=1) to index
       without the daemon"
```
`src/errors/mod.rs:161-164`

Two unit tests added (`src/errors/mod.rs:735-766`):
- `not_ready_message_points_at_direct` — asserts the **rendered** `Display` string
  contains `5000ms`, `--direct`, `ENGRAM_DIRECT=1`, and is brace-free.
- `not_ready_wire_contract_unchanged` — asserts `code == DAEMON_NOT_READY` and
  `name == "DaemonNotReady"`.

`src/bin/engram.rs` intentionally untouched (help enrichment deliberately off).

---

## Reviewer panel

| Reviewer | Tier | Model | Verdict | Findings |
|---|---|---|---|---|
| A | 1 (fast) | gemini-3.5-flash | — | **no output returned** (re-run as A2) |
| A2 | 1 (fast) | claude-haiku-4.5 | APPROVE | 0 issues |
| B | 2 (standard) | gpt-5.4 | APPROVE | 2× MINOR |
| C | 3 (frontier) | claude-opus-4.8 | APPROVE-WITH-FIXES | 1× MINOR |

Valid reviewer instances: **3** (minimum 2 satisfied). Reviewer A (Gemini Flash)
produced no response and was re-run on a different Tier-1 model (Haiku) to restore
the third independent signal.

---

## Verification of the 5 required checks

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | thiserror interpolation safety (no stray `{`/`}`) | ✅ CLEAN | Only `{timeout_ms}` brace pair in attribute (`mod.rs:161-164`); appended text uses only `'`, `(`, `)`, `=`, `;`. Renders brace-free; `not_ready_message_points_at_direct` proves it. thiserror parses `#[error(...)]` as a format string — only literal braces would break it; none present. |
| 2 | Test quality (meaningful, non-tautological) | ✅ CLEAN (2 advisory nits) | Message test runs on **rendered** `Display` (`.to_string()`, `mod.rs:737`), asserting timeout interpolation + both hint tokens + brace-free. Wire test pins `code`+`name` (`mod.rs:759-765`). Substring (not exact-match) assertions are a sound brittleness tradeoff for human-facing text. See advisory F1/F2 below. |
| 3 | Contract/snapshot stability | ✅ CLEAN | No `*.snap`, no contract fixture, and no `tests/` assertion pins the exact NotReady string or literal `8006`. All other occurrences are historical narrative docs (`docs/closure`, `docs/compound/bugs`, `docs/decisions`, archived plans) — point-in-time records needing no update. Unchanged prefix keeps `docs/troubleshooting.md:78`'s substring example valid. |
| 4 | Scope discipline | ✅ CLEAN | `git show c73553c` = exactly one file (`src/errors/mod.rs`, +36/−1). `IpcError::Timeout` (`mod.rs:150`), `BoolishValueParser`, and `src/bin/engram.rs` all untouched. Matches plan's deliberately narrow scope; no creep. |
| 5 | Correctness of guidance | ✅ CLEAN | `--direct` / `ENGRAM_DIRECT=1` are real & correctly spelled: `#[arg(long, env="ENGRAM_DIRECT", value_parser = BoolishValueParser::new())] direct: bool` on `Index` (and `Sync`) at `src/bin/engram.rs:96-104`. `run_index(direct=true)` → `run_direct_sync` (`src/cli/commands/indexing.rs:53-82`) → `src/cli/direct.rs` acquires the lock and indexes **in-process, bypassing the daemon/IPC**. Matches `docs/troubleshooting.md`. Semantically apt for a daemon-**startup** timeout. |

---

## 1. Consensus findings (confidence: HIGH — flagged by all reviewers)

**None.** All three reviewers independently confirmed checks 1, 3, 4, and 5 as CLEAN.
No CRITICAL or MAJOR issues. No merge blockers.

## 2. Majority findings (confidence: MEDIUM — flagged by > half)

### F1 — Wire-contract test pins the symbolic constant, not the literal `8006`
- **Severity:** MINOR · **Confidence:** MEDIUM (Reviewer B + Reviewer C; 2/3)
- **File/line:** `src/errors/mod.rs:764`
- **Issue:** `not_ready_wire_contract_unchanged` asserts
  `payload.error.code == DAEMON_NOT_READY`. Because both the mapper (`mod.rs:491`) and
  the test resolve the same constant, an accidental **renumber** of
  `DAEMON_NOT_READY` (`src/errors/codes.rs:61`, currently `8006`) would change the
  external JSON-RPC code yet keep both tests green. No other test in the repo pins the
  literal `8006`.
- **Context (not a regression):** This mirrors the pre-existing repo convention — e.g.
  `maps_workspace_not_found` (`mod.rs:725-733`) likewise asserts the symbolic
  `WORKSPACE_NOT_FOUND`. The change does not worsen coverage; it inherits the house
  style. Hence **advisory**, not required.
- **Optional fix:** add `assert_eq!(payload.error.code, 8006);` (literal) to pin the
  external numeric contract independently of the constant — or add a dedicated
  `tests/contract/` numeric-code pin covering the 8xxx daemon codes.

## 3. Unique findings (confidence: LOW — flagged by exactly one)

### F2 — `details` payload shape (`{"timeout_ms": …}`) unverified
- **Severity:** MINOR · **Confidence:** LOW (Reviewer B only; 1/3)
- **File/line:** `src/errors/mod.rs:761`
- **Issue:** `to_response` emits `details = Some(json!({ "timeout_ms": timeout_ms }))`
  (`mod.rs:494`), but the wire test asserts only `code`+`name`. A regression dropping or
  renaming the `timeout_ms` detail field would not be caught.
- **Context (not a regression):** Consistent with existing convention
  (`maps_workspace_not_found` also skips `details`). Out of scope for this string-only
  change. **Advisory.**
- **Optional fix:** `assert_eq!(payload.error.details, Some(json!({"timeout_ms": 5000})));`

---

## 4. Remediation plan (sorted by priority = confidence × severity)

| # | Finding | Conf | Sev | Priority | Action class | Required for merge? |
|---|---|---|---|---|---|---|
| 1 | F1 — pin literal `8006` in wire test | MEDIUM (2) | MINOR (2) | 4 | `advisory` | No |
| 2 | F2 — assert `details` payload shape | LOW (1) | MINOR (2) | 2 | `advisory` | No |

No `safe_auto` / `gated_auto` / `manual` items. No P0/P1 findings → **no mandatory
backlog work items** generated. F1/F2 are optional test-hardening that also apply
repo-wide; if pursued, do so as a separate "contract numeric-pin" hygiene task rather
than bolting scope onto this tiny change.

---

## Consensus verdict

✅ **APPROVE — safe to open PR.**

The change is minimal, append-only, and correctly scoped to `DaemonError::NotReady`.
The thiserror format string is brace-safe (only `{timeout_ms}` interpolates), the two
new tests exercise the rendered `Display` output and the machine-readable wire contract
respectively, no snapshot/contract/docs-as-test pins the old string, and the
`engram index --direct` / `ENGRAM_DIRECT=1` guidance points at real, correctly-named,
daemonless code paths. The two MINOR observations (F1 majority, F2 unique) are advisory
test-hardening consistent with existing repo conventions and are **not** merge blockers.

---

## Appendix — raw reviewer JSON

### Reviewer A2 (claude-haiku-4.5, Tier 1) — APPROVE, 0 issues
All checks INFO/clean; verdict: "APPROVE — Change is safe, well-scoped, test-first with
meaningful assertions, wire contract pinned, guidance correct."

### Reviewer B (gpt-5.4, Tier 2) — APPROVE
- MINOR: literal `8006` not pinned (F1)
- MINOR: `details` payload unverified (F2)
- Verdict: "APPROVE — safe interpolation, accurate guidance, tight scope, clean
  contract; only optional MINOR test-hardening."

### Reviewer C (claude-opus-4.8, Tier 3) — APPROVE-WITH-FIXES
- MINOR: WIRE-CONTRACT-COVERAGE — external numeric `8006` unpinned (F1)
- Verdict: "APPROVE-WITH-FIXES — message change is safe, scoped, guidance real; new
  contract test leaves external numeric code 8006 unpinned."
