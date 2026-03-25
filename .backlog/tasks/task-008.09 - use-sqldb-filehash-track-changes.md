---
id: TASK-008.09
title: '008-09: Track changes to workspace files using file hashes stored with full path in SQL DB'
status: Done
type: task
assignee: []
created_date: '2026-03-24 15:45'
updated_date: '2026-03-24 15:45'
labels:
  - database
  - file
  - change-tracking
milestone: 
dependencies: []
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In order to ensure that the file listener detects changes to files even when the server is not running, file hashes need to be stored for each file tracked by storing the files in the database, either as a key/value pair of filepath and hash value or as a relational schema.  Engram cannot only rely on a file listener; it needs to also capture file changes from an offline state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->

<!-- SECTION:FINAL_SUMMARY:END -->
