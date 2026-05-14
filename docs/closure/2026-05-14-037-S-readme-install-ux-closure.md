---
title: "037-S README and install UX improvements — Closure"
type: closure
date: 2026-05-14
feature: 051-F
shipment: 037-S
pr: 136
merge_sha: 37bc92e13713a5f5d0d3079b0eadb10cd63c4a07
branch: feat/readme-install-ux
---

## Summary

Shipped the README and install UX improvements for 051-F. The work added
release-based installer scripts, promoted install-from-release as the primary
onboarding path, and updated the quickstart guide to cover both release and
source-build flows.

## Tasks Completed

| Task | Title | Status |
|---|---|---|
| 051.001-T | Create install scripts (`install.ps1` and `install.sh`) | archived |
| 051.002-T | Add Features and QuickStart sections to `README.md` | archived |
| 051.003-T | Update `docs/quickstart.md` with install-from-release paths | archived |

## Changes Shipped

### 051.001-T — release installers

* Added `scripts/install.sh` for Linux x86_64 and macOS arm64 release installs
* Added `scripts/install.ps1` for Windows x86_64 release installs
* Both installers fetch the latest GitHub release, download the platform asset,
  verify a `.sha256` sidecar when present, and fall back to archive-structure
  validation with a warning when no checksum is published
* Both installers install into a user-local path without requiring elevation

### 051.002-T — README onboarding

* Replaced brochure-style onboarding gaps with a benefit-oriented `Features`
  section
* Added a `QuickStart` section with PowerShell and shell one-liner install
  commands
* Kept the build-from-source path available while making release install the
  primary path

### 051.003-T — quickstart guide

* Added an install-from-release section to `docs/quickstart.md`
* Preserved the existing build-from-source path
* Updated prerequisites and overview text so the guide now matches both install
  flows

## Quality Gates

| Gate | Result |
|---|---|
| Review gate | ✅ Copilot review addressed |
| Bot review threads | ✅ 10 Copilot threads resolved |
| CI | ✅ GitHub Actions `build` succeeded |

## Copilot Review

Copilot review completed on PR #136 and all bot-authored threads were handled
before merge:

* fixed cleanup flow in `install.ps1` by replacing `Write-Error` + `exit` with a
  centralized abort helper
* created the fish profile directory before appending PATH updates in
  `install.sh`
* added optional `.sha256` checksum verification to both installers
* corrected archive provenance metadata for archived backlog items
* updated the quickstart overview to match the new release-install path
* declined the stash-archive schema comment with rationale, then resolved the
  thread

## Pre-Deploy Audit

| Check | Status |
|---|---|
| Feature flags | N/A |
| Rollback procedure | `git revert --no-edit -m 1 37bc92e13713a5f5d0d3079b0eadb10cd63c4a07` |
| Data migration | None |
| Cross-service dependencies | None |
| Monitoring plan | Manual install smoke checks only |

## Healthy Signals

* README QuickStart commands remain valid against current release assets
* `docs/quickstart.md` continues to match the supported install paths
* Installers download and extract the expected release archives on supported
  platforms

## Failure Signals

* Release assets are renamed without matching installer updates
* `.sha256` sidecars are malformed or drift from published archives
* QuickStart commands point to missing files or broken release URLs

## Monitoring Plan

This shipment changes install and onboarding UX, not daemon runtime behavior.
Manual observation is sufficient:

* validate the documented one-liner commands against the next release cut
* confirm the README and quickstart links still resolve after future doc moves
* owner: softwaresalt

## Rollback Trigger

Rollback if the documented install commands stop working for supported platforms
or if the published installers fetch the wrong release asset.

## Follow-Up Items

None created during closure.
