---
title: 'Ship Session — 052-S closure + 064->066 reconcile (PR #186) and 053-S daemonless --direct docs (PR #187)'
type: session-memory
role: ship
date: 2026-07-01
prs:
  - 186
  - 187
shipments:
  - 052-S
  - 053-S
features:
  - 065-F
---

## Session Scope

Two sequential Ship phases in one run, same working tree, both stopping at the
merge gate (open PR + CI green, operator-gated merge — nothing merged).

- **Phase 1** — backlog-hygiene chore PR bundling the 064->066 ID-namespace
  collision reconcile (Stage data-integrity fix) with the 052-S closure move.
- **Phase 2** — ship shipment 053-S (feature 065-F): document the already-shipped
  daemonless `--direct` indexing escape hatch (docs + backlog only).

## Phase 1 — PR #186 (`chore/backlog-064-reconcile`)

- Branched from local `main` so the branch carried the two already-committed
  052-S closure commits (`011dd36` docs + memory, `2d52d38` compound learning).
- Staged **only** concern-1 (064->066 reconcile) + concern-2 (052-S closure move)
  via explicit pathspecs. Verified `git diff --cached --stat` = 17 files, zero
  DRIFT, zero concern-3 before committing.
- Reconcile commit `995f92d`; conventional message + Copilot footer.
- `backlogit doctor` clean for the target classes (zero `duplicate_id` /
  `root_id_collision`; the 43 findings were all benign `archived_from_self_ref`).
- Copilot review COMMENTED (non-blocking): rejected the 052-S `archived_from`
  false-positive (peer archived shipments 001-S..008-S also omit it), but
  **confirmed** a real defect — the reconcile had injected a spurious
  `011-D related_to` link into `066-F.md` (011-D is a verify-domain deliberation,
  unrelated). Removed the link in fix `56905b0`; doctor still clean.
- CI `build` green. **PR #186 OPEN, MERGEABLE, not merged.**

## Phase 2 — PR #187 (`065-daemonless-direct-docs`)

- Branched fresh from `origin/main` (NOT the chore branch) so the reconcile
  changes are not carried into the 053-S PR. Concern-3 untracked planning
  artifacts persisted across checkout; tracked reconcile changes reverted in the
  working tree (desired).
- Verified the documented commands against the clap surface in
  `src/bin/engram.rs`: `Sync { full, direct }`, `Index { direct }`, both
  `--direct` bound to `env = "ENGRAM_DIRECT"` via `BoolishValueParser`,
  `Index` == "Equivalent to `engram sync --full`". Docs match the binary.
- Implemented 065.001-T (`configuration.md` canonical anchor + `troubleshooting.md`
  symptom subsection), 065.002-T (`README.md` QuickStart callout), 065.003-T
  (`start.ps1` + `start.sh` cross-reference comments). 065.004-T (Rust error
  message) intentionally **deferred** / width-isolated out of the docs shipment.
- Committed 4 coherent commits; verified diff vs `origin/main` = 15 intended
  files, zero DRIFT. CI `build` green.
- Copilot review COMMENTED (non-blocking), 7 comments — dispositioned in round 1:
  - **Fixed:** README implied `engram install` manages the daemon → corrected to
    `engram sync` (install only scaffolds `.engram/` + hooks).
  - **Fixed:** 4 archive artifacts lacked `archived_from` → added provenance to
    match peers (backlogit `move --status done` auto-archive omits the field).
  - **Kept by direction:** `start.ps1` `--timeout 300 -> 3000` is a pre-existing
    local modification the operator directed to preserve; PR description corrected
    to drop the inaccurate "comment-only" framing and call it out as intentional.
- Wrote closure note `docs/closure/2026-07-01-053-S-daemonless-direct-docs-closure.md`.
- **PR #187 OPEN, MERGEABLE, not merged.**

## Hygiene / Drift Discipline

- DRIFT never staged in either phase: `.cursor/mcp.json`,
  `.github/copilot-instructions.md`, `.backlogit/memories.json`,
  `.backlogit/telemetry.jsonl`, `.claude/`, `docs/design-docs/.gitkeep`.
- `.backlogit/archive/064-F.md` self-heal collateral (backlogit sync re-injects
  `011-D related_to` + strips `archived_from` on the collision branch) kept
  reverted, never staged. This vindicated the Phase 1 fix — the spurious 011-D
  link is a backlogit-sync artifact.
- Verified `git diff --cached --stat` before every commit.

## Key Learnings

- **backlogit `move --status done` auto-archives** (queue -> archive) and does
  **not** populate `archived_from` / `commit`. Missing `archived_from` produces
  **zero** doctor findings and is peer-consistent for auto-archived items, but
  reviewers flag it — add it manually for provenance when it costs nothing.
- **backlogit sync self-heals on a collision branch**: on the Phase-2 branch
  (which predates the 064->066 reconcile) sync strips `archived_from` from
  `064-F.md` and re-injects `011-D related_to`. Never run backlogit mutations
  when they would re-dirty out-of-scope files; revert with
  `git checkout -- <path>`.
- **Operator-directed drift exception:** a pre-existing local `start.ps1` change
  was explicitly preserved through the one PR that legitimately edits the file.
  When a preserved change is a runtime behavior change, keep the PR description
  accurate and answer review threads as intentional rather than reverting.

## Status at Session End

- PR #186: OPEN, MERGEABLE, CI green, review dispositioned — **not merged**.
- PR #187: OPEN, MERGEABLE, CI green (re-run pending after review-remediation
  push), review dispositioned — **not merged**.
- Both merges are operator-gated. Merge Confirmation Gate not reached.
