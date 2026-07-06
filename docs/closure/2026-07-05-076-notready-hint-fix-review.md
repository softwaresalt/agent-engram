---
title: Review — 076-S NotReady hint fix (F1)
date: 2026-07-05
shipment: 076-S
branch: 076-notready-hint-fix
reviewer: gpt-5.4 (cross-model rubber-duck)
verdict: ship (no blocking issues)
---

# Review — 076-S NotReady hint fix (F1)

Cross-model review (gpt-5.4) of the `DaemonError::NotReady` reword.

## Verdict

**No blocking issues.** The reword (option c) is the right risk/severity
tradeoff. The reviewer independently verified that option (a) — probing
`DaemonLock` acquirability at timeout — is genuinely risky: `poll_until_ready`
only has `endpoint` (3 call sites), the endpoint is not reliably reversible to a
workspace path on all platforms, and `DaemonLock::acquire` is **not** a benign
probe (its stale-PID path deletes `engram.lock` / `engram.pid`,
`src/daemon/lockfile.rs:181-205`).

## Findings and resolution

| Finding | Sev | Resolution |
|---|---|---|
| Message under-specified the "slow but still starting / would recover" case — a user might kill a recoverable daemon | non-blocking | **Fixed** — third branch now says "wait and retry if it is still starting up, or stop that engram process if it appears stuck" |
| Loose test guard (`stop` + `lock`) | suggestion | **Applied** — also assert `exited` to prove the two-branch meaning |

## Contract verification (by reviewer)

- Wire contract unchanged: `NotReady` still maps to code `8006` /
  `"DaemonNotReady"` (`src/errors/codes.rs:61`).
- No in-repo consumer parses this error's message text (only the name/code /
  `data.engram_code` discriminator is used).
- No other test asserts the old wording.

## Tests

`cargo test not_ready --lib` + `contract_error_codes` pass; clippy pedantic + fmt
clean.
