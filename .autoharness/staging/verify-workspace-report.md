# verify-workspace report

- Workspace: `D:\Source\GitHub\agent-engram`
- autoharness_home: `D:\Source\GitHub\agent-engram`
- Staging dir: `D:\Source\GitHub\agent-engram\.autoharness\staging`

## Strict-Schema Blockers

- {"kind": "invalid-profile-yaml", "path": "D:\\Source\\GitHub\\agent-engram\\.autoharness\\workspace-profile.yaml", "message": "mapping values are not allowed here\n  in \"<unicode string>\", line 55, column 48:\n     ... de graph parsing (multi-language: Rust, Python, JS, TS, Go, C#,  ... \n                                         ^"}
- {"kind": "invalid-manifest-schema", "path": "D:\\Source\\GitHub\\agent-engram\\.autoharness\\harness-manifest.yaml", "contract": "harness-manifest", "observed_version": "1.0.0", "current_version": "1.0.0", "message": "schema file is missing: D:\\Source\\GitHub\\agent-engram\\schemas\\harness-manifest.schema.json"}
- {"kind": "invalid-config-schema", "path": "D:\\Source\\GitHub\\agent-engram\\.autoharness\\config.yaml", "contract": "harness-config", "observed_version": "1.0.0", "current_version": "1.0.0", "message": "schema file is missing: D:\\Source\\GitHub\\agent-engram\\schemas\\harness-config.schema.json"}

## Blockers

none

## Warnings

- {"kind": "malformed-artifact-entry", "path": "foundation", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "agents", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "policies", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "prompts", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "instructions", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "skills", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "scripts", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}
- {"kind": "malformed-artifact-entry", "path": "config", "message": "Manifest artifact entry is not an object; expected {path, checksum, template, primitive}. Skipping."}

## Schema Contracts

- harness-manifest: current (observed 1.0.0, current 1.0.0)
- harness-config: current (observed 1.0.0, current 1.0.0)
- workspace-profile: missing-version (observed (missing), current 1.0.0)

## Migration Proposals

none

## Unresolved Placeholders

none

## Targeted Checks

- agent_intercom_instruction: PASS
- review_intercom_workflow: PASS
- agent_engram_instruction: PASS
- backlogit_instruction_guidance: PASS
- backlogit_sql_schema_instruction: PASS
- backlogit_yaml_header_instruction: PASS
- agents_metadata_catalog_guidance: FAIL
  missing: backlogit_get_metadata_catalog, backlogit_export_command_map
- ship_source_artifact_cleanup: FAIL
  missing: source_deliberation_id, backlogit_stash_remove, backlogit_archive_item
- closure_source_artifact_cleanup: FAIL
  missing: Source artifact cleanup, source_stash_id, source_deliberation_id
- strict_safety_instruction: PASS
- release_observability_instruction: PASS
- continuous_learning_instruction: PASS
- adversarial_review_instruction: PASS
- copilot_durable_knowledge_layout: FAIL
  missing: Reusable learnings and hard-won fixes, Session memory and checkpoints, Graduated architecture and design rationale
- copilot_session_memory_guidance: FAIL
  missing: 65%, phase or major task group
- copilot_remote_operator_guidance: FAIL
  missing: ## Remote Operator Integration, ping-loop.prompt.md
- copilot_backlog_workflow_expectations: FAIL
  missing: queue-aware and dependency-aware operations, commit-tracking, parallel markdown trackers
- stage_shipment_determinism: FAIL
  missing: Never skip shipment assembly
- ship_branch_management: PASS
- pr_lifecycle_branch_retention: PASS
