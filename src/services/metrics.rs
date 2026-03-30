//! Metrics collection service for tracking tool call token usage.
//!
//! Provides non-blocking event recording via a `tokio::sync::mpsc` channel
//! and summary computation from persisted JSONL files.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::errors::{EngramError, MetricsError};
use crate::models::metrics::{MetricsConfig, MetricsMessage, MetricsSummary, UsageEvent};

const RECENT_EVENTS_LIMIT: usize = 256;

static METRICS_SENDER: OnceLock<Mutex<Option<mpsc::Sender<MetricsMessage>>>> = OnceLock::new();
static METRICS_HANDLE: OnceLock<Mutex<Option<JoinHandle<()>>>> = OnceLock::new();
static RECENT_EVENTS: OnceLock<Mutex<VecDeque<UsageEvent>>> = OnceLock::new();

fn sender_slot() -> &'static Mutex<Option<mpsc::Sender<MetricsMessage>>> {
    METRICS_SENDER.get_or_init(|| Mutex::new(None))
}

fn handle_slot() -> &'static Mutex<Option<JoinHandle<()>>> {
    METRICS_HANDLE.get_or_init(|| Mutex::new(None))
}

fn recent_events_slot() -> &'static Mutex<VecDeque<UsageEvent>> {
    RECENT_EVENTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn metrics_dir(workspace_path: &Path, branch: &str) -> PathBuf {
    workspace_path.join(".engram").join("metrics").join(branch)
}

fn usage_path(workspace_path: &Path, branch: &str) -> PathBuf {
    metrics_dir(workspace_path, branch).join("usage.jsonl")
}

fn summary_path(workspace_path: &Path, branch: &str) -> PathBuf {
    metrics_dir(workspace_path, branch).join("summary.json")
}

fn remember_recent_event(event: UsageEvent) {
    let mut recent_events = recent_events_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if recent_events.len() >= RECENT_EVENTS_LIMIT {
        recent_events.pop_front();
    }
    recent_events.push_back(event);
}

async fn append_event_line(
    workspace_path: &Path,
    branch: &str,
    event: &UsageEvent,
) -> Result<(), EngramError> {
    let directory = metrics_dir(workspace_path, branch);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!(
                    "failed to create metrics directory '{}': {error}",
                    directory.display()
                ),
            })
        })?;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(usage_path(workspace_path, branch))
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("failed to open usage.jsonl for append: {error}"),
            })
        })?;

    let line = serde_json::to_string(event).map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to serialize usage event: {error}"),
        })
    })?;

    file.write_all(line.as_bytes()).await.map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to write usage event: {error}"),
        })
    })?;
    file.write_all(b"\n").await.map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to terminate usage event line: {error}"),
        })
    })?;

    Ok(())
}

#[tracing::instrument(skip(receiver))]
async fn writer_loop(
    workspace_path: PathBuf,
    initial_branch: String,
    mut receiver: mpsc::Receiver<MetricsMessage>,
) {
    let mut active_branch = initial_branch;

    while let Some(message) = receiver.recv().await {
        match message {
            MetricsMessage::Event(event) => {
                let branch = if event.branch.is_empty() {
                    active_branch.as_str()
                } else {
                    event.branch.as_str()
                };
                if let Err(error) = append_event_line(&workspace_path, branch, &event).await {
                    tracing::warn!(error = %error, branch, "failed to persist metrics event");
                }
            }
            MetricsMessage::SwitchBranch(branch) => {
                tracing::info!(branch, "metrics branch switched");
                active_branch = branch;
            }
            MetricsMessage::Shutdown => {
                while let Ok(pending) = receiver.try_recv() {
                    match pending {
                        MetricsMessage::Event(event) => {
                            let branch = if event.branch.is_empty() {
                                active_branch.as_str()
                            } else {
                                event.branch.as_str()
                            };
                            if let Err(error) =
                                append_event_line(&workspace_path, branch, &event).await
                            {
                                tracing::warn!(error = %error, branch, "failed to persist drained metrics event");
                            }
                        }
                        MetricsMessage::SwitchBranch(branch) => {
                            tracing::info!(branch, "metrics branch switched during shutdown");
                            active_branch = branch;
                        }
                        MetricsMessage::Shutdown => {}
                    }
                }
                break;
            }
        }
    }
}

/// Start the background metrics writer for a workspace snapshot.
///
/// Replaces any previously configured writer in-process.
pub async fn initialize(
    workspace_path: &Path,
    branch: &str,
    config: &MetricsConfig,
) -> Result<(), EngramError> {
    shutdown().await?;

    if !config.enabled {
        return Ok(());
    }

    let (sender, receiver) = mpsc::channel(config.buffer_size);
    {
        let mut sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *sender_guard = Some(sender);
    }

    let handle = tokio::spawn(writer_loop(
        workspace_path.to_path_buf(),
        branch.to_owned(),
        receiver,
    ));
    {
        let mut handle_guard = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *handle_guard = Some(handle);
    }

    Ok(())
}

/// Record a usage event to the metrics channel (non-blocking).
///
/// If the channel is full, the event is dropped with a `tracing::trace!`
/// log. This ensures zero latency impact on tool call responses.
pub fn record(event: UsageEvent) {
    remember_recent_event(event.clone());

    let sender = {
        let sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sender_guard.clone()
    };

    if let Some(sender) = sender {
        if let Err(error) = sender.try_send(MetricsMessage::Event(event)) {
            match error {
                mpsc::error::TrySendError::Full(_) => {
                    tracing::trace!("metrics_event_dropped");
                }
                mpsc::error::TrySendError::Closed(_) => {
                    tracing::trace!("metrics_event_dropped_closed");
                }
            }
        }
    }
}

/// Notify the background writer that the active branch changed.
pub fn switch_branch(branch: String) {
    let sender = {
        let sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sender_guard.clone()
    };

    if let Some(sender) = sender {
        let _ = sender.try_send(MetricsMessage::SwitchBranch(branch));
    }
}

/// Return the most recently recorded usage events kept in-memory for inspection.
#[must_use]
pub fn recent_events() -> Vec<UsageEvent> {
    let recent_events = recent_events_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    recent_events.iter().cloned().collect()
}

/// Clear the in-memory recent-event ledger.
pub fn clear_recent_events() {
    let mut recent_events = recent_events_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    recent_events.clear();
}

/// Shut down the background metrics writer, draining queued messages first.
pub async fn shutdown() -> Result<(), EngramError> {
    let sender = {
        let mut sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sender_guard.take()
    };

    if let Some(sender) = sender {
        let _ = sender.send(MetricsMessage::Shutdown).await;
    }

    let handle = {
        let mut handle_guard = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        handle_guard.take()
    };

    if let Some(handle) = handle {
        match handle.await {
            Ok(()) => {}
            Err(error) if error.is_cancelled() => {
                // Task was cancelled by runtime shutdown or test cleanup — not an error.
                tracing::debug!("metrics writer task cancelled during shutdown");
            }
            Err(error) => {
                return Err(EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!("metrics writer task failed to join: {error}"),
                }));
            }
        }
    }

    Ok(())
}

/// Compute a `MetricsSummary` from the `usage.jsonl` file on disk.
///
/// Reads `{workspace_path}/.engram/metrics/{branch}/usage.jsonl` line by
/// line, deserializes each line as a `UsageEvent`, and aggregates into a
/// `MetricsSummary`. Silently discards the final line if it fails to parse
/// (concurrent-append tolerance).
pub fn compute_summary(workspace_path: &Path, branch: &str) -> Result<MetricsSummary, EngramError> {
    let events = load_events(workspace_path, branch)?;
    Ok(MetricsSummary::from_events(&events))
}

/// Load raw usage events for a branch from the `.engram/` data directory.
///
/// # Errors
///
/// Returns [`MetricsError::NotFound`] when no events file exists for the branch.
/// Returns [`MetricsError::ParseError`] when event lines cannot be parsed.
pub fn load_events(workspace_path: &Path, branch: &str) -> Result<Vec<UsageEvent>, EngramError> {
    let usage_path = usage_path(workspace_path, branch);
    let file = std::fs::File::open(&usage_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            EngramError::Metrics(MetricsError::NotFound {
                branch: branch.to_owned(),
            })
        } else {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("failed to open '{}': {error}", usage_path.display()),
            })
        }
    })?;

    let reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut lines = reader.lines().peekable();
    while let Some(line_result) = lines.next() {
        let line = line_result.map_err(|error| {
            EngramError::Metrics(MetricsError::ParseError {
                reason: format!("failed to read '{}': {error}", usage_path.display()),
            })
        })?;

        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<UsageEvent>(&line) {
            Ok(event) => events.push(event),
            Err(error) if lines.peek().is_none() => {
                tracing::debug!(
                    error = %error,
                    path = %usage_path.display(),
                    "discarding trailing partial metrics line"
                );
            }
            Err(error) => {
                return Err(EngramError::Metrics(MetricsError::ParseError {
                    reason: format!("failed to parse '{}': {error}", usage_path.display()),
                }));
            }
        }
    }

    Ok(events)
}

/// Compute and atomically write `summary.json` for a branch.
///
/// Calls [`compute_summary`] then writes the result using
/// `dehydration::atomic_write`.
pub async fn compute_and_write_summary(
    workspace_path: &Path,
    branch: &str,
) -> Result<(), EngramError> {
    let wp = workspace_path.to_path_buf();
    let br = branch.to_owned();
    let summary = tokio::task::spawn_blocking(move || compute_summary(&wp, &br))
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("metrics computation task panicked: {error}"),
            })
        })??;
    let summary_json = serde_json::to_string_pretty(&summary).map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to serialize summary: {error}"),
        })
    })?;

    let directory = metrics_dir(workspace_path, branch);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!(
                    "failed to create summary directory '{}': {error}",
                    directory.display()
                ),
            })
        })?;

    crate::services::dehydration::atomic_write(
        &summary_path(workspace_path, branch),
        &summary_json,
    )
    .await?;

    Ok(())
}
