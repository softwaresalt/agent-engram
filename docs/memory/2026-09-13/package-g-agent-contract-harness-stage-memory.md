---
type: session-memory
agent: stage
date: 2026-09-13
session: stage-package-g-agent-contract-harness-20260913
branch: chore/checkpoint-resolution-ordering-restage
outcome: blocked
---

# Stage session — Package G (executable agent-contract assertion harness)

## Outcome

**BLOCKED at plan review.** No harvest, no backlog IDs, no shipment, no PR.

## Preflight

* Branch `chore/checkpoint-resolution-ordering-restage` at `fe8040e4`, re-fetched, single worktree.
* `.autoharness/config.yaml` valid (schema 1.1.0). Stage route `claude-opus-5`;
  nested escalation route `gpt-5.6-sol` — distinct, so not `ESCALATION_DEGRADED`.
* backlogit 1.10.1; `backlogit sync` → `INDEX_SYNC_OK` (1366 artifacts).
* Checkpoints: 18 resolved, 6 abandoned, **0 active** → ZERO-CANDIDATE NORMAL
  STARTUP, no recovery required, no anomalies.

## Artifacts produced

| Path | Status |
|---|---|
| `docs/decisions/2026-09-13-package-g-agent-contract-assertion-harness-deliberation.md` | `decided-plan-blocked` |
| `docs/exec-plans/2026-09-13-package-g-agent-contract-assertion-harness-plan.md` | `blocked` (plan + hardening + review R1 + correction R1 + confirmation R2) |

## Decision (stands)

**Option A** — Rust `[[test]]` contract target(s) with a typed assertion registry,
plus removal of `'.github/**/*.md'` from CI `paths-ignore`. Affirmed by all seven
reviewers across both rounds. Option B (extend shipped `engram verify`) rejected —
violates "no product runtime behavior change". Option C (PowerShell/bash pair)
rejected — `Select-String`/`grep` match-absence ambiguity, no type safety.

## Review history

Round 1: 9 P1, 0 P0 → FAIL. All bounded and mechanical → one correction round
authorized per operator instruction.

Round 2 (confirmation): Security **PASS**; Rust, Architecture, Scope, Constitution
all **FAIL**. Correction budget exhausted → Package G BLOCKED.

## Two blocking issues a future attempt must settle FIRST

1. **Validator abstraction is wrong.** `engram::services::verify::verify_markdown`
   is an *ingestion* conformance service that emits `template.unresolved` at
   `Severity::Error` for any `{{…}}` line, and `conformant = findings.is_empty()`.
   **Verified: 20 of 92 harness markdown files contain same-line `{{…}}`**
   (`_orchestrator.agent.md`, `_ship.agent.md`, 11 subagents, …). Reusing it would
   reject legitimate agent contracts on day one. Need a harness-owned validator or
   a filtering adapter accepting only `frontmatter.malformed` + `body.empty`.
2. **Module topology is incompatible with the lint gate.** `.cargo/config.toml`
   sets `rustflags = ["-Dwarnings"]` globally; each `tests/*.rs` is its own crate
   where `pub` does not exempt `dead_code`. One shared `mod.rs` across three
   targets cannot satisfy "no new `#![allow(..)]`". **Verified precedent:
   `tests/helpers/mod.rs:34` already carries `#![allow(dead_code)]` for exactly
   this reason.** Choose per-target `#[path]` inclusion or a workspace-member lib
   crate, then re-derive unit boundaries.

Also open: granularity regression (G.1 ≈ 12 scenarios vs the <4 heuristic; G.3 = 7,
G.5 = 6); G.4's file list omits `mod.rs` so it cannot make G.3 green; G.8 has no
red phase and mixes CI config with Rust authoring; provider-catalog binding does
not prove a catalog entry invokes its own module's function.

## Material research corrections (carried into both artifacts)

* **`ignore = "0.4"` and `globset = "0.4"` are existing regular dependencies.**
  My first dependency grep missed them. `ignore::WalkBuilder` supplies
  `follow_links(false)`, `max_depth`, `max_filesize`, `sort_by_file_path` — it
  resolves the symlink, bounds, and determinism findings natively with no new
  dependency. Caveat: `hidden(true)` is the default and `.github` is a
  dot-directory; depth-0 roots bypass the filter, so add each harness directory as
  its own root and set `hidden(false)` explicitly.
* `main` has **no branch protection** (`GET .../branches/main/protection` → 404),
  so `build` is not a required status check. The `ci.yml` contingency holds and
  narrowing `paths-ignore` cannot hang a PR.
* Only **one** `.github` markdown file sits outside the five harness directories
  (`.github/copilot-instructions.md`), so glob negation is unnecessary — deleting
  the ignore entry outright is the fail-closed choice.
* `.cargo/test-coverage-manifest.toml` exists and maps no `.github` surface, so
  the change-scoped oracle reports PASS on harness-markdown edits — a real
  false-PASS path in the runner agents are told to use.

## Explicit non-actions

No implementation code written. No PR created. No merge. PR #396 untouched.
`143.*` left abandoned. Packages A–F not modified or restaged. No backlog
mutation of any kind — no IDs allocated, no shipment assembled, no stash archived.

## Next step

Re-plan Package G as a **fresh Stage operation** after the two blocking decisions
are settled. Not a third correction round on this plan.
