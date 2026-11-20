---
title: "GSD v1.6.0 branch extraction, gate hygiene, and documentation consistency audit"
date: "2026-11-20T05:53:00Z"
status: "complete"
branch: "chore/gsd-v1.6.0-migration"
last_code_affecting_head: "80cbd937fab405ef362bdd1fd5ed4f53c4b0e1dd"
audit_verdict: "AUDIT_FINDINGS_RESOLVED"
---

## Outcome

Separated the GSD v1.6.0 `.autoharness/` -> `.gsd/` migration and harness tuning
commits out of `feat/176-s-142-f-slot-20c-142-089-t-f54c` onto dedicated branch
`chore/gsd-v1.6.0-migration`, applied all post-v1.6.0 gate and contract hygiene
fixes, and resolved all local review and documentation consistency audit
findings.

## Key Changes Applied

* **Branch isolation**: Reverted `11e1bd5b`, `8496ba5b`, and `9139021e` forward
  on `feat/176-s-142-f-slot-20c-142-089-t-f54c` (`8d857c87`, leaving only the 4
  `176-S` files vs `main`) and cherry-picked them onto
  `chore/gsd-v1.6.0-migration` (`4bc245d6`, `a349d1d2`, `734bc72f`), proving a
  zero-conflict future 3-way merge via `git merge-tree --write-tree`.
* **PA-2 launcher preservation (`TUNE-006`)**: Restored `main`'s 142-F PA-2
  `engram preflight` `start.ps1` and `start.sh` launchers (adding conditional
  `.gsd`/`.kbdb\bin`/`.bin` `PATH` setup on `start.ps1`), moved both into
  `preserved_artifacts` (`10` preserved, `113` managed), and verified all
  `22/22` launcher contract tests pass.
* **Topology, checkpoint, and harness-resolve gates**:
  * Restricted `.gsd/carry-forward.yaml` to `files: [.gsd/config.yaml]`.
  * Normalized 8 legacy `status: done` archived shipments (`029-S`, `052-S`,
    `078-S`, `084-S`, `090-S`, `091-S`, `100-S`, `101-S`) to `status: archived`,
    `archived_status: shipped`, and `archived_from: .backlogit/queue/<id>.md`.
  * Renamed `.backlogit/queue/131.001-R-...-otlp-planning-s.md` to
    `...-otlp-planning.md` so `shipmentIDSuffixRe` (`(?i)-s$`) no longer flags
    `ACTIVE_SHIPMENT_INVARIANT_VIOLATED`.
  * Normalized 16 legacy `.backlogit/archive/checkpoints/*` records/sidecars to
    terminal `LEGACY_NO_SCHEMA_VERSION` statuses (`gsd checkpoint scan` exit
    `0`, `blocking.count = 0`).
  * Declared `harness-surface:harness-architect` on `142.089-T` and `142.058-T`
    and `harness-surface:none` on `142.059-T`, fixed
    `skills/harness-architect/SKILL.md.tmpl` + `TEST_DIR: "tests"` +
    `.gitattributes` `eol=lf` rules, and verified `gsd harness resolve` exits
    `0` (`HARNESS_READY`) across `176-S`, `177-S`, and `178-S`.
* **Hook hardening**: Restored fail-closed `GSD_TOPOLOGY_GATE_BLOCKING` default
  and workspace-local `.gsd/gsd(.exe)` / `.bin/gsd(.exe)` discovery across
  `scripts/pre-commit-pipeline-topology.{ps1,sh}` and
  `scripts/pre-push-quality-gates.{ps1,sh}`, hardened `AGENT_TEMP_DIR` and
  `markdownlint --` delimiters, and added `.gsd/metrics/` to `.gitignore`.

## Verification & Documentation Consistency Audit

* **Observed `last_code_affecting_head`**:
  `80cbd937fab405ef362bdd1fd5ed4f53c4b0e1dd`
* **Documentation Consistency Audit outcome**: `AUDIT_FINDINGS_RESOLVED` across
  Categories 1-10 (`Evidence Consistency Reviewer` Cycle 1: `P0=0, P1=0`;
  delta re-audit over Cycle 2 `80cbd937` and this memory artifact: `AUDIT_CLEAN`
  on new/updated surfaces).
* **Local Multi-Persona + `rubber-duck` Review**: `READY_WITH_FOLLOWUPS`
  (`P0=0, P1=0` across `rubber-duck`, `Evidence Consistency Reviewer`,
  `Template Integrity Reviewer`, and `Security Reviewer`; residual P2/P3
  advisory notes on upstream template backticks and preserved workspace-authored
  subagents recorded as non-blocking follow-ups `TUNE-007`..`TUNE-010`).
* **Local Quality Gates**:
  * `cargo fmt --all -- --check`: PASS
  * `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic`: PASS
  * `cargo test --test contract_start_launcher --test contract_start_launcher_failure --test contract_start_sh_launcher --test contract_diagnose_engram --test integration_installer`: `60/60` PASS (2 ignored)
  * `gsd verify-workspace --workspace . --gsd-home .gsd/gsd-assets-v1.6.0.zip --json`: PASS (`0` blockers, `113/113` `unchanged`)
  * `gsd gate pipeline-topology --mode agent --shipment 176-S --phase pre_claim --json`: PASS (`carry_forward_source: head`)
  * `gsd gate line-endings --base main --json` & `markdownlint`: PASS
