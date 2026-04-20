//! Unit tests for `CozoDB` schema bootstrap (Task 001.003.002-T — U2.1).
//!
//! Verifies that `run_schema_bootstrap` succeeds on a fresh in-memory DB
//! and that all `CozoScript` relation-creation constants are non-empty strings.
//!
//! These tests are gated on the `cozo-backend` feature because they depend
//! on types and modules that are only compiled under that feature.

#[cfg(feature = "cozo-backend")]
mod schema_tests {
    use engram::db::cozo_backend::{CozoHandle, schema};

    /// Phase 2 U2.1: bootstrap must complete without error on a fresh handle.
    ///
    /// Fails until `run_schema_bootstrap` is implemented.
    #[test]
    fn schema_bootstrap_succeeds_on_fresh_handle() {
        let handle = CozoHandle;
        let result = schema::run_schema_bootstrap(&handle);
        assert!(result.is_ok(), "schema bootstrap failed: {result:?}");
    }

    /// Each `:create` constant must be a non-empty `CozoScript` string.
    #[test]
    fn file_node_create_script_is_populated() {
        assert!(
            !schema::CREATE_FILE_NODE.is_empty(),
            "CREATE_FILE_NODE must contain CozoScript"
        );
    }

    #[test]
    fn function_meta_create_script_is_populated() {
        assert!(
            !schema::CREATE_FUNCTION_META.is_empty(),
            "CREATE_FUNCTION_META must contain CozoScript"
        );
    }

    #[test]
    fn function_code_create_script_is_populated() {
        assert!(
            !schema::CREATE_FUNCTION_CODE.is_empty(),
            "CREATE_FUNCTION_CODE must contain CozoScript"
        );
    }

    #[test]
    fn function_embedding_create_script_is_populated() {
        assert!(
            !schema::CREATE_FUNCTION_EMBEDDING.is_empty(),
            "CREATE_FUNCTION_EMBEDDING must contain CozoScript"
        );
    }

    #[test]
    fn class_meta_create_script_is_populated() {
        assert!(
            !schema::CREATE_CLASS_META.is_empty(),
            "CREATE_CLASS_META must contain CozoScript"
        );
    }

    #[test]
    fn interface_meta_create_script_is_populated() {
        assert!(
            !schema::CREATE_INTERFACE_META.is_empty(),
            "CREATE_INTERFACE_META must contain CozoScript"
        );
    }

    #[test]
    fn commit_node_create_script_is_populated() {
        assert!(
            !schema::CREATE_COMMIT_NODE.is_empty(),
            "CREATE_COMMIT_NODE must contain CozoScript"
        );
    }
}
