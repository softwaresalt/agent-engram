use std::fs;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::errors::{HydrationError, TMemError};

#[derive(Debug, Clone, Default)]
pub struct HydrationSummary {
    pub task_count: u64,
    pub context_count: u64,
    pub last_flush: Option<String>,
    pub stale_files: bool,
}

/// Load workspace state from `.tmem/` files.
pub async fn hydrate_workspace(path: &Path) -> Result<HydrationSummary, TMemError> {
    let tmem_dir = path.join(".tmem");

    if !tmem_dir.exists() {
        fs::create_dir_all(&tmem_dir).map_err(|e| {
            TMemError::Hydration(HydrationError::Failed {
                reason: format!("failed to create .tmem directory: {e}"),
            })
        })?;
        return Ok(HydrationSummary::default());
    }

    let tasks_path = tmem_dir.join("tasks.md");
    let task_count = if tasks_path.exists() {
        count_tasks(&tasks_path)?
    } else {
        0
    };

    // Context persistence not implemented yet; return zero until formats are defined.
    let context_count = 0;

    let (last_flush, stale_files) = last_flush_state(&tmem_dir, tasks_path.as_path());

    Ok(HydrationSummary {
        task_count,
        context_count,
        last_flush,
        stale_files,
    })
}

fn count_tasks(path: &Path) -> Result<u64, TMemError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        TMemError::Hydration(HydrationError::Failed {
            reason: format!("failed to read tasks.md: {e}"),
        })
    })?;

    let count = contents
        .lines()
        .filter(|line| line.trim_start().starts_with("## task:"))
        .count();

    Ok(count as u64)
}

fn last_flush_state(tmem_dir: &Path, tasks_path: &Path) -> (Option<String>, bool) {
    let last_flush_path = tmem_dir.join(".lastflush");

    let last_flush_str = fs::read_to_string(&last_flush_path).ok();
    let last_flush = last_flush_str
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339());

    let mut stale_files = false;
    if let (Some(flush), Ok(meta)) = (&last_flush, fs::metadata(tasks_path)) {
        if let Ok(modified) = meta.modified() {
            if is_newer(&modified, flush) {
                stale_files = true;
            }
        }
    }

    (last_flush, stale_files)
}

fn is_newer(modified: &SystemTime, flush: &str) -> bool {
    if let Ok(flush_time) = DateTime::parse_from_rfc3339(flush) {
        if let Ok(modified_time) = DateTime::<Utc>::from(*modified) {
            return modified_time > flush_time.with_timezone(&Utc);
        }
    }
    false
}
