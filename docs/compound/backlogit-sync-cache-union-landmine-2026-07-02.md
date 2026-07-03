---
title: "backlogit sync unions the stale SQLite cache back into Markdown source"
description: "backlogit sync rehydrates from the .backlogit SQLite cache and can resurrect source-only backlog edits (status/location changes, deletions) that were made while the cache held a divergent prior state; stale `backlogit mcp` stdio servers also lock the gitignored backlogit.db* and make writes appear to hang or fail"
problem_type: "stale_state_resurrection"
category: "tooling-hazard"
component: "backlogit CLI / .backlogit workspace cache"
root_cause: "sync treats the disposable SQLite index as a source of truth and unions its rows back into Markdown; a leftover `backlogit mcp` process keeps a WAL lock on backlogit.db* so concurrent writes contend"
resolution_type: "process_workaround"
date: "2026-07-02"
shipment: "053-S"
---
# backlogit sync unions the stale SQLite cache back into Markdown source

## Problem

`backlogit sync` is documented as "Rehydrate the SQLite index from Markdown
source files" — i.e. Markdown is the source of truth and the SQLite cache
(`.backlogit/backlogit.db{,-wal,-shm}`, gitignored) is disposable. In practice
the operation performs a **union**: rows that still live in the SQLite cache get
written **back into the Markdown source**. When the cache holds a *divergent
prior state* (because a status change, queue↔archive move, or deletion was
applied to the Markdown/source layer while the cache lagged), a later `sync`
**resurrects the stale cache state into Markdown**, silently undoing
source-level edits.

A second, compounding hazard: a leftover `backlogit mcp` stdio server keeps a
WAL lock open on the gitignored `backlogit.db*`. While that PID is alive,
mutation commands can appear to hang, fail with a database-locked/busy error, or
contend with the cache — which is easy to misread as data corruption.

Both failure modes were hit twice during shipment 053-S post-merge closure.

## Fix / Workaround

1. **Avoid unnecessary `sync`.** Normal CLI mutations (`move`, `shipment ship`,
   `update`) already write both Markdown and the cache atomically. Do NOT run a
   reflexive `sync` after them just because the CLI prints
   `index may be stale after mutation` — that warning is advisory, and running
   `sync` is precisely what re-unions stale rows.

2. **If backlogit misbehaves (hang / db-locked / resurrected rows), rebuild the
   cache clean instead of syncing on top of it — in this exact order:**

   ```powershell
   # a. Stop stale MCP servers by PID only (never by name)
   Get-Process -Name backlogit -ErrorAction SilentlyContinue   # find the PID
   Stop-Process -Id <pid>                                       # one specific PID

   # b. Delete the gitignored, disposable cache (all three files)
   Remove-Item .backlogit/backlogit.db,.backlogit/backlogit.db-wal,.backlogit/backlogit.db-shm -ErrorAction SilentlyContinue

   # c. NOW sync — the empty cache is rebuilt FROM Markdown, so there is
   #    nothing stale to union back. Markdown is the sole source of truth here.
   backlogit sync
   ```

   Deleting the cache **before** `sync` is what makes `sync` safe: with an empty
   cache there is no divergent state to union back into Markdown.

3. **Never commit `backlogit.db*`.** Confirm they are gitignored
   (`git check-ignore .backlogit/backlogit.db`) — the WAL/SHM sidecars in
   particular must never enter a commit.

## When to Apply

- Before running `backlogit sync`: ask whether the cache might hold a state that
  diverges from what you just changed at the source/Markdown level. If yes,
  delete the cache first (empty-cache rebuild) rather than syncing on top.
- Any time a backlogit write hangs or reports a locked database: check for and
  stop stale `backlogit mcp` PIDs before retrying.
- During Ship/Stage closure work that performs many `move`/`ship`/`update`
  mutations, treat the resulting `index may be stale` lines as expected noise,
  not as a prompt to sync.

## Evidence

- backlogit `--help`: `sync` = "Rehydrate the SQLite index from Markdown source
  files"; workspace stores active work in `.backlogit/queue`, terminal work in
  `.backlogit/archive`, cache in gitignored `.backlogit/backlogit.db*`.
- 053-S closure: `backlogit shipment ship 053-S` and `backlogit move 065-F
  --status active` each emitted `index may be stale after mutation` for every
  touched item — deliberately NOT followed by `sync` to avoid the union.
- `git check-ignore` confirms `.backlogit/backlogit.db`, `-wal`, and `-shm` are
  ignored.
- A `backlogit mcp` process (observed PID during this session) was alive and
  holding the cache open concurrently with CLI mutations.

## Date

2026-07-02 | Shipment 053-S (065-F daemonless --direct docs, post-merge closure)
