---
id: TASK-009.08
title: '009-08: Contract and integration tests for branch isolation'
status: To Do
assignee: []
created_date: '2026-03-22 21:53'
updated_date: '2026-03-26 00:10'
labels:
  - feature
  - 009
  - testing
dependencies: []
references:
  - tests/contract/lifecycle_test.rs
  - tests/integration/multi_workspace_test.rs
parent_task_id: TASK-009
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add contract and integration tests for the branch isolation mechanism that is already implemented via `connect_db(data_dir, branch)` directory structure.\n\n**Contract tests** (`tests/contract/lifecycle_test.rs`):\n- Assert `branch` field exists and is correct in the `set_workspace` response\n- Assert `get_workspace_status` includes branch in response\n\n**Integration tests** (`tests/integration/multi_workspace_test.rs` or new file):\n- Create two temp workspaces with different `.git/HEAD` contents simulating branch switch\n- Call `set_workspace` for each and verify they get separate `data_dir/db/{branch}/` databases\n- Verify hydration works independently per branch\n- Verify data indexed on one branch is not visible from another\n\n**Note**: Existing contract and integration tests already use the `branch` parameter in test fixtures but do not specifically test branch isolation behavior. These tests fill that gap."
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Contract test asserts branch field in workspace binding response from set_workspace
- [ ] #2 Integration test verifies switching branches (different .git/HEAD content) and calling set_workspace connects to different database directory
- [ ] #3 Integration test verifies hydration populates new branch DB from existing JSONL files
- [ ] #4 Integration test verifies old databases from other branches do not interfere
- [ ] #5 cargo test and cargo clippy pass with zero warnings
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Current test coverage**: Existing tests use `branch` as a parameter but don't test isolation. `tests/contract/read_test.rs` uses hardcoded `branch: \"test_ws\"`. `tests/integration/file_tracker_test.rs` derives branch from SHA256 hash. No test verifies that two branches of the same workspace produce separate databases.\n\n**Key assertion**: After `set_workspace` with branch A, index some data, then `set_workspace` with branch B on the same path, verify the data from branch A is not visible from branch B's database."

Harness: `cargo test --test contract_branch_isolation` (C009-03 is RED gate; C009-01, C009-02 are GREEN guards) and `cargo test --test integration_branch_isolation` (all 4 tests I009-01-I009-04 are GREEN guards validating existing connect_db isolation). C009-03 asserts set_workspace on same path with different git HEAD produces a different workspace_id field - fails until workspace_hash includes branch in digest.
<!-- SECTION:NOTES:END -->
