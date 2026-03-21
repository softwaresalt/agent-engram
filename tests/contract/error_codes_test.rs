//! Verify error codes in code match contracts/error-codes.md (T107).

use engram::errors::codes::*;
use engram::errors::*;

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

/// Verify all query error codes match the contract.
#[test]
fn query_error_codes_match_contract() {
    assert_eq!(QUERY_TOO_LONG, 4001);
    assert_eq!(MODEL_NOT_LOADED, 4002);
    assert_eq!(SEARCH_FAILED, 4003);
    assert_eq!(QUERY_EMPTY, 4004);
}

/// Verify all system error codes match the contract.
#[test]
fn system_error_codes_match_contract() {
    assert_eq!(DATABASE_ERROR, 5001);
    assert_eq!(FLUSH_FAILED, 5002);
    assert_eq!(RATE_LIMITED, 5003);
    assert_eq!(SHUTTING_DOWN, 5004);
    assert_eq!(INVALID_PARAMS, 5005);
    assert_eq!(MODEL_LOAD_FAILED, 5006);
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
    let cases: Vec<(EngramError, u16, &str)> = vec![
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
        (QueryError::QueryTooLong.into(), 4001, "QueryTooLong"),
        (QueryError::QueryEmpty.into(), 4004, "QueryEmpty"),
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
            SystemError::ModelLoadFailed { reason: "x".into() }.into(),
            5006,
            "ModelLoadFailed",
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

/// T094: Verify the serialized JSON shape of `ErrorResponse` conforms to
/// the v0 error taxonomy — `{ error: { code, name, message, details? } }`.
#[test]
fn t094_error_response_json_shape() {
    let test_cases: Vec<(EngramError, bool)> = vec![
        // Errors WITH details
        (
            WorkspaceError::NotFound {
                path: "/tmp".into(),
            }
            .into(),
            true,
        ),
        (
            ConfigError::InvalidValue {
                key: "k".into(),
                reason: "r".into(),
            }
            .into(),
            true,
        ),
        // Errors WITHOUT details
        (WorkspaceError::NotSet.into(), false),
        (SystemError::RateLimited.into(), false),
    ];

    for (err, has_details) in test_cases {
        let response = err.to_response();
        let json = serde_json::to_value(&response).expect("serialize ErrorResponse");

        // Verify top-level structure
        let error_obj = json.get("error").expect("should have 'error' key");
        assert!(error_obj.is_object(), "error should be an object");
        assert!(
            error_obj.get("code").and_then(|v| v.as_u64()).is_some(),
            "error.code should be a number: {json}"
        );
        assert!(
            error_obj.get("name").and_then(|v| v.as_str()).is_some(),
            "error.name should be a string: {json}"
        );
        assert!(
            error_obj.get("message").and_then(|v| v.as_str()).is_some(),
            "error.message should be a string: {json}"
        );

        // Verify message is non-empty
        let msg = error_obj.get("message").and_then(|v| v.as_str()).unwrap();
        assert!(!msg.is_empty(), "error.message should not be empty: {json}");

        // Verify details presence/absence
        if has_details {
            assert!(
                error_obj.get("details").is_some(),
                "expected details for error: {json}"
            );
        } else {
            // details may be absent (skip_serializing_if) or null
            let details = error_obj.get("details");
            assert!(
                details.is_none() || details.unwrap().is_null(),
                "expected no details for error: {json}"
            );
        }
    }
}