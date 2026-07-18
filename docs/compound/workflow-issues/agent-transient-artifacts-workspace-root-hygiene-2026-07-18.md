---
title: "Write transient artifacts to designated dirs, never the workspace root"
description: "Agents littered the workspace root with *.log and test_*.rs files instead of using logs/ and tests/; enforce directory conventions for all generated/temporary output."
problem_type: "workspace-hygiene"
category: "workflow-issues"
component: "agent-workflow"
root_cause: "Transient log and test files were written with bare relative filenames, defaulting to the current working directory (workspace root), bypassing the existing logs/ and tests/ directory conventions."
resolution_type: "workaround"
severity: "low"
message: "stray *.log and test_*.rs files accumulate at workspace root"
file_path: "logs/"
citations:
  - "AGENTS.md:286-287 (Terminal Command Execution Policy: relative paths for output redirection; temporary output files go in logs/)"
  - ".gitignore (ignores *.log, logs/, *.txt, temp/, tmp/ — but root clutter still harms navigability)"
  - ".github/instructions/context-efficiency.instructions.md (docs/scratch/ for large tool output)"
  - "Constitution Principle II / tests/ three-tier layout: contract/, integration/, unit/, helpers/"
  - "Cargo.toml:124-170 (nested test files are registered as explicit [[test]] targets; Cargo does not auto-discover files under tests/{tier}/)"
tags:
  - "workspace-hygiene"
  - "logs"
  - "tests"
  - "temporary-files"
  - "output-redirection"
  - "agent-discipline"
---

## Problem

During a long autonomous session the agent created many `*.log` files and several
`test_*.rs` files directly at the **workspace root** (`C:\Source\GitHub\engram\`),
even though the repository already provides dedicated homes for both:

* a `logs/` directory for transient/temporary output, and
* a `tests/` tree (`contract/`, `integration/`, `unit/`, `helpers/`) for test code.

The two artifact kinds behaved differently, and conflating them hides the real
risk:

* **`*.log` / `*.txt` files** *are* matched by `.gitignore` (`*.log`, `*.txt`,
  `logs/`, `temp/`, `tmp/`). They were never at risk of being committed, but a
  plain `git clean -fd` does **not** remove ignored files, so they survived the
  routine cleanup and had to be deleted explicitly (e.g. `Remove-Item` /
  targeted `rm`). Being ignored also means they silently accumulate — the
  working tree never flags them.
* **Root-level `test_*.rs` files** are *not* ignored by `.gitignore`, so they
  show up as untracked in `git status` and carry a real **accidental-commit
  risk**. They are removed by a plain `git clean -fd` (untracked, non-ignored),
  but if left in place they can be staged by a broad `git add`.

Root `test_*.rs` files are worse than clutter for a second reason: even if they
compiled, Cargo does **not** auto-discover test files nested in subdirectories —
only top-level `tests/*.rs`. This repository registers every nested test file as
an explicit `[[test]]` target in `Cargo.toml`, so an unregistered probe (whether
at the root or nested under `tests/`) is never run by `cargo test`, giving a
false sense of coverage.

## Root Cause

Transient files were written with **bare relative filenames** (e.g.
`some-run.log`, `test_probe.rs`). A bare relative path resolves against the
current working directory, which for this session was the workspace root. There
was no explicit directory prefix, so every "temporary" file landed at the top
level instead of in the existing `logs/` or `tests/` directories. The conventions
exist and are documented (AGENTS.md Terminal Command Execution Policy #3–#4) but
were not applied at file-creation time.

## Resolution

1. Removed the stray artifacts using the correct tool for each kind:
   * untracked, non-ignored probes (`test_*.rs`) were removed by `git clean -fd`;
   * ignored artifacts (`*.log`, `*.txt`) are skipped by `git clean -fd`, so they
     were deleted explicitly (`Remove-Item` / targeted `rm <file>`). Do **not**
     reach for `git clean -fdx` to sweep them — `-x` deletes *all* ignored
     workspace state (build caches, local env files, tool databases) and is a
     high-blast-radius operation.
2. Codified the placement rule as institutional knowledge (this document).

Correct placement going forward:

| Artifact kind | Correct location | Never |
|---|---|---|
| Command/run logs, redirected stdout/stderr | `logs/` (e.g. `logs/build-2026-07-18.log`) | workspace root |
| Large tool output to offload from context | `docs/scratch/{YYYY-MM-DD}-{tool}-{slug}.md` | workspace root |
| Permanent test code | `tests/{contract,integration,unit}/` per tier **and** registered as a `[[test]]` target in `Cargo.toml` | workspace root, `src/` |
| Throwaway/probe test code | delete immediately after use; if it must persist, place it under a `tests/` tier **and** register it in `Cargo.toml` (otherwise `cargo test` never runs it) | workspace root, or nested but unregistered |
| Temp scratch files | `logs/`, `temp/`, or `tmp/` (all git-ignored) | workspace root |

## Prevention

* **Always prefix a directory** when creating logs or temp files. Write
  `logs/<name>.log`, not `<name>.log`. For redirection, use relative paths that
  begin with `logs/` (AGENTS.md policy #3–#4).
* **Never create test files at the root, and never rely on nested placement
  alone.** Even temporary/probe test code goes under a `tests/` tier; if it is
  truly throwaway, delete it in the same task. Cargo auto-discovers only
  top-level `tests/*.rs` — files nested under `tests/unit/`,
  `tests/integration/`, or `tests/contract/` must be registered as `[[test]]`
  targets in `Cargo.toml`, or `cargo test` will silently skip them and report
  false coverage. Root `test_*.rs` files are additionally *not* git-ignored, so
  they risk accidental commits.
* **Prefer `docs/scratch/`** for large tool-output offloads
  (context-efficiency.instructions.md), not root-level `*.txt`/`*.log`.
* **`.gitignore` is not a substitute for hygiene.** Being ignored keeps files
  out of commits but not out of the working tree; ignored clutter survives
  `git clean -fd` and must be cleaned explicitly (not via the high-blast-radius
  `git clean -fdx`).
* **End-of-session check** — confirm no stray `*.log`, `*.txt`, or `test_*.rs`
  artifacts remain at the root, using the shell for the current platform:
  * PowerShell: `Get-ChildItem -File | Where-Object Name -match '\.(log|txt)$|^test.*\.rs$'`
  * Bash/zsh (Linux, macOS): `ls -1 *.log *.txt test_*.rs 2>/dev/null`
