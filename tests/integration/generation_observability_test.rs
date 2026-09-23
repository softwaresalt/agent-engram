#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::generations::{
    BranchIdentity, ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationActivator,
    GenerationId, GenerationManifest, GenerationProvenance, GenerationRevision, GenerationStore,
    ManifestFileDigest, SUPPORTED_MANIFEST_SCHEMA_VERSION, SealedInventory, WorkspaceIdentity,
};
use engram::tools;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SERVED_BRANCH: &str = "served-branch";
const LIVE_BRANCH: &str = "live-branch";
const HARNESS_WORKSPACE: &str = "workspace-generation-observability";
const PLACEHOLDER_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const ACTIVATION_DEADLINE: Duration = Duration::from_secs(42);

struct Fixture {
    workspace: TempDir,
    data_dir: TempDir,
    generations_root: TempDir,
    runtime_root: TempDir,
}

#[derive(Clone, Copy)]
struct SeededGeneration {
    bytes: u64,
    sealed_inventory_bytes: u64,
}

impl Fixture {
    fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        fs::create_dir(workspace.path().join(".git")).expect("create .git");
        fs::create_dir_all(workspace.path().join(".engram")).expect("create .engram");
        Self {
            workspace,
            data_dir: tempfile::tempdir().expect("data_dir tempdir"),
            generations_root: tempfile::tempdir().expect("generations root tempdir"),
            runtime_root: tempfile::tempdir().expect("runtime root tempdir"),
        }
    }

    fn store(&self) -> GenerationStore {
        GenerationStore::new(self.generations_root.path()).expect("generation store")
    }

    fn activator(&self) -> Arc<GenerationActivator> {
        Arc::new(GenerationActivator::new(
            self.store(),
            self.runtime_root.path(),
            ExpectedIdentity::new(SERVED_BRANCH, HARNESS_WORKSPACE),
            ACTIVATION_DEADLINE,
        ))
    }

    fn seed_generation(&self, label: &str) -> SeededGeneration {
        self.seed_generation_with_extra_artifacts(label, &[])
    }

    fn seed_generation_with_extra_artifacts(
        &self,
        label: &str,
        extra_artifacts: &[(&str, &[u8])],
    ) -> SeededGeneration {
        let dir = self.generations_root.path().join(label);
        fs::create_dir_all(&dir).expect("generation directory");
        let db_path = dir.join(GENERATION_DATABASE_FILE_NAME);
        {
            let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
                .expect("create db");
            db.run_default(":create probe_row {id => val}")
                .expect("create relation");
            db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
                .expect("seed row");
        }
        let bytes = fs::metadata(&db_path).expect("db metadata").len();
        let sealed_inventory_bytes =
            extra_artifacts
                .iter()
                .fold(bytes, |total, (relative_path, contents)| {
                    let artifact_path = dir.join(relative_path);
                    if let Some(parent) = artifact_path.parent() {
                        fs::create_dir_all(parent).expect("extra artifact parent directory");
                    }
                    fs::write(&artifact_path, contents).expect("write extra artifact");
                    total
                        + fs::metadata(&artifact_path)
                            .expect("extra artifact metadata")
                            .len()
                });
        SeededGeneration {
            bytes,
            sealed_inventory_bytes,
        }
    }

    fn publish_valid_revision(&self, label: &str, revision: u64) -> SeededGeneration {
        self.publish_valid_revision_with_extra_artifacts(label, revision, &[])
    }

    fn publish_valid_revision_with_extra_artifacts(
        &self,
        label: &str,
        revision: u64,
        extra_artifacts: &[(&str, &[u8])],
    ) -> SeededGeneration {
        let seeded = self.seed_generation_with_extra_artifacts(label, extra_artifacts);
        let mut inventory = vec![ManifestFileDigest::new(
            format!("{label}/{GENERATION_DATABASE_FILE_NAME}"),
            sha256_hex(
                &self
                    .generations_root
                    .path()
                    .join(label)
                    .join(GENERATION_DATABASE_FILE_NAME),
            ),
        )];
        for (relative_path, _contents) in extra_artifacts {
            let artifact_path = self.generations_root.path().join(label).join(relative_path);
            inventory.push(ManifestFileDigest::new(
                format!("{label}/{relative_path}"),
                sha256_hex(&artifact_path),
            ));
        }
        self.publish_manifest_with_inventory(label, revision, inventory);
        seeded
    }

    fn publish_invalid_revision(&self, label: &str, revision: u64) -> SeededGeneration {
        let seeded = self.seed_generation(label);
        self.publish_manifest(label, revision, PLACEHOLDER_DIGEST);
        seeded
    }

    fn publish_manifest(&self, label: &str, revision: u64, digest: &str) {
        self.publish_manifest_with_inventory(
            label,
            revision,
            vec![ManifestFileDigest::new(
                format!("{label}/{GENERATION_DATABASE_FILE_NAME}"),
                digest,
            )],
        );
    }

    fn publish_manifest_with_inventory(
        &self,
        label: &str,
        revision: u64,
        inventory: Vec<ManifestFileDigest>,
    ) {
        let manifest = GenerationManifest::new(
            GenerationId::new(label).expect("fixture generation id"),
            GenerationRevision::new(revision),
            SUPPORTED_MANIFEST_SCHEMA_VERSION,
            BranchIdentity::new(SERVED_BRANCH, Some(format!("source-rev-{revision}"))),
            WorkspaceIdentity::new(HARNESS_WORKSPACE),
            SealedInventory::new(inventory),
            GenerationProvenance::new("generation-observability-harness", fixture_created_at()),
        );
        fs::write(
            self.generations_root.path().join("active.json"),
            serde_json::to_vec(&manifest).expect("serialize manifest"),
        )
        .expect("write active manifest");
    }

    fn workspace_snapshot(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: HARNESS_WORKSPACE.to_owned(),
            workspace_uuid: "workspace-generation-observability-uuid".to_owned(),
            branch: LIVE_BRANCH.to_owned(),
            data_dir: self.data_dir.path().to_path_buf(),
            path: self.workspace.path().display().to_string(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        }
    }
}

fn fixture_created_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-09-20T00:00:00Z")
        .expect("fixture timestamp literal must parse")
        .with_timezone(&Utc)
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file for digest");
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

fn capture_file_inventory(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut inventory = BTreeMap::new();
    capture_file_inventory_recursive(root, root, &mut inventory);
    inventory
}

fn capture_file_inventory_recursive(
    base: &Path,
    path: &Path,
    inventory: &mut BTreeMap<String, Vec<u8>>,
) {
    let mut entries = fs::read_dir(path)
        .expect("read_dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry in entries {
        if entry.is_dir() {
            capture_file_inventory_recursive(base, &entry, inventory);
        } else {
            let relative = entry
                .strip_prefix(base)
                .expect("relative path")
                .display()
                .to_string();
            inventory.insert(relative, fs::read(&entry).expect("read inventory file"));
        }
    }
}

fn generation_report(status: &Value) -> &serde_json::Map<String, Value> {
    status
        .get("generation")
        .and_then(Value::as_object)
        .expect("generation report present")
}

fn object_field<'a>(
    parent: &'a serde_json::Map<String, Value>,
    field: &str,
) -> &'a serde_json::Map<String, Value> {
    parent
        .get(field)
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{field} object present"))
}

async fn bound_state(fixture: &Fixture, activator: Arc<GenerationActivator>) -> Arc<AppState> {
    let state = Arc::new(AppState::with_mode(
        DaemonMode::ReadServer,
        1,
        StaleStrategy::Warn,
        20,
        60,
    ));
    state
        .set_workspace_and_config(
            fixture.workspace_snapshot(),
            Some(WorkspaceConfig::default()),
        )
        .await
        .expect("bind workspace snapshot");
    state.set_generation_activator(Some(activator)).await;
    state
}

fn assert_generation_observability(
    status: &Value,
    first: SeededGeneration,
    second: SeededGeneration,
) {
    let generation = generation_report(status);
    assert_eq!(
        generation.get("active_revision").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        generation.get("published_revision").and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        generation
            .get("last_failed_revision")
            .and_then(Value::as_u64),
        Some(3)
    );

    let branch_divergence = object_field(generation, "branch_divergence");
    assert_eq!(
        branch_divergence
            .get("served_branch")
            .and_then(Value::as_str),
        Some(SERVED_BRANCH)
    );
    assert_eq!(
        branch_divergence
            .get("current_branch")
            .and_then(Value::as_str),
        Some(LIVE_BRANCH)
    );
    assert_eq!(
        branch_divergence.get("diverged").and_then(Value::as_bool),
        Some(true)
    );

    let retained_runtime_copies = object_field(generation, "retained_runtime_copies");
    assert_eq!(
        retained_runtime_copies.get("count").and_then(Value::as_u64),
        Some(2)
    );

    let retained_contexts = object_field(generation, "retained_contexts");
    assert_eq!(
        retained_contexts.get("count").and_then(Value::as_u64),
        Some(2)
    );

    let activation_deadlines = object_field(generation, "activation_deadlines");
    assert_eq!(
        activation_deadlines
            .get("configured_ms")
            .and_then(Value::as_u64),
        Some(42_000)
    );

    let disk_usage_bytes = object_field(generation, "disk_usage_bytes");
    assert_eq!(
        disk_usage_bytes
            .get("active_generation")
            .and_then(Value::as_u64),
        Some(second.bytes)
    );
    assert_eq!(
        disk_usage_bytes
            .get("retained_runtime_copies")
            .and_then(Value::as_u64),
        Some(first.bytes + second.bytes)
    );
}

fn assert_inventory_unchanged(
    fixture: &Fixture,
    generations_before: &BTreeMap<String, Vec<u8>>,
    runtime_before: &BTreeMap<String, Vec<u8>>,
) {
    let generations_after = capture_file_inventory(fixture.generations_root.path());
    let runtime_after = capture_file_inventory(fixture.runtime_root.path());
    assert_eq!(generations_after, *generations_before);
    assert_eq!(runtime_after, *runtime_before);
}

#[tokio::test]
async fn workspace_status_reports_runtime_copy_bytes_not_whole_inventory_bytes() {
    let fixture = Fixture::new();
    let activator = fixture.activator();
    let state = bound_state(&fixture, Arc::clone(&activator)).await;

    let seeded = fixture.publish_valid_revision_with_extra_artifacts(
        "gen-observe-extra",
        1,
        &[("sealed/notes.bin", b"extra sealed artifact bytes")],
    );
    assert!(
        seeded.sealed_inventory_bytes > seeded.bytes,
        "the extra sealed artifact must make the inventory larger than the copied database"
    );

    activator
        .activate_initial()
        .await
        .expect("initial activation must succeed");

    let status = tools::dispatch(Arc::clone(&state), "get_workspace_status", None)
        .await
        .expect("get_workspace_status should succeed");
    let generation = generation_report(&status);
    let retained_runtime_copies = object_field(generation, "retained_runtime_copies");
    assert_eq!(
        retained_runtime_copies.get("count").and_then(Value::as_u64),
        Some(1)
    );

    let disk_usage_bytes = object_field(generation, "disk_usage_bytes");
    assert_eq!(
        disk_usage_bytes
            .get("active_generation")
            .and_then(Value::as_u64),
        Some(seeded.bytes)
    );
    assert_eq!(
        disk_usage_bytes
            .get("retained_runtime_copies")
            .and_then(Value::as_u64),
        Some(seeded.bytes)
    );
}

#[tokio::test]
async fn workspace_status_reports_generation_observability_without_deleting_anything() {
    let fixture = Fixture::new();
    let activator = fixture.activator();
    let state = bound_state(&fixture, Arc::clone(&activator)).await;

    let first = fixture.publish_valid_revision("gen-observe-1", 1);
    let retired_context = activator
        .activate_initial()
        .await
        .expect("initial activation must succeed");

    let second = fixture.publish_valid_revision("gen-observe-2", 2);
    let activated = activator
        .maybe_activate_newer()
        .await
        .expect("newer revision activation must succeed");
    assert!(activated.is_some(), "revision 2 must activate");
    drop(activated);

    let _failed = fixture.publish_invalid_revision("gen-observe-3", 3);
    let failure = activator
        .maybe_activate_newer()
        .await
        .expect_err("invalid digest must record a failed revision");
    assert!(failure.to_string().contains("digest"));

    let generations_before = capture_file_inventory(fixture.generations_root.path());
    let runtime_before = capture_file_inventory(fixture.runtime_root.path());

    let status = tools::dispatch(Arc::clone(&state), "get_workspace_status", None)
        .await
        .expect("get_workspace_status should succeed");
    assert_generation_observability(&status, first, second);
    assert_inventory_unchanged(&fixture, &generations_before, &runtime_before);
    assert_eq!(retired_context.generation_id().as_str(), "gen-observe-1");
}
