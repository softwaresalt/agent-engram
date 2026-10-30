---
title: Session startup and index maintenance separation
description: Keep full index preparation outside AI session startup and reuse per-file hashes.
doc_type: spec
date: "2026-10-30"
scope: lightweight
handoff_status: ready_for_plan
dark_factory_ready: false
requirement_ids: [R1, R2, R3, R4, R5]
---

## Problem frame

The launchers currently force a fresh index generation before starting Copilot.
Increasing their timeout does not address the operator's requirement: session
startup must not rebuild the workspace.

## Requirements

* R1. Neither launcher invokes Engram preflight, sync, indexing, generation
  building, or publication as a condition of starting Copilot.
* R2. Full index preparation remains an explicit maintenance operation outside
  MCP/session startup. Ordinary maintenance uses the existing index; forced
  re-extraction requires an explicit request.
* R3. A file's identity is its validated, normalized path relative to the workspace
  root, including its name and parent directories, not its basename alone.
* R4. Non-forced indexing and incremental synchronization skip a file when its
  stored content hash matches. Equal basenames in different directories remain
  independent. Existing explicit topology/migration invalidations remain intact.
* R5. Verify the actual Copilot launcher, argument forwarding, absence of Engram
  subprocesses, and unchanged-file behavior rather than relying on timeout mocks.

## Scope and assumptions

* Preserve workspace-local Copilot configuration and normal executable errors.
* Existing incremental daemon/watch updates are distinct from a forced rebuild;
  this change does not disable them.
* A missing index must not block Copilot itself. Index preparation is explicit.
* Do not retry the old full-build startup workflow or weaken its standalone
  maintenance verification contract.
* Do not add a new cache, identifier scheme, or background scheduler.

## Evidence and handoff

The CLI workspace-status query now succeeds. The existing indexer uses
`get_code_file_by_path` and compares `content_hash` before parsing unchanged
files; startup instead bypasses reuse with a fresh forced candidate.

Requirements reviewed against the reported failure and source. No blocking
questions remain; the operator was unavailable for the missing-index question.

`BRAINSTORM_HANDOFF_READY`: this artifact; unresolved questions: none;
handoff: implementation planning for existing work item 143-F.
