//! Verify error codes in code match contracts/error-codes.md (T107).

use t_mem::errors::codes::*;
use t_mem::errors::*;

/// Verify all workspace error codes match the contract.
#[test]
fn workspace_error_codes_match_contract() {
    assert_eq!(WORKSPACE_NOT_FOUND, 1001);
    assert_eq!(NOT_A_GIT_ROOT, 1002);
    assert_eq!(WORKSPACE_NOT_SET, 1003);
    assert_eq!(WORKSPACE_ALREADY_ACTIVE, 1004);
    assert_eq!(WORKSPACE_LIMIT_REACHED, 1005);
}

/// Verify all hydration error codes match the contract.
#[test]
fn hydration_error_codes_match_contract() {
    assert_eq!(HYDRATION_FAILED, 2001);
    assert_eq!(SCHEMA_MISMATCH, 2002);
    assert_eq!(CORRUPTED_STATE, 2003);
    assert_eq!(STALE_WORKSPACE, 2004);
}

/// Verify all task error codes match the contract.
#[test]
fn task_error_codes_match_contract() {
    assert_eq!(TASK_NOT_FOUND, 3001);
    assert_eq!(INVALID_STATUS, 3002);
    assert_eq!(CYCLIC_DEPENDENCY, 3003);
    assert_eq!(BLOCKER_EXISTS, 3004);
    assert_eq!(TASK_ALREADY_CLAIMED, 3005);
    assert_eq!(LABEL_VALIDATION, 3006);
    assert_eq!(BATCH_PARTIAL_FAILURE, 3007);
    assert_eq!(COMPACTION_FAILED, 3008);
    assert_eq!(INVALID_PRIORITY, 3009);
    assert_eq!(INVALID_ISSUE_TYPE, 3010);
    assert_eq!(DUPLICATE_LABEL, 3011);
    assert_eq!(TASK_NOT_CLAIMABLE, 3012);
    assert_eq!(TASK_TITLE_EMPTY, 3013);
}

/// Verify all query error codes match the contract.
#[test]
fn query_error_codes_match_contract() {
    assert_eq!(QUERY_TOO_LONG, 4001);
    assert_eq!(MODEL_NOT_LOADED, 4002);
    assert_eq!(SEARCH_FAILED, 4003);
}

/// Verify all system error codes match the contract.
#[test]
fn system_error_codes_match_contract() {
    assert_eq!(DATABASE_ERROR, 5001);
    assert_eq!(FLUSH_FAILED, 5002);
    assert_eq!(RATE_LIMITED, 5003);
    assert_eq!(SHUTTING_DOWN, 5004);
    assert_eq!(INVALID_PARAMS, 5005);
}

/// Verify all config error codes match the contract.
#[test]
fn config_error_codes_match_contract() {
    assert_eq!(CONFIG_PARSE_ERROR, 6001);
    assert_eq!(CONFIG_INVALID_VALUE, 6002);
    assert_eq!(UNKNOWN_CONFIG_KEY, 6003);
}

/// Verify error-to-response mapping produces the correct code for each variant.
#[test]
#[allow(clippy::too_many_lines)]
fn error_response_codes_are_consistent() {
    let cases: Vec<(TMemError, u16, &str)> = vec![
        (
            WorkspaceError::NotFound { path: "x".into() }.into(),
            1001,
            "WorkspaceNotFound",
        ),
        (
            WorkspaceError::NotGitRoot { path: "x".into() }.into(),
            1002,
            "NotAGitRoot",
        ),
        (WorkspaceError::NotSet.into(), 1003, "WorkspaceNotSet"),
        (
            WorkspaceError::AlreadyActive { path: "x".into() }.into(),
            1004,
            "WorkspaceAlreadyActive",
        ),
        (
            WorkspaceError::LimitReached { limit: 10 }.into(),
            1005,
            "WorkspaceLimitReached",
        ),
        (
            HydrationError::Failed { reason: "x".into() }.into(),
            2001,
            "HydrationFailed",
        ),
        (
            HydrationError::SchemaMismatch {
                expected: "a".into(),
                found: "b".into(),
            }
            .into(),
            2002,
            "SchemaMismatch",
        ),
        (
            HydrationError::CorruptedState { reason: "x".into() }.into(),
            2003,
            "CorruptedState",
        ),
        (
            HydrationError::StaleWorkspace.into(),
            2004,
            "StaleWorkspace",
        ),
        (
            TaskError::NotFound { id: "x".into() }.into(),
            3001,
            "TaskNotFound",
        ),
        (
            TaskError::InvalidStatus { status: "x".into() }.into(),
            3002,
            "InvalidStatus",
        ),
        (TaskError::CyclicDependency.into(), 3003, "CyclicDependency"),
        (
            TaskError::BlockerExists { id: "x".into() }.into(),
            3004,
            "BlockerExists",
        ),
        (TaskError::TitleEmpty.into(), 3013, "TaskTitleEmpty"),
        (
            TaskError::AlreadyClaimed {
                id: "x".into(),
                assignee: "a".into(),
            }
            .into(),
            3005,
            "TaskAlreadyClaimed",
        ),
        (
            TaskError::LabelValidation { reason: "x".into() }.into(),
            3006,
            "LabelValidation",
        ),
        (
            TaskError::BatchPartialFailure {
                succeeded: 1,
                failed: 1,
                results: serde_json::json!([]),
            }
            .into(),
            3007,
            "BatchPartialFailure",
        ),
        (
            TaskError::CompactionFailed {
                id: "x".into(),
                reason: "x".into(),
            }
            .into(),
            3008,
            "CompactionFailed",
        ),
        (
            TaskError::InvalidPriority {
                priority: "x".into(),
            }
            .into(),
            3009,
            "InvalidPriority",
        ),
        (
            TaskError::InvalidIssueType {
                issue_type: "x".into(),
            }
            .into(),
            3010,
            "InvalidIssueType",
        ),
        (
            TaskError::DuplicateLabel {
                task_id: "x".into(),
                label: "x".into(),
            }
            .into(),
            3011,
            "DuplicateLabel",
        ),
        (
            TaskError::NotClaimable {
                id: "x".into(),
                status: "done".into(),
            }
            .into(),
            3012,
            "TaskNotClaimable",
        ),
        (QueryError::QueryTooLong.into(), 4001, "QueryTooLong"),
        (QueryError::ModelNotLoaded.into(), 4002, "ModelNotLoaded"),
        (
            QueryError::SearchFailed { reason: "x".into() }.into(),
            4003,
            "SearchFailed",
        ),
        (
            SystemError::DatabaseError { reason: "x".into() }.into(),
            5001,
            "DatabaseError",
        ),
        (
            SystemError::FlushFailed { path: "x".into() }.into(),
            5002,
            "FlushFailed",
        ),
        (SystemError::RateLimited.into(), 5003, "RateLimited"),
        (SystemError::ShuttingDown.into(), 5004, "ShuttingDown"),
        (
            SystemError::InvalidParams { reason: "x".into() }.into(),
            5005,
            "InvalidParams",
        ),
        (
            ConfigError::ParseError { reason: "x".into() }.into(),
            6001,
            "ConfigParseError",
        ),
        (
            ConfigError::InvalidValue {
                key: "k".into(),
                reason: "x".into(),
            }
            .into(),
            6002,
            "ConfigInvalidValue",
        ),
        (
            ConfigError::UnknownKey { key: "k".into() }.into(),
            6003,
            "UnknownConfigKey",
        ),
    ];

    for (err, expected_code, expected_name) in cases {
        let response = err.to_response();
        assert_eq!(
            response.error.code, expected_code,
            "code mismatch for {expected_name}: got {}, expected {expected_code}",
            response.error.code
        );
        assert_eq!(
            response.error.name, expected_name,
            "name mismatch for code {expected_code}: got {}, expected {expected_name}",
            response.error.name
        );
    }
}
