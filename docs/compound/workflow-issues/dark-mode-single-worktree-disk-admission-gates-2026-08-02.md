---
title: "Dark-mode shipping requires one-worktree, disk, process, and cleanup admission gates"
doc_type: learning
source: "103-S / 105-S dark-mode incident and closure"
description: >-
  Multiple sibling worktrees retained large Cargo targets until the repository
  consumed about 152 GiB and the core target alone reached 142.6 GiB. Prevent
  recurrence by admitting work only through one-worktree, disk, process, and
  cleanup-before-next-claim gates.
problem_type: workflow_issue
category: workflow-issues
component: "Ship orchestration and dark-mode execution"
root_cause: >-
  Shipment transitions were allowed without cleanup-before-next-claim, the
  one-active-shipment policy lacked one-worktree and one-build admission gates,
  and queued control messages were incorrectly treated as interrupts even
  though they cannot stop an agent already executing a long turn.
resolution_type: process_fix
severity: critical
message: "dark_mode_single_worktree_disk_process_admission"
file_path: "docs/compound/workflow-issues/dark-mode-single-worktree-disk-admission-gates-2026-08-02.md"
date: 2026-08-02
confidence: high
shipments: [105-S, 103-S]
citations:
  - ".backlogit/checkpoints/checkpoint-20260802-071505.json"
  - ".backlogit/checkpoints/checkpoint-20260802-082001.json"
  - ".backlogit/checkpoints/checkpoint-20260802-083951.json"
  - ".backlogit/checkpoints/checkpoint-20260802-085748.json"
  - "docs/closure/2026-08-02-105-S-windows-pid-identity-stale-recovery-closure.md"
  - "docs/memory/2026-08-02/ship-105-S-windows-pid-identity-stale-recovery.md"
  - "docs/closure/2026-08-02-103-S-ordinary-index-fail-closed-readiness.md"
  - "docs/closure/2026-08-02-103-S-ordinary-index-runtime-verification.md"
  - "docs/memory/2026-08-02/ship-103-S-ordinary-index-fail-closed.md"
  - "https://github.com/softwaresalt/agent-engram/pull/310"
  - "https://github.com/softwaresalt/agent-engram/pull/311"
  - "https://github.com/softwaresalt/agent-engram/pull/312"
  - "https://github.com/softwaresalt/agent-engram/pull/313"
tags:
  - dark-mode
  - worktree
  - disk-pressure
  - cargo-target
  - admission-gate
  - process-containment
  - cleanup
  - ship-agent
  - 105-S
  - 103-S
---

## Incident

Dark-mode execution retained or created multiple sibling worktrees, each with
large Cargo build output. The core `target` directory reached 142.6 GiB and the
repository footprint approached 152 GiB. Free-space pressure then surfaced as
disk and I/O failures while build activity continued.

A background agent also continued after pause instructions were queued. This
was not a messaging-delivery failure: queued control messages are delivered
only after the active long-running turn yields. They are not an interrupt or
an emergency stop mechanism.

## Root Cause

Three workflow assumptions combined:

1. finishing one release unit did not gate the next claim on cleanup;
2. one active shipment was not reinforced by one registered worktree and one
   active Cargo/Rust process;
3. orchestration treated a queued pause message as if it could preempt a
   command or agent turn already in progress.

Each behavior can appear harmless alone. Together they allow duplicated build
state, continuing disk growth, and an operator who cannot regain control
through normal queued messaging.

## Required Admission Gate

Before every build/test phase and before claiming the next release unit:

1. `git worktree list --porcelain` reports exactly the core worktree;
2. no `cargo` or `rustc` process is running;
3. free disk is at least the configured admission floor;
4. the previous release is committed, pushed, merged, closed, and archived;
5. its exact disposable test-data tree is deleted;
6. approved `cargo clean` has removed the core target;
7. the core is clean before branch switching and the next claim.

Use branch switching in the core worktree. Do not create or copy sibling
repositories to isolate shipments.

## Emergency Control

Dark continuation is forbidden when an agent or shell has not yielded. For an
attached command, stop the known shell session or exact known PID and then
verify its process tree is gone. Never use name-wide termination. After an
emergency stop, snapshot worktree, process, and disk state before deciding
whether execution can resume.

Remote checkpointing must happen before expensive validation so a forced stop
does not strand the only implementation copy.

## Proven Recovery

105-S and 103-S were serialized in the sole core worktree. After each release
unit merged and closed, only its exact hermetic data was removed and the core
target was cleaned. The two Cargo cleanups reclaimed approximately 30.5 GiB
and 29.9 GiB respectively. Final state was one clean `main` worktree, no
Cargo/Rust process, no target directory, and about 193.68 GiB free.

> **Rule:** a shipment is not operationally finished until its disposable
> build/test footprint is removed, and the next shipment cannot be admitted
> before that cleanup is verified.
