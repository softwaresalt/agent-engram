---
id: TASK-008.08
title: '008.08 - Vector/graph database is being stored under AppData instead of ENGRAM_DATA_DIR'
status: Done
type: bug
assignee: []
created_date: '2026-03-24 21:47'
labels:
  - vector
  - graph
  - database
  - location
  - bug
dependencies: []
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Currently, the database file location being used is the AppData folder under the user profile, which is incorrect.
Additionally, the system is still using a hash system, which appears to be duplicating databases for every session.
The database should be consolidated locally under a branch name that matches the current git branch.
<!-- SECTION:DESCRIPTION:END -->
