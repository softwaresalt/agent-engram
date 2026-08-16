---
title: "HCL family parser Stage session memory"
date: "2026-08-15"
agent: "stage"
feature_id: "121-F"
shipment_id: "117-S"
status: "fully-staged"
---

# HCL Family Parser Stage Session Memory

## Outcome

Consumed sole active stash entry `4BC7A6DE`, promoted it to high priority, completed deliberation `018-D`, safely investigated dependency compatibility, created and hardened a reviewed plan, passed standard multi-persona and mandatory multi-model adversarial gates, harvested feature `121-F` with 16 tasks and 4 subtasks, and assembled exactly one queued shipment `117-S`. No implementation, source/test/config edit, build, lint, shipment claim, or Ship action occurred.

## Tool and Safety State

- Backlogit MCP was unavailable as declared by the operator; patched CLI `1.9.1-0.20260815082408-1235bcd80879` was used for every backlog operation.
- Metadata catalog, types, templates, and relevant WIT definitions were refreshed before mutation.
- Backlog index sync reported 19 pre-existing parse failures; direct CLI mutations/gets remained functional.
- Engram CLI sync timed out and indexed search then hit a locked database. Degraded discovery was declared and targeted Git/file reads were used only after MCP/CLI insufficiency.
- Work occurred on isolated worktree `.copilot/session-state/9ec2cd49-aa90-4186-9c44-d92bca7e5afc/files/stage-121-hcl-family-parser`, branch `121-hcl-family-parser-stage`, based on `origin/main`.
- Root pre-existing `.backlogit/stash.jsonl` and untracked `docs/memory/2026-08-15/backlogit-mcp-startup-repair-memory.md` remained untouched.
- Shipment `116-S` and feature/tasks `120-*` were treated as active/Ship-owned and not modified, replanned, claimed, or inspected for implementation changes.

## Dependency Decision

Proceed with exact crates.io `tree-sitter-hcl = "=1.1.0"`. Official and independently streamed archive SHA-256: `5a7b2cc3d7121553b84309fab9d11b3ff3d420403eef9ae50f9fd1cd9d9cf012`. Published `node-types.json` SHA-256: `d86638c95d20335b960abb62f6758ab53f78fd0efbe4b6669473b5a20dfd1fb5`. The package is non-yanked, Apache-2.0, uses normal `tree-sitter-language 0.1`, and tests against tree-sitter 0.25.3; Engram already locks tree-sitter 0.25.10 and tree-sitter-language 0.1.7.

The Git `v1.1.0` tag exposes older non-equivalent Rust metadata. The registry artifact/checksum is therefore the reviewed authority and the tag mismatch is an explicit provenance exception. No untrusted grammar code was executed by Stage. Ship must block on checksum/source/ABI drift and may not substitute a Git/path/vendored grammar or unsafe shim.

## Design Decisions

- One `Language::Hcl`, parser service, and path classifier shared by `.hcl`, `.tf`, `.tfvars`; no `terraform` language token or parser trait framework.
- Namespaced structural declarations: `hcl.block.*` and `hcl.attribute.*`.
- Only plain variable/get-attribute traversals become normalized hints; dynamic/index/splat/template/function forms are skipped.
- HCL references are always file self-loops with `target_hint` in v1 and bypass global name resolution to prevent cross-language collisions.
- U11 persistence guard must complete before U12 startup/default enablement and U13 live routing.
- Runtime verification and operational closure remain separate Ship-owned, single-domain tasks.

## Review Record

Standard plan review used Constitution, Rust, Scope, Learnings, Architecture, Agent-Native Parity, and Security personas. Initial P1 groups were remediated; two re-reviews returned PASS. Mandatory adversarial review used independent `gpt-5.4`, `claude-opus-4.6`, and `gemini-3.1-pro-preview` reviewers. The HIGH 3/3 RED/dependency-order finding was fixed by inserting test-only ABI harness U7 after dependency U6 and before registration U8. Medium graph/provenance findings were fixed or explicitly accepted. A low-confidence enable-before-guard finding was fixed. Final verdict: three PASS, zero P0/P1.

Artifacts:
- `docs/decisions/2026-08-15-hcl-family-parser-deliberation.md`
- `docs/decisions/2026-08-15-tree-sitter-hcl-compatibility-spike.md`
- `docs/exec-plans/2026-08-15-hcl-family-parser-plan.md`
- `docs/closure/2026-08-15-hcl-family-parser-stage-adversarial-review.md`
- archived backlog review `121.001-R`

## Backlog Hierarchy

Parent feature: `121-F`. Tasks: `121.001-T` through `121.016-T`. Subtasks: `121.002.001-ST`, `121.003.001-ST`, `121.015.001-ST`, `121.016.001-ST`. All items are queued and scoped to 30-120 minutes, one domain, and an atomic outcome.

Execution order: U1-U5 dependency-agnostic RED; U6 exact dependency; U7 ABI/registration RED; U8 registration; U9 symbols; U10 traversal hints; U11 persistence guard; U12 startup/default; U13 live routing; U14 docs; U15 runtime evidence; U16 closure. Dependency edges are persisted in backlogit.

Shipment `117-S` is queued with parent-first manifest containing `121-F`, all 16 tasks, and all 4 subtasks. It was never claimed or activated. Accepted review `121.001-R` and deliberation `018-D` are archived for traceability and intentionally not execution manifest members.

## Compact-Context Assessment

Invoked assessment because `docs/memory` exceeds the 40-file trigger: 112 files, 468.7 KB. No files are older than 14 days, and no inactive stale candidate was identified. Result: 0 files compacted, 0 plans consolidated, all active checkpoints preserved; no destructive move was attempted.

## Next Steps

Ship may consider queued `117-S` only after active isolated release `116-S` is safely resolved under P-001. Ship must claim `117-S` itself, execute RED-first dependencies exactly, stop on dependency/ABI/provenance failure, run runtime/closure tasks, and preserve queued status until that handoff.
