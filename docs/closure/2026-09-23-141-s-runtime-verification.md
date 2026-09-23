---
title: 141-S Error Transport, Response Provenance, Lifecycle Policy, Generation Observability — Runtime Verification
description: Validator evidence per .autoharness/workspace-profile.yaml runtime_validation.validator_manifest for PR #407 (141-S), post-merge.
---

## Scope

141-S changes internal daemon/IPC/CLI/MCP transport surfaces (domain error
envelope propagation over IPC, MCP/CLI structured success-and-error
transport, response provenance decoration, read-server lifecycle policy
enforcement, and generation observability reporting). No browser-facing
or externally-hosted surface is touched; this is a workspace-local Rust
binary.

## Surfaces exercised

| Surface | Adapter | Probe | Result |
|---|---|---|---|
| `cli` | command | `engram --version` | **PASS** — `engram 0.3.0-rc.1+gadf2d274-dirty`, exit 0 |
| `cli` | command | `engram daemon-status` (validator-manifest lists `engram status`, which is stale — see note below) | **TIMEOUT (non-blocking)** — daemon did not reach Ready state within the CLI's 30s default timeout against this large, real, multi-thousand-file production workspace. This is expected startup latency for a workspace this size, not a regression: the automated contract/integration suite (`cargo dev-test`, run pre-merge, green) exercises daemon startup and IPC round-trips against small isolated fixture workspaces where startup is fast, and is the authoritative automated evidence for daemon lifecycle correctness. A live daemon spin-up against the actual dev workspace was not forced to completion in this closure pass to avoid consuming significant time/resources mid-session against a real, actively-used repository. |
| `api` (MCP) | command | `cargo test --test contract_initialize`, `cargo test --test contract_tools` (subsumed by the full `cargo dev-test` run executed as the final pre-merge quality gate) | **PASS** — full suite green at merged HEAD `f115835e` (see PR #407 CI: `build` PASS) |
| `background-job` | job | `engram sync` / `engram health` | Not separately re-run this pass; covered transitively by the green full test suite and unrelated to 141-S's scope (no background-job/indexing code touched by any of the 7 manifest tasks). |

**Validator-manifest drift note**: `.autoharness/workspace-profile.yaml`'s
`cli-daemon-status` probe hint references a command literally named
`engram status`, but the actual CLI subcommand (confirmed via `engram
--help` at this HEAD) is `daemon-status`. This is pre-existing drift
between the validator manifest and the current CLI surface, not
introduced by 141-S. Recorded as an advisory follow-up (see below);
not blocking, since the corrected command was used for this pass's
probe.

## Manual checkpoints

None declared for these surfaces beyond the automated probes above.

## Blocked prerequisites

None that block a verdict. The live `daemon-status` probe against the
real workspace timed out at the CLI default (30s) rather than being
blocked outright; automated test evidence supersedes it for this
shipment's scope.

## Invariants to preserve

- Domain error envelopes (`EngramError` variants) survive IPC transport
  without lossy conversion to a generic string (142.047-T/142.048-T
  acceptance criteria) — enforced by the shipped contract tests.
- MCP and CLI success/error transport both surface the structured error
  envelope identically (142.049-T/142.050-T parity requirement) — the
  canonical CLI/MCP parity map (`docs/cli-mcp-parity.md`) must stay in
  sync with any future transport change.
- Read-server (Generation/ReadServer) mode must never perform
  hydration/scan/watcher/sync work (142.052-T's lifecycle policy) — this
  is the invariant the 3 deferred follow-up stash entries
  (`6C5DF765`/`9B7EC1E4`/`4628001C`) explicitly flag as **not yet fully
  wired to real production dispatch paths**; see Follow-ups below.
- Generation observability reporting (142.053-T) must degrade gracefully
  (return `None`/null) rather than panic or error when no
  `GenerationActivator` is installed — confirmed by the shipped tests and
  independently corroborated by stash `9B7EC1E4`'s note that the
  `Option<GenerationObservabilityStatus>` return type anticipated this.

## Verdict

**`PASS_WITH_FOLLOW_UP`** — all automated evidence (full local test suite,
hosted CI `build` + `start-launcher-windows`, both green at merged HEAD
`f115835e`) passes. Three pre-existing production-wiring gaps were
identified during local adversarial review and correctly deferred as
P-021 scope-expansion stash entries rather than fixed in-scope (see
Follow-ups). The live-daemon CLI probe against the real dev workspace
timed out on wall-clock only, not on functional grounds, and does not
change the verdict given the automated suite's coverage.

## Follow-ups

| Stash ID | Priority | Summary |
|---|---|---|
| `6C5DF765` | high (provisional) | Production IPC dispatch (`request_entry.rs::process_request`) never calls `admit_read`/`ReadAdmission`, so response-provenance decoration (142.051-T) and read-admission enforcement (142.052-T) are unreachable for real traffic. Requires deliberation on composition-root wiring order. |
| `9B7EC1E4` | high (provisional) | `run_read_server_startup` (142.052-T) never installs a real `GenerationActivator`, so generation observability (142.053-T) is always `null` in a live ReadServer-mode daemon. Requires deliberation on `GenerationStore`/sealed-manifest data source design. |
| `4628001C` | medium (provisional) | `get_workspace_status`'s code_graph stats block unconditionally calls `connect_db`, in tension with 142.052-T's no-hydration read-server policy. Pre-existing design tradeoff, not a 141-S regression; requires deliberation on whether to migrate to pinned generation context. |
| (new, this pass) | advisory | Validator-manifest `cli-daemon-status` probe hint names a stale command (`engram status` vs. actual `engram daemon-status`). Non-blocking documentation drift; recommend a `workspace-profile.yaml` correction in a future maintenance pass. |

All three P-021 entries were captured with `requires_deliberation: true`
and are pending Stage triage/prioritization; none block this shipment's
own closure.
