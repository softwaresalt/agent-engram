#![allow(dead_code)]

use std::path::Path;

use crate::errors::TMemError;

/// Placeholder hydration that will load workspace state from .tmem/ files and SurrealDB.
/// Currently returns zero counts until persistence is implemented.
pub async fn hydrate_workspace(_path: &Path) -> Result<(u64, u64), TMemError> {
    Ok((0, 0))
}
