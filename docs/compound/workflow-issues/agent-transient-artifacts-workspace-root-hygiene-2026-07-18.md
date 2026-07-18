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

The files were caught by `.gitignore` (`*.log`, `logs/`, `*.txt`, `temp/`, `tmp/`
are all ignored), so they were never committed — but they still cluttered the
workspace root, hurt navigability, and had to be removed with `git clean -fd`.
Root-level `test_*.rs` files are worse than clutter: they are not part of the
`tests/` tier layout, so they neither compile into the suite nor get discovered
by `cargo test`, giving a false sense of coverage while polluting the root.

## Root Cause

Transient files were written with **bare relative filenames** (e.g.
`some-run.log`, `test_probe.rs`). A bare relative path resolves against the
current working directory, which for this session was the workspace root. There
was no explicit directory prefix, so every "temporary" file landed at the top
level instead of in the existing `logs/` or `tests/` directories. The conventions
exist and are documented (AGENTS.md Terminal Command Execution Policy #3–#4) but
were not applied at file-creation time.

## Resolution

1. Removed the stray artifacts (`git clean -fd` for ignored/untracked files;
   direct `Remove-Item` for a specific ignored file that `git clean` skips).
2. Codified the placement rule as institutional knowledge (this document).

Correct placement going forward:

| Artifact kind | Correct location | Never |
|---|---|---|
| Command/run logs, redirected stdout/stderr | `logs/` (e.g. `logs/build-2026-07-18.log`) | workspace root |
| Large tool output to offload from context | `docs/scratch/{YYYY-MM-DD}-{tool}-{slug}.md` | workspace root |
| Permanent test code | `tests/{contract,integration,unit}/` per tier | workspace root, `src/` |
| Throwaway/probe test code | a `tests/` subdir (still tiered) or delete immediately after use | workspace root |
| Temp scratch files | `logs/`, `temp/`, or `tmp/` (all git-ignored) | workspace root |

## Prevention

* **Always prefix a directory** when creating logs or temp files. Write
  `logs/<name>.log`, not `<name>.log`. For redirection, use relative paths that
  begin with `logs/` (AGENTS.md policy #3–#4).
* **Never create test files at the root.** Even temporary/probe test code goes
  under a `tests/` tier; if it is truly throwaway, delete it in the same task
  rather than leaving it at the root. Root `test_*.rs` files are not discovered
  by `cargo test` and give false coverage signals.
* **Prefer `docs/scratch/`** for large tool-output offloads
  (context-efficiency.instructions.md), not root-level `*.txt`/`*.log`.
* **`.gitignore` is not a substitute for hygiene.** Being ignored keeps files
  out of commits but not out of the working tree; ignored clutter still degrades
  navigability and must be cleaned manually.
* **End-of-session check:** run `Get-ChildItem -File` at the root and confirm no
  stray `*.log`, `*.txt`, or `test_*.rs` artifacts remain before wrapping up.
