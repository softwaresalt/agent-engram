//! Metrics collection service for tracking tool call token usage.
//!
//! Provides non-blocking event recording via a `tokio::sync::mpsc` channel
//! and summary computation from persisted JSONL files.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::errors::{EngramError, MetricsError};
use crate::models::metrics::{MetricsConfig, MetricsSummary, UsageEvent};

const RECENT_EVENTS_LIMIT: usize = 256;
#[cfg(not(test))]
const BRANCH_CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const BRANCH_CONTROL_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
enum MetricsMessage {
    Event(Box<UsageEvent>),
    SwitchBranch {
        branch: String,
        generation: u64,
        acknowledged: tokio::sync::oneshot::Sender<()>,
    },
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WriterIdentity {
    workspace_path: PathBuf,
    branch: String,
}

#[derive(Debug, Default)]
enum WriterAvailability {
    #[default]
    Unavailable,
    Disabled(WriterIdentity),
    Enabled(WriterIdentity),
}

#[derive(Debug, Default)]
struct WriterState {
    generation: u64,
    availability: WriterAvailability,
}

#[derive(Clone, Debug)]
pub(crate) struct WriterControlToken {
    generation: u64,
    expected_writer: Option<WriterIdentity>,
}

#[derive(Clone, Debug)]
struct WriterTaskGuard {
    handle: Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl WriterTaskGuard {
    fn new(handle: JoinHandle<()>) -> Self {
        Self {
            handle: Arc::new(tokio::sync::Mutex::new(Some(handle))),
        }
    }

    async fn join(&self) -> Result<(), tokio::task::JoinError> {
        let mut handle_guard = self.handle.lock().await;
        let Some(handle) = handle_guard.as_mut() else {
            return Ok(());
        };
        let result = handle.await;
        let _ = handle_guard.take();
        result
    }

    async fn abort_and_join(&self) -> Result<(), tokio::task::JoinError> {
        let mut handle_guard = self.handle.lock().await;
        let Some(handle) = handle_guard.as_mut() else {
            return Ok(());
        };
        handle.abort();
        let result = handle.await;
        let _ = handle_guard.take();
        result
    }

    fn owns_same_task(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.handle, &other.handle)
    }
}

static METRICS_SENDER: OnceLock<Mutex<Option<mpsc::Sender<MetricsMessage>>>> = OnceLock::new();
static METRICS_HANDLE: OnceLock<Mutex<Option<WriterTaskGuard>>> = OnceLock::new();
static RECENT_EVENTS: OnceLock<Mutex<VecDeque<UsageEvent>>> = OnceLock::new();
static WRITER_STATE: OnceLock<Mutex<WriterState>> = OnceLock::new();
static METRICS_LIFECYCLE: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static BRANCH_CONTROL: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
#[cfg(test)]
static METRICS_TEST: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
#[cfg(test)]
static ACTIVE_TEST_WRITERS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static MAX_ACTIVE_TEST_WRITERS: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static SHUTDOWN_BEFORE_JOIN_PROBE: OnceLock<Mutex<Option<ShutdownBeforeJoinProbe>>> =
    OnceLock::new();
#[cfg(test)]
static FAIL_NEXT_INITIALIZE_AFTER_SHUTDOWN: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
struct ShutdownBeforeJoinProbe {
    reached: tokio::sync::oneshot::Sender<()>,
    resume: tokio::sync::oneshot::Receiver<()>,
}

#[cfg(test)]
struct TestWriterActivity;

#[cfg(test)]
impl TestWriterActivity {
    fn enter() -> Self {
        let active = ACTIVE_TEST_WRITERS.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_ACTIVE_TEST_WRITERS.fetch_max(active, Ordering::SeqCst);
        Self
    }
}

#[cfg(test)]
impl Drop for TestWriterActivity {
    fn drop(&mut self) {
        ACTIVE_TEST_WRITERS.fetch_sub(1, Ordering::SeqCst);
    }
}

fn sender_slot() -> &'static Mutex<Option<mpsc::Sender<MetricsMessage>>> {
    METRICS_SENDER.get_or_init(|| Mutex::new(None))
}

fn handle_slot() -> &'static Mutex<Option<WriterTaskGuard>> {
    METRICS_HANDLE.get_or_init(|| Mutex::new(None))
}

fn recent_events_slot() -> &'static Mutex<VecDeque<UsageEvent>> {
    RECENT_EVENTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn writer_state_slot() -> &'static Mutex<WriterState> {
    WRITER_STATE.get_or_init(|| Mutex::new(WriterState::default()))
}

fn lifecycle_lock() -> &'static tokio::sync::Mutex<()> {
    METRICS_LIFECYCLE.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn branch_control_lock() -> &'static tokio::sync::Mutex<()> {
    BRANCH_CONTROL.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn lock_with_timeout(
    lock: &'static tokio::sync::Mutex<()>,
    name: &str,
) -> Result<tokio::sync::MutexGuard<'static, ()>, EngramError> {
    tokio::time::timeout(BRANCH_CONTROL_TIMEOUT, lock.lock())
        .await
        .map_err(|_| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!(
                    "metrics {name} lock timed out after {} ms",
                    BRANCH_CONTROL_TIMEOUT.as_millis()
                ),
            })
        })
}

fn next_generation(generation: u64) -> Result<u64, EngramError> {
    generation.checked_add(1).ok_or_else(|| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: "metrics writer generation exhausted".to_owned(),
        })
    })
}

fn make_writer_unavailable() -> Result<(), EngramError> {
    let mut state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.availability = WriterAvailability::Unavailable;
    state.generation = next_generation(state.generation)?;
    Ok(())
}

pub(crate) fn mark_writer_unavailable() -> Result<(), EngramError> {
    make_writer_unavailable()
}

fn reserve_writer_generation() -> Result<u64, EngramError> {
    let state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    next_generation(state.generation)
}

fn configure_writer(workspace_path: &Path, branch: &str, enabled: bool, generation: u64) {
    let mut state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let identity = WriterIdentity {
        workspace_path: workspace_path.to_path_buf(),
        branch: branch.to_owned(),
    };
    state.generation = generation;
    state.availability = if enabled {
        WriterAvailability::Enabled(identity)
    } else {
        WriterAvailability::Disabled(identity)
    };
}

fn acknowledge_writer_branch(generation: u64, branch: &str) {
    let mut state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.generation != generation {
        return;
    }
    if let WriterAvailability::Enabled(identity) = &mut state.availability {
        branch.clone_into(&mut identity.branch);
    }
}

fn writer_changed_error() -> EngramError {
    EngramError::Metrics(MetricsError::WriteFailed {
        reason: "metrics writer changed while branch control was pending".to_owned(),
    })
}

fn writer_unavailable_error() -> EngramError {
    EngramError::Metrics(MetricsError::WriteFailed {
        reason: "metrics writer is unavailable for branch control".to_owned(),
    })
}

pub(crate) fn writer_control_token(
    workspace_path: &Path,
    branch: &str,
) -> Result<WriterControlToken, EngramError> {
    let state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let expected_writer = WriterIdentity {
        workspace_path: workspace_path.to_path_buf(),
        branch: branch.to_owned(),
    };
    match &state.availability {
        WriterAvailability::Enabled(identity) | WriterAvailability::Disabled(identity)
            if identity != &expected_writer =>
        {
            Err(writer_changed_error())
        }
        WriterAvailability::Enabled(_) | WriterAvailability::Disabled(_) => {
            Ok(WriterControlToken {
                generation: state.generation,
                expected_writer: Some(expected_writer),
            })
        }
        WriterAvailability::Unavailable => Err(writer_unavailable_error()),
    }
}

fn current_writer_control_token() -> WriterControlToken {
    let state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let expected_writer = match &state.availability {
        WriterAvailability::Enabled(identity) | WriterAvailability::Disabled(identity) => {
            Some(identity.clone())
        }
        WriterAvailability::Unavailable => None,
    };
    WriterControlToken {
        generation: state.generation,
        expected_writer,
    }
}

#[cfg(test)]
pub(crate) async fn test_writer_guard() -> tokio::sync::MutexGuard<'static, ()> {
    METRICS_TEST
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

#[cfg(test)]
pub(crate) fn reset_test_writer_activity_peak() {
    MAX_ACTIVE_TEST_WRITERS.store(ACTIVE_TEST_WRITERS.load(Ordering::SeqCst), Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) fn max_test_writer_activity() -> usize {
    MAX_ACTIVE_TEST_WRITERS.load(Ordering::SeqCst)
}

#[cfg(test)]
pub(crate) fn pause_next_shutdown_before_join() -> (
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Sender<()>,
) {
    let (reached, wait_until_reached) = tokio::sync::oneshot::channel();
    let (resume, wait_until_resumed) = tokio::sync::oneshot::channel();
    let mut probe = SHUTDOWN_BEFORE_JOIN_PROBE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(
        probe.is_none(),
        "shutdown-before-join probe already installed"
    );
    *probe = Some(ShutdownBeforeJoinProbe {
        reached,
        resume: wait_until_resumed,
    });
    (wait_until_reached, resume)
}

#[cfg(test)]
pub(crate) fn fail_next_initialize_after_shutdown() {
    assert!(
        !FAIL_NEXT_INITIALIZE_AFTER_SHUTDOWN.swap(true, Ordering::SeqCst),
        "initialize-after-shutdown failure already installed"
    );
}

#[cfg(test)]
async fn pause_shutdown_before_join() {
    let probe = SHUTDOWN_BEFORE_JOIN_PROBE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(probe) = probe {
        let _ = probe.reached.send(());
        let _ = probe.resume.await;
    }
}

#[cfg(test)]
pub(crate) async fn configure_test_disabled_writer(
    _guard: &tokio::sync::MutexGuard<'static, ()>,
    workspace_path: &Path,
    branch: &str,
) -> Result<(), EngramError> {
    initialize(
        workspace_path,
        branch,
        &MetricsConfig {
            enabled: false,
            ..MetricsConfig::default()
        },
    )
    .await
}

#[cfg(test)]
pub(crate) struct SaturatedTestWriter {
    receiver: mpsc::Receiver<MetricsMessage>,
    pending_acknowledgment: Option<tokio::sync::oneshot::Sender<()>>,
}

#[cfg(test)]
impl SaturatedTestWriter {
    pub(crate) async fn wait_for_pending_branch_control(
        &mut self,
        expected_branch: &str,
    ) -> Result<(), EngramError> {
        if !matches!(self.receiver.recv().await, Some(MetricsMessage::Event(_))) {
            return Err(EngramError::Metrics(MetricsError::WriteFailed {
                reason: "saturated test writer did not contain its blocking event".to_owned(),
            }));
        }
        match self.receiver.recv().await {
            Some(MetricsMessage::SwitchBranch {
                branch,
                acknowledged,
                ..
            }) if branch == expected_branch => {
                self.pending_acknowledgment = Some(acknowledged);
                Ok(())
            }
            Some(MetricsMessage::SwitchBranch { branch, .. }) => {
                Err(EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!(
                        "saturated test writer received branch '{branch}', expected \
                         '{expected_branch}'"
                    ),
                }))
            }
            Some(MetricsMessage::Event(_) | MetricsMessage::Shutdown) | None => {
                Err(EngramError::Metrics(MetricsError::WriteFailed {
                    reason: "saturated test writer did not receive pending branch control"
                        .to_owned(),
                }))
            }
        }
    }
}

#[cfg(test)]
impl Drop for SaturatedTestWriter {
    fn drop(&mut self) {
        let _ = self.pending_acknowledgment.take();
        self.receiver.close();
    }
}

#[cfg(test)]
pub(crate) fn configure_test_saturated_writer(
    _guard: &tokio::sync::MutexGuard<'static, ()>,
    workspace_path: &Path,
    branch: &str,
) -> Result<SaturatedTestWriter, EngramError> {
    let generation = reserve_writer_generation()?;
    let (sender, receiver) = mpsc::channel(1);
    sender
        .try_send(MetricsMessage::Event(Box::default()))
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("failed to saturate test metrics writer: {error}"),
            })
        })?;
    {
        let mut sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *sender_guard = Some(sender);
    }
    configure_writer(workspace_path, branch, true, generation);
    Ok(SaturatedTestWriter {
        receiver,
        pending_acknowledgment: None,
    })
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

/// Resolve the target `usage.jsonl` path for a workspace/branch, honoring an
/// optional `usage_path_override` from configuration.
///
/// The override may be absolute or relative; relative overrides are joined to
/// `workspace_path`. In both cases the resolved path is lexically normalized and
/// rejected when it escapes the workspace root. Containment is a lexical
/// `starts_with` check that does not resolve symlinks: it defends against `..`
/// and absolute-path escapes, but a symlink placed inside the workspace could
/// still redirect writes outside the root.
///
/// # Errors
///
/// Returns [`MetricsError::WriteFailed`] when the override escapes the workspace
/// root.
pub fn resolve_usage_path(
    workspace_path: &Path,
    branch: &str,
    config: &MetricsConfig,
) -> Result<PathBuf, EngramError> {
    let Some(raw_override) = config.usage_path_override.as_deref() else {
        return Ok(usage_path(workspace_path, branch));
    };
    let raw_override = raw_override.trim();
    if raw_override.is_empty() {
        return Ok(usage_path(workspace_path, branch));
    }

    let over = Path::new(raw_override);
    let candidate = if over.is_absolute() {
        over.to_path_buf()
    } else {
        workspace_path.join(over)
    };

    let normalized = normalize_lexical(&candidate);
    let root = normalize_lexical(workspace_path);
    if !normalized.starts_with(&root) {
        return Err(EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!(
                "usage_path_override '{raw_override}' escapes the workspace root '{}'",
                workspace_path.display()
            ),
        }));
    }

    Ok(normalized)
}

/// Lexically normalize a path by resolving `.`/`..` components without touching
/// the filesystem. A `..` that cannot pop is retained so an escaping relative
/// path fails a subsequent containment check rather than silently resolving.
fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Compute the rotated filename for generation `n` (1-based). For a base of
/// `usage.jsonl`, generation `1` is `usage.1.jsonl`.
fn rotated_path(base: &Path, n: usize) -> PathBuf {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("usage");
    let name = match base.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{stem}.{n}.{ext}"),
        None => format!("{stem}.{n}"),
    };
    base.with_file_name(name)
}

async fn path_exists(path: &Path) -> bool {
    tokio::fs::try_exists(path).await.unwrap_or(false)
}

/// Rotate the usage file: `usage.jsonl` → `usage.1.jsonl`, existing
/// `usage.N.jsonl` → `usage.(N+1).jsonl`, dropping generations beyond
/// `max_rotated_files`. When `max_rotated_files` is `0`, no history is retained
/// and the current file is removed.
async fn rotate_usage_file(base: &Path, max_rotated_files: usize) -> Result<(), EngramError> {
    if max_rotated_files == 0 {
        if path_exists(base).await {
            tokio::fs::remove_file(base).await.map_err(|error| {
                EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!("failed to drop usage file during rotation: {error}"),
                })
            })?;
        }
        return Ok(());
    }

    // Drop the oldest retained generation plus any stale generations left at or
    // above the current retention (e.g. produced by a previously higher
    // `max_rotated_files`), so history stays bounded even after the cap is
    // lowered.
    let mut stale = max_rotated_files;
    while path_exists(&rotated_path(base, stale)).await {
        tokio::fs::remove_file(rotated_path(base, stale))
            .await
            .map_err(|error| {
                EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!("failed to prune rotated usage file: {error}"),
                })
            })?;
        stale += 1;
    }

    // Shift remaining generations up by one (highest first to avoid clobber).
    for n in (1..max_rotated_files).rev() {
        let from = rotated_path(base, n);
        if path_exists(&from).await {
            let to = rotated_path(base, n + 1);
            tokio::fs::rename(&from, &to).await.map_err(|error| {
                EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!("failed to shift rotated usage file: {error}"),
                })
            })?;
        }
    }

    // Move the live file to generation 1.
    if path_exists(base).await {
        tokio::fs::rename(base, rotated_path(base, 1))
            .await
            .map_err(|error| {
                EngramError::Metrics(MetricsError::WriteFailed {
                    reason: format!("failed to rotate live usage file: {error}"),
                })
            })?;
    }

    Ok(())
}

/// Append one serialized [`UsageEvent`] line to `usage_path`, rotating first
/// when the existing file has reached `max_file_bytes` (`0` disables size-cap
/// rotation). The parent directory is created as needed. The serialized line and
/// its terminator are written in a single append to preserve JSONL line
/// integrity.
///
/// # Errors
///
/// Returns [`MetricsError::WriteFailed`] on directory, rotation, serialization,
/// or write failure.
pub async fn append_usage_line(
    usage_path: &Path,
    event: &UsageEvent,
    max_file_bytes: u64,
    max_rotated_files: usize,
) -> Result<(), EngramError> {
    if let Some(parent) = usage_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!(
                    "failed to create metrics directory '{}': {error}",
                    parent.display()
                ),
            })
        })?;
    }

    if max_file_bytes > 0 {
        if let Ok(meta) = tokio::fs::metadata(usage_path).await {
            if meta.len() >= max_file_bytes {
                rotate_usage_file(usage_path, max_rotated_files).await?;
            }
        }
    }

    let line = serde_json::to_string(event).map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to serialize usage event: {error}"),
        })
    })?;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(usage_path)
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("failed to open usage.jsonl for append: {error}"),
            })
        })?;

    let mut buffer = line.into_bytes();
    buffer.push(b'\n');
    file.write_all(&buffer).await.map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to write usage event: {error}"),
        })
    })?;

    // Drain the tokio file's pending write to the OS before returning.
    // `write_all` alone does not guarantee the bytes have landed; a subsequent
    // rotation `rename` could otherwise run before the write completes and drop
    // a just-recorded line.
    file.flush().await.map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("failed to flush usage event: {error}"),
        })
    })?;

    Ok(())
}

async fn append_event_line(
    workspace_path: &Path,
    branch: &str,
    event: &UsageEvent,
    config: &MetricsConfig,
) -> Result<(), EngramError> {
    let target = resolve_usage_path(workspace_path, branch, config)?;
    append_usage_line(
        &target,
        event,
        config.max_file_bytes,
        config.max_rotated_files,
    )
    .await
}

#[tracing::instrument(skip(receiver, config))]
async fn writer_loop(
    workspace_path: PathBuf,
    initial_branch: String,
    mut receiver: mpsc::Receiver<MetricsMessage>,
    config: MetricsConfig,
) {
    #[cfg(test)]
    let _activity = TestWriterActivity::enter();
    let mut active_branch = initial_branch;

    while let Some(message) = receiver.recv().await {
        match message {
            MetricsMessage::Event(event) => {
                let event = *event;
                let branch = if event.branch.is_empty() {
                    active_branch.as_str()
                } else {
                    event.branch.as_str()
                };
                if let Err(error) =
                    append_event_line(&workspace_path, branch, &event, &config).await
                {
                    tracing::warn!(error = %error, branch, "failed to persist metrics event");
                }
            }
            MetricsMessage::SwitchBranch {
                branch,
                generation,
                acknowledged,
            } => {
                tracing::info!(branch, "metrics branch switched");
                active_branch = branch;
                acknowledge_writer_branch(generation, &active_branch);
                let _ = acknowledged.send(());
            }
            MetricsMessage::Shutdown => {
                while let Ok(pending) = receiver.try_recv() {
                    match pending {
                        MetricsMessage::Event(event) => {
                            let event = *event;
                            let branch = if event.branch.is_empty() {
                                active_branch.as_str()
                            } else {
                                event.branch.as_str()
                            };
                            if let Err(error) =
                                append_event_line(&workspace_path, branch, &event, &config).await
                            {
                                tracing::warn!(error = %error, branch, "failed to persist drained metrics event");
                            }
                        }
                        MetricsMessage::SwitchBranch {
                            branch,
                            generation,
                            acknowledged,
                        } => {
                            tracing::info!(branch, "metrics branch switched during shutdown");
                            active_branch = branch;
                            acknowledge_writer_branch(generation, &active_branch);
                            let _ = acknowledged.send(());
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
    let _lifecycle = lock_with_timeout(lifecycle_lock(), "lifecycle").await?;
    let _branch_control = lock_with_timeout(branch_control_lock(), "branch-control").await?;
    shutdown_inner().await?;
    #[cfg(test)]
    if FAIL_NEXT_INITIALIZE_AFTER_SHUTDOWN.swap(false, Ordering::SeqCst) {
        return Err(EngramError::Metrics(MetricsError::WriteFailed {
            reason: "injected initialize failure after predecessor shutdown".to_owned(),
        }));
    }

    if !config.enabled {
        let generation = reserve_writer_generation()?;
        configure_writer(workspace_path, branch, false, generation);
        return Ok(());
    }

    let generation = reserve_writer_generation()?;
    // Clamp to >= 1: `tokio::sync::mpsc::channel` panics on a zero buffer.
    // `validate_config` also rejects `buffer_size == 0`, so this is
    // defense-in-depth against a config that bypasses validation.
    let (sender, receiver) = mpsc::channel(config.buffer_size.max(1));
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
        config.clone(),
    ));
    {
        let mut handle_guard = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *handle_guard = Some(WriterTaskGuard::new(handle));
    }
    configure_writer(workspace_path, branch, true, generation);

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
        if let Err(error) = sender.try_send(MetricsMessage::Event(Box::new(event))) {
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
///
/// Returns only after the writer has adopted the branch. Metrics-disabled
/// operation is a no-op; a closed writer is reported explicitly.
pub async fn switch_branch(branch: String) -> Result<(), EngramError> {
    let mut control = current_writer_control_token();
    switch_branch_for(&mut control, branch).await
}

pub(crate) async fn switch_branch_for(
    control: &mut WriterControlToken,
    branch: String,
) -> Result<(), EngramError> {
    tokio::time::timeout(BRANCH_CONTROL_TIMEOUT, switch_branch_inner(control, branch))
        .await
        .map_err(|_| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!(
                    "metrics branch control timed out after {} ms",
                    BRANCH_CONTROL_TIMEOUT.as_millis()
                ),
            })
        })?
}

async fn switch_branch_inner(
    control: &mut WriterControlToken,
    branch: String,
) -> Result<(), EngramError> {
    let _control = branch_control_lock().lock().await;
    {
        let mut state = writer_state_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.generation != control.generation {
            return Err(writer_changed_error());
        }
        match &mut state.availability {
            WriterAvailability::Unavailable => return Err(writer_unavailable_error()),
            WriterAvailability::Disabled(identity) => {
                if control.expected_writer.as_ref() != Some(identity) {
                    return Err(writer_changed_error());
                }
                identity.branch.clone_from(&branch);
                control.expected_writer = Some(identity.clone());
                return Ok(());
            }
            WriterAvailability::Enabled(identity) => {
                if control.expected_writer.as_ref() != Some(identity) {
                    return Err(writer_changed_error());
                }
            }
        }
    }
    let sender = {
        let sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sender_guard.clone()
    };

    let Some(sender) = sender else {
        return Err(EngramError::Metrics(MetricsError::WriteFailed {
            reason: "metrics enabled but branch-control writer is unavailable".to_owned(),
        }));
    };
    if sender.is_closed() {
        return Err(EngramError::Metrics(MetricsError::WriteFailed {
            reason: "metrics enabled but branch-control writer is closed".to_owned(),
        }));
    }
    if control
        .expected_writer
        .as_ref()
        .is_some_and(|identity| identity.branch == branch)
    {
        return Ok(());
    }
    let (acknowledged, acknowledgment) = tokio::sync::oneshot::channel();
    sender
        .send(MetricsMessage::SwitchBranch {
            branch: branch.clone(),
            generation: control.generation,
            acknowledged,
        })
        .await
        .map_err(|error| {
            EngramError::Metrics(MetricsError::WriteFailed {
                reason: format!("metrics branch-control channel closed: {error}"),
            })
        })?;
    acknowledgment.await.map_err(|error| {
        EngramError::Metrics(MetricsError::WriteFailed {
            reason: format!("metrics branch-control acknowledgment dropped: {error}"),
        })
    })?;
    let state = writer_state_slot()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.generation != control.generation {
        return Err(writer_changed_error());
    }
    let WriterAvailability::Enabled(identity) = &state.availability else {
        return Err(writer_changed_error());
    };
    if identity.workspace_path
        != control
            .expected_writer
            .as_ref()
            .ok_or_else(writer_changed_error)?
            .workspace_path
        || identity.branch != branch
    {
        return Err(writer_changed_error());
    }
    control.expected_writer = Some(identity.clone());
    Ok(())
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
    let _lifecycle = lock_with_timeout(lifecycle_lock(), "lifecycle").await?;
    let _branch_control = lock_with_timeout(branch_control_lock(), "branch-control").await?;
    shutdown_inner().await
}

async fn shutdown_inner() -> Result<(), EngramError> {
    let state_error = make_writer_unavailable().err();
    let sender = {
        let mut sender_guard = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sender_guard.take()
    };

    let mut shutdown_error = None;
    if let Some(sender) = sender {
        match tokio::time::timeout(
            BRANCH_CONTROL_TIMEOUT,
            sender.send(MetricsMessage::Shutdown),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::debug!(%error, "metrics writer channel already closed during shutdown");
            }
            Err(_) => {
                shutdown_error = Some(format!(
                    "metrics shutdown send timed out after {} ms",
                    BRANCH_CONTROL_TIMEOUT.as_millis()
                ));
            }
        }
    }

    let handle = {
        let handle_guard = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        handle_guard.clone()
    };
    #[cfg(test)]
    pause_shutdown_before_join().await;

    if let Some(handle) = handle {
        match tokio::time::timeout(BRANCH_CONTROL_TIMEOUT, handle.join()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) if error.is_cancelled() => {
                tracing::debug!("metrics writer task cancelled during shutdown");
            }
            Ok(Err(error)) => {
                shutdown_error = Some(format!("metrics writer task failed to join: {error}"));
            }
            Err(_) => {
                let _ = handle.abort_and_join().await;
                shutdown_error = Some(format!(
                    "metrics writer shutdown timed out after {} ms",
                    BRANCH_CONTROL_TIMEOUT.as_millis()
                ));
            }
        }

        let mut handle_guard = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if handle_guard
            .as_ref()
            .is_some_and(|current| current.owns_same_task(&handle))
        {
            handle_guard.take();
        }
    }

    if let Some(error) = state_error {
        Err(error)
    } else if let Some(reason) = shutdown_error {
        Err(EngramError::Metrics(MetricsError::WriteFailed { reason }))
    } else {
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct DropNotice(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropNotice {
        fn drop(&mut self) {
            if let Some(notice) = self.0.take() {
                let _ = notice.send(());
            }
        }
    }

    fn empty_branch_event(tool_name: &str) -> UsageEvent {
        UsageEvent {
            tool_name: tool_name.to_owned(),
            timestamp: "2026-08-08T00:00:00Z".to_owned(),
            branch: String::new(),
            ..UsageEvent::default()
        }
    }

    fn force_enabled_writer(workspace_path: &Path, branch: &str) {
        let mut state = writer_state_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.availability = WriterAvailability::Enabled(WriterIdentity {
            workspace_path: workspace_path.to_path_buf(),
            branch: branch.to_owned(),
        });
    }

    #[tokio::test]
    async fn full_channel_branch_switch_is_acknowledged_before_following_event() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        let workspace = tempfile::tempdir().expect("metrics workspace");
        let config = MetricsConfig {
            buffer_size: 1,
            ..MetricsConfig::default()
        };
        let (sender, receiver) = mpsc::channel(1);
        {
            let mut sender_guard = sender_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *sender_guard = Some(sender.clone());
        }
        force_enabled_writer(workspace.path(), "main");
        sender
            .try_send(MetricsMessage::Event(Box::new(empty_branch_event(
                "before_switch",
            ))))
            .expect("fill the event channel");

        let mut switch =
            tokio::spawn(async { switch_branch("feature__acknowledged".to_owned()).await });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut switch)
                .await
                .is_err(),
            "branch control must wait rather than drop when the channel is full"
        );

        let handle = tokio::spawn(writer_loop(
            workspace.path().to_path_buf(),
            "main".to_owned(),
            receiver,
            config,
        ));
        {
            let mut handle_guard = handle_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *handle_guard = Some(WriterTaskGuard::new(handle));
        }

        switch
            .await
            .expect("switch task must join")
            .expect("writer must acknowledge branch control");
        record(empty_branch_event("after_switch"));
        shutdown().await.expect("drain metrics writer");

        let main = load_events(workspace.path(), "main").expect("main branch events");
        let switched =
            load_events(workspace.path(), "feature__acknowledged").expect("switched branch events");
        assert_eq!(main.len(), 1);
        assert_eq!(main[0].tool_name, "before_switch");
        assert_eq!(switched.len(), 1);
        assert_eq!(switched[0].tool_name, "after_switch");
    }

    #[tokio::test]
    async fn disabled_metrics_branch_switch_is_a_no_op() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        let workspace = tempfile::tempdir().expect("metrics workspace");
        let config = MetricsConfig {
            enabled: false,
            ..MetricsConfig::default()
        };
        initialize(workspace.path(), "main", &config)
            .await
            .expect("disable metrics");
        switch_branch("feature__disabled".to_owned())
            .await
            .expect("disabled metrics must not require a writer");
        shutdown().await.expect("reset disabled metrics");
    }

    #[tokio::test]
    async fn unavailable_metrics_rejects_branch_control() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");

        let switch_error = tokio::time::timeout(
            BRANCH_CONTROL_TIMEOUT,
            switch_branch("feature__unavailable".to_owned()),
        )
        .await
        .expect("unavailable branch control must remain bounded")
        .expect_err("unavailable metrics must reject branch control");
        assert!(
            matches!(
                &switch_error,
                EngramError::Metrics(MetricsError::WriteFailed { reason })
                    if reason.contains("unavailable")
            ),
            "unexpected unavailable-writer error: {switch_error}"
        );

        let token_error = writer_control_token(Path::new("unavailable-writer"), "main")
            .expect_err("unavailable metrics must reject writer control acquisition");
        assert!(
            matches!(
                &token_error,
                EngramError::Metrics(MetricsError::WriteFailed { reason })
                    if reason.contains("unavailable")
            ),
            "unexpected unavailable-token error: {token_error}"
        );
    }

    #[tokio::test]
    async fn enabled_missing_or_stalled_writer_returns_an_explicit_error() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        force_enabled_writer(Path::new("missing-writer"), "main");

        let unavailable = switch_branch("feature__missing".to_owned())
            .await
            .expect_err("enabled metrics without a writer must fail");
        assert!(
            unavailable.to_string().contains("writer is unavailable"),
            "unexpected unavailable-writer error: {unavailable}"
        );

        let (closed_sender, closed_receiver) = mpsc::channel(1);
        drop(closed_receiver);
        {
            let mut sender_guard = sender_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *sender_guard = Some(closed_sender);
        }
        let closed = switch_branch("feature__closed".to_owned())
            .await
            .expect_err("acknowledged branch with a closed writer must fail");
        assert!(
            closed.to_string().contains("writer is closed"),
            "unexpected closed-writer error: {closed}"
        );

        let (sender, receiver) = mpsc::channel(1);
        sender
            .try_send(MetricsMessage::Event(Box::new(empty_branch_event(
                "fill_stalled_writer",
            ))))
            .expect("fill stalled channel");
        {
            let mut sender_guard = sender_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *sender_guard = Some(sender);
        }
        let stalled = switch_branch("feature__stalled".to_owned())
            .await
            .expect_err("stalled metrics writer must time out");
        assert!(
            stalled.to_string().contains("timed out"),
            "unexpected stalled-writer error: {stalled}"
        );

        {
            let mut sender_guard = sender_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sender_guard.take();
        }
        drop(receiver);
        shutdown().await.expect("reset stalled metrics state");
    }

    #[tokio::test]
    async fn cancelling_shutdown_cannot_detach_the_writer_task() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (dropped_tx, dropped_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            let _drop_notice = DropNotice(Some(dropped_tx));
            let _ = started_tx.send(());
            std::future::pending::<()>().await;
        });
        let abort_handle = handle.abort_handle();
        {
            let mut handle_guard = handle_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *handle_guard = Some(WriterTaskGuard::new(handle));
        }
        started_rx.await.expect("writer task must start");

        let registered_handle = handle_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .cloned()
            .expect("writer handle must be registered");
        let shutdown_task = tokio::spawn(shutdown());
        let mut join_started = false;
        for _ in 0..100 {
            join_started = registered_handle.handle.try_lock().is_err();
            if join_started {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(join_started, "shutdown must begin joining the writer");
        shutdown_task.abort();
        let _ = shutdown_task.await;

        assert!(
            handle_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some(),
            "cancelled shutdown must leave the live writer registered"
        );
        let mut dropped_rx = dropped_rx;
        let writer_was_detached = tokio::time::timeout(Duration::from_millis(20), &mut dropped_rx)
            .await
            .is_ok();
        assert!(
            !writer_was_detached,
            "cancelling shutdown must not detach or abort the registered writer"
        );

        let workspace = tempfile::tempdir().expect("replacement workspace");
        let disabled = MetricsConfig {
            enabled: false,
            ..MetricsConfig::default()
        };
        initialize(workspace.path(), "replacement", &disabled)
            .await
            .expect_err("replacement must wait for and reject a stalled predecessor");
        tokio::time::timeout(BRANCH_CONTROL_TIMEOUT, dropped_rx)
            .await
            .expect("replacement shutdown must reap the predecessor")
            .expect("predecessor drop notice");
        assert!(
            handle_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_none(),
            "reaped predecessor must be removed before any replacement"
        );
        assert!(
            abort_handle.is_finished(),
            "predecessor task must be finished"
        );
    }

    #[tokio::test]
    async fn failed_replacement_shutdown_leaves_metrics_cleanly_unavailable() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        let (sender, receiver) = mpsc::channel(1);
        sender
            .try_send(MetricsMessage::Event(Box::new(empty_branch_event(
                "block_shutdown",
            ))))
            .expect("fill stalled writer channel");
        {
            let mut sender_guard = sender_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *sender_guard = Some(sender);
        }
        {
            let mut handle_guard = handle_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *handle_guard = Some(WriterTaskGuard::new(tokio::spawn(std::future::pending())));
        }
        force_enabled_writer(Path::new("stalled-writer"), "stale");

        let workspace = tempfile::tempdir().expect("replacement workspace");
        let disabled = MetricsConfig {
            enabled: false,
            ..MetricsConfig::default()
        };
        initialize(workspace.path(), "replacement", &disabled)
            .await
            .expect_err("stalled shutdown must reject replacement");

        let unavailable = matches!(
            writer_state_slot()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .availability,
            WriterAvailability::Unavailable
        );
        let sender_present = sender_slot()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some();
        let switch_result = switch_branch("replacement".to_owned()).await;
        drop(receiver);
        shutdown().await.expect("clean failed replacement state");

        assert!(
            unavailable,
            "failed replacement must leave metrics unavailable"
        );
        assert!(!sender_present);
        let switch_error =
            switch_result.expect_err("unavailable metrics must reject branch control");
        assert!(
            matches!(
                &switch_error,
                EngramError::Metrics(MetricsError::WriteFailed { reason })
                    if reason.contains("unavailable")
            ),
            "unexpected unavailable-writer error: {switch_error}"
        );
    }

    #[tokio::test]
    async fn stale_writer_control_cannot_relabel_a_replacement_writer() {
        let _test_guard = test_writer_guard().await;
        shutdown().await.expect("reset metrics writer");
        let original = tempfile::tempdir().expect("original workspace");
        let replacement = tempfile::tempdir().expect("replacement workspace");
        let config = MetricsConfig::default();
        initialize(original.path(), "main", &config)
            .await
            .expect("initialize original writer");
        let mut stale_control =
            writer_control_token(original.path(), "main").expect("capture original writer");

        initialize(replacement.path(), "main", &config)
            .await
            .expect("initialize replacement writer");
        let stale_error =
            switch_branch_for(&mut stale_control, "feature__stale_dispatch".to_owned())
                .await
                .expect_err("stale control must not relabel the replacement writer");
        assert!(
            stale_error.to_string().contains("writer changed"),
            "unexpected stale-control error: {stale_error}"
        );

        record(empty_branch_event("replacement_event"));
        shutdown().await.expect("drain replacement writer");
        let events = load_events(replacement.path(), "main").expect("replacement branch events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tool_name, "replacement_event");
        assert!(matches!(
            load_events(replacement.path(), "feature__stale_dispatch"),
            Err(EngramError::Metrics(MetricsError::NotFound { .. }))
        ));
    }
}
