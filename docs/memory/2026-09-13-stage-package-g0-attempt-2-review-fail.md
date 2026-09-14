# Stage session memory — Package G0 attempt 2 (plan-review FAIL)

**Date**: 2026-09-13
**Agent**: Stage
**Branch**: `chore/checkpoint-resolution-ordering-restage`
**Start HEAD**: `0cd3d957` · **Artifacts HEAD**: `38b3120c`
**Scope**: G0 attempt 2 only. G split, G1, G2, Packages A–F, PR #396, and `143.*` were not reopened.

## Preflight

| Check | Result |
|---|---|
| Tool availability (P-012) | `TOOL_OK` — backlogit 1.10.1 CLI reachable |
| Index sync | `INDEX_SYNC_OK` — 1366 artifacts indexed |
| Checkpoint recovery | **ZERO-CANDIDATE NORMAL STARTUP** — 25 checkpoints, 0 active, 0 quarantined, 0 needs_quarantine |
| Topology (P-016) | Single worktree, single branch. No parallel implementation worktree. |

## What was produced

| Artifact | Path |
|---|---|
| Deliberation (attempt 2, supersedes attempt 1) | `docs/decisions/2026-09-13-package-g0-test-filesystem-seam-deliberation-attempt-2.md` |
| Implementation plan (attempt 2, supersedes attempt 1) | `docs/exec-plans/2026-09-13-package-g0-test-filesystem-seam-plan-attempt-2.md` |
| Plan-review record | `docs/closure/2026-09-13-package-g0-attempt-2-plan-review-record.md` |

Attempt-1 artifacts were read as evidence and left unmodified.

## Contracts settled in the deliberation

* **Contract A (complete failure channel)** — derived from a 3-sites × 4-operations
  invocation table yielding exactly 8 channels. Design: adapter returns site-less
  `FsFault { operation, path, kind }`; resolver lifts via the single total
  constructor `FsError::io(site, fault)` into
  `FsError::Io { site, operation, path, kind: io::ErrorKind }`. `ErrorKind` rather
  than `io::Error` so `FsError` can derive `PartialEq`. Narrow `RootMissing`
  specialization: only `NotFound` at `(Root, SymlinkMetadata)`.
* **Contract B (unique corpus identity)** — three layers: Phase-0 lexical root
  grammar; Phase-2/3 canonical root validation plus `DuplicateRoot` /
  `OverlappingRoots` rejection before any traversal; Phase-4 canonical file
  identity map yielding `DuplicateFileIdentity`. Reject, never dedupe.
  `ResolvedFile { rel_path (display), canonical_path (identity) }`.
* **Windows case** — no case folding, no `cfg` fork; exact comparison of
  canonical paths lets the platform decide. FakeFs models it with two
  canonicalization-map entries.
* **`FileKind::Other`** — kept, with a `FakeFs::other()` producer (resolves the
  attempt-1 A-5 / M-8 tension).
* **`thiserror = "1"`** — adopted for convention alignment; already a resolved
  root dependency, so zero new external crates.

## Review outcome — FAIL, terminal at round 1

Seven personas, seven distinct models. **3 FAIL / 4 PASS.**

| Persona | Model | Verdict |
|---|---|---|
| Rust Reviewer | `gpt-5.6-sol` | FAIL (4 P1) |
| Correctness Reviewer | `gpt-5.6-terra` | FAIL (3 P1) |
| Architecture Strategist | `claude-opus-4.8` | PASS (3 P3) |
| Constitution Reviewer | `grok-4.6` | PASS (1 P2) |
| Security Reviewer | `claude-sonnet-5` | PASS (3 P2) |
| Scope Boundary Auditor | `gemini-3.8-flash` | PASS (2 P2) |
| Maintainability Reviewer | `claude-opus-4.7` | FAIL (2 P1) |

Correction budget is restricted to mechanical findings; four architecture-level
P1 clusters meant it **never opened**. No harvest, no IDs, no shipment, no PR.

### Terminal findings

* **A** — `read_corpus` has no containment check and `ResolvedFile`'s public
  fields make its input forgeable. Three reviewers, three models.
* **B** — failure channels are not independently injectable
  (`FakeFs::unreadable` is pinned to `symlink_metadata`); the `ReadToString`
  channel has no proving scenario. Two reviewers, two models.
* **C** — the entry `symlink_metadata` channel does not exist during traversal,
  because Phase 4 uses `DirEntry.kind`. G0.21(c)/G0.22 have no driving operation.
* **D** — `-Dwarnings` forces a `[G]` unit that introduces a seam call to wire
  its failure mapping in the same commit, so G0.13(c), G0.21(a) and G0.21(c)
  pass green before their owning unit ships. Plus `rel_path` needed by G0.24 but
  owned by G0.26, and G0.27(a) already satisfied before G0.28.
* **E** (P1 mechanical) — declared `FsError` is not compilation-complete: nested
  derives and per-variant `#[error(...)]` attributes unstated.

## Recurring meta-defect across three consecutive attempts

v3, attempt 1, and attempt 2 each died on the same class: **the plan claims a
guarantee its mechanism does not deliver.** v3 claimed a seam proof it could not
produce; attempt 1 claimed a complete Failure Semantics table that was not
complete; attempt 2 claims containment (`read_corpus`), channel completeness
(unprovable), a compile-time barrier (H9), a TOCTOU closure (H6), and an
unchanged `Cargo.lock` — five overstatements. **The next attempt should audit
every declarative claim against its named mechanism before review, not after.**

## What is now permanently settled (do not re-derive)

Attempt 2's review adds ten settled items on top of attempt 1's nine. Most
importantly: the `FsFault`/`FsError::Io` split is the **correct** architectural
repair for finding A; the three-layer reject-not-dedupe identity design is the
**correct** repair for finding B; the `thiserror` dependency, the `tests/unit/`
tier, the Constitution mapping, the `-p` clippy command, and the
canonicalize-driven Windows case handling are all affirmed. See the review
record's "What this review SETTLES" section.

## Next steps for a G0 attempt 3

Work the review record's "What the next G0 attempt must settle first" list, in
order: (1) decide `read_corpus`'s trust posture; (2) make every channel
independently injectable and give `ReadToString` a scenario; (3) decide whether
traversal lstats each entry; (4) re-cut red/green staging around `-Dwarnings`;
(5) make `FsError` compilation-complete; (6) fix the three convergent mechanical
defects (G0.11a call log, `Cargo.lock` proof, path-rendering lift); (7) correct
H9 and H6 overstatements.

## Boundary confirmation

No source, test, or configuration file was created, modified, or deleted. No
build, test, or lint was run. No branch or worktree was created. No PR was
created, pushed, or merged. No backlog item, task, or shipment was created. Only
`docs/decisions/`, `docs/exec-plans/`, `docs/closure/`, and `docs/memory/`
artifacts were written, on the existing branch.
