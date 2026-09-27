//! Descriptor-driven behavior contract for the read-server IPC, CLI, and MCP
//! surfaces (F54, task 142.058-T).
//!
//! This test talks to the real `engram` daemon and shipped `engram` executable
//! over isolated temporary workspaces. It does not call tool dispatch directly
//! or manufacture response observations.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant, UNIX_EPOCH};

use chrono::Utc;
use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::{IpcRequest, IpcResponse};
use engram::db::workspace::workspace_hash;
use engram::services::generations::{
    BranchIdentity, GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision,
    GenerationStore, ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory,
    WorkspaceIdentity, publish_generation_manifest,
};
use engram::shim::ipc_client::send_request;
use engram::tools::capabilities::{
    CapabilityClass, InputOwnership, ToolDescriptor, ToolSurface, all_descriptors, descriptor,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sysinfo::{Pid, System};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::process::Command as TokioCommand;

const F38_READ_SERVER_WRITE_CONTROL_REFUSAL: u16 = 16_001;
const F38_WORKSPACE_RETARGET_REFUSAL: u16 = 16_003;
const F54_RED_MARKER: &str = "F54-RED";
const F54_BLOCK_MARKER: &str = "F54-BLOCK";
const EXPECTED_GENERATION: &str = "generation-142";
const FIXTURE_BRANCH: &str = "main";
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const IPC_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    accepted: bool,
    refusal_code: Option<u16>,
    generation: Option<String>,
    side_effect_count: usize,
    response: Option<Value>,
    raw_response: String,
    surface_advertised: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedOutcome {
    ReadyHealth,
    SuccessfulWorkflow,
    GenerationBackedRead,
    Refusal { code: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeclaredSurfaceExpectation {
    surface: ToolSurface,
    outcome: ExpectedOutcome,
}

#[derive(Debug)]
struct DescriptorParityRow<'a> {
    descriptor: &'a ToolDescriptor,
    surfaces: Vec<DeclaredSurfaceExpectation>,
}

#[derive(Debug)]
struct RawObservation {
    accepted: bool,
    refusal_code: Option<u16>,
    response: Option<Value>,
    raw_response: String,
    surface_advertised: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChildMarker {
    task: String,
    pid: u32,
    executable: PathBuf,
}

struct MarkedDaemon {
    child: Child,
    marker_path: PathBuf,
    pid: u32,
    executable: PathBuf,
}

impl MarkedDaemon {
    fn verify_before_stop(&mut self) -> Result<(), String> {
        let marker_bytes = fs::read(&self.marker_path).map_err(|error| {
            format!(
                "cannot read child marker {}: {error}",
                self.marker_path.display()
            )
        })?;
        let marker: ChildMarker = serde_json::from_slice(&marker_bytes)
            .map_err(|error| format!("cannot decode child marker: {error}"))?;
        if marker.task != "142.058-T"
            || marker.pid != self.pid
            || marker.executable != self.executable
            || self.child.id() != self.pid
        {
            return Err(format!(
                "child marker does not identify the owned daemon: marker={marker:?}, \
                 child_pid={}, expected_executable={}",
                self.child.id(),
                self.executable.display()
            ));
        }

        if self
            .child
            .try_wait()
            .map_err(|error| format!("cannot inspect child status: {error}"))?
            .is_some()
        {
            return Err("owned daemon has already exited".to_owned());
        }

        let mut system = System::new();
        system.refresh_processes();
        let process = system
            .process(Pid::from_u32(self.pid))
            .ok_or_else(|| format!("cannot verify executable for child PID {}", self.pid))?;
        let running_executable = process
            .exe()
            .ok_or_else(|| format!("child PID {} has no inspectable executable path", self.pid))?
            .canonicalize()
            .map_err(|error| format!("cannot canonicalize child executable: {error}"))?;
        if running_executable != marker.executable {
            return Err(format!(
                "child PID {} executable mismatch: running={}, marker={}",
                self.pid,
                running_executable.display(),
                marker.executable.display()
            ));
        }
        Ok(())
    }
}
struct ReadServerFixture {
    temporary_root: TempDir,
    workspace: PathBuf,
    alternate_workspace: PathBuf,
    endpoint: String,
    executable: PathBuf,
    daemon: Option<MarkedDaemon>,
}

impl ReadServerFixture {
    async fn new() -> Self {
        let temporary_root = tempfile::tempdir()
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: create fixture: {error}"));
        let temporary_root_path = temporary_root
            .path()
            .canonicalize()
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: canonicalize fixture: {error}"));
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: canonicalize repository: {error}"));
        if temporary_root_path.starts_with(&repository)
            || repository.starts_with(&temporary_root_path)
        {
            panic!(
                "{F54_BLOCK_MARKER}: temporary fixture {} overlaps repository {}",
                temporary_root_path.display(),
                repository.display()
            );
        }

        let workspace = temporary_root_path.join("workspace");
        let alternate_workspace = temporary_root_path.join("alternate-workspace");
        create_workspace(&workspace, true);
        create_workspace(&alternate_workspace, false);
        let workspace = workspace
            .canonicalize()
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: canonicalize workspace: {error}"));
        let alternate_workspace = alternate_workspace.canonicalize().unwrap_or_else(|error| {
            panic!("{F54_BLOCK_MARKER}: canonicalize alternate workspace: {error}")
        });
        let endpoint = ipc_endpoint(&workspace)
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: derive IPC endpoint: {error}"));
        let executable = fs::canonicalize(engram_binary())
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: resolve engram binary: {error}"));
        let source_dir = workspace.join("src");
        fs::create_dir_all(&source_dir)
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: create source fixture: {error}"));
        fs::write(
            source_dir.join("f54_fixture.rs"),
            "pub fn f54_fixture_root() {}\n\
             pub fn f54_fixture_caller() { f54_fixture_root(); }\n",
        )
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: write source fixture: {error}"));
        engram::services::code_graph::sync_workspace(
            &workspace,
            &workspace.join(".engram"),
            FIXTURE_BRANCH,
            &engram::models::config::CodeGraphConfig::default(),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("{F54_BLOCK_MARKER}: seed isolated code graph fixture: {error}")
        });
        publish_fixture_generation(&workspace);

        Self {
            temporary_root,
            workspace,
            alternate_workspace,
            endpoint,
            executable,
            daemon: None,
        }
    }

    async fn ensure_daemon(&mut self) -> Result<(), String> {
        let daemon_is_running = match self.daemon.as_mut() {
            Some(daemon) => daemon
                .child
                .try_wait()
                .map_err(|error| format!("cannot inspect daemon status: {error}"))?
                .is_none(),
            None => false,
        };
        if !daemon_is_running {
            self.daemon.take();
            self.spawn_daemon()?;
        }

        let deadline = Instant::now() + READY_TIMEOUT;
        loop {
            if let Some(daemon) = self.daemon.as_mut() {
                if let Some(status) = daemon
                    .child
                    .try_wait()
                    .map_err(|error| format!("cannot inspect daemon status: {error}"))?
                {
                    return Err(format!(
                        "isolated read-server child exited before readiness: {status}"
                    ));
                }
            }

            if let Ok(response) = self.send_ipc("_health", None).await
                && response
                    .result
                    .as_ref()
                    .and_then(|result| result.get("status"))
                    .and_then(Value::as_str)
                    == Some("ready")
            {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "isolated read-server did not become ready at {}",
                    self.endpoint
                ));
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn spawn_daemon(&mut self) -> Result<(), String> {
        let workspace = self
            .workspace
            .to_str()
            .ok_or_else(|| "fixture path is not UTF-8".to_owned())?;
        let child = Command::new(&self.executable)
            .args(["daemon", "--workspace", workspace])
            .current_dir(&self.workspace)
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_WORKSPACE")
            .env_remove("ENGRAM_DIRECT")
            .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("cannot spawn isolated engram daemon: {error}"))?;

        let pid = child.id();
        let marker_path = self
            .temporary_root
            .path()
            .join(format!("142.058-child-{pid}.json"));
        let marker = ChildMarker {
            task: "142.058-T".to_owned(),
            pid,
            executable: self.executable.clone(),
        };
        let marker_bytes = serde_json::to_vec(&marker)
            .map_err(|error| format!("cannot serialize child marker: {error}"))?;
        fs::write(&marker_path, marker_bytes).map_err(|error| {
            format!("cannot record spawned daemon PID {pid} before any stop action: {error}")
        })?;

        self.daemon = Some(MarkedDaemon {
            child,
            marker_path,
            pid,
            executable: self.executable.clone(),
        });
        Ok(())
    }

    async fn send_ipc(&self, method: &str, params: Option<Value>) -> Result<IpcResponse, String> {
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::from(1_u64)),
            method: method.to_owned(),
            params,
        };
        send_request(&self.endpoint, &request, IPC_TIMEOUT)
            .await
            .map_err(|error| error.to_string())
    }

    async fn workspace_binding_fingerprint(&self) -> Option<Value> {
        let response = self.send_ipc("get_workspace_status", None).await.ok()?;
        let result = response.result?;
        Some(json!({
            "path": result.get("path"),
            "branch": result.get("branch"),
            "db_path": result.get("db_path"),
            "generation": result.get("generation"),
        }))
    }

    fn filesystem_snapshot(&self) -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut snapshot = BTreeMap::new();
        snapshot_directory(
            self.temporary_root.path(),
            self.temporary_root.path(),
            &mut snapshot,
        )
        .map_err(|error| format!("cannot snapshot fixture side effects: {error}"))?;
        Ok(snapshot)
    }

    fn verify_daemon_marker(&mut self) -> Result<(), String> {
        self.daemon
            .as_mut()
            .ok_or_else(|| "no fixture daemon child is recorded".to_owned())?
            .verify_before_stop()
    }

    async fn stop_daemon(&mut self) -> Result<(), String> {
        let Some(mut daemon) = self.daemon.take() else {
            return Ok(());
        };
        if daemon
            .child
            .try_wait()
            .map_err(|error| format!("cannot inspect daemon before cleanup: {error}"))?
            .is_some()
        {
            return Ok(());
        }

        daemon.verify_before_stop()?;
        let request = IpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(Value::from(9_142_u64)),
            method: "_shutdown".to_owned(),
            params: None,
        };
        let _ = send_request(&self.endpoint, &request, Duration::from_secs(2)).await;

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if daemon
                .child
                .try_wait()
                .map_err(|error| format!("cannot inspect daemon during cleanup: {error}"))?
                .is_some()
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                daemon.verify_before_stop()?;
                daemon
                    .child
                    .kill()
                    .map_err(|error| format!("cannot stop verified fixture daemon: {error}"))?;
                daemon
                    .child
                    .wait()
                    .map_err(|error| format!("cannot reap verified fixture daemon: {error}"))?;
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

impl Drop for ReadServerFixture {
    fn drop(&mut self) {
        let Some(daemon) = self.daemon.as_mut() else {
            return;
        };
        match daemon.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => match daemon.verify_before_stop() {
                Ok(()) => {
                    if let Err(error) = daemon.child.kill() {
                        eprintln!(
                            "{F54_BLOCK_MARKER}: failed to stop verified fixture child {}: {error}",
                            daemon.pid
                        );
                    } else if let Err(error) = daemon.child.wait() {
                        eprintln!(
                            "{F54_BLOCK_MARKER}: failed to reap verified fixture child {}: {error}",
                            daemon.pid
                        );
                    }
                }
                Err(error) => eprintln!(
                    "{F54_BLOCK_MARKER}: refusing to stop unverified child {}: {error}",
                    daemon.pid
                ),
            },
            Err(error) => eprintln!(
                "{F54_BLOCK_MARKER}: refusing to stop child {} after status error: {error}",
                daemon.pid
            ),
        }
    }
}

fn create_workspace(path: &Path, read_server: bool) {
    fs::create_dir_all(path.join(".git"))
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: create fixture git root: {error}"));
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n")
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: write fixture HEAD: {error}"));
    if read_server {
        fs::create_dir_all(path.join(".engram")).unwrap_or_else(|error| {
            panic!("{F54_BLOCK_MARKER}: create fixture data root: {error}")
        });
        fs::write(
            path.join(".engram").join("config.toml"),
            "mode = \"read_server\"\n",
        )
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: write fixture config: {error}"));
        let metrics_dir = path.join(".engram").join("metrics").join("main");
        fs::create_dir_all(&metrics_dir)
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: create metrics fixture: {error}"));
        let usage_event = json!({
            "tool_name": "f54_fixture",
            "timestamp": "2026-09-01T00:00:00Z",
            "response_bytes": 0,
            "estimated_tokens": 0,
            "symbols_returned": 0,
            "results_returned": 0,
            "branch": "main",
            "workspace": path.display().to_string()
        });
        fs::write(metrics_dir.join("usage.jsonl"), format!("{usage_event}\n"))
            .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: write metrics fixture: {error}"));
    }
}

fn publish_fixture_generation(workspace: &Path) {
    let generations_root = workspace.join(".engram").join("generations");
    let generation_directory = generations_root.join(EXPECTED_GENERATION);
    fs::create_dir_all(&generation_directory).unwrap_or_else(|error| {
        panic!("{F54_BLOCK_MARKER}: create published-generation fixture: {error}")
    });

    let database_path = generation_directory.join("engram.db");
    {
        let database =
            cozo::DbInstance::new("sqlite", database_path.to_string_lossy().as_ref(), "")
                .unwrap_or_else(|error| {
                    panic!("{F54_BLOCK_MARKER}: create published-generation database: {error}")
                });
        database
            .run_default(":create probe_row {id => val}")
            .unwrap_or_else(|error| {
                panic!("{F54_BLOCK_MARKER}: initialize published-generation database: {error}")
            });
    }

    let database_bytes = fs::read(&database_path).unwrap_or_else(|error| {
        panic!("{F54_BLOCK_MARKER}: read published-generation database: {error}")
    });
    let database_relative_path = format!("{EXPECTED_GENERATION}/engram.db");
    let generation_store = GenerationStore::new(&generations_root).unwrap_or_else(|error| {
        panic!("{F54_BLOCK_MARKER}: construct published-generation store: {error}")
    });
    let manifest = GenerationManifest::new(
        GenerationId::new(EXPECTED_GENERATION).unwrap_or_else(|error| {
            panic!("{F54_BLOCK_MARKER}: validate fixture generation ID: {error}")
        }),
        GenerationRevision::new(1),
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(FIXTURE_BRANCH, None),
        WorkspaceIdentity::new(workspace_hash(workspace, FIXTURE_BRANCH)),
        SealedInventory::new(vec![ManifestFileDigest::new(
            database_relative_path,
            hex::encode(Sha256::digest(&database_bytes)),
        )]),
        GenerationProvenance::new("f54-contract-fixture", Utc::now()),
    );
    publish_generation_manifest(&generation_store, &manifest).unwrap_or_else(|error| {
        panic!("{F54_BLOCK_MARKER}: publish active fixture generation: {error}")
    });
}

fn engram_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_engram"))
}

fn request_parameters(descriptor: &ToolDescriptor, fixture: &ReadServerFixture) -> Value {
    let schema = descriptor.input_schema.as_ref();
    let properties = schema.get("properties").and_then(Value::as_object);
    let required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let mut arguments = serde_json::Map::new();

    if let Some(properties) = properties {
        for name in required {
            if let Some(property_schema) = properties.get(name) {
                arguments.insert(
                    name.to_owned(),
                    sample_schema_value(name, property_schema, descriptor, fixture),
                );
            }
        }
    }

    match descriptor.name {
        "impact_analysis" => {
            arguments.insert(
                "symbol_name".to_owned(),
                Value::String("f54_fixture_root".to_owned()),
            );
        }
        "query_graph" => {
            arguments.insert(
                "root".to_owned(),
                Value::String("fn:f54-missing".to_owned()),
            );
        }
        "get_branch_metrics" => {
            arguments.insert("branch_name".to_owned(), Value::String("main".to_owned()));
        }
        _ => {}
    }

    Value::Object(arguments)
}

fn sample_schema_value(
    name: &str,
    schema: &Value,
    descriptor: &ToolDescriptor,
    fixture: &ReadServerFixture,
) -> Value {
    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && let Some(value) = values.first()
    {
        return value.clone();
    }
    if let Some(value) = schema.get("const") {
        return value.clone();
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("string") => {
            let value = match name {
                "path" if descriptor.name == "set_workspace" => {
                    fixture.alternate_workspace.to_string_lossy().into_owned()
                }
                "path" => fixture.workspace.to_string_lossy().into_owned(),
                "query" | "search_query" | "query_text" => "isolated F54 fixture query".to_owned(),
                "symbol" | "symbol_name" => "f54_fixture_root".to_owned(),
                "operation" => "neighborhood".to_owned(),
                "root" | "from" | "to" => "fn:f54-missing".to_owned(),
                "branch" => "main".to_owned(),
                _ => "f54-fixture-value".to_owned(),
            };
            let min_length = schema
                .get("minLength")
                .and_then(Value::as_u64)
                .and_then(|length| usize::try_from(length).ok())
                .unwrap_or_default();
            if value.len() >= min_length {
                Value::String(value)
            } else {
                Value::String(format!("{value}{}", "x".repeat(min_length - value.len())))
            }
        }
        Some("integer") => schema
            .get("minimum")
            .and_then(Value::as_i64)
            .map_or_else(|| Value::from(1_i64), Value::from),
        Some("number") => schema
            .get("minimum")
            .and_then(Value::as_f64)
            .and_then(serde_json::Number::from_f64)
            .map_or_else(|| Value::from(1_i64), Value::Number),
        Some("boolean") => Value::Bool(false),
        Some("array") => {
            let count = schema
                .get("minItems")
                .and_then(Value::as_u64)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or_default();
            let item_schema = schema.get("items").unwrap_or(&Value::Null);
            Value::Array(
                (0..count)
                    .map(|_| sample_schema_value("item", item_schema, descriptor, fixture))
                    .collect(),
            )
        }
        Some("object") => sample_object_value(schema, descriptor, fixture),
        _ => Value::Null,
    }
}

fn sample_object_value(
    schema: &Value,
    descriptor: &ToolDescriptor,
    fixture: &ReadServerFixture,
) -> Value {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return json!({});
    };
    let required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    Value::Object(
        required
            .into_iter()
            .filter_map(|name| {
                properties.get(name).map(|property_schema| {
                    (
                        name.to_owned(),
                        sample_schema_value(name, property_schema, descriptor, fixture),
                    )
                })
            })
            .collect(),
    )
}

async fn exercise_declared_surface(
    descriptor: &ToolDescriptor,
    surface: ToolSurface,
    fixture: &mut ReadServerFixture,
) -> Result<Observation, String> {
    fixture.ensure_daemon().await?;
    if descriptor.name == "_shutdown" {
        fixture.verify_daemon_marker()?;
    }

    let expect_refusal = matches!(
        expected_outcome(descriptor),
        ExpectedOutcome::Refusal { .. }
    );
    let binding_before = if expect_refusal {
        Some(
            fixture
                .workspace_binding_fingerprint()
                .await
                .ok_or_else(|| {
                    "cannot observe the isolated workspace binding before refusal probe".to_owned()
                })?,
        )
    } else {
        None
    };
    let files_before = fixture.filesystem_snapshot()?;

    let raw = match surface {
        ToolSurface::DirectIpc => {
            let arguments = request_parameters(descriptor, fixture);
            let has_declared_properties = descriptor
                .input_schema
                .get("properties")
                .and_then(Value::as_object)
                .is_some_and(|properties| !properties.is_empty());
            let params = has_declared_properties.then_some(arguments);
            invoke_direct_ipc(descriptor.name, params, fixture).await?
        }
        ToolSurface::Cli => invoke_cli(descriptor, fixture).await?,
        ToolSurface::StdioMcp => {
            invoke_stdio_mcp(
                descriptor.name,
                request_parameters(descriptor, fixture),
                fixture,
            )
            .await?
        }
    };

    if descriptor.name == "_shutdown" {
        wait_for_child_exit_or_keepalive(fixture).await?;
    }

    let files_after = fixture.filesystem_snapshot()?;
    let binding_after = if expect_refusal {
        fixture.workspace_binding_fingerprint().await
    } else {
        None
    };
    let filesystem_changes = changed_entry_count(&files_before, &files_after);
    let state_changes = match (binding_before, binding_after) {
        (Some(before), Some(after)) => usize::from(before != after),
        (Some(_), None) => 1,
        _ => 0,
    };
    let side_effect_count = filesystem_changes.saturating_add(state_changes);
    let generation = raw
        .response
        .as_ref()
        .and_then(|response| response.pointer("/provenance/generation_id"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(Observation {
        accepted: raw.accepted,
        refusal_code: raw.refusal_code,
        generation,
        side_effect_count,
        response: raw.response,
        raw_response: raw.raw_response,
        surface_advertised: raw.surface_advertised,
    })
}

async fn wait_for_child_exit_or_keepalive(fixture: &mut ReadServerFixture) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let Some(daemon) = fixture.daemon.as_mut() else {
            return Ok(());
        };
        if daemon
            .child
            .try_wait()
            .map_err(|error| format!("cannot inspect isolated daemon after _shutdown: {error}"))?
            .is_some()
            || Instant::now() >= deadline
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn invoke_direct_ipc(
    method: &str,
    params: Option<Value>,
    fixture: &ReadServerFixture,
) -> Result<RawObservation, String> {
    let response = fixture.send_ipc(method, params).await?;
    let result = response.result.clone();
    let error = response.error.as_ref();
    let response_value = result
        .clone()
        .or_else(|| error.and_then(|error| serde_json::to_value(error).ok()));
    let refusal_code = error
        .and_then(|error| error.data.as_ref())
        .and_then(|data| data.get("engram_code"))
        .and_then(value_as_u16);
    let raw_response = serde_json::to_string(&response)
        .unwrap_or_else(|error| format!("cannot serialize actual IPC response: {error}"));

    Ok(RawObservation {
        accepted: result.is_some() && error.is_none(),
        refusal_code,
        response: response_value,
        raw_response,
        surface_advertised: None,
    })
}

async fn invoke_cli(
    descriptor: &ToolDescriptor,
    fixture: &ReadServerFixture,
) -> Result<RawObservation, String> {
    let mut command = cli_arguments(descriptor, fixture)?;
    let workspace = fixture
        .workspace
        .to_str()
        .ok_or_else(|| "fixture path is not UTF-8".to_owned())?;
    let output = TokioCommand::new(&fixture.executable)
        .args(["--workspace", workspace, "--json"])
        .args(command.drain(..))
        .current_dir(&fixture.workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_WORKSPACE")
        .env_remove("ENGRAM_DIRECT")
        .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
        .env("ENGRAM_READY_TIMEOUT_MS", "10000")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| format!("cannot run real CLI surface: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let parsed = stdout
        .lines()
        .find_map(|line| serde_json::from_str::<Value>(line).ok());
    let payload = parsed
        .as_ref()
        .and_then(|value| value.get("result").or_else(|| value.get("error")).cloned());
    let refusal_code = parsed.as_ref().and_then(refusal_code_from);
    let raw_response = format!(
        "exit={:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status.code()
    );

    Ok(RawObservation {
        accepted: output.status.success()
            && parsed
                .as_ref()
                .is_some_and(|value| value.get("result").is_some()),
        refusal_code,
        response: payload,
        raw_response,
        surface_advertised: None,
    })
}

fn cli_arguments(
    descriptor: &ToolDescriptor,
    fixture: &ReadServerFixture,
) -> Result<Vec<String>, String> {
    let alternate = fixture.alternate_workspace.to_string_lossy().into_owned();
    let arguments = match descriptor.name {
        "set_workspace" => vec!["bind".to_owned(), alternate],
        "get_daemon_status" => vec!["daemon-status".to_owned()],
        "get_workspace_status" => vec!["workspace-status".to_owned()],
        "flush_state" => vec!["flush".to_owned()],
        "index_workspace" => vec!["index".to_owned()],
        "sync_workspace" => vec!["sync".to_owned()],
        "query_memory" => vec!["query-memory".to_owned(), "F54 fixture".to_owned()],
        "get_workspace_statistics" => vec!["stats".to_owned()],
        "map_code" => vec!["map-code".to_owned(), "F54FixtureSymbol".to_owned()],
        "list_symbols" => vec!["symbols".to_owned()],
        "unified_search" => vec!["search".to_owned(), "F54 fixture".to_owned()],
        "impact_analysis" => vec!["impact".to_owned(), "f54_fixture_root".to_owned()],
        "query_graph" => vec![
            "query-graph".to_owned(),
            "--operation".to_owned(),
            "neighborhood".to_owned(),
            "--root".to_owned(),
            "fn:f54-missing".to_owned(),
        ],
        "lint_dax" => vec!["lint-dax".to_owned()],
        "get_health_report" => vec!["health".to_owned()],
        "get_branch_metrics" => vec!["branch-metrics".to_owned()],
        "get_token_savings_report" => {
            vec!["report".to_owned(), "token-savings".to_owned()]
        }
        "get_evaluation_report" => vec!["report".to_owned(), "eval".to_owned()],
        "get_mutable_script_retry_metrics" => {
            vec!["report".to_owned(), "retry-metrics".to_owned()]
        }
        "run_retrieval_eval" => vec!["eval".to_owned()],
        "doctor --smoke" => vec!["doctor".to_owned(), "--smoke".to_owned()],
        other => {
            return Err(format!(
                "{F54_BLOCK_MARKER}: no real CLI invocation is available for descriptor \
                 '{other}' declared on the CLI surface"
            ));
        }
    };
    Ok(arguments)
}

async fn invoke_stdio_mcp(
    method: &str,
    arguments: Value,
    fixture: &ReadServerFixture,
) -> Result<RawObservation, String> {
    let workspace = fixture
        .workspace
        .to_str()
        .ok_or_else(|| "fixture path is not UTF-8".to_owned())?;
    let mut child = TokioCommand::new(&fixture.executable)
        .args(["shim", "--workspace", workspace])
        .current_dir(&fixture.workspace)
        .env_remove("ENGRAM_DATA_DIR")
        .env_remove("ENGRAM_WORKSPACE")
        .env_remove("ENGRAM_DIRECT")
        .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
        .env("ENGRAM_READY_TIMEOUT_MS", "10000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot launch real stdio MCP shim: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "stdio MCP shim has no piped stdin".to_owned())?;
    write_mcp_request_frames(&mut stdin, method, arguments).await?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("cannot reap naturally-exiting MCP shim: {error}"))?;
    Ok(decode_mcp_output(&output, method))
}

async fn write_mcp_request_frames(
    stdin: &mut tokio::process::ChildStdin,
    method: &str,
    arguments: Value,
) -> Result<(), String> {
    let frames = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "f54-parity-contract", "version": "1" }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": method, "arguments": arguments }
        }),
    ];
    for frame in frames {
        let mut line = serde_json::to_vec(&frame)
            .map_err(|error| format!("cannot encode MCP request: {error}"))?;
        line.push(b'\n');
        stdin
            .write_all(&line)
            .await
            .map_err(|error| format!("cannot write MCP stdio request: {error}"))?;
    }
    Ok(())
}

fn decode_mcp_output(output: &Output, method: &str) -> RawObservation {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let frames = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    let list_response = frames
        .iter()
        .find(|frame| frame.get("id") == Some(&json!(2)));
    let surface_advertised = list_response
        .and_then(|frame| frame.pointer("/result/tools"))
        .and_then(Value::as_array)
        .is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool.get("name").and_then(Value::as_str) == Some(method))
        });
    let call_response = frames
        .iter()
        .find(|frame| frame.get("id") == Some(&json!(3)));
    let tool_result = call_response.and_then(|frame| frame.get("result"));
    let payload = tool_result
        .and_then(|result| result.get("structuredContent"))
        .cloned()
        .or_else(|| {
            call_response
                .and_then(|frame| frame.pointer("/error/data"))
                .cloned()
        });
    let refusal_code = call_response.and_then(refusal_code_from);
    let catalog_names = list_response
        .and_then(|frame| frame.pointer("/result/tools"))
        .and_then(Value::as_array)
        .map_or_else(
            || "<tools/list unavailable>".to_owned(),
            |tools| {
                tools
                    .iter()
                    .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join(",")
            },
        );
    let call_frame = call_response.map_or_else(
        || "<tools/call response unavailable>".to_owned(),
        |frame| serde_json::to_string(frame).unwrap_or_else(|error| error.to_string()),
    );
    let raw_response = format!(
        "exit={:?}; advertised={surface_advertised}; catalog=[{catalog_names}]; \
         call={call_frame}; stderr={stderr}",
        output.status.code(),
    );

    RawObservation {
        accepted: output.status.success()
            && tool_result
                .is_some_and(|result| result.get("isError").and_then(Value::as_bool) != Some(true)),
        refusal_code,
        response: payload.or_else(|| call_response.cloned()),
        raw_response,
        surface_advertised: Some(surface_advertised),
    }
}

fn refusal_code_from(response: &Value) -> Option<u16> {
    [
        "/error/engram_code",
        "/error/code",
        "/error/data/engram_code",
        "/result/structuredContent/engram_code",
        "/result/structuredContent/data/engram_code",
    ]
    .iter()
    .find_map(|pointer| response.pointer(pointer).and_then(value_as_u16))
}

fn value_as_u16(value: &Value) -> Option<u16> {
    value.as_u64().and_then(|number| u16::try_from(number).ok())
}

fn snapshot_directory(
    root: &Path,
    directory: &Path,
    snapshot: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(std::io::Error::other)?
            .to_string_lossy()
            .into_owned();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            snapshot.insert(relative, vec![b'd']);
            snapshot_directory(root, &path, snapshot)?;
        } else if file_type.is_file() {
            let mut contents = vec![b'f'];
            let metadata = fs::symlink_metadata(&path)?;
            contents.extend(metadata.len().to_le_bytes());
            let modified = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)?;
            contents.extend(modified.as_secs().to_le_bytes());
            contents.extend(modified.subsec_nanos().to_le_bytes());

            let file_name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            let is_database_file = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("db"))
                || file_name.to_ascii_lowercase().contains(".db-");
            if !is_database_file {
                match fs::read(&path) {
                    Ok(bytes) => contents.extend(bytes),
                    Err(error) if is_file_lock_error(&error) => {}
                    Err(error) => return Err(error),
                }
            }
            snapshot.insert(relative, contents);
        } else if file_type.is_symlink() {
            let target = fs::read_link(&path)?;
            let mut contents = vec![b'l'];
            contents.extend(target.to_string_lossy().as_bytes());
            snapshot.insert(relative, contents);
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                snapshot.insert(
                    relative,
                    vec![if file_type.is_socket() { b's' } else { b'o' }],
                );
            }
            #[cfg(not(unix))]
            snapshot.insert(relative, vec![b'o']);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_file_lock_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(33)
}

#[cfg(not(windows))]
fn is_file_lock_error(_error: &std::io::Error) -> bool {
    false
}

fn changed_entry_count(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> usize {
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .count()
}

fn generation_read(descriptor: &ToolDescriptor) -> bool {
    descriptor.capability == CapabilityClass::Read
        && descriptor.read_server_available
        && descriptor.input_ownership == InputOwnership::DaemonHandler
}

fn expected_refusal_code(descriptor: &ToolDescriptor) -> u16 {
    match descriptor.name {
        "set_workspace" => F38_WORKSPACE_RETARGET_REFUSAL,
        // These calls use daemon dispatch on every exercised surface. The
        // specialized direct-sync code belongs to the separate --direct CLI
        // path, which this descriptor matrix does not invoke.
        _ => F38_READ_SERVER_WRITE_CONTROL_REFUSAL,
    }
}

fn expected_outcome(descriptor: &ToolDescriptor) -> ExpectedOutcome {
    if descriptor.capability == CapabilityClass::Control {
        ExpectedOutcome::Refusal {
            code: expected_refusal_code(descriptor),
        }
    } else if descriptor.name == "_health" {
        ExpectedOutcome::ReadyHealth
    } else if descriptor.input_ownership == InputOwnership::CliFrontend
        && descriptor.capability == CapabilityClass::Read
        && descriptor.read_server_available
    {
        ExpectedOutcome::SuccessfulWorkflow
    } else if generation_read(descriptor) {
        ExpectedOutcome::GenerationBackedRead
    } else {
        ExpectedOutcome::Refusal {
            code: expected_refusal_code(descriptor),
        }
    }
}

fn descriptor_parity_matrix(descriptors: &[ToolDescriptor]) -> Vec<DescriptorParityRow<'_>> {
    descriptors
        .iter()
        .map(|descriptor| {
            let outcome = expected_outcome(descriptor);
            DescriptorParityRow {
                descriptor,
                surfaces: descriptor
                    .surfaces
                    .iter()
                    .copied()
                    .map(|surface| DeclaredSurfaceExpectation { surface, outcome })
                    .collect(),
            }
        })
        .collect()
}

fn describe_failure(
    descriptor: &ToolDescriptor,
    surface: ToolSurface,
    observation: &Observation,
    expectation: &str,
) -> String {
    format!(
        "{} via {}: expected {}; accepted={}, refusal_code={:?}, generation={:?}, \
         side_effect_count={}, advertised={:?}, response={:?}, raw_response={}",
        descriptor.name,
        surface,
        expectation,
        observation.accepted,
        observation.refusal_code,
        observation.generation,
        observation.side_effect_count,
        observation.surface_advertised,
        observation.response,
        observation.raw_response
    )
}

fn record_matrix_failures(
    descriptor: &ToolDescriptor,
    expectation: DeclaredSurfaceExpectation,
    observation: &Observation,
    failures: &mut Vec<String>,
) {
    let surface = expectation.surface;
    if descriptor.name == "_health" && descriptor.surfaces != [ToolSurface::DirectIpc] {
        failures.push(format!(
            "{F54_RED_MARKER}: _health must be direct-IPC-only, declared {:?}",
            descriptor.surfaces
        ));
    }
    if surface == ToolSurface::StdioMcp && observation.surface_advertised != Some(true) {
        failures.push(format!(
            "{F54_RED_MARKER}: {} is declared on stdio MCP but was not advertised: {}",
            descriptor.name, observation.raw_response
        ));
    }

    match expectation.outcome {
        ExpectedOutcome::ReadyHealth => {
            let is_ready = observation
                .response
                .as_ref()
                .and_then(|response| response.get("status"))
                .and_then(Value::as_str)
                == Some("ready");
            if !observation.accepted || !is_ready {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "a ready direct-IPC health result",
                ));
            }
        }
        ExpectedOutcome::SuccessfulWorkflow => {
            // `doctor --smoke` is a declared, non-destructive CLI workflow
            // rather than a generation-backed read tool. Its smoke result
            // does not claim generation provenance.
            if !observation.accepted {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "a successful non-destructive read-server workflow",
                ));
            }
        }
        ExpectedOutcome::GenerationBackedRead => {
            if !observation.accepted {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "an accepted read result",
                ));
            }
            if observation.generation.as_deref() != Some(EXPECTED_GENERATION) {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "serving-generation provenance",
                ));
            }
        }
        ExpectedOutcome::Refusal { code } => {
            if observation.accepted {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "a read-server refusal",
                ));
            }
            if observation.refusal_code != Some(code) {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    &format!("stable F38 refusal code {code}"),
                ));
            }
            if observation.side_effect_count != 0 {
                failures.push(describe_failure(
                    descriptor,
                    surface,
                    observation,
                    "zero filesystem and workspace-binding side effects",
                ));
            }
        }
    }
}

#[test]
fn generated_matrix_structurally_matches_f19_descriptors_and_declared_surfaces() {
    let descriptors = all_descriptors();
    assert!(
        !descriptors.is_empty(),
        "{F54_RED_MARKER}: the F19 registry must declare descriptors"
    );
    let matrix = descriptor_parity_matrix(&descriptors);

    assert_eq!(
        matrix.len(),
        descriptors.len(),
        "{F54_RED_MARKER}: the generated matrix must have one row per F19 descriptor"
    );
    for (descriptor, row) in descriptors.iter().zip(&matrix) {
        assert!(
            std::ptr::eq(descriptor, row.descriptor),
            "{F54_RED_MARKER}: each F19 descriptor must appear exactly once in the matrix"
        );
        let matrix_surfaces = row
            .surfaces
            .iter()
            .map(|expectation| expectation.surface)
            .collect::<Vec<_>>();
        assert_eq!(
            matrix_surfaces, descriptor.surfaces,
            "{F54_RED_MARKER}: matrix surfaces for {} must exactly match its F19 declaration",
            descriptor.name
        );
    }
}

#[tokio::test]
async fn generated_matrix_exercises_only_declared_surfaces_and_checks_real_behavior() {
    let mut fixture = ReadServerFixture::new().await;
    fixture
        .ensure_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: {error}"));

    let descriptors = all_descriptors();
    let matrix = descriptor_parity_matrix(&descriptors);

    let expected_count = matrix.iter().map(|row| row.surfaces.len()).sum::<usize>();
    let mut cases = matrix
        .iter()
        .flat_map(|row| {
            row.surfaces
                .iter()
                .copied()
                .map(move |expectation| (row.descriptor, expectation))
        })
        .collect::<Vec<_>>();
    // A live `_shutdown` request must be last because an incorrect acceptance
    // would stop this marker-recorded test daemon.
    cases.sort_by_key(|(descriptor, _)| descriptor.name == "_shutdown");

    let mut failures = Vec::new();
    let mut exercised = 0;
    for (descriptor, expectation) in cases {
        let observation = exercise_declared_surface(descriptor, expectation.surface, &mut fixture)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{F54_BLOCK_MARKER}: cannot exercise {} via {}: {error}",
                    descriptor.name, expectation.surface
                )
            });
        exercised += 1;
        record_matrix_failures(descriptor, expectation, &observation, &mut failures);
    }

    assert_eq!(
        exercised, expected_count,
        "{F54_RED_MARKER}: every descriptor surface declaration must be exercised"
    );
    fixture
        .stop_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: safe daemon cleanup failed: {error}"));
    assert!(
        failures.is_empty(),
        "{F54_RED_MARKER}: descriptor-driven read-server parity failures:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn control_descriptors_are_refused_without_side_effects() {
    let descriptors = all_descriptors();
    let matrix = descriptor_parity_matrix(&descriptors);
    let control_rows = matrix
        .into_iter()
        .filter(|row| row.descriptor.capability == CapabilityClass::Control)
        .collect::<Vec<_>>();
    assert!(
        !control_rows.is_empty(),
        "{F54_RED_MARKER}: the F19 registry must declare Control descriptors"
    );

    for row in &control_rows {
        let code = expected_refusal_code(row.descriptor);
        assert_eq!(
            expected_outcome(row.descriptor),
            ExpectedOutcome::Refusal { code },
            "{F54_RED_MARKER}: Control descriptor {} must be refusal-only",
            row.descriptor.name
        );
        if row.descriptor.name == "set_workspace" {
            assert_eq!(
                code, F38_WORKSPACE_RETARGET_REFUSAL,
                "{F54_RED_MARKER}: set_workspace must retain its retarget refusal code"
            );
        } else if row.descriptor.read_server_available {
            assert_eq!(
                code, F38_READ_SERVER_WRITE_CONTROL_REFUSAL,
                "{F54_RED_MARKER}: read-server-available Control descriptors must use the \
                 write/control refusal code"
            );
        }
    }

    let mut cases = control_rows
        .iter()
        .flat_map(|row| {
            row.surfaces
                .iter()
                .copied()
                .map(move |expectation| (row.descriptor, expectation))
        })
        .collect::<Vec<_>>();
    // Keep `_shutdown` last in case the daemon incorrectly accepts it.
    cases.sort_by_key(|(descriptor, _)| descriptor.name == "_shutdown");

    let mut fixture = ReadServerFixture::new().await;
    fixture
        .ensure_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: {error}"));
    let mut failures = Vec::new();
    for (descriptor, expectation) in cases {
        let observation = exercise_declared_surface(descriptor, expectation.surface, &mut fixture)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{F54_BLOCK_MARKER}: cannot exercise Control descriptor {} via {}: {error}",
                    descriptor.name, expectation.surface
                )
            });
        record_matrix_failures(descriptor, expectation, &observation, &mut failures);
    }

    fixture
        .stop_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: safe daemon cleanup failed: {error}"));
    assert!(
        failures.is_empty(),
        "{F54_RED_MARKER}: Control descriptor refusal failures:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
async fn unknown_ipc_methods_are_refused_without_side_effects() {
    let mut fixture = ReadServerFixture::new().await;
    fixture
        .ensure_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: {error}"));
    let observation = exercise_unknown_method(
        "not_a_declared_method",
        ToolSurface::DirectIpc,
        &mut fixture,
    )
    .await
    .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: unknown IPC call failed: {error}"));
    fixture
        .stop_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: safe daemon cleanup failed: {error}"));

    let failures = [
        observation
            .accepted
            .then_some("unknown method was accepted instead of refused".to_owned()),
        (observation.refusal_code != Some(F38_READ_SERVER_WRITE_CONTROL_REFUSAL)).then_some(
            "unknown method did not use read-server write/control refusal code 16_001".to_owned(),
        ),
        (observation.side_effect_count != 0).then_some(format!(
            "unknown method changed isolated state {} times",
            observation.side_effect_count
        )),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "{F54_RED_MARKER}: {}: {}; response={:?}; raw_response={}",
        "not_a_declared_method",
        failures.join("; "),
        observation.response,
        observation.raw_response
    );
}

async fn exercise_unknown_method(
    method: &str,
    surface: ToolSurface,
    fixture: &mut ReadServerFixture,
) -> Result<Observation, String> {
    if surface != ToolSurface::DirectIpc {
        return Err(format!(
            "{F54_BLOCK_MARKER}: unknown methods have no descriptor; exercise direct IPC only"
        ));
    }
    fixture.ensure_daemon().await?;
    let binding_before = fixture
        .workspace_binding_fingerprint()
        .await
        .ok_or_else(|| "cannot observe workspace binding before unknown-method probe".to_owned())?;
    let files_before = fixture.filesystem_snapshot()?;
    let raw = invoke_direct_ipc(method, None, fixture).await?;
    let files_after = fixture.filesystem_snapshot()?;
    let binding_after = fixture.workspace_binding_fingerprint().await;
    let effects =
        changed_entry_count(&files_before, &files_after).saturating_add(match binding_after {
            Some(binding) => usize::from(binding_before != binding),
            None => 1,
        });

    Ok(Observation {
        accepted: raw.accepted,
        refusal_code: raw.refusal_code,
        generation: None,
        side_effect_count: effects,
        response: raw.response,
        raw_response: raw.raw_response,
        surface_advertised: None,
    })
}

fn normalize_parity_result(result: &Value) -> Value {
    let mut normalized = result.clone();
    if let Some(object) = normalized.as_object_mut() {
        object.remove("connection_count");
    }
    normalized
}

#[tokio::test]
async fn cli_and_stdio_mcp_return_equivalent_results_for_the_same_declared_read() {
    let mut fixture = ReadServerFixture::new().await;
    fixture
        .ensure_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: {error}"));
    let read_descriptor = descriptor("get_workspace_status").unwrap_or_else(|| {
        panic!("{F54_RED_MARKER}: get_workspace_status must be declared in F19")
    });
    if !read_descriptor.supports(ToolSurface::Cli)
        || !read_descriptor.supports(ToolSurface::StdioMcp)
    {
        panic!(
            "{F54_RED_MARKER}: parity fixture requires declared CLI and MCP surfaces, got {:?}",
            read_descriptor.surfaces
        );
    }

    let cli = exercise_declared_surface(&read_descriptor, ToolSurface::Cli, &mut fixture)
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: CLI read failed to run: {error}"));
    let mcp = exercise_declared_surface(&read_descriptor, ToolSurface::StdioMcp, &mut fixture)
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: MCP read failed to run: {error}"));
    fixture
        .stop_daemon()
        .await
        .unwrap_or_else(|error| panic!("{F54_BLOCK_MARKER}: safe daemon cleanup failed: {error}"));

    let mut failures = Vec::new();
    if !cli.accepted || !mcp.accepted {
        failures.push(format!(
            "both shipped read surfaces must accept get_workspace_status; CLI={cli:?}; MCP={mcp:?}"
        ));
    }
    if cli.generation.as_deref() != Some(EXPECTED_GENERATION) {
        failures.push(format!(
            "CLI must report serving-generation provenance; observed {:?}",
            cli.generation
        ));
    }
    if mcp.generation.as_deref() != Some(EXPECTED_GENERATION) {
        failures.push(format!(
            "MCP must report serving-generation provenance; observed {:?}",
            mcp.generation
        ));
    }
    if cli.response.as_ref().map(normalize_parity_result)
        != mcp.response.as_ref().map(normalize_parity_result)
    {
        failures.push(format!(
            "CLI and stdio MCP read results differ; CLI={:?}; MCP={:?}",
            cli.response.as_ref().map(normalize_parity_result),
            mcp.response.as_ref().map(normalize_parity_result)
        ));
    }
    assert!(
        failures.is_empty(),
        "{F54_RED_MARKER}: CLI/MCP read parity failures: {}",
        failures.join("; ")
    );
}
