---
type: session-memory
date: 2026-09-13
agent: stage
session: g0-attempt-3-final
branch: chore/checkpoint-resolution-ordering-restage
outcome: FAIL — G0 circuit OPEN
---

# Stage session memory — package G0 attempt 3 (final allowed attempt)

## Outcome

**FAIL at plan-review round 1. The G0 circuit is now OPEN.** No harvest, no
backlog IDs, no shipment, no PR. No fourth G0 plan will be authored.

## Scope honoured

Touched only G0 attempt-3 artifacts. Did **not** touch G1, G2, packages A–F,
PR #396, or `143.*`. No source, test, or configuration file modified. No stash
entry archived. No dependency change.

## Artifacts produced

| Path | Purpose |
|---|---|
| `docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-3.md` | Records the operator-fixed API boundary B1–B4; settles Q1–Q10 and decisions D1–D14 |
| `docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan-attempt-3.md` | 18-pair / 36-unit plan with integrated hardening H1–H11; now `plan_status: blocked` |
| `docs/closure/2026-09-13-package-g0-attempt-3-plan-review-record.md` | Seven-persona / seven-model review record, verdict FAIL, circuit OPEN |

Attempt-1 and attempt-2 plans, deliberations and review records were read as
evidence and preserved unmodified.

## Preflight verified fresh against `e0accf82`

Single worktree, clean tree apart from an untracked checkpoint and a modified
stash file. Workspace members, `-Dwarnings` in `.cargo/config.toml`, the
`dev-test` / `lint` / `ci` aliases, `thiserror = "1"`, `tempfile = "3"`, the
coverage-oracle modes and the powerbi `[[surface]]` precedent were each
re-verified read-only.

## Review panel

| Persona | Model | Verdict |
|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | FAIL |
| Correctness Reviewer | `gpt-5.6-terra` | FAIL |
| Architecture Strategist (adjudicator) | `claude-opus-4.8` | FAIL (P1 adjudicated MECHANICAL) |
| Constitution Reviewer | `grok-4.6` | FAIL (P1 MECHANICAL) |
| Security Reviewer | `claude-sonnet-5` | PASS |
| Scope Boundary Auditor | `gemini-3.8-flash` | PASS |
| Maintainability Reviewer | `claude-opus-4.7` | PASS |

4 FAIL / 3 PASS.

## Why the circuit opened

The terminal finding is architecture-level with two-model consensus:
`read_corpus` re-canonicalizes and re-checks **workspace** containment but never
compares the result against the **authorized identity**. An ancestor directory
replaced by a symlink that redirects to another location *inside* the workspace
passes every check and yields a different file — potentially outside the
configured allow-listed roots. Plan H6 claims that class is closed. It is closed
only for redirects that leave the workspace.

The correction budget is restricted to mechanical findings, so it never opened.

## The pattern across three attempts

Every G0 failure — and package-G v3 before them — has the same root cause: **the
plan asserted a guarantee its mechanism did not deliver.**

* v3 / attempt 1 — the Failure Semantics table claimed completeness it lacked.
* attempt 2 — the containment claim was forgeable at the type level.
* attempt 3 — H6's ancestor-substitution closure claim is only partial.

Notably, attempt 3 *closed* attempts 1 and 2's blockers convincingly (findings
A, B, C, D, E all confirmed closed by multiple reviewers) and still died on a
newly-introduced over-claim inside the very hardening section written to
disclose residuals honestly.

## What is settled and must be inherited if G0 is ever restaged

Eleven items, recorded in full in the review record's "What this review SETTLES"
section. Headline: the opaque containment design, corpus-carries-its-workspace,
the `compile_fail` doctest, operation-keyed `(FsOp, PathBuf)` fault injection
with all nine call sites proven, the explicit per-entry lstat with
`read_dir -> Vec<OsString>`, the compilation-complete `FsError` taxonomy, the
channel-bundling rule, and `todo!()`-arm staged completion are all **affirmed**.

## Carried-forward findings

One architecture-level P1 (finding A), one four-model-consensus P1 adjudicated
mechanical (finding B — `ResolvedCorpus::workspace` staged at G0.36 but first
read at G0.32), two single-reviewer mechanical P1s (M-1 missing `FakeFs`
constructor; M-2 `clippy::missing_panics_doc` on staged `todo!()`), ten P2s and
six advisories. All enumerated in the review record.

## Next steps

Reopening the G0 circuit requires an explicit operator decision. Until then G1
and G2 stay unreviewed and unharvested, their prerequisite authority absent,
their draft plans preserved.
