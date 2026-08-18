---
title: "HCL source-read TOCTOU security review"
doc_type: security-review
date: 2026-08-16
shipment_id: "117-S"
feature_id: "121-F"
status: BLOCKED
severity: P1
confidence: high
evaluated_implementation_head: "40c5b1fbdba38e371cc53244969ec08ca0b5bf83"
evaluated_tree: "e12e19ee1f7a7708e28f16e6a2dca28900f45351"
---

# HCL Source-Read TOCTOU Security Review

## Decision

Shipment `117-S` is blocked. The current discovery-time symlink checks do not
provide the claimed outside-workspace containment guarantee when a concurrent
workspace writer replaces a discovered file or ancestor directory before the
later pathname-based metadata/read operation.

This is a P1 merge blocker. It was not dismissed merely because the GitHub
observation was suppressed, and no check-only mitigation is represented as a
complete fix.

## Threat model and impact

An attacker or concurrent tool that can mutate the indexed workspace can:

1. allow discovery to observe a regular `.hcl`, `.tf`, or `.tfvars` file;
2. replace that file, or an ancestor directory, with a symlink/reparse point;
3. redirect the later pathname read to an external daemon-readable file; and
4. cause external content to be persisted under an in-workspace relative path.

The impact is material because `.tfvars` commonly contains credentials and
indexed source bodies are persisted and exposed through graph read surfaces.
The race is practical on Unix through symlink plus atomic rename. Windows
symlink/junction availability varies, but the shipment's Windows verification
environment successfully created both directory and file links.

## Evidence

The evaluated implementation is
`40c5b1fbdba38e371cc53244969ec08ca0b5bf83`, tree
`e12e19ee1f7a7708e28f16e6a2dca28900f45351`.

Relevant paths:

| Surface | Evidence |
|---|---|
| Discovery-only link rejection | `src/services/code_graph.rs` `discover_files` uses `follow_links(false)` and non-following `DirEntry` state |
| Full-index reads | `src/services/code_graph.rs` performs later path metadata and `read_to_string` operations |
| Startup/explicit-sync reads | `src/services/code_graph.rs` incremental sync performs the same later pathname operations |
| Runtime entry points | daemon cold start, MCP/IPC sync, and direct CLI index/sync all reach those readers |
| Persistence/exposure | parsed class bodies are stored by code-graph queries and returned by graph read tools |
| Existing controls | `tests/unit/hcl_security_test.rs` rejects links that already exist before discovery, not replacement after discovery |

The same review identified Rust prepass and Rust/Python canonical postpass reads
that must use the eventual shared containment primitive. Fixing only the HCL
call site would leave the shared source boundary inconsistent.

## Why a local `std` patch is insufficient

These mitigations do not close the race:

- `symlink_metadata` immediately before `read_to_string`;
- `canonicalize().starts_with(workspace_root)` followed by a pathname reopen;
- final-component-only `O_NOFOLLOW` or Windows open-reparse flags; or
- another discovery-time `DirEntry` check.

Each check and subsequent pathname open remain separable, and
final-component-only protections do not stop ancestor-directory replacement.
The crate forbids unsafe code, so portable raw platform handle code is not an
acceptable shipment-local workaround.

A complete portable fix requires capability/open-handle semantics:

1. open the canonical workspace directory as a capability;
2. accept only validated relative candidate paths;
3. open every component beneath that capability without following links or
   escaping the root;
4. derive regular-file identity and size from the opened handle;
5. read from that same handle without reopening by path; and
6. route discovery, full index, sync, prepass, and postpass source reads through
   the shared primitive.

An audited safe wrapper such as `cap-std`, or an equivalent capability
filesystem abstraction, is the smallest credible direction. Adding that
dependency and rerouting all source readers is a broader architectural release
unit, not a safe incidental patch to `117-S`.

## Required RED and control coverage

A follow-up implementation must begin with:

1. final-file replacement: discover regular `victim.tf`, replace it with an
   external-file link at a deterministic reader barrier, and prove no sentinel
   content reaches code files, class bodies, or `map_code`;
2. ancestor replacement: replace `nested/` with an external-directory link
   after discovery and prove the same fail-closed result;
3. startup/cold-sync coverage through the shared reader;
4. explicit-sync coverage that retains the last-known-good graph and never
   persists the external replacement;
5. unchanged regular-file and regular-to-regular replacement controls;
6. static directory/file link rejection; and
7. enforced Windows reparse coverage rather than treating an unsupported link
   setup as a security pass.

After remediation, any potentially contaminated graph requires a forced rebuild
and zero-external-body reconciliation.

## Actionable backlog proposal

**Title:** Introduce a capability-rooted source reader for code-graph indexing

**Type:** security bug / architecture task

**Priority:** critical

**Scope:** shared source-open boundary for discovery, full indexing,
startup/explicit sync, Rust prepass, and canonical postpasses.

**Acceptance:** all RED/control coverage above passes on supported platforms;
no source body is read through a pathname reopened after containment
validation; no unsafe workspace code; no HCL or non-HCL regression; full
quality, runtime, and current-HEAD review gates pass.

Ship did not create a twenty-first implementation item or mutate planning
fields. This proposal and evidence are the blocked handoff for the appropriate
Stage/security planning workflow.

## Scope and stash disposition

No eighth operator stash entry was published after repeated supported backlogit
reads across contained worktrees and refreshed local/remote refs. The seven
known entries remain unchanged. No later shipment, rejected `116-S`, or
`120-*` item was selected or mutated.
