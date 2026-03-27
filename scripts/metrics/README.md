---
title: "Metrics analysis scripts"
description: "Usage notes for the feature-010 baseline extraction and calibration PowerShell scripts."
---

## Purpose

These scripts support feature 010's offline analysis workflow.

`Extract-SearchBaseline.ps1` reads the Copilot CLI session store and emits
JSONL records for non-engram search sequences.

`Compare-Metrics.ps1` compares those baseline JSONL records with Engram's
`.engram/metrics/<branch>/usage.jsonl` files and derives per-purpose
calibration multipliers.

## Commands

```powershell
pwsh scripts\metrics\Extract-SearchBaseline.ps1 -Repository "softwaresalt/agent-engram"
pwsh scripts\metrics\Extract-SearchBaseline.ps1 -Validate
pwsh scripts\metrics\Compare-Metrics.ps1 -BaselinePaths .\baseline.jsonl -EngramPaths .\.engram\metrics\main\usage.jsonl
pwsh scripts\metrics\Compare-Metrics.ps1 -BaselinePaths .\baseline.jsonl -EngramPaths .\.engram\metrics\main\usage.jsonl -OutputFormat json
pwsh scripts\metrics\Compare-Metrics.ps1 -Validate
```

## Notes

The baseline extractor looks for the Copilot CLI session store in
`$env:COPILOT_CLI_SESSION_STORE`, then common `%LOCALAPPDATA%` locations.

Branch sanitization matches the daemon's filesystem convention: `/` becomes
`__`.
