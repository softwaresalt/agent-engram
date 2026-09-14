# Package G v3 — Stage session memory

**Date**: 2026-09-13
**Branch**: `chore/checkpoint-resolution-ordering-restage`
**Session**: `stage-package-g-v3-2026-09-13`
**Outcome**: plan-review **FAIL** (terminal, round 1). No harvest.

## Scope

Fresh Stage operation for **Package G v3 only**. New plan authority after v2
closed all twenty original findings and failed only on two correction-induced
verification gaps (B1, B2). Affirmed architecture, Packages A–F, PR #396, and
`143.*` were out of scope and were not revisited — confirmed by all seven
reviewers, none of whom re-litigated an affirmed item.

## Preflight (fresh)

* HEAD at session start: `fde75189` (re-fetched; local == `origin/`).
* Checkpoints: `backlogit checkpoint list --status active` → **0 active**,
  `needs_quarantine: 0`, `quarantined: 0` → zero-candidate normal startup, no
  recovery needed.
* `INDEX_SYNC_OK` (1,366 artifacts indexed). `TOOL_OK: backlogit 1.10.1`.
* Stage checkpoint written: `checkpoint-20260914-045836.json` (phase
  `triage-complete`).

## Evidence gathered this session

| Item | Value |
|---|---|
| Harness corpus | 92 files across 5 roots, **all** `.md` |
| Max relative depth | **2** (v2 cap was 8 — inert) |
| Max file size | **98,546** bytes, `.github/agents/_ship.agent.md` (v2 cap was 262,144 — inert) |
| `[[test]]` targets | 267; `autotests` absent; 0 root `tests/*.rs`; 63 `contract_*` |
| Coverage oracle baseline | `TARGET_COUNT=267`, `UNMAPPED_TARGETS_COUNT=0`, `STATUS=PASS` |
| Manifest `src/**` surfaces | declare `targets = ["contract_*", ...]` → a new `contract_` target is absorbed |
| `.github/**/*.md` in `paths-ignore` | present in **both** push and pull_request blocks |
| `allow(dead_code)` in `tests/` | **11** (v2 deliberation claimed "exactly once" — wrong) |
| `#[path = ...]` in `tests/` | **46** (v2 deliberation claimed "one precedent" — wrong) |
| `ignore 0.4` live call sites in `src/` | **zero** |

The depth/size measurements are the decisive evidence for B1: both v2 caps were
inert on every real input, yet capable of silently deleting the largest and most
contract-dense file in the corpus once it grew.

## Artifacts produced

| Artifact | Path |
|---|---|
| Deliberation v3 | `docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation-v3.md` |
| Plan v3 | `docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan-v3.md` |
| Plan hardening | `## Plan Hardening` H1–H7, in the plan |
| Plan-review record v3 | `docs/closure/2026-09-13-package-g-v3-plan-review-record.md` |
| v2 superseded markers | frontmatter of the v2 plan and v2 deliberation (review record **untouched**) |

## Decisions taken in v3

* **B1** — remove `max_depth`/`max_filesize` entirely; exhaustive non-filtering
  `std::fs` recursion; `ignore::WalkBuilder` not used (it filters five ways by
  default). Glob narrowing applied after full enumeration; F4 evaluated last.
* **B2** — honest layer ownership with a published reachability matrix; F5/F6
  owned by resolver/read unit tests against an injected two-method `FileAccess`
  boundary; `run_registry` left un-instrumented.
* **B3** — scenario-counting convention **SC** defined explicitly.
* **B4** — red-phase wording narrowed to API-introducing vs data-asserting units.
* Corrected v2 advisory A-1 with real counts (11 / 46).

## Review outcome

Seven personas, seven distinct models, one round. **5 FAIL / 2 PASS.**

* Rust `gpt-5.6-sol` FAIL · Correctness `gpt-5.6-terra` FAIL · Constitution
  `grok-4.6` FAIL · Architecture `claude-opus-4.8` FAIL · Maintainability
  `claude-opus-4.7` FAIL · Security `claude-sonnet-5` **PASS** · Scope
  `gemini-3.8-flash` **PASS**.

**Terminal (architecture-level P1, each raised by two reviewers, adjudicated
architecture-level by the Architecture Strategist):**

* **A — root-symlink fail-open.** `canonicalize()` follows a symlinked root;
  symlink rejection only inspects entries found *during* recursion, never the
  root. An allow-listed root replaced by a symlink is traversed, so contracts
  pass against a substitute tree. Contradicts the plan's own symlink policy and
  H6. No unit owns it.
* **B — canonicalization not injectable.** G.8 claims "pure path logic" but G.9
  calls `std::fs::canonicalize`; the canonical-escape F5 case needs a real
  symlink, which is non-deterministic on Windows and is the platform trick the
  plan forbids. `FakeFs` cannot inject it — a claimed-but-unprovable proof, the
  v2-B2 defect class at a new layer.

Common root cause: the two-method `FileAccess` seam is **under-provisioned** for
what the `resolve` layer asks of it.

Per the operator gate (*correction budget is for mechanical findings only; any
architecture-level P0/P1 ⇒ stop*), **no correction round was opened**.

## What is affirmed and must not be re-derived

* B1's **filter class** is fully resolved — probe "no speculative limits" passed
  **7/7**, and the Security Reviewer (whose v2 P2 caused B1) explicitly declined
  to re-request bounds. Finding A is *adjacent to* B1, not a regression of it.
* G.10's positive no-omission proof, the layering *direction* of B2, the
  un-instrumented public runner, module cohesion, and all prior affirmed v2
  constraints.
* Scope discipline: zero P0/P1/P2 from the Scope Boundary Auditor.

## Next session must settle first

1. Provision the `FileAccess` seam for A **and** B together (path-level lstat;
   canonicalization behind the boundary if deterministic containment proof is
   kept), and reject a symlink root **before** canonicalization/traversal.
2. Do **not** reopen B1.
3. Disambiguate **SC** — may one `form`×`scope` scenario assert two codes? The
   `exactly_one` case (F1 and F8) forces the question. Unit count will rise above 35.
4. Rebuild the Constitution Check against the real `constitution.instructions.md`
   I–XI (8 of 11 rows currently map invented names); pairs are **16**, not 17.
5. Give the G.1 placeholder an owner; move `classify` out of G.6 into G.20.
6. Relabel the reachability matrix into two columns (runtime-reachable vs
   deterministically inducible).

## Explicit non-actions (boundary confirmation)

No implementation code. No `src/` change. No dependency change. No edit to
`Cargo.toml`, `.github/workflows/ci.yml`, `.cargo/test-coverage-manifest.toml`,
or anything under `tests/`. **No backlog IDs allocated. No shipment assembled.
No PR created.** PR #396 untouched; `143.*` left abandoned; Packages A–F not
modified. No stash entry archived. The v2 plan-review record was read as
evidence and not modified. Stage remained within its planning/decomposition
role boundary throughout (P-010).
