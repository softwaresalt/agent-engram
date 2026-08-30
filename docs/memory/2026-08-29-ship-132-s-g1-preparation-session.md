---
title: "Ship session - 132-S G1 preparation"
date: 2026-08-29
type: session-memory
doc_type: memory
agent: ship
worktree: .worktrees/ship-132-s-v0.3.0-rc.1
branch: feat/139-f-v0.3.0-rc.1
base: bdda5aac8e6326d646e3b30d3144e4349db770a9
shipment: 132-S
feature: 139-F
status: active
---

## Scope and release state

This session prepared truthful G1 evidence that now supports open PR #368. The
live PR head is the authoritative exact head; this memory does not embed that
value because a document cannot self-reference its own commit. Hosted CI and
exact-head Copilot review remain pending until verified externally. Their
results will be recorded in the PR description and check rollup without
another source commit.

No local or remote `v0.3.0-rc.1` tag or release was created. `v0.2.0` remains
**Latest**. G2 merge and G3 tag/publication remain separately approval-gated.

## G1 preparation evidence

### Content gates

* Rendered YAML frontmatter: 2,303 files passed; intentional malformed tests
  and fixtures were excluded, and one templated frontmatter was rendered
* Markdownlint: all 13 changed Markdown files passed
* Variable scan: all 20 changed files passed; the accepted plan contains two
  intentional literal `release.toml` references to `v{{version}}`
* Cross-references: all four links passed

### Rust and repository gates

* Format passed
* Locked pedantic Clippy for all targets passed
* The full locked all-target run with bounded `--test-threads=4` passed the
  666-test library target (665 passed, one ignored) and all targets reached
  before one transient Windows failure in
  `contract_start_launcher::launcher_timeout_does_not_terminate_unowned_descendant`
* Cargo stopped after that target failure, so later targets were not claimed as
  passed
* The immediate isolated single-thread rerun passed 3/3
* Hosted PR CI owns completeness for all remaining targets on the exact final
  head; no production or test change widened or masked the unrelated launcher
  timing issue
* Cargo audit passed with 14 allowed warnings
* Oracle independence passed; the coverage report and completeness were each
  218/218, with zero omitted or unmapped entries

### Release gates

Release contract tests were observed red first and are now 8/8 green after
review fixes. Actionlint and Python syntax passed. Static checks confirmed the
RC matches the trigger glob and regular expression, prerelease is true, the
body is curated only, draft and generated notes are false, and fail-fast and
unmatched-file failure are true. The matrix contains exactly Linux x86_64,
Windows x86_64, and Apple ARM, with no Apple Intel target.

Local Windows archive evidence remains provisional and is preserved exactly in
`docs/closure/2026-08-29-v0.3.0-rc.1-verification.md`; it is not final hosted
asset evidence.

## Review outcome

The initial security review found a P1 where substring matching could accept an
RC as stable and a P2 in the fixed output delimiter. Both were fixed
test-first: validation now requires exact SemVer identity and exact archive
basenames, and the delimiter is randomized and collision-checked. The second
security review passed. Rust review had no findings. Final constitution and
scope reassessments had no actionable P0 or P1 findings.

## Degraded tools and evidence limits

Backlogit MCP degraded to its CLI fallback. Engram daemon and workspace probes
each failed after a bounded approximately 30-second attempt, so direct-read
fallback was used. Intercom was unavailable.

The `git-cliff` executable was unavailable locally, and the operator forbids
creating the local RC tag during G1. No output was fabricated. The PR will
disclose the intended changelog and `git-cliff --latest-tag` range from tag
`v0.2.0`, commit `fd46a9eac7b9a1b68a8e6b405573b8a5a0d0b603`, through its eventual
final head. The PR #368 description will record the authoritative range and
check rollup without requiring another source commit. The branch base remains
`bdda5aac8e6326d646e3b30d3144e4349db770a9`.

## Work-item boundary and next steps

Shipment `132-S`, feature `139-F`, and task `139.005-T` remain active.
Completed release-preparation tasks do not close the shipment.

1. Complete hosted CI and exact-head Copilot review for PR #368, then record
   the results in its description and check rollup without another source
   commit.
2. Disclose the authoritative selected range and local `git-cliff` limitation
   in PR #368.
3. Keep G2 merge separately approval-gated.
4. Keep G3 tag and publication separately approval-gated.

## Files modified in this documentation session

* `docs/closure/2026-08-29-v0.3.0-rc.1-verification.md`
* `docs/memory/2026-08-29-ship-132-s-g1-preparation-session.md`

No code, workflow, backlog artifact, release, tag, commit, or remote state was
modified.
