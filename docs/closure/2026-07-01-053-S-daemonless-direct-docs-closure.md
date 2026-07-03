---
title: '053-S Daemonless --direct Documentation — Closure'
type: closure
date: 2026-07-01
feature: 065-F
shipment: 053-S
merge_sha: ad0b63297e104054261abeb28aa9790a2b67dbd7
merged_by: softwaresalt
merged_at: 2026-07-02T18:42:19Z
status: shipped
shipment_status: done
prs:
  - 187
---

## Shipment Summary

Shipped feature 065-F: surface and cross-link the already-shipped daemonless
`--direct` indexing escape hatch (`engram index --direct`,
`engram sync --full --direct`, `ENGRAM_DIRECT=1`) across the README,
configuration reference, troubleshooting guide, and startup scripts. The
capability itself shipped in feature 045-F / shipment 030-S; deliberation 010-D
established that the residual gap was **discoverability**, not implementation, so
this shipment is net-new documentation with zero Rust source changes.

## Merge Confirmation

- **PR #187** (`065-daemonless-direct-docs` → `main`): **MERGED** by
  `softwaresalt` at 2026-07-02T18:42:19Z via merge commit
  **`ad0b63297e104054261abeb28aa9790a2b67dbd7`**. Shipment `053-S` is `done`
  (relocated to `.backlogit/archive/053-S.md`); feature `065-F` was reopened
  active for the deferred `065.004-T` code task.

## Tasks Completed

| Task | Title | Status |
|------|-------|--------|
| 065.001-T | Canonical anchor: `docs/configuration.md` + `docs/troubleshooting.md` | ✅ done |
| 065.002-T | README QuickStart `--direct` callout | ✅ done |
| 065.003-T | Cross-reference from `start.ps1` + `start.sh` | ✅ done |
| 065.004-T | Rust: `DaemonError::NotReady` message points at `--direct` | ⏸ deferred (width-isolated to a code shipment) |

## Files Shipped

### Documentation
- `README.md` — new "Daemonless indexing (`--direct`)" QuickStart callout (README
  previously had zero mentions); cross-links both docs.
- `docs/configuration.md` — new canonical "Daemonless direct indexing" reference
  section (three entry points + daemon-lock mutual exclusion prose).
- `docs/troubleshooting.md` — upgraded "Indexing problems" step and new
  "Daemon startup or IPC timeout" symptom subsection routing to direct mode.
- `docs/exec-plans/2026-06-30-document-daemonless-direct-mode-plan.md` — reviewed
  plan artifact (Units 1–4).

### Startup scripts
- `start.ps1` — cross-reference comment above the Engram pre-warm block. A
  pre-existing local pre-warm `--timeout 300 -> 3000` modification was preserved
  intentionally (operator-directed) alongside the comment; called out in the PR
  description and review threads.
- `start.sh` — cross-reference comment above the pre-warm block.

### Backlog artifacts
- `.backlogit/queue/053-S.md`, `.backlogit/queue/010-D.md`,
  `.backlogit/queue/065.004-T.md` (deferred code task).
- `.backlogit/archive/065-F.md`, `065.001-T.md`, `065.002-T.md`, `065.003-T.md`
  (archived on completion; `archived_from` provenance added in review round 1).
- `docs/memory/2026-06-30-stage-0E042A84-session.md`,
  `docs/memory/2026-06-30/065-F-staging-session-memory.md` (Stage provenance).

## Review Rounds

One Copilot review round on PR #187 — state COMMENTED (non-blocking), 7 comments:

- **README daemon attribution (1):** `engram install` was implied to manage the
  daemon; corrected — `engram sync` starts/manages the daemon, `install` only
  scaffolds `.engram/` + hooks (verified against `src/bin/engram.rs` `Install`
  doc comment). **Fixed.**
- **`start.ps1` `--timeout 300 -> 3000` (2):** flagged as a runtime change in a
  docs PR. Confirmed intentional — a pre-existing local modification the operator
  directed to preserve. PR description corrected to drop the inaccurate
  "comment-only / no runtime change" framing; threads answered as intentional.
  **Kept by direction.**
- **Missing `archived_from` on 4 archive files (4):** backlogit's
  `move --status done` auto-archive does not populate `archived_from`. Added the
  provenance field (`.backlogit/queue/065-*.md`) to match peer archive artifacts.
  **Fixed.**

## Runtime / Accuracy Verification

Documented commands verified against the shipped clap surface in
`src/bin/engram.rs`:

- `Sync { full, direct }` and `Index { direct }` subcommands exist.
- Both `--direct` flags bind to `env = "ENGRAM_DIRECT"` via `BoolishValueParser`.
- `Index` help = "Equivalent to `engram sync --full`".

Documented entry points (`engram index --direct`, `engram sync --full --direct`,
`ENGRAM_DIRECT=1 engram sync`) match the binary exactly.

## Healthy Signals

- CI `build` green on PR #187.
- `cargo fmt --all -- --check` clean (zero Rust changed — insurance run).
- Markdown gates: no unresolved `{{...}}` placeholders; code fences balanced; no
  trailing whitespace; all cross-reference anchors resolve.
- `start.ps1` parses (`[ScriptBlock]::Create`); `start.sh` parses (`bash -n`).

## Deferred Items

- **065.004-T** — augment `DaemonError::NotReady` to point operators at
  `--direct` (plus optional top-level clap help). Touches compiled Rust under
  Test-First + cargo gates; width-isolated out of this docs shipment. Remains a
  queued `deferred` task for a future code shipment.

## Rollback

```bash
git revert --no-edit ad0b63297e104054261abeb28aa9790a2b67dbd7   # PR #187 merge commit
```
