#![forbid(unsafe_code)]

use std::path::Path;

use engram::errors::EngramError;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph::{IndexResult, index_sealed_target};
use engram::services::generations::IndexTarget;

/// Run the supervisor against a sealed indexing target.
///
/// Uses the target generation ID as the branch namespace, the default code
/// graph configuration, and incremental indexing semantics.
///
/// # Errors
///
/// Returns [`EngramError`] when the sealed target cannot be indexed.
pub async fn run_for_target(
    target: &IndexTarget,
    data_dir: &Path,
) -> Result<IndexResult, EngramError> {
    let config = CodeGraphConfig::default();
    run_for_target_with_options(
        target,
        data_dir,
        target.generation_id().as_str(),
        &config,
        false,
    )
    .await
}

/// Run the supervisor against a sealed indexing target with explicit options.
///
/// This is a thin facade over [`engram::services::code_graph::index_sealed_target`].
///
/// # Errors
///
/// Returns [`EngramError`] when the sealed target cannot be indexed.
pub async fn run_for_target_with_options(
    target: &IndexTarget,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
) -> Result<IndexResult, EngramError> {
    index_sealed_target(target, data_dir, branch, config, force).await
}
