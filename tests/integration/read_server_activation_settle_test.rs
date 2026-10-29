//! Integration coverage for PRE-3F (142.063-T): a generation published before
//! spawn is activated at startup, the activation settles, and the settled
//! server is quiescent.

#![forbid(unsafe_code)]

#[path = "../helpers/activation_settle.rs"]
mod activation_settle;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use activation_settle::{Settled, binding_fingerprint, filesystem_snapshot, send_ipc};
use chrono::Utc;
use engram::daemon::ipc_server::ipc_endpoint;
use engram::services::generations::candidate_build::read_server_layout;
use engram::services::generations::{
    BranchIdentity, GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision,
    GenerationStore, ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory,
    WorkspaceIdentity, publish_generation_manifest,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const PRE3F_RED_MARKER: &str = "PRE3F-RED";
const PRE3F_BLOCK_MARKER: &str = "PRE3F-BLOCK";
const EXPECTED_GENERATION: &str = "generation-142";
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const QUIET_WINDOW: Duration = Duration::from_millis(4500);

type SetupResult<T> = Result<T, Box<dyn Error>>;

struct SettleFixture {
    temporary_root: TempDir,
    workspace: PathBuf,
    endpoint: String,
    daemon: Option<Child>,
}

impl SettleFixture {
    async fn new() -> SetupResult<Self> {
        let temporary_root = tempfile::tempdir()?;
        let root = temporary_root.path().canonicalize()?;
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize()?;
        if root.starts_with(&repository) || repository.starts_with(&root) {
            return Err("temporary fixture overlaps the repository".into());
        }
        let workspace = root.join("workspace");
        create_workspace(&workspace)?;
        let workspace = workspace.canonicalize()?;
        let endpoint = ipc_endpoint(&workspace).map_err(|error| error.to_string())?;
        fs::create_dir_all(workspace.join("src"))?;
        fs::write(
            workspace.join("src/pre3f_fixture.rs"),
            "pub fn pre3f_root() {}\n",
        )?;
        let graph = engram::models::config::CodeGraphConfig::default();
        engram::services::code_graph::sync_workspace(
            &workspace,
            &workspace.join(".engram"),
            "main",
            &graph,
        )
        .await?;
        publish_fixture_generation(&workspace)?;
        Ok(Self {
            temporary_root,
            workspace,
            endpoint,
            daemon: None,
        })
    }

    fn spawn_daemon(&mut self) {
        let child = Command::new(env!("CARGO_BIN_EXE_engram"))
            .args(["daemon", "--workspace"])
            .arg(&self.workspace)
            .current_dir(&self.workspace)
            .env_remove("ENGRAM_DATA_DIR")
            .env_remove("ENGRAM_WORKSPACE")
            .env_remove("ENGRAM_DIRECT")
            .env("ENGRAM_IDLE_TIMEOUT_MS", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| block("spawn daemon", &error));
        self.daemon = Some(child);
    }

    async fn stop_daemon(&mut self) -> SetupResult<()> {
        let Some(child) = self.daemon.as_mut() else {
            return Ok(());
        };
        let _ = send_ipc(&self.endpoint, "_shutdown").await; // may exit first
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        self.daemon = None;
        Ok(())
    }
}

impl Drop for SettleFixture {
    fn drop(&mut self) {
        if let Some(child) = self.daemon.as_mut()
            && matches!(child.try_wait(), Ok(None))
            && (child.kill().is_err() || child.wait().is_err())
        {
            eprintln!("{PRE3F_BLOCK_MARKER}: failed to stop or reap the fixture child");
        }
    }
}

fn block(what: &str, error: &dyn std::fmt::Display) -> ! {
    panic!("{PRE3F_BLOCK_MARKER}: {what}: {error}")
}

fn create_workspace(path: &Path) -> SetupResult<()> {
    let engram_dir = path.join(".engram");
    let metrics_dir = engram_dir.join("metrics").join("main");
    fs::create_dir_all(path.join(".git"))?;
    fs::create_dir_all(&metrics_dir)?;
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n")?;
    fs::write(engram_dir.join("config.toml"), "mode = \"read_server\"\n")?;
    let usage_event = r#"{"tool_name":"pre3f_fixture","timestamp":"2026-09-01T00:00:00Z","response_bytes":0,"estimated_tokens":0,"symbols_returned":0,"results_returned":0,"branch":"main"}"#;
    fs::write(metrics_dir.join("usage.jsonl"), format!("{usage_event}\n"))?;
    Ok(())
}

fn publish_fixture_generation(workspace: &Path) -> SetupResult<()> {
    let layout = read_server_layout(&workspace.to_string_lossy())?;
    let generations_root = workspace.join(".engram").join("generations");
    let database_dir = generations_root.join(EXPECTED_GENERATION);
    fs::create_dir_all(&database_dir)?;
    let database_path = database_dir.join("engram.db");
    {
        let path = database_path.to_string_lossy();
        let database = cozo::DbInstance::new("sqlite", path.as_ref(), "")
            .map_err(|error| error.to_string())?;
        for query in [
            ":create probe_row {id => val}",
            "?[id, val] <- [[1, 'probe']] :put probe_row {id => val}",
        ] {
            database.run_default(query).map_err(|e| e.to_string())?;
        }
    }
    let database_bytes = fs::read(&database_path)?;
    let manifest = GenerationManifest::new(
        GenerationId::new(EXPECTED_GENERATION)?,
        GenerationRevision::new(1),
        SUPPORTED_MANIFEST_SCHEMA_VERSION,
        BranchIdentity::new(&layout.branch, None),
        WorkspaceIdentity::new(layout.workspace_id.clone()),
        SealedInventory::new(vec![ManifestFileDigest::new(
            format!("{EXPECTED_GENERATION}/engram.db"),
            hex::encode(Sha256::digest(&database_bytes)),
        )]),
        GenerationProvenance::new("pre3f-fixture", Utc::now()),
    );
    publish_generation_manifest(&GenerationStore::new(&generations_root)?, &manifest)?;
    Ok(())
}

async fn status_generation(endpoint: &str) -> Value {
    let response = send_ipc(endpoint, "get_workspace_status").await;
    response
        .ok()
        .and_then(|response| response.result)
        .and_then(|result| result.get("generation").cloned())
        .unwrap_or(Value::Null)
}

async fn wait_ready_with_generation(fixture: &SettleFixture) {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        let health = send_ipc(&fixture.endpoint, "_health").await;
        let status = health.ok().and_then(|response| response.result);
        if status.is_some_and(|r| r.get("status").and_then(Value::as_str) == Some("ready")) {
            break;
        }
        assert!(Instant::now() < deadline, "PRE3F-BLOCK: not ready");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(
        !status_generation(&fixture.endpoint).await.is_null(),
        "{PRE3F_RED_MARKER}: (a) generation not installed before readiness (PRE-3)"
    );
}

async fn assert_quiescent(fixture: &SettleFixture) {
    let root = fixture.temporary_root.path();
    let snapshot = || filesystem_snapshot(root).unwrap_or_else(|error| block("snapshot", &error));
    let f1 = binding_fingerprint(&fixture.endpoint).await;
    let s1 = snapshot();
    tokio::time::sleep(QUIET_WINDOW).await;
    let s2 = snapshot();
    let f2 = binding_fingerprint(&fixture.endpoint).await;
    assert_eq!(s1, s2, "{PRE3F_RED_MARKER}: (d) filesystem not quiescent");
    assert_eq!(f1, f2, "{PRE3F_RED_MARKER}: (d) binding not quiescent");
}

#[tokio::test]
async fn published_fixture_generation_activates_at_startup_and_quiesces() {
    let mut fixture = SettleFixture::new()
        .await
        .unwrap_or_else(|e| block("fixture", &e));
    fixture.spawn_daemon();
    wait_ready_with_generation(&fixture).await;
    let settled = activation_settle::await_activation_settled(
        &fixture.endpoint,
        fixture.temporary_root.path(),
        1,
    )
    .await
    .unwrap_or_else(|error| panic!("{PRE3F_RED_MARKER}: (b) activation did not settle: {error}"));
    assert_eq!(settled, Settled::Active, "{PRE3F_RED_MARKER}: (b)");
    let generation = status_generation(&fixture.endpoint).await;
    assert_eq!(generation["active_revision"], 1, "PRE3F-RED: (c)");
    let runtime = fixture.workspace.join(".engram/generations/runtime");
    let copy = runtime.join(EXPECTED_GENERATION).join("engram.db");
    assert!(copy.is_file(), "{PRE3F_RED_MARKER}: (c) no runtime copy");
    assert_quiescent(&fixture).await;
    fixture
        .stop_daemon()
        .await
        .unwrap_or_else(|error| block("stop daemon", &error));
}
