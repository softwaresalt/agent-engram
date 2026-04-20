//! Dual-backend test helper macros.
//!
//! Compile-time-switched assertion macros for writing tests that compile and
//! pass under both `surreal-backend` and `cozo-backend` feature sets.
//!
//! # Usage
//!
//! Include this file in your test binary:
//!
//! ```rust,ignore
//! #[path = "../helpers/dual_backend.rs"]
//! mod dual_backend;
//!
//! // Use the macros directly (exported to the test binary root via #[macro_export]).
//! assert_ok_or_stub!(cg_queries.count_functions().await);
//! assert_empty_count_or_stub!(cg_queries.count_code_files().await);
//! ```
//!
//! # Behaviour by feature
//!
//! | Macro                       | `surreal-backend`         | `cozo-backend`                              |
//! |-----------------------------|---------------------------|---------------------------------------------|
//! | `assert_ok_or_stub!`        | asserts `result.is_ok()`  | asserts stub error contains "not yet implemented" |
//! | `assert_empty_count_or_stub!` | asserts `result == Ok(0)` | asserts stub error contains "not yet implemented" |

/// Assert that a `Result` is `Ok` under `surreal-backend`, or is the CozoDB
/// Phase 2 stub error under `cozo-backend`.
///
/// # Examples
///
/// ```rust,ignore
/// assert_ok_or_stub!(cg_queries.count_functions().await);
/// ```
#[macro_export]
macro_rules! assert_ok_or_stub {
    ($expr:expr) => {{
        let result = $expr;
        #[cfg(feature = "surreal-backend")]
        {
            assert!(
                result.is_ok(),
                "expected Ok under surreal-backend, got: {:?}",
                result
            );
        }
        #[cfg(feature = "cozo-backend")]
        {
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("not yet implemented") || err_msg.contains("Phase 2"),
                "expected cozo stub sentinel ('not yet implemented' or 'Phase 2'), \
                 got: {}",
                err_msg
            );
        }
    }};
}

/// Assert that a count query result is `Ok(0)` under `surreal-backend` (empty
/// DB), or is the CozoDB stub error under `cozo-backend`.
///
/// # Examples
///
/// ```rust,ignore
/// assert_empty_count_or_stub!(cg_queries.count_code_files().await);
/// ```
#[macro_export]
macro_rules! assert_empty_count_or_stub {
    ($expr:expr) => {{
        let result: Result<u64, _> = $expr;
        #[cfg(feature = "surreal-backend")]
        {
            let count = result.expect("count query failed under surreal-backend");
            assert_eq!(
                count, 0,
                "expected empty-DB count of 0 under surreal-backend, got {}",
                count
            );
        }
        #[cfg(feature = "cozo-backend")]
        {
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("not yet implemented") || err_msg.contains("Phase 2"),
                "expected cozo stub sentinel, got: {}",
                err_msg
            );
        }
    }};
}
