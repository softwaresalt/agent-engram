//! Read-server layout, identity, generation-ID minting, and candidate build.
//!
//! This is the single derivation of the read-server identity, generations
//! root, runtime root, and activation deadline. It also builds a candidate
//! generation (`<root>/<id>/engram.db`) and seals its one-entry inventory.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use thiserror::Error;

use super::{
    ExpectedIdentity, GenerationId, GenerationIdError, GenerationStore, IndexTargetKind,
    ManifestFileDigest, PublishError, StoreError,
};
use crate::db::workspace::{canonicalize_workspace, resolve_git_branch, workspace_hash};
use crate::errors::{EngramError, WorkspaceError};
use crate::models::config::CodeGraphConfig;
use crate::services::code_graph::index_workspace;
use crate::services::file_tracker::compute_file_hash;

/// Errors from candidate-generation build, seal, and publish preparation.
///
/// The variant list is closed by the plan; a new variant needs a plan
/// revision. No variant carries a path, environment value, or payload bytes
/// of its own (wrapped source errors keep their own messages).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CandidateBuildError {
    /// The generation ID is reserved for the runtime-copy directory.
    #[error("generation ID is reserved for the runtime copy")]
    ReservedGenerationId,
    /// The sealed target was not an exclusive candidate directory.
    #[error("sealed target is not an exclusive candidate")]
    CandidateNotExclusive,
    /// Exactly one `engram.db` was required under the candidate `cozo/` tree.
    #[error("expected exactly one engram.db under the candidate, found {count}")]
    DatabaseNotFound {
        /// Number of `engram.db` files found.
        count: usize,
    },
    /// A `-wal`, `-shm`, or `-journal` sidecar remained beside the database.
    #[error("uncheckpointed database sidecar present")]
    UncheckpointedSidecar,
    /// The database rename stayed blocked by a sharing violation.
    #[error("database rename remained busy after bounded retry")]
    RenameBusy,
    /// A generations path escaped the workspace containment.
    #[error("generations path escapes workspace containment")]
    ContainmentEscape,
    /// The generations store is absent and creation was not requested.
    #[error("generations store is absent")]
    StoreAbsent,
    /// The active manifest could not be read or parsed.
    #[error("active manifest is unreadable")]
    ManifestUnreadable,
    /// The next publication revision overflowed.
    #[error("publication revision overflow")]
    RevisionOverflow,
    /// Indexing into the candidate failed.
    #[error("candidate indexing failed")]
    Index(#[source] EngramError),
    /// A generation-store operation failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// A publication operation failed.
    #[error(transparent)]
    Publish(#[from] PublishError),
    /// A filesystem operation failed.
    #[error("i/o failure during {op}: {kind:?}")]
    Io {
        /// Static operation label.
        op: &'static str,
        /// Kind of the underlying I/O error.
        kind: std::io::ErrorKind,
    },
}

/// A candidate whose database was built and renamed into place.
#[derive(Debug)]
pub struct BuiltCandidate {
    id: GenerationId,
    root: PathBuf,
}

/// A built candidate with its single-entry sealed inventory computed.
#[derive(Debug)]
pub struct SealedCandidate {
    id: GenerationId,
    inventory: Vec<ManifestFileDigest>,
}

/// Deadline for read-server generation activation.
pub const READ_SERVER_ACTIVATION_DEADLINE: Duration = Duration::from_secs(60);

/// Derived read-server layout; only [`read_server_layout`] constructs it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReadServerLayout {
    /// Canonical workspace root, exactly as `canonicalize_workspace` returned.
    pub workspace: PathBuf,
    /// Resolved branch, or `default` when it cannot be resolved.
    pub branch: String,
    /// Workspace hash for `workspace` and `branch`.
    pub workspace_id: String,
    /// `<workspace>/.engram/generations`.
    pub generations_root: PathBuf,
    /// `<generations_root>/runtime`.
    pub runtime_root: PathBuf,
    /// Activation deadline, always [`READ_SERVER_ACTIVATION_DEADLINE`].
    pub activation_deadline: Duration,
}

impl ReadServerLayout {
    /// Identity activation must match, built from the layout's own fields.
    #[must_use]
    pub fn expected_identity(&self) -> ExpectedIdentity {
        ExpectedIdentity::new(self.branch.clone(), self.workspace_id.clone())
    }
}

/// Derive the read-server layout for `workspace` without any writes.
///
/// # Errors
///
/// Returns [`WorkspaceError`] from workspace canonicalization.
pub fn read_server_layout(workspace: &str) -> Result<ReadServerLayout, WorkspaceError> {
    // `canonicalize_workspace` reports a missing path as `NotGitRoot` (the
    // capability anchor fails first), so the missing case is classified here.
    // Only a definite NotFound is classified here; other metadata errors
    // (e.g. permissions) fall through to canonicalization.
    if let Err(err) = std::fs::metadata(workspace)
        && err.kind() == std::io::ErrorKind::NotFound
    {
        return Err(WorkspaceError::NotFound {
            path: workspace.to_owned(),
        });
    }
    let workspace = canonicalize_workspace(workspace)?;
    let branch = resolve_git_branch(&workspace).unwrap_or_else(|_| "default".to_owned());
    let workspace_id = workspace_hash(&workspace, &branch);
    let generations_root = workspace.join(".engram").join("generations");
    let runtime_root = generations_root.join("runtime");
    Ok(ReadServerLayout {
        workspace,
        branch,
        workspace_id,
        generations_root,
        runtime_root,
        activation_deadline: READ_SERVER_ACTIVATION_DEADLINE,
    })
}

/// Mint a pure `preflight-<unix_ms>-<pid>` generation ID.
///
/// A clock before the Unix epoch yields `0` for the millisecond component.
/// Uniqueness within one process requires distinct timestamps: identical
/// `(now, pid)` inputs always produce identical IDs.
///
/// # Errors
///
/// Returns [`GenerationIdError`] if the minted value fails validation.
pub fn mint_generation_id(now: SystemTime) -> Result<GenerationId, GenerationIdError> {
    let unix_ms = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    GenerationId::new(format!("preflight-{unix_ms}-{}", std::process::id()))
}

impl BuiltCandidate {
    /// Generation ID of the built candidate.
    #[must_use]
    pub const fn id(&self) -> &GenerationId {
        &self.id
    }

    /// Path of the renamed `<root>/<id>/engram.db`.
    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.root.join("engram.db")
    }
}

impl SealedCandidate {
    /// Generation ID of the sealed candidate.
    #[must_use]
    pub const fn id(&self) -> &GenerationId {
        &self.id
    }

    /// The sealed inventory entries (exactly one: `<id>/engram.db`).
    #[must_use]
    pub fn inventory(&self) -> &[ManifestFileDigest] {
        &self.inventory
    }
}

/// Directory beneath the generations root that holds the runtime copy.
const RUNTIME_DIRECTORY_ID: &str = "runtime";
/// File name of the database inside a candidate.
const DATABASE_FILE_NAME: &str = "engram.db";
/// Sidecar suffixes that must not exist beside the database before rename.
const SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];
/// Raw Windows `ERROR_SHARING_VIOLATION`, the only retryable rename error.
///
/// On Unix, raw error 32 is `EPIPE`, which `rename` never returns, so the
/// retry branch is inert there.
const SHARING_VIOLATION_OS_ERROR: i32 = 32;
/// Maximum rename attempts before [`CandidateBuildError::RenameBusy`].
const RENAME_MAX_ATTEMPTS: u32 = 20;
/// Pause between rename attempts.
const RENAME_RETRY_DELAY: Duration = Duration::from_millis(50);

fn io_error(op: &'static str, source: &std::io::Error) -> CandidateBuildError {
    CandidateBuildError::Io {
        op,
        kind: source.kind(),
    }
}

/// Collect every regular file named `engram.db` beneath `dir` (recursive).
///
/// Symlinks are never followed, so a link cannot smuggle in an outside file.
fn find_databases(dir: &Path) -> Result<Vec<PathBuf>, CandidateBuildError> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries =
            std::fs::read_dir(&current).map_err(|e| io_error("read candidate directory", &e))?;
        for entry in entries {
            let entry = entry.map_err(|e| io_error("read candidate directory entry", &e))?;
            let file_type = entry
                .file_type()
                .map_err(|e| io_error("read candidate entry type", &e))?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() && entry.file_name() == DATABASE_FILE_NAME {
                found.push(entry.path());
            }
        }
    }
    Ok(found)
}

/// Locate the single `engram.db` under `<candidate>/cozo/`.
///
/// A missing `cozo/` directory counts as zero databases.
fn locate_database(candidate: &Path) -> Result<PathBuf, CandidateBuildError> {
    let cozo = candidate.join("cozo");
    let databases = match find_databases(&cozo) {
        Ok(found) => found,
        Err(CandidateBuildError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) if !cozo.exists() => Vec::new(),
        Err(other) => return Err(other),
    };
    match databases.as_slice() {
        [only] => Ok(only.clone()),
        other => Err(CandidateBuildError::DatabaseNotFound { count: other.len() }),
    }
}

/// Fail when any `-wal`, `-shm`, or `-journal` sidecar exists beside `database`.
///
/// The runtime-copy open path deletes stale sidecars, so un-checkpointed WAL
/// content would otherwise be silently lost.
fn reject_sidecars(database: &Path) -> Result<(), CandidateBuildError> {
    for suffix in SIDECAR_SUFFIXES {
        let mut name = database.as_os_str().to_os_string();
        name.push(suffix);
        let exists = Path::new(&name)
            .try_exists()
            .map_err(|e| io_error("probe database sidecar", &e))?;
        if exists {
            return Err(CandidateBuildError::UncheckpointedSidecar);
        }
    }
    Ok(())
}

/// Rename `from` to `to`, retrying only on raw OS error 32 (sharing violation).
async fn rename_with_retry(from: &Path, to: &Path) -> Result<(), CandidateBuildError> {
    for attempt in 1..=RENAME_MAX_ATTEMPTS {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if e.raw_os_error() == Some(SHARING_VIOLATION_OS_ERROR) => {
                if attempt < RENAME_MAX_ATTEMPTS {
                    tokio::time::sleep(RENAME_RETRY_DELAY).await;
                }
            }
            Err(e) => return Err(io_error("rename database", &e)),
        }
    }
    Err(CandidateBuildError::RenameBusy)
}

/// Build a candidate generation `<root>/<id>/engram.db` from the workspace.
///
/// Seals an exclusive candidate, indexes the workspace into it, requires
/// exactly one `engram.db` under `<candidate>/cozo/`, rejects checkpoint
/// sidecars, and renames the database into `<candidate>/engram.db`. The
/// leftover `cozo/` directory stays in place and is not part of the
/// inventory. The active manifest is never touched.
///
/// # Errors
///
/// Returns [`CandidateBuildError`] for the reserved ID `runtime`, a store
/// sealing failure (including an already-claimed candidate), an indexing
/// failure, a missing or ambiguous database, a remaining sidecar, or a rename
/// that stays blocked after bounded retry.
pub async fn build_candidate_generation(
    layout: &ReadServerLayout,
    store: &GenerationStore,
    id: &GenerationId,
    config: &CodeGraphConfig,
) -> Result<BuiltCandidate, CandidateBuildError> {
    // Case-insensitive: on case-insensitive filesystems `Runtime` aliases the
    // runtime-copy directory just like `runtime`.
    if id.as_str().eq_ignore_ascii_case(RUNTIME_DIRECTORY_ID) {
        return Err(CandidateBuildError::ReservedGenerationId);
    }

    let target = store.seal_candidate(id.clone(), id.as_str())?;
    if target.kind() != IndexTargetKind::Candidate {
        return Err(CandidateBuildError::CandidateNotExclusive);
    }
    let candidate = target.path().to_path_buf();

    index_workspace(&layout.workspace, &candidate, &layout.branch, config, true)
        .await
        .map_err(CandidateBuildError::Index)?;

    let database = locate_database(&candidate)?;
    reject_sidecars(&database)?;
    // `index_workspace` has returned, so its database handle is closed and no
    // late writer can create a sidecar between this check and the rename.
    // A failure after sealing leaves the partial candidate directory in
    // place; production mints a fresh ID per attempt and cleanup is the
    // caller's concern (plan PRE-2/PRE-3).
    rename_with_retry(&database, &candidate.join(DATABASE_FILE_NAME)).await?;

    Ok(BuiltCandidate {
        id: id.clone(),
        root: candidate,
    })
}

/// Seal a built candidate's inventory with one `<id>/engram.db` SHA-256 entry.
///
/// # Errors
///
/// Returns [`CandidateBuildError::Io`] when the database cannot be hashed.
pub fn seal_candidate_inventory(
    built: BuiltCandidate,
) -> Result<SealedCandidate, CandidateBuildError> {
    let database = built.database_path();
    // Open first so a failure reports the real I/O kind; the hash error
    // itself embeds a path and is deliberately discarded.
    std::fs::File::open(&database).map_err(|e| io_error("open database for hashing", &e))?;
    let digest = compute_file_hash(&database).map_err(|_| CandidateBuildError::Io {
        op: "hash database",
        kind: std::io::ErrorKind::Other,
    })?;
    let entry = ManifestFileDigest::new(format!("{}/{DATABASE_FILE_NAME}", built.id), digest);
    Ok(SealedCandidate {
        id: built.id,
        inventory: vec![entry],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn find_databases_reports_zero_one_and_many() {
        let dir = temp();
        assert!(find_databases(dir.path()).expect("scan").is_empty());

        let a = dir.path().join("a");
        std::fs::create_dir_all(&a).expect("mkdir a");
        std::fs::write(a.join("engram.db"), b"x").expect("write a");
        std::fs::write(a.join("engram.db.lock"), b"x").expect("write lock");
        assert_eq!(find_databases(dir.path()).expect("scan").len(), 1);

        let b = dir.path().join("b");
        std::fs::create_dir_all(&b).expect("mkdir b");
        std::fs::write(b.join("engram.db"), b"x").expect("write b");
        assert_eq!(find_databases(dir.path()).expect("scan").len(), 2);
    }

    #[test]
    fn locate_database_maps_missing_and_ambiguous_to_database_not_found() {
        let dir = temp();
        let err = locate_database(dir.path()).expect_err("no cozo dir");
        assert!(
            matches!(err, CandidateBuildError::DatabaseNotFound { count: 0 }),
            "{err:?}"
        );

        let cozo = dir.path().join("cozo");
        std::fs::create_dir_all(cozo.join("one")).expect("mkdir one");
        std::fs::write(cozo.join("one").join("engram.db"), b"x").expect("write one");
        assert!(locate_database(dir.path()).is_ok());

        std::fs::create_dir_all(cozo.join("two")).expect("mkdir two");
        std::fs::write(cozo.join("two").join("engram.db"), b"x").expect("write two");
        let err = locate_database(dir.path()).expect_err("two databases");
        assert!(
            matches!(err, CandidateBuildError::DatabaseNotFound { count: 2 }),
            "{err:?}"
        );
    }

    #[test]
    fn reject_sidecars_flags_each_suffix() {
        for suffix in SIDECAR_SUFFIXES {
            let dir = temp();
            let db = dir.path().join("engram.db");
            std::fs::write(&db, b"x").expect("write db");
            assert!(reject_sidecars(&db).is_ok());
            std::fs::write(dir.path().join(format!("engram.db{suffix}")), b"x")
                .expect("write sidecar");
            assert!(matches!(
                reject_sidecars(&db),
                Err(CandidateBuildError::UncheckpointedSidecar)
            ));
        }
    }

    #[tokio::test]
    async fn rename_with_retry_moves_file_and_surfaces_other_errors() {
        let dir = temp();
        let from = dir.path().join("from.db");
        let to = dir.path().join("to.db");
        std::fs::write(&from, b"x").expect("write from");
        rename_with_retry(&from, &to).await.expect("rename");
        assert!(to.is_file() && !from.exists());

        let err = rename_with_retry(&from, &to).await.expect_err("missing");
        assert!(
            matches!(
                err,
                CandidateBuildError::Io {
                    op: "rename database",
                    kind: std::io::ErrorKind::NotFound
                }
            ),
            "{err:?}"
        );
    }
}
