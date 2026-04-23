---
title: "Carry forward entire local working tree to main"
date: 2026-04-22
status: "in-flight"
source_branch: "release/006-s-daemon-reliability-b1"
target_branch: "carry-forward-everything"
base_main: "880e09cbbae704341726bb4e33a3e49e4bb1cdcf"
---

# Carry forward entire local working tree to main

## Why

Operator confirmed the full dirty working tree must be preserved onto `main`,
not selectively stashed away. In particular, the queued backlogit work for
future Stage triage cannot be lost, and some items may have belonged with the
previous release.

## What is being carried

* all modified tracked files from the source checkout
* all untracked files from the source checkout
* backlog queue, stash, stage plans/decisions/memory, scripts, docs, and local config files exactly as present in the source checkout

## Execution path

1. Created clean worktree branch `carry-forward-everything` from `origin/main`
2. Copied the entire current local delta from the old feature checkout into the clean worktree
3. Open a PR and merge it onto `main` so the state is durable and reviewable

## Source preservation

The original checkout on `release/006-s-daemon-reliability-b1` remains untouched
and dirty during this carry-forward lane.
