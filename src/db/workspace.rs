use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::shim::pidfile::PidFile;

/// Strip the Windows extended-length path prefix (`\\?\`) from a canonicalized path.
///
/// `std::fs::canonicalize` on Windows returns paths prefixed with `\\?\` for
/// extended-length path support. This prefix causes hash instability (the same
/// workspace produces a different SHA-256 depending on how the path was derived)
/// and can cause compatibility issues with crates that do not handle UNC paths.
/// Stripping it gives a regular absolute path while preserving full path fidelity
/// for paths under 260 characters, which all workspace roots in practice are.
pub(crate) fn normalize_canonical(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};

        // Inspect the leading path component to detect any verbatim prefix
        // (`\\?\C:\...` or `\\?\UNC\server\share\...`).  String round-trips
        // miss the UNC variant and produce non-canonical output on non-UTF-8
        // paths; component inspection is exact.
        if let Some(Component::Prefix(prefix_component)) = path.components().next() {
            let rebuilt: Option<PathBuf> = match prefix_component.kind() {
                Prefix::VerbatimDisk(drive) => {
                    // \\?\C:\rest  →  C:\rest
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(format!("{}:\\", drive as char)).join(suffix))
                }
                Prefix::VerbatimUNC(server, share) => {
                    // \\?\UNC\server\share\rest  →  \\server\share\rest
                    let server = server.to_string_lossy();
                    let share = share.to_string_lossy();
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(format!(r"\\{server}\{share}")).join(suffix))
                }
                Prefix::Verbatim(inner) => {
                    // \\?\other  →  other (best-effort)
                    let inner = inner.to_string_lossy();
                    let suffix: PathBuf = path.components().skip(1).collect();
                    Some(PathBuf::from(inner.as_ref()).join(suffix))
                }
                _ => None,
            };
            if let Some(p) = rebuilt {
                return p;
            }
        }
    }
    path
}

/// Canonicalize and validate a workspace path; ensures .git exists at root.
pub fn canonicalize_workspace(path: &str) -> Result<PathBuf, WorkspaceError> {
    resolve_git_metadata(Path::new(path)).map(|metadata| metadata.workspace)
}

#[derive(Debug)]
struct GitMetadata {
    workspace: PathBuf,
    head_path: PathBuf,
}

/// Resolve the workspace's Git metadata without following an unvalidated
/// gitfile. Linked-worktree metadata is outside the workspace by design, so
/// every native backlink is checked before the admin directory is trusted.
fn resolve_git_metadata(path: &Path) -> Result<GitMetadata, WorkspaceError> {
    let canonical_workspace = path.canonicalize().map_err(|_| WorkspaceError::NotFound {
        path: path.display().to_string(),
    })?;
    let workspace = normalize_canonical(canonical_workspace.clone());
    let git_entry = workspace.join(".git");
    let entry_metadata =
        std::fs::symlink_metadata(&git_entry).map_err(|_| WorkspaceError::NotGitRoot {
            path: workspace.display().to_string(),
        })?;

    if entry_metadata.file_type().is_symlink() {
        return Err(not_git_root(&workspace));
    }

    if entry_metadata.is_dir() {
        let canonical_git_dir = canonical_path(&git_entry, &workspace)?;
        if canonical_git_dir.parent() != Some(workspace.as_path()) {
            return Err(not_git_root(&workspace));
        }
        return Ok(GitMetadata {
            workspace,
            head_path: canonical_git_dir.join("HEAD"),
        });
    }

    if !entry_metadata.is_file() {
        return Err(not_git_root(&workspace));
    }

    let gitfile = read_metadata_file(&git_entry, &workspace)?;
    let directive = parse_single_line(&gitfile, &workspace)?;
    let admin_text = directive
        .strip_prefix("gitdir: ")
        .ok_or_else(|| not_git_root(&workspace))?;
    if admin_text.is_empty() || admin_text.trim() != admin_text {
        return Err(not_git_root(&workspace));
    }
    let admin_candidate = resolve_metadata_pointer(admin_text, &workspace, &workspace)?;
    let admin_dir = canonical_path(&admin_candidate, &workspace)?;
    require_plain_directory(&admin_candidate, &workspace)?;
    if normalize_metadata_pointer(admin_text, &admin_candidate) != admin_dir {
        return Err(not_git_root(&workspace));
    }

    let commondir_path = admin_dir.join("commondir");
    let commondir_content = read_metadata_file(&commondir_path, &workspace)?;
    let common_directive = parse_single_line(&commondir_content, &workspace)?;
    if common_directive.is_empty() || common_directive.trim() != common_directive {
        return Err(not_git_root(&workspace));
    }
    let commondir_path = PathBuf::from(common_directive);
    let common_candidate = if commondir_path.is_absolute() {
        commondir_path
    } else {
        admin_dir.join(commondir_path)
    };
    require_plain_directory(&common_candidate, &workspace)?;
    let common_dir = canonical_path(&common_candidate, &workspace)?;

    let worktrees_candidate = common_dir.join("worktrees");
    require_plain_directory(&worktrees_candidate, &workspace)?;
    require_plain_directory(&common_dir.join("objects"), &workspace)?;
    require_plain_reference_storage(&common_dir, &workspace)?;
    let worktrees_dir = canonical_path(&worktrees_candidate, &workspace)?;
    let _ = read_metadata_file(&common_dir.join("HEAD"), &workspace)?;
    if admin_dir.parent() != Some(worktrees_dir.as_path()) {
        return Err(not_git_root(&workspace));
    }

    let backlink_path = admin_dir.join("gitdir");
    let backlink_content = read_metadata_file(&backlink_path, &workspace)?;
    let backlink = parse_single_line(&backlink_content, &workspace)?;
    if backlink.is_empty() || backlink.trim() != backlink {
        return Err(not_git_root(&workspace));
    }
    let backlink_candidate = resolve_metadata_pointer(backlink, &admin_dir, &workspace)?;
    if normalize_metadata_pointer(backlink, &backlink_candidate) != git_entry {
        return Err(not_git_root(&workspace));
    }
    let canonical_backlink = canonical_path(&backlink_candidate, &workspace)?;
    let canonical_gitfile = canonical_path(&git_entry, &workspace)?;
    if canonical_backlink != canonical_gitfile {
        return Err(not_git_root(&workspace));
    }

    let head_path = admin_dir.join("HEAD");
    let _ = read_metadata_file(&head_path, &workspace)?;
    Ok(GitMetadata {
        // Match the platform's canonical spelling for linked worktrees; the
        // native admin backlink is defined in terms of that same identity.
        workspace: canonical_workspace,
        head_path,
    })
}

fn is_parent_component(component: std::path::Component<'_>) -> bool {
    matches!(component, std::path::Component::ParentDir)
}

fn resolve_metadata_pointer(
    directive: &str,
    containing_dir: &Path,
    workspace: &Path,
) -> Result<PathBuf, WorkspaceError> {
    let pointer = PathBuf::from(directive);
    if pointer.is_absolute() {
        if pointer.components().any(is_parent_component) {
            return Err(not_git_root(workspace));
        }
        Ok(pointer)
    } else {
        Ok(containing_dir.join(pointer))
    }
}

fn normalize_metadata_pointer(directive: &str, resolved: &Path) -> PathBuf {
    if Path::new(directive).is_absolute() {
        normalize_canonical(resolved.to_path_buf())
    } else {
        normalize_lexical(resolved)
    }
}

fn not_git_root(workspace: &Path) -> WorkspaceError {
    WorkspaceError::NotGitRoot {
        path: workspace.display().to_string(),
    }
}

fn canonical_path(path: &Path, workspace: &Path) -> Result<PathBuf, WorkspaceError> {
    path.canonicalize()
        .map(normalize_canonical)
        .map_err(|_| not_git_root(workspace))
}

fn require_plain_directory(path: &Path, workspace: &Path) -> Result<(), WorkspaceError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| not_git_root(workspace))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        Ok(())
    } else {
        Err(not_git_root(workspace))
    }
}

fn require_plain_reference_storage(
    common_dir: &Path,
    workspace: &Path,
) -> Result<(), WorkspaceError> {
    let refs = common_dir.join("refs");
    match std::fs::symlink_metadata(&refs) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            require_plain_directory(&common_dir.join("reftable"), workspace)
        }
        Ok(_) | Err(_) => Err(not_git_root(workspace)),
    }
}

fn read_metadata_file(path: &Path, workspace: &Path) -> Result<String, WorkspaceError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| not_git_root(workspace))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(not_git_root(workspace));
    }
    std::fs::read_to_string(path).map_err(|_| not_git_root(workspace))
}

fn parse_single_line<'a>(content: &'a str, workspace: &Path) -> Result<&'a str, WorkspaceError> {
    let mut lines = content.lines();
    let line = lines.next().ok_or_else(|| not_git_root(workspace))?;
    if line.is_empty() || lines.next().is_some() {
        return Err(not_git_root(workspace));
    }
    Ok(line)
}

/// Load or create the persisted workspace identifier for a workspace root.
///
/// # Errors
///
/// Returns an error when the workspace root is invalid, the write would escape
/// the workspace root, or the persisted `.workspace-id` file cannot be read or
/// written safely.
pub fn load_or_create_workspace_id(path: &Path) -> Result<Uuid, EngramError> {
    let canonical = normalize_canonical(resolve_git_metadata(path)?.workspace);

    let engram_dir = canonical.join(".engram");
    std::fs::create_dir_all(&engram_dir).map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: engram_dir.display().to_string(),
        })
    })?;
    let canonical_engram = normalize_canonical(engram_dir.canonicalize().map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: engram_dir.display().to_string(),
        })
    })?);
    if !canonical_engram.starts_with(&canonical) {
        return Err(EngramError::Workspace(WorkspaceError::PathEscape {
            attempted: canonical_engram,
            root: canonical,
        }));
    }

    let id_path = canonical_engram.join(".workspace-id");
    if workspace_id_entry_exists(&id_path)? {
        return read_workspace_id(&id_path);
    }

    let workspace_id = Uuid::new_v4();
    let mut temp_file = tempfile::NamedTempFile::new_in(&canonical_engram).map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: id_path.display().to_string(),
        })
    })?;
    writeln!(temp_file, "{workspace_id}").map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: id_path.display().to_string(),
        })
    })?;
    temp_file.as_file().sync_all().map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: id_path.display().to_string(),
        })
    })?;
    match temp_file.persist_noclobber(&id_path) {
        Ok(_) => Ok(workspace_id),
        Err(_) if workspace_id_entry_exists(&id_path)? => read_workspace_id(&id_path),
        Err(_) => Err(EngramError::System(SystemError::FlushFailed {
            path: id_path.display().to_string(),
        })),
    }
}

/// Return the persisted workspace identity as a string key.
///
/// # Errors
///
/// Returns any error from [`load_or_create_workspace_id`].
pub fn workspace_key(path: &Path) -> Result<String, EngramError> {
    load_or_create_workspace_id(path).map(|workspace_id| workspace_id.to_string())
}

/// Return the daemon discovery key for `path`.
///
/// If the workspace has already been upgraded to a persisted `.workspace-id`,
/// use that UUID. When a legacy daemon is still live and `.workspace-id` is
/// absent, fall back to the historical path hash so the shim can reconnect to
/// the already-running daemon long enough to retire it.
///
/// # Errors
///
/// Returns any error from workspace canonicalization or workspace-id loading.
pub fn daemon_key_for_workspace(path: &Path) -> Result<String, EngramError> {
    let canonical =
        normalize_canonical(path.canonicalize().map_err(|_| WorkspaceError::NotFound {
            path: path.display().to_string(),
        })?);
    let id_path = workspace_id_path(&canonical);
    if workspace_id_entry_exists(&id_path)? {
        return workspace_key(&canonical);
    }

    if let Some(pid_file) = PidFile::read(&canonical) {
        if pid_file.verify_alive()? {
            let branch = resolve_git_branch(&canonical).unwrap_or_else(|_| "default".to_string());
            tracing::info!(
                event_type = "workspace_id_fallback",
                workspace = %canonical.display(),
                "workspace-id missing while legacy daemon is live; using path-hash fallback"
            );
            return Ok(workspace_hash(&canonical, &branch));
        }
    }

    workspace_key(&canonical)
}

fn workspace_id_path(path: &Path) -> PathBuf {
    path.join(".engram").join(".workspace-id")
}

fn workspace_id_entry_exists(id_path: &Path) -> Result<bool, EngramError> {
    match std::fs::symlink_metadata(id_path) {
        Ok(metadata) => {
            if is_workspace_id_link_or_reparse(&metadata) {
                return Err(unsafe_workspace_id_error(
                    id_path,
                    "linked or reparse leaves are not allowed",
                ));
            }
            if !metadata.is_file() {
                return Err(unsafe_workspace_id_error(
                    id_path,
                    "the identity leaf must be a regular file",
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(workspace_id_io_error(id_path)),
    }
}

fn is_workspace_id_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    let is_link = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        is_link || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        is_link
    }
}

fn read_workspace_id(id_path: &Path) -> Result<Uuid, EngramError> {
    if !workspace_id_entry_exists(id_path)? {
        return Err(workspace_id_io_error(id_path));
    }

    let parent = id_path.parent().ok_or_else(|| {
        unsafe_workspace_id_error(id_path, "the identity leaf has no parent directory")
    })?;
    let file_name = id_path
        .file_name()
        .ok_or_else(|| unsafe_workspace_id_error(id_path, "the identity leaf has no file name"))?;
    let directory = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|_| workspace_id_io_error(id_path))?;
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt as _;
        use rustix::fs::OFlags;

        let flags = OFlags::NOFOLLOW | OFlags::NONBLOCK;
        let custom_flags = i32::try_from(flags.bits()).map_err(|error| {
            unsafe_workspace_id_error(
                id_path,
                format!("no-follow flags are not representable: {error}"),
            )
        })?;
        options.custom_flags(custom_flags);
    }
    let mut file = directory
        .open_with(Path::new(file_name), &options)
        .map_err(|_| workspace_id_io_error(id_path))?;
    let metadata = file
        .metadata()
        .map_err(|_| workspace_id_io_error(id_path))?;
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt as _;

        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(unsafe_workspace_id_error(
                id_path,
                "linked or reparse leaves are not allowed",
            ));
        }
    }
    if !metadata.is_file() {
        return Err(unsafe_workspace_id_error(
            id_path,
            "the opened identity leaf is not a regular file",
        ));
    }

    let mut raw = String::new();
    file.read_to_string(&mut raw)
        .map_err(|_| workspace_id_io_error(id_path))?;
    Uuid::parse_str(raw.trim()).map_err(|e| {
        EngramError::System(SystemError::InvalidParams {
            reason: format!("invalid workspace-id '{}': {e}", id_path.display()),
        })
    })
}

fn workspace_id_io_error(id_path: &Path) -> EngramError {
    EngramError::System(SystemError::FlushFailed {
        path: id_path.display().to_string(),
    })
}

fn unsafe_workspace_id_error(id_path: &Path, reason: impl Into<String>) -> EngramError {
    EngramError::System(SystemError::InvalidParams {
        reason: format!(
            "unsafe workspace-id '{}': {}",
            id_path.display(),
            reason.into()
        ),
    })
}

/// Compute a stable SHA256 hash for the workspace `(path, branch)` pair.
///
/// The digest covers the canonical workspace path, a `:` separator, and the
/// sanitised branch name so `workspace_id` uniquely identifies each
/// `(path, branch)` combination.  Two workspaces at the same path but on
/// different branches therefore produce distinct identifiers, matching the
/// per-branch DB isolation already provided by `connect_db`.
pub fn workspace_hash(path: &Path, branch: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b":");
    hasher.update(branch.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

/// Resolve the current git branch name for the workspace.
///
/// Reads `.git/HEAD` directly (no subprocess) and extracts the branch name.
/// Returns a truncated commit SHA when HEAD is detached.
pub fn resolve_git_branch(workspace: &Path) -> Result<String, WorkspaceError> {
    let metadata = resolve_git_metadata(workspace)?;
    let head_path = metadata.head_path;
    let head_content = read_metadata_file(&head_path, &metadata.workspace)?;

    let head = head_content.trim();
    if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
        Ok(sanitize_branch_for_path(branch))
    } else {
        // Detached HEAD: use first 12 chars of the commit SHA
        Ok(head.chars().take(12).collect())
    }
}

/// Sanitize a git branch name for use as a filesystem directory name.
///
/// Replaces `/` with `__` so branches like `feature/foo` become `feature__foo`.
pub(crate) fn sanitize_branch_for_path(branch: &str) -> String {
    branch.replace('/', "__")
}

/// Resolve the data directory for database storage.
///
/// Priority:
/// 1. `ENGRAM_DATA_DIR` env var (resolved relative to workspace if not absolute)
/// 2. Default: `{workspace}/.engram`
pub fn resolve_data_dir(workspace: &Path) -> PathBuf {
    if let Ok(env_dir) = std::env::var("ENGRAM_DATA_DIR") {
        let p = PathBuf::from(&env_dir);
        if p.is_absolute() {
            p
        } else {
            workspace.join(p)
        }
    } else {
        workspace.join(".engram")
    }
}

/// True when `data_dir` resolves to a location inside `workspace` — i.e. it is
/// covered by the workspace-rooted daemon lock.
///
/// Both paths are normalised for containment (canonicalising the longest
/// existing prefix, stripping the Windows verbatim prefix, and resolving
/// `.`/`..`), then compared component-wise. The caller passes an already
/// canonical workspace root. Used by the destructive `migrate-down` guard
/// (086.003-T) to fail closed on a shared/external `ENGRAM_DATA_DIR`.
pub(crate) fn is_data_dir_within_workspace(workspace: &Path, data_dir: &Path) -> bool {
    let ws = normalize_for_containment(workspace);
    let dd = normalize_for_containment(data_dir);
    dd.starts_with(&ws)
}

/// Resolve `.`/`..` in `path` lexically, without touching the filesystem.
fn normalize_lexical(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.components().next_back(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(comp.as_os_str());
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Normalise `path` for containment comparison. Canonicalises the longest
/// existing prefix (resolving symlinks/short names and stripping the Windows
/// verbatim prefix) and re-appends the lexically-normalised remainder, so a
/// not-yet-created directory still compares correctly against a canonical
/// workspace root. Falls back to a purely lexical normalisation when nothing
/// along the path exists.
fn normalize_for_containment(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return normalize_canonical(canon);
    }
    for ancestor in path.ancestors().skip(1) {
        if ancestor.as_os_str().is_empty() {
            break;
        }
        if let Ok(canon) = ancestor.canonicalize() {
            let mut base = normalize_canonical(canon);
            if let Ok(rest) = path.strip_prefix(ancestor) {
                base.push(rest);
            }
            return normalize_lexical(&base);
        }
    }
    normalize_lexical(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use tempfile::TempDir;
    use uuid::Uuid;

    use super::daemon_key_for_workspace;
    use super::is_data_dir_within_workspace;
    use super::load_or_create_workspace_id;
    use crate::errors::{EngramError, WorkspaceError};

    #[cfg(unix)]
    fn symlink_file(source: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(source, link)
    }

    #[cfg(windows)]
    fn symlink_file(source: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(source, link)
    }

    fn create_symlink_file(source: &std::path::Path, link: &std::path::Path) -> bool {
        match symlink_file(source, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("create file symlink: {error}"),
        }
    }

    fn create_workspace() -> TempDir {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        fs::create_dir(workspace.path().join(".git")).expect("create .git");
        workspace
    }

    // ── 086.003-T: fail-closed data-dir containment guard ────────────────────

    #[test]
    fn data_dir_within_workspace_accepts_workspace_owned_dirs() {
        let ws = create_workspace();
        let root = ws.path().canonicalize().expect("canonicalize workspace");
        assert!(
            is_data_dir_within_workspace(&root, &root.join(".engram")),
            "the default workspace-local data dir must be accepted"
        );
        assert!(
            is_data_dir_within_workspace(&root, &root.join(".engram").join("cozo")),
            "a nested workspace-owned data dir must be accepted"
        );
        assert!(
            is_data_dir_within_workspace(&root, &root),
            "the workspace root itself must be accepted"
        );
    }

    #[test]
    fn data_dir_within_workspace_rejects_external_dir() {
        let ws = create_workspace();
        let root = ws.path().canonicalize().expect("canonicalize workspace");
        let external = tempfile::tempdir().expect("external tempdir");
        let external_root = external
            .path()
            .canonicalize()
            .expect("canonicalize external");
        assert!(
            !is_data_dir_within_workspace(&root, &external_root),
            "a shared/external data dir must be rejected (fail closed)"
        );
    }

    #[test]
    fn data_dir_within_workspace_rejects_parent_escape() {
        let ws = create_workspace();
        let root = ws.path().canonicalize().expect("canonicalize workspace");
        let escape = root.join("..").join("engram-shared-data");
        assert!(
            !is_data_dir_within_workspace(&root, &escape),
            "a `..` escape out of the workspace must be rejected"
        );
    }

    #[test]
    fn workspace_id_stable_across_canonical_forms() {
        let workspace = create_workspace();
        let canonical = workspace
            .path()
            .canonicalize()
            .expect("canonical workspace path");

        let canonical_id =
            load_or_create_workspace_id(&canonical).expect("workspace-id should load");
        let original_id = load_or_create_workspace_id(workspace.path())
            .expect("workspace-id should remain stable");

        assert_eq!(canonical_id, original_id);
    }

    #[test]
    fn first_bind_creates_workspace_id() {
        let workspace = create_workspace();
        let id_path = workspace.path().join(".engram").join(".workspace-id");

        let created = load_or_create_workspace_id(workspace.path()).expect("workspace-id created");
        let persisted = fs::read_to_string(&id_path).expect("read persisted workspace-id");

        assert_eq!(persisted.trim(), created.to_string());
    }

    #[test]
    fn concurrent_cold_starts_share_the_atomically_created_workspace_id() {
        const STARTERS: usize = 32;

        let workspace = create_workspace();
        fs::create_dir(workspace.path().join(".engram")).expect("create .engram");
        let workspace_path = Arc::new(workspace.path().to_path_buf());
        let start = Arc::new(Barrier::new(STARTERS));
        let handles: Vec<_> = (0..STARTERS)
            .map(|_| {
                let workspace_path = Arc::clone(&workspace_path);
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    load_or_create_workspace_id(&workspace_path)
                })
            })
            .collect();

        let returned_ids: Vec<_> = handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .expect("workspace-id starter must not panic")
                    .expect("workspace-id starter must succeed")
            })
            .collect();
        let persisted_id = fs::read_to_string(workspace.path().join(".engram/.workspace-id"))
            .expect("read winning workspace-id");

        assert!(
            returned_ids.windows(2).all(|pair| pair[0] == pair[1]),
            "every concurrent cold start must return the winning persisted identity: {returned_ids:?}"
        );
        assert_eq!(
            returned_ids[0].to_string(),
            persisted_id.trim(),
            "the identity returned to every starter must match the persisted winner"
        );
    }

    #[test]
    fn workspace_id_symlink_leaf_is_rejected_by_load_and_daemon_discovery() {
        let workspace = create_workspace();
        let outside = tempfile::tempdir().expect("outside tempdir");
        let external_id = Uuid::new_v4();
        let external_id_path = outside.path().join("external-workspace-id");
        fs::write(&external_id_path, format!("{external_id}\n"))
            .expect("write external workspace-id");

        let engram_dir = workspace.path().join(".engram");
        fs::create_dir(&engram_dir).expect("create .engram");
        let id_path = engram_dir.join(".workspace-id");
        if !create_symlink_file(&external_id_path, &id_path) {
            return;
        }

        let load_result = load_or_create_workspace_id(workspace.path());
        let daemon_key_result = daemon_key_for_workspace(workspace.path());

        assert!(
            load_result.is_err(),
            "workspace-id load must not source identity through a symlink leaf"
        );
        assert!(
            daemon_key_result.is_err(),
            "daemon discovery must not treat a workspace-id symlink as a legacy or readable leaf"
        );
        assert_eq!(
            fs::read_to_string(external_id_path).expect("read external workspace-id"),
            format!("{external_id}\n"),
            "rejected identity lookup must not alter the external target"
        );
    }

    #[test]
    fn ambiguous_bind_returns_typed_error() {
        let workspace = create_workspace();
        let typed_error = WorkspaceError::AmbiguousBind {
            expected_id: Uuid::new_v4(),
            found_id: Uuid::new_v4(),
            path: PathBuf::from(workspace.path()),
        };
        let wrapped_error = EngramError::from(typed_error);

        assert!(matches!(
            wrapped_error,
            EngramError::Workspace(WorkspaceError::AmbiguousBind { .. })
        ));

        assert!(load_or_create_workspace_id(workspace.path()).is_ok());
    }
}
