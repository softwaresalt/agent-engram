---
id: TASK-008.07
title: '008.07 - Re-indexing of the vector store fails with a NaN error (a zero-vector vs real-vector distance bug)'
status: Done
type: bug
assignee: []
created_date: '2026-03-24 21:47'
labels:
  - search
  - vector
  - indexing
  - graph
  - bug
dependencies: []
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Need to identify the source of this issue and resolve so that initial indexing works from first install.
Also, index_workspace does not appear to start up automatically nor does get_daemon_status on initial load of copilot and connection via MCP.
The server should automatically check the daemon status and index status and initialize the database as well as scan for changes.
<!-- SECTION:DESCRIPTION:END -->
