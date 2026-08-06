#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

use serde_json::{Value, json};

const REQUEST_ID: &str = "62046B37-cold-1";
const CORRELATION_ID: &str = "62046B37";
const INDEX_TOOL: &str = "index_workspace";
const CLIENT_EVENT: &str = "client_disposition";
const FRAME_EVENT: &str = "response_frame_result";

#[derive(Clone, Debug)]
struct Observations {
    clients: Vec<Value>,
    usage: Vec<Value>,
    frames: Vec<Value>,
}

#[derive(Debug, PartialEq, Eq)]
struct CorrelationEvidence {
    request_id: String,
    correlation_id: String,
    client_disposition: String,
    dispatch_outcome: String,
    frame_outcome: String,
}

#[derive(Debug, PartialEq, Eq)]
enum CorrelationError {
    Cardinality {
        source: &'static str,
        expected: usize,
        actual: usize,
    },
    MissingField {
        source: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for CorrelationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cardinality {
                source,
                expected,
                actual,
            } => write!(
                formatter,
                "missing-observability: expected exactly {expected} {source} record, found {actual}"
            ),
            Self::MissingField { source, field } => {
                write!(
                    formatter,
                    "{source} record is missing required field `{field}`"
                )
            }
        }
    }
}

impl Error for CorrelationError {}

fn event_text<'a>(record: &'a Value, field: &str) -> Option<&'a str> {
    record
        .get(field)
        .or_else(|| record.get("fields").and_then(|fields| fields.get(field)))
        .and_then(Value::as_str)
}

fn exactly_one<'a>(
    source: &'static str,
    mut records: impl Iterator<Item = &'a Value>,
) -> Result<&'a Value, CorrelationError> {
    let first = records.next();
    let actual = usize::from(first.is_some()) + records.count();
    if actual != 1 {
        return Err(CorrelationError::Cardinality {
            source,
            expected: 1,
            actual,
        });
    }
    first.ok_or(CorrelationError::Cardinality {
        source,
        expected: 1,
        actual: 0,
    })
}

fn required_text(
    record: &Value,
    source: &'static str,
    field: &'static str,
) -> Result<String, CorrelationError> {
    event_text(record, field)
        .map(str::to_owned)
        .ok_or(CorrelationError::MissingField { source, field })
}

fn correlate(observations: &Observations) -> Result<CorrelationEvidence, CorrelationError> {
    let client = exactly_one(
        "client disposition",
        observations.clients.iter().filter(|record| {
            event_text(record, "event_type") == Some(CLIENT_EVENT)
                && event_text(record, "request_id") == Some(REQUEST_ID)
                && event_text(record, "disposition").is_some()
        }),
    )?;
    let usage = exactly_one(
        "dispatch usage",
        observations.usage.iter().filter(|record| {
            event_text(record, "correlation_id") == Some(CORRELATION_ID)
                && event_text(record, "tool_name") == Some(INDEX_TOOL)
        }),
    )?;
    let frame = exactly_one(
        "response frame",
        observations.frames.iter().filter(|record| {
            event_text(record, "event_type") == Some(FRAME_EVENT)
                && event_text(record, "response_id") == Some(REQUEST_ID)
        }),
    )?;

    Ok(CorrelationEvidence {
        request_id: REQUEST_ID.to_owned(),
        correlation_id: CORRELATION_ID.to_owned(),
        client_disposition: required_text(client, "client disposition", "disposition")?,
        dispatch_outcome: required_text(usage, "dispatch usage", "outcome")?,
        frame_outcome: required_text(frame, "response frame", "outcome")?,
    })
}

fn complete_observations() -> Observations {
    Observations {
        clients: vec![json!({
            "event_type": CLIENT_EVENT,
            "request_id": REQUEST_ID,
            "disposition": "timeout",
            "timestamp": "2026-08-06T04:00:01Z",
        })],
        usage: vec![json!({
            "tool_name": INDEX_TOOL,
            "correlation_id": CORRELATION_ID,
            "outcome": "success",
            "timestamp": "2026-08-06T04:00:01Z",
        })],
        frames: vec![json!({
            "fields": {
                "event_type": FRAME_EVENT,
                "response_id": REQUEST_ID,
                "outcome": "flushed",
            },
            "timestamp": "2026-08-06T04:00:01Z",
        })],
    }
}

fn assert_cardinality(
    result: &Result<CorrelationEvidence, CorrelationError>,
    source: &'static str,
    actual: usize,
) {
    assert_eq!(
        result,
        &Err(CorrelationError::Cardinality {
            source,
            expected: 1,
            actual,
        })
    );
}

#[test]
fn correlation_requires_one_explicit_client_usage_and_matching_frame_id() {
    let complete = complete_observations();
    assert_eq!(
        correlate(&complete),
        Ok(CorrelationEvidence {
            request_id: REQUEST_ID.to_owned(),
            correlation_id: CORRELATION_ID.to_owned(),
            client_disposition: "timeout".to_owned(),
            dispatch_outcome: "success".to_owned(),
            frame_outcome: "flushed".to_owned(),
        })
    );

    let mut missing_client = complete.clone();
    missing_client.clients.clear();
    assert_cardinality(&correlate(&missing_client), "client disposition", 0);

    let mut duplicate_client = complete.clone();
    duplicate_client
        .clients
        .push(duplicate_client.clients[0].clone());
    assert_cardinality(&correlate(&duplicate_client), "client disposition", 2);

    let mut missing_usage = complete.clone();
    missing_usage.usage.clear();
    assert_cardinality(&correlate(&missing_usage), "dispatch usage", 0);

    let mut duplicate_usage = complete.clone();
    duplicate_usage.usage.push(duplicate_usage.usage[0].clone());
    assert_cardinality(&correlate(&duplicate_usage), "dispatch usage", 2);

    let mut missing_frame = complete.clone();
    missing_frame.frames.clear();
    assert_cardinality(&correlate(&missing_frame), "response frame", 0);

    let mut duplicate_frame = complete.clone();
    duplicate_frame
        .frames
        .push(duplicate_frame.frames[0].clone());
    assert_cardinality(&correlate(&duplicate_frame), "response frame", 2);

    let mut adjacency_only = complete;
    adjacency_only.frames = vec![json!({
        "fields": {
            "event_type": FRAME_EVENT,
            "outcome": "flushed",
        },
        "timestamp": "2026-08-06T04:00:01Z",
    })];
    assert_cardinality(&correlate(&adjacency_only), "response frame", 0);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OwnedDaemonIdentity {
    pid: u32,
    endpoint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CleanupObservation {
    pid: u32,
    endpoint: String,
    pid_alive: bool,
    endpoint_reachable: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum CleanupError {
    OwnershipMismatch,
    PidStillAlive(u32),
    EndpointStillReachable(String),
}

fn validate_owned_cleanup(
    owner: &OwnedDaemonIdentity,
    observation: &CleanupObservation,
) -> Result<(), CleanupError> {
    if owner.pid != observation.pid || owner.endpoint != observation.endpoint {
        return Err(CleanupError::OwnershipMismatch);
    }
    if observation.pid_alive {
        return Err(CleanupError::PidStillAlive(owner.pid));
    }
    if observation.endpoint_reachable {
        return Err(CleanupError::EndpointStillReachable(owner.endpoint.clone()));
    }
    Ok(())
}

#[tokio::test]
async fn synthetic_cleanup_state_requires_owned_pid_death_and_closed_endpoint() {
    let owner = OwnedDaemonIdentity {
        pid: 62_046,
        endpoint: r"\\.\pipe\engram-62046B37".to_owned(),
    };
    let clean = CleanupObservation {
        pid: owner.pid,
        endpoint: owner.endpoint.clone(),
        pid_alive: false,
        endpoint_reachable: false,
    };
    assert_eq!(validate_owned_cleanup(&owner, &clean), Ok(()));

    let mut mismatched = clean.clone();
    mismatched.pid += 1;
    assert_eq!(
        validate_owned_cleanup(&owner, &mismatched),
        Err(CleanupError::OwnershipMismatch)
    );

    let mut live = clean.clone();
    live.pid_alive = true;
    assert_eq!(
        validate_owned_cleanup(&owner, &live),
        Err(CleanupError::PidStillAlive(owner.pid))
    );

    let mut reachable = clean;
    reachable.endpoint_reachable = true;
    assert_eq!(
        validate_owned_cleanup(&owner, &reachable),
        Err(CleanupError::EndpointStillReachable(owner.endpoint))
    );

    #[cfg(windows)]
    {
        let mut endpoint_states = [true, true, false].into_iter();
        let endpoint_reachable = windows_live::poll_endpoint_until_unreachable(
            std::time::Instant::now() + std::time::Duration::from_secs(1),
            |_| std::future::ready(endpoint_states.next().unwrap_or(true)),
        )
        .await;
        assert!(!endpoint_reachable);
        assert_eq!(endpoint_states.next(), None);
    }
}

#[cfg(windows)]
mod windows_live {
    // Keep the repository MSRV-compatible spelling rather than using the
    // newer `Duration::from_mins` constructor suggested by current Clippy.
    #![allow(clippy::duration_suboptimal_units)]

    use std::fs;
    use std::future::Future;
    use std::io::{self, Write as _};
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Stdio};
    use std::time::{Duration, Instant};

    use anyhow::{Context as _, Result, anyhow, bail, ensure};
    use chrono::{SecondsFormat, Utc};
    use engram::daemon::ipc_server::ipc_endpoint;
    use engram::daemon::protocol::IpcRequest;
    use engram::shim::ipc_client::{probe, send_request};
    use engram::shim::pidfile::PidFile;
    use serde_json::{Value, json};
    use sha2::{Digest as _, Sha256};
    use tempfile::TempDir;
    use tokio::io::{AsyncRead, AsyncReadExt as _};
    use tokio::process::{Child, Command};
    use tokio::task::JoinHandle;

    use super::{
        CLIENT_EVENT, CORRELATION_ID, CleanupObservation, FRAME_EVENT, INDEX_TOOL, Observations,
        OwnedDaemonIdentity, REQUEST_ID, correlate, event_text, validate_owned_cleanup,
    };

    const AGGREGATE_LIMIT: Duration = Duration::from_secs(5 * 60);
    const CLEANUP_RESERVE: Duration = Duration::from_secs(60);
    const IDLE_FALLBACK_MS: &str = "20000";
    const CAPTURE_SWITCH: &str = "ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE";
    const TRACE_STDOUT: &str = "test-autospawn.stdout.log";
    const TRACE_STDERR: &str = "test-autospawn.stderr.log";
    const EXPECTED_CORPUS_SHA256: &str =
        "58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25";
    const CORPUS: [(&str, &str); 2] = [
        (
            "Cargo.toml",
            "[package]\nname = \"cold-correlation-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        ),
        (
            "src/lib.rs",
            "pub fn cold_probe() -> &'static str {\n    \"62046B37\"\n}\n",
        ),
    ];

    #[derive(Clone, Copy, Debug)]
    struct Supervisor {
        started: Instant,
        work_deadline: Instant,
        aggregate_deadline: Instant,
    }

    impl Supervisor {
        fn start() -> Result<Self> {
            let started = Instant::now();
            let aggregate_deadline = started
                .checked_add(AGGREGATE_LIMIT)
                .context("cannot represent aggregate five-minute deadline")?;
            let work_deadline = aggregate_deadline
                .checked_sub(CLEANUP_RESERVE)
                .context("cannot reserve owned-daemon cleanup time")?;
            Ok(Self {
                started,
                work_deadline,
                aggregate_deadline,
            })
        }

        fn remaining_work(self) -> Duration {
            self.work_deadline.saturating_duration_since(Instant::now())
        }

        fn remaining_cleanup(self) -> Duration {
            self.aggregate_deadline
                .saturating_duration_since(Instant::now())
        }

        fn elapsed_ms(self) -> u64 {
            u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
        }
    }

    struct CliProcess {
        child: Child,
        stdout: JoinHandle<io::Result<Vec<u8>>>,
        stderr: JoinHandle<io::Result<Vec<u8>>>,
        pid: Option<u32>,
    }

    fn now_timestamp() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    fn write_frozen_corpus(root: &Path) -> Result<String> {
        let mut aggregate = Sha256::new();
        for (relative, content) in CORPUS {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create corpus directory {}", parent.display()))?;
            }
            fs::write(&path, content)
                .with_context(|| format!("write frozen corpus file {}", path.display()))?;
            aggregate.update(relative.as_bytes());
            aggregate.update([0]);
            aggregate.update(content.as_bytes());
            aggregate.update([0xff]);
        }
        let hash = hex::encode(aggregate.finalize());
        ensure!(
            hash == EXPECTED_CORPUS_SHA256,
            "frozen corpus hash changed: expected {EXPECTED_CORPUS_SHA256}, got {hash}"
        );
        Ok(hash)
    }

    fn prepare_workspace() -> Result<(TempDir, PathBuf, String)> {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let temp_root = repository.join("tmp");
        fs::create_dir_all(&temp_root).context("create repository-contained temp root")?;
        let temp = tempfile::Builder::new()
            .prefix("cold-cli-correlation-")
            .tempdir_in(&temp_root)
            .context("create owned temporary workspace")?;
        let root = temp
            .path()
            .canonicalize()
            .context("canonicalize owned temporary workspace")?;

        let git = root.join(".git");
        fs::create_dir_all(&git).context("create isolated .git directory")?;
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n")
            .context("write isolated main-branch HEAD")?;
        let corpus_hash = write_frozen_corpus(&root)?;

        let engram_dir = root.join(".engram");
        fs::create_dir_all(&engram_dir).context("create workspace-local .engram directory")?;
        fs::write(
            engram_dir.join("config.toml"),
            "log_level = \"debug\"\nlog_format = \"json\"\n\n[metrics]\nenabled = true\nbuffer_size = 128\n",
        )
        .context("write workspace-local trace and metrics configuration")?;
        Ok((temp, root, corpus_hash))
    }

    async fn read_stream(mut stream: impl AsyncRead + Unpin) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    fn spawn_cli(root: &Path) -> Result<CliProcess> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_engram"));
        command
            .arg("--workspace")
            .arg(root)
            .args([
                "--json",
                "--id",
                REQUEST_ID,
                "--correlation-id",
                CORRELATION_ID,
                "--timeout",
                "1",
                "index",
                "--force",
            ])
            .current_dir(root)
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_DIRECT")
            .env_remove("ENGRAM_WORKSPACE")
            .env("ENGRAM_IDLE_TIMEOUT_MS", IDLE_FALLBACK_MS)
            .env("ENGRAM_READY_TIMEOUT_MS", "180000")
            .env("ENGRAM_LOG_FORMAT", "json")
            .env("RUST_LOG", "engram=debug")
            .env(CAPTURE_SWITCH, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().context("launch real engram CLI")?;
        let pid = child.id();
        let stdout = child.stdout.take().context("capture CLI stdout")?;
        let stderr = child.stderr.take().context("capture CLI stderr")?;
        Ok(CliProcess {
            child,
            stdout: tokio::spawn(read_stream(stdout)),
            stderr: tokio::spawn(read_stream(stderr)),
            pid,
        })
    }

    fn read_json_lines(paths: &[PathBuf]) -> Result<Vec<Value>> {
        let mut records = Vec::new();
        for path in paths {
            let content = match fs::read(path) {
                Ok(content) => content,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("read observation file {}", path.display()));
                }
            };
            let completed_len = content
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            for (line_index, line) in content[..completed_len]
                .split(|byte| *byte == b'\n')
                .enumerate()
            {
                let line = std::str::from_utf8(line).with_context(|| {
                    format!(
                        "decode observation file {} line {} as UTF-8",
                        path.display(),
                        line_index + 1
                    )
                })?;
                if line.trim().is_empty() {
                    continue;
                }
                let record = serde_json::from_str(line).with_context(|| {
                    format!(
                        "parse observation file {} line {}: {line}",
                        path.display(),
                        line_index + 1
                    )
                })?;
                records.push(record);
            }
        }
        Ok(records)
    }

    #[test]
    fn capture_polling_defers_partial_tail_and_rejects_completed_malformed_line() {
        let mut capture = tempfile::NamedTempFile::new().expect("capture file");
        capture
            .write_all(
                b"{\"fields\":{\"event_type\":\"response_frame_result\"}}\n\
                  {\"message\":\"\xF0\x9F",
            )
            .expect("write complete record and partial UTF-8 tail");

        let records =
            read_json_lines(&[capture.path().to_path_buf()]).expect("read completed capture lines");
        assert_eq!(records.len(), 1);
        assert_eq!(event_text(&records[0], "event_type"), Some(FRAME_EVENT));

        capture
            .write_all(b"\x98\x80\"}\n")
            .expect("finish partial UTF-8 record");
        let records =
            read_json_lines(&[capture.path().to_path_buf()]).expect("read appended capture lines");
        assert_eq!(records.len(), 2);
        assert_eq!(records[1]["message"], "\u{1f600}");

        capture
            .write_all(b"not-json\n")
            .expect("append malformed completed line");

        let error = read_json_lines(&[capture.path().to_path_buf()])
            .expect_err("malformed capture line must not be discarded");
        let message = format!("{error:#}");
        assert!(message.contains("line 3"), "{message}");
        assert!(message.contains("not-json"), "{message}");
    }

    async fn wait_for_usage(root: &Path, supervisor: Supervisor) -> Result<Vec<Value>> {
        let usage_path = root
            .join(".engram")
            .join("metrics")
            .join("main")
            .join("usage.jsonl");
        loop {
            let records = read_json_lines(std::slice::from_ref(&usage_path))?;
            if records.iter().any(|record| {
                record["correlation_id"] == CORRELATION_ID && record["tool_name"] == INDEX_TOOL
            }) || supervisor.remaining_work().is_zero()
            {
                return Ok(records);
            }
            tokio::time::sleep(supervisor.remaining_work().min(Duration::from_millis(50))).await;
        }
    }

    async fn wait_for_frame(root: &Path, supervisor: Supervisor) -> Result<Vec<Value>> {
        let paths = [
            root.join(".engram").join(TRACE_STDOUT),
            root.join(".engram").join(TRACE_STDERR),
        ];
        loop {
            let records = read_json_lines(&paths)?;
            if records.iter().any(|record| {
                event_text(record, "event_type") == Some(FRAME_EVENT)
                    && event_text(record, "response_id") == Some(REQUEST_ID)
                    && event_text(record, "outcome").is_some()
            }) || supervisor.remaining_work().is_zero()
            {
                return Ok(records);
            }
            tokio::time::sleep(supervisor.remaining_work().min(Duration::from_millis(50))).await;
        }
    }

    async fn wait_for_pid(root: &Path, supervisor: Supervisor) -> Option<PidFile> {
        loop {
            if let Some(pid) = PidFile::read(root) {
                return Some(pid);
            }
            let remaining = supervisor.remaining_work();
            if remaining.is_zero() {
                return None;
            }
            tokio::time::sleep(remaining.min(Duration::from_millis(25))).await;
        }
    }

    fn client_disposition(stdout: &str, stderr: &str, status: Option<ExitStatus>) -> Value {
        if let Some(envelope) = stdout
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok())
        {
            let disposition = if envelope.get("result").is_some() {
                "completion"
            } else {
                "error"
            };
            return json!({
                "event_type": CLIENT_EVENT,
                "request_id": envelope.get("id").cloned().unwrap_or(Value::Null),
                "disposition": disposition,
                "exit_code": status.and_then(|value| value.code()),
                "envelope": envelope,
            });
        }

        let stderr_lower = stderr.to_ascii_lowercase();
        let disposition = if stderr_lower.contains("timed out") || stderr_lower.contains("timeout")
        {
            "timeout"
        } else {
            "error"
        };
        json!({
            "event_type": CLIENT_EVENT,
            "request_id": REQUEST_ID,
            "disposition": disposition,
            "exit_code": status.and_then(|value| value.code()),
            "stderr": stderr,
        })
    }

    async fn request_graceful_shutdown(endpoint: &str, supervisor: Supervisor) {
        let remaining = supervisor.remaining_cleanup();
        if remaining.is_zero() {
            return;
        }
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(json!("62046B37-cleanup")),
            method: "_shutdown".to_owned(),
            params: None,
        };
        let _ = send_request(endpoint, &request, remaining.min(Duration::from_secs(2))).await;
    }

    async fn cleanup_owned_daemon(
        owner: &OwnedDaemonIdentity,
        pid_file: &PidFile,
        supervisor: Supervisor,
    ) -> Result<CleanupObservation> {
        request_graceful_shutdown(&owner.endpoint, supervisor).await;

        let pid_alive = loop {
            let alive = pid_file.verify_alive().context("verify exact owned PID")?;
            if !alive || supervisor.remaining_cleanup().is_zero() {
                break alive;
            }
            tokio::time::sleep(
                supervisor
                    .remaining_cleanup()
                    .min(Duration::from_millis(50)),
            )
            .await;
        };

        let endpoint_reachable = if pid_alive {
            true
        } else {
            poll_endpoint_until_unreachable(supervisor.aggregate_deadline, |timeout| {
                let endpoint = &owner.endpoint;
                async move { probe(endpoint, timeout).await.is_ok() }
            })
            .await
        };
        Ok(CleanupObservation {
            pid: owner.pid,
            endpoint: owner.endpoint.clone(),
            pid_alive,
            endpoint_reachable,
        })
    }

    pub(super) async fn poll_endpoint_until_unreachable<Probe, ProbeFuture>(
        aggregate_deadline: Instant,
        mut endpoint_is_reachable: Probe,
    ) -> bool
    where
        Probe: FnMut(Duration) -> ProbeFuture,
        ProbeFuture: Future<Output = bool>,
    {
        loop {
            let remaining = aggregate_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return true;
            }
            if !endpoint_is_reachable(remaining.min(Duration::from_millis(200))).await {
                return false;
            }

            let remaining = aggregate_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return true;
            }
            tokio::time::sleep(remaining.min(Duration::from_millis(50))).await;
        }
    }

    async fn finish_capture(
        task: JoinHandle<io::Result<Vec<u8>>>,
        supervisor: Supervisor,
        stream: &str,
    ) -> Result<String> {
        let remaining = supervisor.remaining_cleanup();
        ensure!(
            !remaining.is_zero(),
            "aggregate deadline expired reading CLI {stream}"
        );
        let bytes = tokio::time::timeout(remaining, task)
            .await
            .with_context(|| format!("timed out reading CLI {stream}"))?
            .with_context(|| format!("join CLI {stream} reader"))?
            .with_context(|| format!("read CLI {stream}"))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn command_text(root: &Path) -> String {
        format!(
            "\"{}\" --workspace \"{}\" --json --id {REQUEST_ID} --correlation-id \
             {CORRELATION_ID} --timeout 1 index --force",
            env!("CARGO_BIN_EXE_engram"),
            root.display()
        )
    }

    fn preserve_blocked_workspace(workspace: TempDir, message: &str) -> anyhow::Error {
        let path = workspace.keep();
        anyhow!("{message}; preserved_workspace={}", path.display())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "opt-in bounded Windows cold real-CLI request/frame characterization"]
    #[allow(clippy::too_many_lines)]
    async fn windows_cold_cli_request_frame_correlation() -> Result<()> {
        let (workspace, root, corpus_hash) = prepare_workspace()?;
        let pid_path = PidFile::path(&root);
        ensure!(
            !pid_path.exists(),
            "fresh owned workspace unexpectedly has PID state at {}",
            pid_path.display()
        );
        let endpoint = ipc_endpoint(&root).context("derive owned named-pipe endpoint")?;
        ensure!(
            probe(&endpoint, Duration::from_millis(100)).await.is_err(),
            "fresh owned named pipe is unexpectedly reachable: {endpoint}"
        );

        // The one aggregate supervisor starts before the real CLI launch and
        // reserves its final minute for non-destructive owned-daemon cleanup.
        let supervisor = Supervisor::start()?;
        let command = command_text(&root);
        let cli_started_at = now_timestamp();
        let cli_started = Instant::now();
        let mut cli = spawn_cli(&root)?;
        let mut cli_status =
            match tokio::time::timeout(supervisor.remaining_work(), cli.child.wait()).await {
                Ok(result) => result.context("wait for real engram CLI").map(Some)?,
                Err(_) => None,
            };
        let cli_finished_at = now_timestamp();
        let cli_elapsed_ms = u64::try_from(cli_started.elapsed().as_millis()).unwrap_or(u64::MAX);

        let owned_pid_file = wait_for_pid(&root, supervisor).await;
        let usage_result = wait_for_usage(&root, supervisor).await;
        let frame_result = wait_for_frame(&root, supervisor).await;

        let Some(pid_file) = owned_pid_file else {
            let endpoint_reachable = probe(&endpoint, Duration::from_millis(200)).await.is_ok();
            let error = preserve_blocked_workspace(
                workspace,
                &format!(
                    "BLOCKED: endpoint ownership cannot be proven; cli_pid={:?}; pipe={endpoint}; \
                     endpoint_reachable={endpoint_reachable}",
                    cli.pid
                ),
            );
            return Err(error);
        };
        let owner = OwnedDaemonIdentity {
            pid: pid_file.pid,
            endpoint: endpoint.clone(),
        };
        let cleanup = match cleanup_owned_daemon(&owner, &pid_file, supervisor).await {
            Ok(cleanup) => cleanup,
            Err(error) => {
                return Err(preserve_blocked_workspace(
                    workspace,
                    &format!(
                        "BLOCKED: owned cleanup probe failed: {error}; pid={}; pipe={endpoint}",
                        owner.pid
                    ),
                ));
            }
        };
        if let Err(error) = validate_owned_cleanup(&owner, &cleanup) {
            return Err(preserve_blocked_workspace(
                workspace,
                &format!(
                    "BLOCKED: {error:?}; pid={}; pipe={endpoint}; \
                     pid_alive={}; endpoint_reachable={}",
                    owner.pid, cleanup.pid_alive, cleanup.endpoint_reachable
                ),
            ));
        }

        if cli_status.is_none() && !supervisor.remaining_cleanup().is_zero() {
            cli_status = match tokio::time::timeout(
                supervisor.remaining_cleanup(),
                cli.child.wait(),
            )
            .await
            {
                Ok(result) => result
                    .context("wait for CLI after daemon cleanup")
                    .map(Some)?,
                Err(_) => None,
            };
        }
        if cli_status.is_none() {
            return Err(preserve_blocked_workspace(
                workspace,
                &format!(
                    "BLOCKED: owned CLI remained live; cli_pid={:?}; daemon_pid={}; \
                     pipe={endpoint}",
                    cli.pid, owner.pid
                ),
            ));
        }

        let stdout = finish_capture(cli.stdout, supervisor, "stdout").await?;
        let stderr = finish_capture(cli.stderr, supervisor, "stderr").await?;
        let usage = usage_result?;
        let frames = frame_result?;
        let observations = Observations {
            clients: vec![client_disposition(&stdout, &stderr, cli_status)],
            usage,
            frames,
        };
        let correlation = correlate(&observations);

        let evidence = json!({
            "run_label": "bounded-cold-cli-characterization",
            "command": command,
            "cli_started_at": cli_started_at,
            "cli_finished_at": cli_finished_at,
            "cli_elapsed_ms": cli_elapsed_ms,
            "aggregate_elapsed_ms": supervisor.elapsed_ms(),
            "aggregate_limit_ms": 300_000_u64,
            "cleanup_reserve_ms": 60_000_u64,
            "request_id": REQUEST_ID,
            "correlation_id": CORRELATION_ID,
            "corpus_sha256": corpus_hash,
            "cli_exit_code": cli_status.and_then(|status| status.code()),
            "cli_stdout": stdout,
            "cli_stderr": stderr,
            "owned_pid": owner.pid,
            "named_pipe": endpoint,
            "cleanup": {
                "graceful_shutdown_requested": true,
                "idle_fallback_ms": IDLE_FALLBACK_MS,
                "exact_pid_dead": !cleanup.pid_alive,
                "endpoint_unreachable": !cleanup.endpoint_reachable,
                "force_kill_used": false,
            },
            "correlation_result": format!("{correlation:?}"),
        });

        if let Err(error) = correlation {
            bail!(
                "COLD_CLI_CORRELATION_BLOCKED missing-observability: {error}; evidence={}",
                serde_json::to_string(&evidence).context("serialize RED evidence")?
            );
        }
        println!(
            "COLD_CLI_CORRELATION_RESULT={}",
            serde_json::to_string(&evidence).context("serialize correlated evidence")?
        );
        Ok(())
    }
}
