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

struct GitMetadata {
    workspace: PathBuf,
    // `None` for a primary checkout whose `.git/HEAD` is absent or unreadable —
    // resolving the workspace identity never required HEAD, only branch
    // resolution does. A linked worktree always carries a validated HEAD.
    head_content: Option<String>,
    // The retained, validated workspace-root capability handle (U5). Identity
    // persistence reaches `.engram/.workspace-id` THROUGH this handle instead
    // of reopening the path from scratch, so an ancestor swap staged between
    // the Git authenticity proof and the identity read cannot redirect the read
    // to an attacker-controlled directory (threat T5). `GitMetadata` is private,
    // so carrying a `CapRoot` here introduces no public API surface.
    root: CapRoot,
}

// Test-only interception seam for the metadata-resolution TOCTOU windows.
//
// In production (`cfg(not(test))`) `toctou_checkpoint` is a no-op with zero
// runtime surface. Under `cfg(test)` it invokes a thread-local hook, letting
// the colocated adversarial tests perform a deterministic filesystem swap at a
// named point during `resolve_git_metadata` — reproducing the check/use race
// without any timing dependency.
#[cfg(test)]
type ToctouHook = Box<dyn FnMut(&str)>;

#[cfg(test)]
thread_local! {
    static TOCTOU_SWAP_HOOK: std::cell::RefCell<Option<ToctouHook>> =
        const { std::cell::RefCell::new(None) };
}

/// Invoke the installed swap hook, if any, for the named checkpoint.
#[cfg(test)]
fn toctou_checkpoint(name: &str) {
    TOCTOU_SWAP_HOOK.with(|hook| {
        if let Some(callback) = hook.borrow_mut().as_mut() {
            callback(name);
        }
    });
}

/// No-op in production builds; the checkpoint carries no runtime cost or surface.
#[cfg(not(test))]
fn toctou_checkpoint(_name: &str) {}

/// `FILE_ATTRIBUTE_REPARSE_POINT` — set on every Windows reparse object
/// (symlinks, junctions/mount points, and any other reparse tag such as cloud
/// placeholders or container-isolation links). The uniform gate rejects the
/// whole class rather than the `SYMLINK`/`MOUNT_POINT` subset `is_symlink()`
/// covers.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// The one shared reparse/symlink rejection policy (U6). Every handle-derived
/// metadata in the validated chain — the Git backlink chain and `.workspace-id`
/// alike — flows through this single predicate so the two never diverge.
///
/// `windows_attributes` is ignored off Windows; on Unix a symlink is the only
/// reparse-equivalent and `is_symlink` already captures it.
fn is_link_or_reparse(is_symlink: bool, windows_attributes: u32) -> bool {
    #[cfg(windows)]
    {
        is_symlink || (windows_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0)
    }
    #[cfg(not(windows))]
    {
        let _ = windows_attributes;
        is_symlink
    }
}

/// Apply the uniform policy to handle-derived `cap_std` metadata.
fn cap_metadata_is_link_or_reparse(metadata: &cap_std::fs::Metadata) -> bool {
    let is_symlink = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use cap_std::fs::MetadataExt as _;
        is_link_or_reparse(is_symlink, metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        is_link_or_reparse(is_symlink, 0)
    }
}

/// Classification of a directory entry, taken from handle-derived, no-follow
/// metadata. `NotFound` is distinguished from every other I/O error so the
/// `.git` dir-vs-file dispatch and the `refs`-vs-`reftable` fallback can tell
/// "absent" apart from "present but unreadable".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildKind {
    Dir,
    File,
    Absent,
    Other,
}

/// A retained, capability-rooted directory handle.
///
/// `dir` is the load-bearing authority: every child is reached from it with an
/// `openat`-style no-follow operation, one component at a time, so validation
/// and use address the same object. `display` is carried for error messages
/// only and is NEVER re-resolved for access (invariant 1).
struct CapRoot {
    dir: Dir,
    display: PathBuf,
}

impl CapRoot {
    /// Ambient (full-path) resolution that FOLLOWS a link/reparse point at the
    /// path's final component. Used for the workspace root and, in the identity
    /// probe, for the workspace root before a no-follow `.engram` descent.
    ///
    /// This asymmetry with [`Self::open_anchor_nofollow_leaf`] is deliberate: a
    /// workspace root that is ITSELF a junction or symlink (e.g.
    /// `mklink /J C:\work C:\Users\me\repos\work`) is a legitimate, supported
    /// layout and is admitted today. Rejecting it would be an availability
    /// regression (rollback trigger 1). The reparse gate is scoped to the
    /// validated Git chain BELOW a root, never to the root component itself.
    /// The common Git directory, by contrast, must not be a link at its leaf and
    /// therefore uses [`Self::open_anchor_nofollow_leaf`].
    fn open_anchor(path: &Path) -> Result<Self, WorkspaceError> {
        let dir =
            Dir::open_ambient_dir(path, ambient_authority()).map_err(|_| not_git_root(path))?;
        Ok(Self {
            dir,
            display: path.to_path_buf(),
        })
    }

    /// Open an anchor whose FINAL component must not be a link or reparse point.
    /// The parent is resolved ambiently; the leaf is opened no-follow from it
    /// and validated through handle-derived metadata via the uniform reparse
    /// gate. Used ONLY for the common Git directory anchor, restoring the leaf
    /// rejection the pre-change `require_plain_directory(&common_candidate, ..)`
    /// provided: an attacker who replaces the common Git directory with a
    /// junction/symlink to a forged tree is rejected instead of having
    /// `objects`, `refs`, `worktrees/<name>`, the backlinks and `HEAD` accepted
    /// from the substituted target.
    fn open_anchor_nofollow_leaf(path: &Path) -> Result<Self, WorkspaceError> {
        let parent = path.parent().ok_or_else(|| not_git_root(path))?;
        let leaf = path.file_name().ok_or_else(|| not_git_root(path))?;
        let parent_dir =
            Dir::open_ambient_dir(parent, ambient_authority()).map_err(|_| not_git_root(path))?;
        // Reuse the uniform `open_child_dir` no-follow/reparse validation on the
        // leaf so the common Git directory passes through the same gate every
        // other validated directory does. `display` is set to the full anchor
        // path for error fidelity.
        let parent_root = Self {
            dir: parent_dir,
            display: path.to_path_buf(),
        };
        let mut anchor = parent_root.open_child_dir(leaf)?;
        anchor.display = path.to_path_buf();
        Ok(anchor)
    }

    /// Classify a single child component from handle-derived no-follow metadata.
    fn child_kind(&self, name: impl AsRef<Path>) -> ChildKind {
        let name = name.as_ref();
        debug_assert!(
            is_single_component(name),
            "child_kind must be called with exactly one path component"
        );
        match self.dir.symlink_metadata(name) {
            Ok(metadata) => {
                if cap_metadata_is_link_or_reparse(&metadata) {
                    ChildKind::Other
                } else if metadata.is_dir() {
                    ChildKind::Dir
                } else if metadata.is_file() {
                    ChildKind::File
                } else {
                    ChildKind::Other
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ChildKind::Absent,
            Err(_) => ChildKind::Other,
        }
    }

    /// Open a single child directory with `open_dir_nofollow`, then validate it
    /// through HANDLE-DERIVED metadata: it must be a directory and must pass the
    /// uniform reparse gate. `name` must be exactly one path component; the
    /// runtime guard rejects anything else so the helper cannot be misused to
    /// smuggle a multi-component or `.`/`..`/root/prefix path.
    fn open_child_dir(&self, name: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        use cap_fs_ext::DirExt as _;

        let name = name.as_ref();
        if !is_single_component(name) {
            return Err(not_git_root(&self.display));
        }
        let child = self
            .dir
            .open_dir_nofollow(name)
            .map_err(|_| not_git_root(&self.display))?;
        let metadata = child
            .dir_metadata()
            .map_err(|_| not_git_root(&self.display))?;
        if !metadata.is_dir() || cap_metadata_is_link_or_reparse(&metadata) {
            return Err(not_git_root(&self.display));
        }
        Ok(Self {
            dir: child,
            display: self.display.join(name),
        })
    }

    /// Open + validate + drop a child directory, for existence proofs.
    fn require_child_dir(&self, name: &str) -> Result<(), WorkspaceError> {
        self.open_child_dir(name).map(|_| ())
    }

    /// Read a single child file. The metadata is taken from the OPEN FILE HANDLE
    /// and the content is read from that SAME handle — never a reopen by path
    /// (the load-bearing P0 constraint from plan review S1).
    fn read_child_file(&self, name: &str) -> Result<String, WorkspaceError> {
        let mut file = self.open_child_file_nofollow(name)?;
        let metadata = file.metadata().map_err(|_| not_git_root(&self.display))?;
        let is_reparse = {
            #[cfg(windows)]
            {
                use cap_std::fs::MetadataExt as _;
                is_link_or_reparse(
                    metadata.file_type().is_symlink(),
                    metadata.file_attributes(),
                )
            }
            #[cfg(not(windows))]
            {
                is_link_or_reparse(metadata.file_type().is_symlink(), 0)
            }
        };
        if !metadata.is_file() || is_reparse {
            return Err(not_git_root(&self.display));
        }
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .map_err(|_| not_git_root(&self.display))?;
        Ok(raw)
    }

    /// Open a child file no-follow using the same `OFlags::NOFOLLOW |
    /// OFlags::NONBLOCK` custom-flags pattern `read_workspace_id` established.
    fn open_child_file_nofollow(&self, name: &str) -> Result<cap_std::fs::File, WorkspaceError> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt as _;
            use rustix::fs::OFlags;

            let flags = OFlags::NOFOLLOW | OFlags::NONBLOCK;
            let custom_flags =
                i32::try_from(flags.bits()).map_err(|_| not_git_root(&self.display))?;
            options.custom_flags(custom_flags);
        }
        self.dir
            .open_with(Path::new(name), &options)
            .map_err(|_| not_git_root(&self.display))
    }

    /// Create a child file relative to this retained handle, failing if it
    /// already exists.
    ///
    /// `create_new` maps to `O_CREAT | O_EXCL` (and the Windows equivalent), so
    /// this is an atomic exclusive publish. It replaces the previous
    /// temp-file-plus-`persist_noclobber` pair, which named the destination by
    /// ambient pathname and therefore wrote outside the proof: `.engram` could
    /// be renamed aside and substituted between the authenticity proof and the
    /// publish, landing the identity file in an attacker-controlled directory.
    ///
    /// Returns the raw `io::Error` so the caller can distinguish
    /// `AlreadyExists` (lost the create race, re-read through the handle) from a
    /// genuine failure.
    fn create_new_child_file(&self, name: &str) -> std::io::Result<cap_std::fs::File> {
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        self.dir.open_with(Path::new(name), &options)
    }

    /// Handle-derived object identity: `(device, inode)`.
    ///
    /// Unix only. `cap-std` exposes the Windows equivalent (volume serial plus
    /// file index) solely through an unstable internal trait, and obtaining it
    /// from a `std` handle would require `unsafe` to borrow the directory
    /// handle, which the crate forbids. See [`Self::prove_names_same_object`]
    /// for why Windows is nonetheless not exposed by this gap.
    #[cfg(unix)]
    fn object_identity(&self) -> Option<(u64, u64)> {
        use cap_std::fs::MetadataExt as _;

        let metadata = self.dir.dir_metadata().ok()?;
        Some((metadata.dev(), metadata.ino()))
    }

    /// Prove that `candidate` names the same object this handle is bound to.
    ///
    /// The workspace root is opened once, from the caller-supplied path, and
    /// that handle is the only authority: every metadata check, every content
    /// read, identity persistence and the daemon key all descend from it. The
    /// canonical spelling is derived by a second pathname resolution because it
    /// is the workspace *identity* — it feeds `workspace_hash`, the containment
    /// guard, and operator-facing output.
    ///
    /// An ancestor swapped between those two steps cannot cause attacker
    /// content to be admitted, because nothing is ever read through the
    /// canonical name; it would only make the reported identity stale relative
    /// to the authority. On Unix this check removes even that inconsistency by
    /// comparing handle-derived object identity, and fails closed on a
    /// mismatch. On Windows the identity accessor is not reachable without
    /// `unsafe`, so the residual there is a naming inconsistency, not a false
    /// accept.
    #[cfg_attr(not(unix), allow(clippy::unnecessary_wraps, clippy::unused_self))]
    fn prove_names_same_object(&self, candidate: &Path) -> Result<(), WorkspaceError> {
        #[cfg(unix)]
        {
            let Some(authority) = self.object_identity() else {
                // Identity unavailable: treat as "cannot prove", never as
                // "proven different". Failing closed here would reject
                // legitimate checkouts on filesystems that report no inode.
                return Ok(());
            };
            let named = CapRoot::open_anchor(candidate)?;
            let Some(named_identity) = named.object_identity() else {
                return Ok(());
            };
            if authority != named_identity {
                return Err(not_git_root(candidate));
            }
        }
        #[cfg(not(unix))]
        {
            let _ = candidate;
        }
        Ok(())
    }
}

/// Resolve the workspace's Git metadata over retained, capability-rooted,
/// no-follow handles. A linked worktree's admin metadata lives outside the
/// workspace root by design, so the common Git directory gets its own retained
/// anchor (invariant 5); every backlink value is read from an already-validated
/// handle and the admitted objects are proven identical to the validated ones.
fn resolve_git_metadata(path: &Path) -> Result<GitMetadata, WorkspaceError> {
    // Root #1: the retained workspace root, opened ONCE from the caller-supplied
    // path. This handle — not any pathname — is the authority for everything
    // below, and it is what identity persistence and the daemon key consume.
    let root = CapRoot::open_anchor(path)?;

    // The canonical spelling is a NAME, not an authority. It is still needed:
    // it is the workspace identity that feeds `workspace_hash`, the containment
    // guard, and every operator-facing path. Deriving it requires a second
    // pathname resolution, and an ancestor swapped between the two would
    // otherwise let the retained handle bind one object while the reported
    // identity named another.
    let canonical_workspace = path.canonicalize().map_err(|_| WorkspaceError::NotFound {
        path: path.display().to_string(),
    })?;
    let workspace = normalize_canonical(canonical_workspace.clone());

    // Close that gap by proving the canonical name denotes the SAME object the
    // retained authority is bound to. A swap in the window changes the object
    // identity and the proof fails closed, so identity and authority can never
    // diverge.
    root.prove_names_same_object(&workspace)?;

    let git_entry = workspace.join(".git");

    match root.child_kind(".git") {
        ChildKind::Dir => {
            // Primary checkout: `.git` is a plain directory under the root.
            let git_dir = root.open_child_dir(".git")?;
            // HEAD is read best-effort: identity resolution has never required
            // it (only `resolve_git_branch` does), so a `.git` directory without
            // a readable HEAD still admits, exactly as before.
            let head_content = git_dir.read_child_file("HEAD").ok();
            // Primary branches return the normalized workspace (today's behaviour).
            Ok(GitMetadata {
                workspace,
                head_content,
                root,
            })
        }
        ChildKind::File => {
            let head_content = resolve_linked_worktree(&root, &workspace, &git_entry)?;
            Ok(GitMetadata {
                // Linked worktrees keep the platform's canonical spelling; the
                // native admin backlink is defined in terms of that identity.
                workspace: canonical_workspace,
                head_content: Some(head_content),
                root,
            })
        }
        ChildKind::Absent | ChildKind::Other => Err(not_git_root(&workspace)),
    }
}

/// Prove a native linked worktree and return its `HEAD` content in a single
/// pass over retained, capability-rooted, no-follow handles.
///
/// The two anchors (the common Git directory and, through it, the admin
/// directory) and every child handle are opened once and **kept alive** until
/// every value the proof depends on has been read from them. Holding the
/// capability handles for the whole resolution IS the defence against a
/// check/use swap: on Unix an open directory fd keeps addressing the original
/// object regardless of any rename above it, and on Windows the OS refuses to
/// rename a directory that has an open handle to it or to a descendant, so the
/// swap is prevented outright. Attacker-controlled content therefore cannot
/// influence the result — the proof reads the legitimate object or fails
/// closed. Structural preconditions are computed lexically.
fn resolve_linked_worktree(
    root: &CapRoot,
    workspace: &Path,
    git_entry: &Path,
) -> Result<String, WorkspaceError> {
    // The gitfile is read from the workspace root handle, no-follow.
    let gitfile = root.read_child_file(".git")?;
    let directive = parse_single_line(&gitfile, workspace)?;
    let admin_text = directive
        .strip_prefix("gitdir: ")
        .ok_or_else(|| not_git_root(workspace))?;
    if admin_text.is_empty() || admin_text.trim() != admin_text {
        return Err(not_git_root(workspace));
    }
    let admin_candidate = resolve_metadata_pointer(admin_text, workspace, workspace)?;
    let admin_lexical = normalize_lexical(&admin_candidate);

    // Structural preconditions, computed lexically: `.../worktrees/<name>`.
    let worktrees_lexical = admin_lexical
        .parent()
        .ok_or_else(|| not_git_root(workspace))?;
    if worktrees_lexical.file_name() != Some(std::ffi::OsStr::new("worktrees")) {
        return Err(not_git_root(workspace));
    }
    let common_lexical = worktrees_lexical
        .parent()
        .ok_or_else(|| not_git_root(workspace))?
        .to_path_buf();
    let admin_name = admin_lexical
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| not_git_root(workspace))?
        .to_owned();

    // Cross-boundary exception: the linked-worktree admin metadata is outside
    // the workspace root, so the common Git directory gets its own retained
    // anchor (invariant 5). This is the smallest root containing everything the
    // proof reads (`objects`, `refs`/`reftable`, `HEAD`, `worktrees/<name>`),
    // and reaching the admin dir from it by a no-follow walk makes the admitted
    // admin object the validated one by construction.
    let common_root = CapRoot::open_anchor_nofollow_leaf(&common_lexical)?;

    // Deterministic swap window for the colocated adversarial tests; a no-op in
    // production. The common authority has just been established and is HELD for
    // the remainder of this function, so any swap staged here cannot influence
    // the values read below through the retained handle.
    toctou_checkpoint("common_validated");

    let worktrees_root = common_root.open_child_dir("worktrees")?;
    let admin_root = worktrees_root.open_child_dir(&admin_name)?;

    // Deterministic swap window for the colocated adversarial tests; a no-op in
    // production. The admin authority has just been established and is HELD for
    // the remainder of this function, so a swap of any ancestor here cannot
    // influence the admin values read below through the retained handle.
    toctou_checkpoint("admin_validated");

    common_root.require_child_dir("objects")?;
    // Reference storage: files backend (`refs`) or reftable backend, matching
    // today's semantics exactly.
    match common_root.child_kind("refs") {
        ChildKind::Absent => common_root.require_child_dir("reftable")?,
        _ => common_root.require_child_dir("refs")?,
    }
    let _ = common_root.read_child_file("HEAD")?;

    // `commondir`: a single clean line resolved lexically relative to the admin
    // path (absolute stays absolute); it must name the common directory. The
    // content is read from the retained admin handle.
    let commondir_content = admin_root.read_child_file("commondir")?;
    let commondir_line = parse_single_line(&commondir_content, workspace)?;
    if commondir_line.is_empty() || commondir_line.trim() != commondir_line {
        return Err(not_git_root(workspace));
    }
    let commondir_pointer = PathBuf::from(commondir_line);
    let resolved_common = if commondir_pointer.is_absolute() {
        normalize_lexical(&commondir_pointer)
    } else {
        normalize_lexical(&admin_lexical.join(&commondir_pointer))
    };
    if resolved_common != normalize_lexical(&common_lexical) {
        return Err(not_git_root(workspace));
    }

    // `gitdir` backlink: same hygiene, resolved with the same semantics as
    // today's `resolve_metadata_pointer`, and required to name the workspace's
    // `.git` entry. Both absolute and relative directives are accepted. Read
    // from the retained admin handle.
    let backlink_content = admin_root.read_child_file("gitdir")?;
    let backlink = parse_single_line(&backlink_content, workspace)?;
    if backlink.is_empty() || backlink.trim() != backlink {
        return Err(not_git_root(workspace));
    }
    let backlink_candidate = resolve_metadata_pointer(backlink, &admin_lexical, workspace)?;
    if normalize_metadata_pointer(backlink, &backlink_candidate) != git_entry {
        return Err(not_git_root(workspace));
    }

    // The admin `HEAD` is read from the SAME retained admin handle every other
    // admin value came from, so the returned identity is the legitimate one.
    let head_content = admin_root.read_child_file("HEAD")?;

    // Every capability handle above (`common_root`, `worktrees_root`,
    // `admin_root`, and the caller's workspace-root `root`) is still alive at
    // this point; they are dropped only now, as this function returns, after
    // every proof-bearing value has been read.
    Ok(head_content)
}

/// True when `path` names exactly one ordinary component — no separator, no
/// `.`/`..`, no root and no prefix.
///
/// The capability-root child helpers accept only single components so they
/// cannot be handed a multi-component path, which would re-introduce the very
/// full-path resolution semantics this module exists to eliminate.
fn is_single_component(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
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
    workspace_id_from_metadata(resolve_git_metadata(path)?)
}

/// Load or create the persisted workspace identifier from an ALREADY-PROVEN
/// workspace, consuming the root handle retained by that proof.
///
/// Callers that have just resolved the Git metadata must use this instead of
/// [`load_or_create_workspace_id`]; re-entering through the path-based wrapper
/// would resolve the workspace a second time and reopen the check/use window
/// the proof just closed.
fn workspace_id_from_metadata(metadata: GitMetadata) -> Result<Uuid, EngramError> {
    let canonical = normalize_canonical(metadata.workspace);

    // The workspace root handle RETAINED from the Git authenticity proof — not a
    // second ambient resolution. Reopening `canonical` here would reintroduce a
    // check/use window between the proof and the identity read (threat T5): an
    // ancestor swapped in that gap would redirect `.engram/.workspace-id` to an
    // attacker-controlled directory even though the proof had just succeeded.
    let root = metadata.root;
    let engram_dir = canonical.join(".engram");
    // Create-only; not a trust decision, so it may stay path-relative to the
    // handle. `create_dir_all` is idempotent under concurrent cold starts.
    root.dir.create_dir_all(".engram").map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: engram_dir.display().to_string(),
        })
    })?;
    let canonical_engram = normalize_canonical(engram_dir.canonicalize().map_err(|_| {
        EngramError::System(SystemError::FlushFailed {
            path: engram_dir.display().to_string(),
        })
    })?);
    // Containment guard preserved: the identity state must live inside the root.
    if !canonical_engram.starts_with(&canonical) {
        return Err(EngramError::Workspace(WorkspaceError::PathEscape {
            attempted: canonical_engram,
            root: canonical,
        }));
    }
    let id_path = canonical_engram.join(".workspace-id");

    // Retained `.engram` handle, opened no-follow through the root handle; every
    // `.workspace-id` access below is derived from it.
    let engram_root = root
        .open_child_dir(".engram")
        .map_err(|_| workspace_id_io_error(&engram_dir))?;

    if let Some(existing) = read_workspace_id_via(&engram_root, &id_path)? {
        return Ok(existing);
    }

    let workspace_id = Uuid::new_v4();
    // Create AND publish `.workspace-id` through the retained `.engram` handle.
    // The old shape staged a temp file and called `persist_noclobber` by ambient
    // pathname AFTER the handle was retained, so `.engram` could be renamed
    // aside and substituted between the proof and the publish, landing the
    // identity in an attacker-controlled directory — a write outside the proof
    // that re-reading through the handle can detect but cannot undo.
    //
    // `create_new` on the final name is `O_CREAT | O_EXCL`: exactly one cold
    // start wins, and it never clobbers an identity another start already
    // published. A handle-relative temp-then-rename cannot be used instead,
    // because `rename` replaces the destination and concurrent first binds would
    // then diverge onto different identities.
    match engram_root.create_new_child_file(".workspace-id") {
        Ok(mut file) => {
            writeln!(file, "{workspace_id}").map_err(|_| {
                EngramError::System(SystemError::FlushFailed {
                    path: id_path.display().to_string(),
                })
            })?;
            file.sync_all().map_err(|_| {
                EngramError::System(SystemError::FlushFailed {
                    path: id_path.display().to_string(),
                })
            })?;
            drop(file);
            // Re-read through the retained handle so the returned value is
            // handle-derived even on the winning path.
            read_workspace_id_via(&engram_root, &id_path)?
                .ok_or_else(|| workspace_id_io_error(&id_path))
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Lost the create race. The winner creates the leaf before it writes
            // to it, so a read here can briefly observe an empty file; wait for
            // the winner to finish rather than reporting a corrupt identity.
            read_workspace_id_awaiting_writer(&engram_root, &id_path)
        }
        Err(_) => Err(EngramError::System(SystemError::FlushFailed {
            path: id_path.display().to_string(),
        })),
    }
}

/// Read `.workspace-id` through the retained handle, tolerating the brief window
/// in which the winning cold start has created the leaf but not yet written it.
///
/// Bounded so a genuinely corrupt or unsafe leaf still surfaces as an error
/// rather than hanging: the writer only has to emit one UUID line, so the budget
/// is far larger than the real window.
fn read_workspace_id_awaiting_writer(
    engram_root: &CapRoot,
    id_path: &Path,
) -> Result<Uuid, EngramError> {
    const ATTEMPTS: u32 = 100;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(10);

    let mut last_error = None;
    for attempt in 0..ATTEMPTS {
        match read_workspace_id_via(engram_root, id_path) {
            Ok(Some(existing)) => return Ok(existing),
            Ok(None) => {}
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < ATTEMPTS {
            std::thread::sleep(BACKOFF);
        }
    }
    Err(last_error.unwrap_or_else(|| workspace_id_io_error(id_path)))
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
    // Resolve the workspace ONCE and keep the proof's retained root handle for
    // both the identity probe and the identity read. The previous shape probed
    // through a handle, dropped it, and then re-entered `workspace_key`, which
    // resolved the pathname again — so a root substitution between those two
    // calls could make the probe inspect one workspace while the UUID backing
    // the daemon IPC endpoint came from another, preserving the very TOCTOU
    // this release unit closes.
    let metadata = resolve_git_metadata(path)?;
    let canonical = normalize_canonical(metadata.workspace.clone());

    // The `.workspace-id` existence probe descends from the proof's root by a
    // no-follow `.engram` open (U5). An absent `.engram/` means no persisted
    // identity yet; a linked/reparse `.engram` fails closed rather than
    // downgrading to the legacy path-hash fallback.
    if workspace_id_present_via(&metadata.root, &canonical)? {
        return workspace_id_from_metadata(metadata).map(|id| id.to_string());
    }

    if let Some(pid_file) = read_pid_file_via(&metadata.root) {
        if pid_file.verify_alive()? {
            // Branch comes from the HEAD content this proof already validated,
            // not from a fresh `resolve_git_branch` walk.
            let branch = metadata
                .head_content
                .as_deref()
                .map_or_else(|| "default".to_string(), branch_from_head);
            tracing::info!(
                event_type = "workspace_id_fallback",
                workspace = %canonical.display(),
                "workspace-id missing while legacy daemon is live; using path-hash fallback"
            );
            return Ok(workspace_hash(&canonical, &branch));
        }
    }

    workspace_id_from_metadata(metadata).map(|id| id.to_string())
}

fn workspace_id_path(path: &Path) -> PathBuf {
    path.join(".engram").join(".workspace-id")
}

/// Read the legacy daemon PID metadata through the workspace's ALREADY-VALIDATED
/// root handle.
///
/// The legacy path-hash fallback is the one branch of `daemon_key_for_workspace`
/// that can return a key without re-proving the workspace, so reading its input
/// by ambient pathname would let an attacker who substitutes the root after the
/// proof plant a PID file naming any live process and force the legacy key to be
/// selected. Descending `.engram/run/engram.pid` through retained no-follow
/// handles binds the whole daemon-key decision to a single proof.
fn read_pid_file_via(root: &CapRoot) -> Option<PidFile> {
    let engram_root = root.open_child_dir(".engram").ok()?;
    let run_root = engram_root
        .open_child_dir(crate::shim::pidfile::PID_RUN_DIR)
        .ok()?;
    let raw = run_root
        .read_child_file(crate::shim::pidfile::PID_FILE)
        .ok()?;
    PidFile::parse(&raw)
}

/// Probe `.workspace-id` presence through the caller's ALREADY-VALIDATED root
/// handle. Returns `false` when `.engram/` or the leaf is absent, `true` when a
/// valid identity leaf exists, and an error when `.engram/` or the leaf is
/// present but unsafe (linked / reparse / non-regular / unparseable).
///
/// Taking the root by reference rather than reopening it keeps the probe and
/// the subsequent identity read anchored to the same proven object, and applies
/// the SAME no-follow reparse gate the load path applies.
fn workspace_id_present_via(root: &CapRoot, canonical: &Path) -> Result<bool, EngramError> {
    let engram_dir = canonical.join(".engram");
    let Ok(engram_root) = root.open_child_dir(".engram") else {
        // Distinguish "no identity yet" from "the identity directory is
        // unsafe". An absent `.engram` is the ordinary cold-start case; a
        // present-but-rejected `.engram` must NOT silently downgrade to the
        // legacy path-hash fallback, so it fails closed.
        return match root.child_kind(".engram") {
            ChildKind::Absent => Ok(false),
            _ => Err(unsafe_workspace_id_error(
                &engram_dir,
                "the identity directory is a link, reparse point, or not a directory",
            )),
        };
    };
    let id_path = workspace_id_path(canonical);
    Ok(read_workspace_id_via(&engram_root, &id_path)?.is_some())
}

/// Read `.workspace-id` through the retained `.engram` handle, no-follow. The
/// authoritative safety check is taken from the OPEN FILE HANDLE and the
/// content is read from that same handle — never a reopen by path. Returns
/// `Ok(None)` only when the leaf is genuinely absent.
fn read_workspace_id_via(
    engram_root: &CapRoot,
    id_path: &Path,
) -> Result<Option<Uuid>, EngramError> {
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
    let mut file = match engram_root
        .dir
        .open_with(Path::new(".workspace-id"), &options)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        // A no-follow open that fails on an existing leaf (e.g. a Unix symlink
        // rejected with `ELOOP`) must not be treated as absent.
        Err(_) => {
            return Err(unsafe_workspace_id_error(
                id_path,
                "linked or reparse leaves are not allowed",
            ));
        }
    };
    let metadata = file
        .metadata()
        .map_err(|_| workspace_id_io_error(id_path))?;
    let is_reparse = {
        #[cfg(windows)]
        {
            use cap_std::fs::MetadataExt as _;
            is_link_or_reparse(
                metadata.file_type().is_symlink(),
                metadata.file_attributes(),
            )
        }
        #[cfg(not(windows))]
        {
            is_link_or_reparse(metadata.file_type().is_symlink(), 0)
        }
    };
    if is_reparse {
        return Err(unsafe_workspace_id_error(
            id_path,
            "linked or reparse leaves are not allowed",
        ));
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
    Uuid::parse_str(raw.trim()).map(Some).map_err(|e| {
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
    let head_content = metadata
        .head_content
        .ok_or_else(|| not_git_root(&metadata.workspace))?;

    Ok(branch_from_head(&head_content))
}

/// Derive the branch name from already-validated `HEAD` content.
///
/// Takes the content, never a path, so a caller that already holds a proof can
/// name the branch without triggering a second resolution.
fn branch_from_head(head_content: &str) -> String {
    let head = head_content.trim();
    if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
        sanitize_branch_for_path(branch)
    } else {
        // Detached HEAD: use first 12 chars of the commit SHA
        head.chars().take(12).collect()
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

    // ── U6: uniform reparse-tag rejection breadth ────────────────────────────
    //
    // The load-bearing U6 claim is that the gate rejects the WHOLE reparse
    // class, not the `is_symlink()` subset. Rust's Windows `is_symlink()`
    // already covers the `SYMLINK` and `MOUNT_POINT` tags, so a junction
    // fixture cannot distinguish the broadened policy from the old one — it
    // passes against the pre-fix code too. Creating a reparse point with an
    // arbitrary third-party tag requires `DeviceIoControl`/`FSCTL_SET_REPARSE_POINT`,
    // which this crate cannot reach under `#![forbid(unsafe_code)]`.
    //
    // These tests therefore assert the policy at its single decision point,
    // where the "tag outside SYMLINK/MOUNT_POINT" case IS representable: a
    // directory entry that is not a symlink but does carry
    // `FILE_ATTRIBUTE_REPARSE_POINT`. That is exactly the input the pre-fix
    // `is_symlink()`-only gate admitted and the new gate must reject.

    #[test]
    fn plain_entry_is_not_treated_as_a_link_or_reparse_point() {
        assert!(
            !super::is_link_or_reparse(false, 0),
            "a plain, non-symlink entry must be admitted"
        );
    }

    #[test]
    fn symlink_entry_is_rejected_on_every_platform() {
        assert!(
            super::is_link_or_reparse(true, 0),
            "a symlink must be rejected on every platform"
        );
    }

    #[cfg(windows)]
    #[test]
    fn non_symlink_reparse_tag_is_rejected() {
        // The regression this test exists for: `is_symlink` is FALSE (the tag is
        // neither SYMLINK nor MOUNT_POINT) but the reparse attribute is set. The
        // pre-fix `is_symlink()`-only gate admitted this; the U6 gate must not.
        assert!(
            super::is_link_or_reparse(false, super::FILE_ATTRIBUTE_REPARSE_POINT),
            "a reparse point whose tag is outside SYMLINK/MOUNT_POINT must be rejected"
        );
    }

    #[cfg(windows)]
    #[test]
    fn reparse_attribute_is_detected_alongside_unrelated_attributes() {
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
        const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;

        let attributes = FILE_ATTRIBUTE_DIRECTORY
            | FILE_ATTRIBUTE_READONLY
            | super::FILE_ATTRIBUTE_REPARSE_POINT;
        assert!(
            super::is_link_or_reparse(false, attributes),
            "the reparse bit must be detected when other attributes are also set"
        );
        assert!(
            !super::is_link_or_reparse(false, FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READONLY),
            "unrelated attributes alone must not trigger rejection"
        );
    }

    // ── U8: admission latency measurement ────────────────────────────────────
    //
    // Ignored by default: this is a measurement, not an assertion, and its
    // numbers are environment-specific. Run explicitly for the closure record:
    //   cargo test --lib db::workspace::tests::measure_admission_latency \
    //     -- --ignored --nocapture
    #[test]
    #[ignore = "measurement for the closure record, not a pass/fail gate"]
    fn measure_admission_latency() {
        use std::time::Instant;

        let fixture = tempfile::tempdir().expect("latency fixture tempdir");
        let primary = fixture.path().join("primary");
        std::fs::create_dir_all(&primary).expect("create primary");
        run_git_fixture(&primary, &["init", "--initial-branch=main"]);
        run_git_fixture(&primary, &["config", "user.name", "Engram Test"]);
        run_git_fixture(&primary, &["config", "user.email", "t@example.invalid"]);
        std::fs::write(primary.join("README.md"), "# fixture\n").expect("write fixture file");
        run_git_fixture(&primary, &["add", "README.md"]);
        run_git_fixture(&primary, &["commit", "-m", "fixture"]);

        let worktree = fixture.path().join("worktree");
        let worktree_arg = worktree.to_string_lossy().to_string();
        run_git_fixture(
            &primary,
            &["worktree", "add", "-b", "latency", &worktree_arg],
        );

        for (label, target) in [
            ("primary checkout", &primary),
            ("linked worktree", &worktree),
        ] {
            let mut samples = Vec::new();
            for _ in 0..64 {
                let started = Instant::now();
                let admitted = super::canonicalize_workspace(&target.to_string_lossy());
                let elapsed = started.elapsed();
                assert!(admitted.is_ok(), "{label} must be admitted: {admitted:?}");
                samples.push(elapsed.as_secs_f64() * 1000.0);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "ADMISSION_LATENCY {label}: median={:.3}ms p95={:.3}ms min={:.3}ms max={:.3}ms",
                samples[samples.len() / 2],
                samples[(samples.len() * 95) / 100],
                samples[0],
                samples[samples.len() - 1]
            );
        }
    }

    fn run_git_fixture(repo: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("run git fixture command");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
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

/// Deterministic ancestor-swap interception tests for `resolve_git_metadata`.
///
/// These colocated tests use the `#[cfg(test)]` `toctou_checkpoint` seam to
/// reproduce the check/use race (threats T1–T3) without any timing dependency:
/// a hook renames a validated ancestor directory aside and moves an
/// attacker-controlled directory into its place at a named checkpoint, then the
/// resolver is asserted to reject the swapped namespace.
#[cfg(test)]
mod toctou_tests {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;

    use tempfile::TempDir;

    use crate::errors::WorkspaceError;

    /// Restores the thread-local swap hook to `None` when the test scope ends,
    /// so an installed hook never leaks into another test on the same thread.
    struct HookGuard;

    impl Drop for HookGuard {
        fn drop(&mut self) {
            super::TOCTOU_SWAP_HOOK.with(|slot| *slot.borrow_mut() = None);
        }
    }

    fn install_hook<F>(hook: F) -> HookGuard
    where
        F: FnMut(&str) + 'static,
    {
        super::TOCTOU_SWAP_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
        HookGuard
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("run git fixture command");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn initialize_primary(primary: &Path) {
        fs::create_dir_all(primary).expect("create primary checkout");
        run_git(primary, &["init", "--initial-branch=main"]);
        run_git(primary, &["config", "user.name", "Engram Test"]);
        run_git(
            primary,
            &["config", "user.email", "engram-test@example.invalid"],
        );
        fs::write(primary.join("README.md"), "# fixture\n").expect("write tracked fixture");
        run_git(primary, &["add", "README.md"]);
        run_git(primary, &["commit", "-m", "fixture"]);
    }

    fn add_linked_worktree(primary: &Path, worktree: &Path, branch: &str) {
        let output = Command::new("git")
            .args(["worktree", "add", "-b", branch])
            .arg(worktree)
            .current_dir(primary)
            .output()
            .expect("create linked worktree");
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn admin_dir_of(worktree: &Path) -> PathBuf {
        let gitfile = fs::read_to_string(worktree.join(".git")).expect("read linked gitfile");
        let pointer = gitfile
            .trim()
            .strip_prefix("gitdir: ")
            .expect("native linked gitfile directive")
            .to_owned();
        PathBuf::from(pointer)
    }

    fn copy_dir_all(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).expect("create copy destination");
        for entry in fs::read_dir(src).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let file_type = entry.file_type().expect("source entry file type");
            let target = dst.join(entry.file_name());
            if file_type.is_dir() {
                copy_dir_all(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).expect("copy source file");
            }
        }
    }

    /// The physical outcome of the attempted directory swap staged by the hook.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum SwapOutcome {
        /// The checkpoint never fired: resolution bailed before the window.
        NotFired,
        /// Both renames completed — the attacker tree is now on the path (Unix).
        Succeeded,
        /// The OS refused a rename (open-handle protection on Windows). This is
        /// the prevention path: the swap was stopped outright.
        Blocked,
    }

    /// Shared state written by the swap hook and read after resolution. An
    /// `Rc<Cell<…>>`/`Rc<RefCell<…>>` pair keeps the whole scenario on a single
    /// thread with no timing dependency whatsoever.
    struct SwapState {
        outcome: Cell<SwapOutcome>,
        reason: RefCell<String>,
    }

    /// What the resolver admitted, reduced to a provenance discriminator: either
    /// a fail-closed rejection or the branch/`HEAD` text it served.
    enum Admitted {
        Rejected,
        Branch(String),
    }

    /// Everything the hook needs to stage a single deterministic swap.
    struct SwapPlan {
        checkpoint: &'static str,
        real: PathBuf,
        aside: PathBuf,
        attacker: PathBuf,
    }

    /// Build a linked-worktree fixture on `branch`, let `build` stage the
    /// attacker tree and describe the swap, install the swap hook, then drive
    /// `entry` (the resolver under test) exactly once. The hook TOLERATES a
    /// refused rename — it records `Blocked` instead of panicking — because on
    /// Windows a blocked rename is the prevention path working as intended.
    fn run_scenario<B, E>(branch: &str, build: B, entry: E) -> (SwapOutcome, String, Admitted)
    where
        B: FnOnce(&Path, &Path, &Path) -> SwapPlan,
        E: FnOnce(&Path) -> Admitted,
    {
        let fixture = TempDir::new().expect("fixture tempdir");
        let primary = fixture.path().join("primary");
        let worktree = fixture.path().join("worktree");
        initialize_primary(&primary);
        add_linked_worktree(&primary, &worktree, branch);

        let SwapPlan {
            checkpoint,
            real,
            aside,
            attacker,
        } = build(fixture.path(), &primary, &worktree);

        let state = Rc::new(SwapState {
            outcome: Cell::new(SwapOutcome::NotFired),
            reason: RefCell::new(String::new()),
        });
        let hook_state = Rc::clone(&state);
        let guard = install_hook(move |name| {
            if name != checkpoint || hook_state.outcome.get() != SwapOutcome::NotFired {
                return;
            }
            // Attempt the swap. A refused rename is recorded, never `.expect()`ed:
            // on Windows the open capability handle makes the OS refuse it, which
            // is exactly the prevention we want to observe.
            match fs::rename(&real, &aside) {
                Ok(()) => match fs::rename(&attacker, &real) {
                    Ok(()) => hook_state.outcome.set(SwapOutcome::Succeeded),
                    Err(error) => {
                        *hook_state.reason.borrow_mut() =
                            format!("attacker move-in refused by the OS: {error}");
                        hook_state.outcome.set(SwapOutcome::Blocked);
                    }
                },
                Err(error) => {
                    *hook_state.reason.borrow_mut() =
                        format!("rename of the real directory refused by the OS: {error}");
                    hook_state.outcome.set(SwapOutcome::Blocked);
                }
            }
        });

        let admitted = entry(&worktree);
        drop(guard);
        let outcome = state.outcome.get();
        let reason = state.reason.borrow().clone();
        (outcome, reason, admitted)
    }

    /// The load-bearing property, asserted as an explicit fail-closed
    /// disjunction. The discriminator is **which object the admitted data came
    /// from**, never merely `Ok` vs `Err`. Both branches are printed so nothing
    /// can pass silently.
    fn assert_provenance_fail_closed(
        scenario: &str,
        outcome: SwapOutcome,
        reason: &str,
        admitted: &Admitted,
    ) {
        match outcome {
            // The fixture never reached its named checkpoint. That is NOT a
            // prevention result — it means the scenario never attempted the
            // swap it exists to attempt, so a deleted or renamed checkpoint
            // would leave this security test silently green. Treat it as a
            // broken fixture.
            SwapOutcome::NotFired => panic!(
                "{scenario}: the interception checkpoint never fired, so no ancestor swap was \
                 attempted. This is a broken fixture, not a prevention result — the scenario \
                 proves nothing unless it reaches its checkpoint."
            ),
            // The OS refused the rename (Windows open-handle protection), so the
            // retained handle prevented the swap: resolution MUST admit the
            // legitimate object.
            SwapOutcome::Blocked => {
                println!("PREVENTED: {scenario}: {reason}");
                match admitted {
                    Admitted::Branch(branch) => {
                        assert!(
                            branch.contains("legit-branch"),
                            "{scenario}: the prevention path must admit the legitimate branch; \
                             got {branch:?}"
                        );
                        assert!(
                            !branch.contains("attacker-branch"),
                            "{scenario}: the prevention path must NEVER admit attacker-branch; \
                             got {branch:?}"
                        );
                    }
                    Admitted::Rejected => panic!(
                        "{scenario}: the prevention path must ADMIT the legitimate object, not \
                         reject it"
                    ),
                }
            }
            // The rename physically completed (Unix). The retained capability
            // handle must still bind the legitimate object, so the result is a
            // fail-closed rejection OR the legitimate branch — but NEVER the
            // attacker's branch.
            SwapOutcome::Succeeded => {
                println!(
                    "SWAPPED: {scenario}: the on-disk rename completed; the retained handle must \
                     still bind the legitimate object"
                );
                match admitted {
                    Admitted::Rejected => {}
                    Admitted::Branch(branch) => {
                        assert!(
                            branch.contains("legit-branch"),
                            "{scenario}: after a completed swap the admitted branch must be the \
                             legitimate one; got {branch:?}"
                        );
                        assert!(
                            !branch.contains("attacker-branch"),
                            "{scenario}: attacker-controlled content must NEVER influence the \
                             result; got {branch:?}"
                        );
                    }
                }
            }
        }
    }

    /// Map a private-struct resolution to a provenance discriminator.
    fn admitted_from_metadata(result: Result<super::GitMetadata, WorkspaceError>) -> Admitted {
        match result {
            Ok(metadata) => Admitted::Branch(metadata.head_content.unwrap_or_default()),
            Err(_) => Admitted::Rejected,
        }
    }

    /// Map a public-API branch resolution to a provenance discriminator.
    fn admitted_from_branch(result: Result<String, WorkspaceError>) -> Admitted {
        match result {
            Ok(branch) => Admitted::Branch(branch),
            Err(_) => Admitted::Rejected,
        }
    }

    /// Stage the `worktrees`-ancestor swap: build an attacker `worktrees` tree
    /// whose same-named admin dir is internally consistent (real `commondir`,
    /// real `gitdir`) but whose `HEAD` names `attacker-branch`. A naive
    /// path-based resolver accepts it; a retained-handle resolver never reads
    /// from it. The swap fires at `admin_validated`.
    fn build_admin_swap(fixture: &Path, _primary: &Path, worktree: &Path) -> SwapPlan {
        let admin_dir = admin_dir_of(worktree);
        let worktrees_dir = admin_dir
            .parent()
            .expect("admin dir has a worktrees parent")
            .to_path_buf();
        let admin_name = admin_dir
            .file_name()
            .expect("admin dir has a name")
            .to_os_string();

        let commondir = fs::read_to_string(admin_dir.join("commondir")).expect("read commondir");
        let backlink = fs::read_to_string(admin_dir.join("gitdir")).expect("read admin backlink");
        let attacker_worktrees = fixture.join("attacker_worktrees");
        let attacker_admin = attacker_worktrees.join(&admin_name);
        fs::create_dir_all(&attacker_admin).expect("create attacker admin dir");
        fs::write(attacker_admin.join("commondir"), &commondir).expect("write attacker commondir");
        fs::write(attacker_admin.join("gitdir"), &backlink).expect("write attacker backlink");
        fs::write(
            attacker_admin.join("HEAD"),
            "ref: refs/heads/attacker-branch\n",
        )
        .expect("write attacker HEAD");

        SwapPlan {
            checkpoint: "admin_validated",
            aside: worktrees_dir.with_file_name("worktrees__aside"),
            real: worktrees_dir,
            attacker: attacker_worktrees,
        }
    }

    /// Stage the whole-common-dir swap: a full, internally consistent copy of
    /// the common `.git` whose admin `HEAD` is rewritten to `attacker-branch`.
    /// The copy is a distinct object with otherwise-identical content, so only a
    /// retained handle — not path re-resolution — keeps the result legitimate.
    /// The swap fires at `common_validated`.
    fn build_common_swap(fixture: &Path, primary: &Path, worktree: &Path) -> SwapPlan {
        let admin_name = admin_dir_of(worktree)
            .file_name()
            .expect("admin dir has a name")
            .to_os_string();
        let common_dir = primary.join(".git");
        let attacker_common = fixture.join("attacker_git");
        copy_dir_all(&common_dir, &attacker_common);
        // Rewrite the copied admin HEAD so the attacker tree names a clearly
        // different branch; the returned identity then discriminates provenance.
        let attacker_admin_head = attacker_common
            .join("worktrees")
            .join(&admin_name)
            .join("HEAD");
        fs::write(&attacker_admin_head, "ref: refs/heads/attacker-branch\n")
            .expect("rewrite attacker admin HEAD");

        SwapPlan {
            checkpoint: "common_validated",
            aside: common_dir.with_file_name(".git__aside"),
            real: common_dir,
            attacker: attacker_common,
        }
    }

    // ── T1: worktrees-ancestor swap while the admin handle is held ───────────

    /// After the admin authority is established the `worktrees` ancestor is
    /// renamed aside and an internally consistent attacker tree is moved into
    /// its place. Because the admin handle is HELD across the checkpoint, the
    /// swap either cannot happen (Windows) or has no effect (Unix): the result
    /// must never carry `attacker-branch`. Asserted through both the private
    /// `resolve_git_metadata` struct and the public `resolve_git_branch` API.
    #[test]
    fn ancestor_swap_after_admin_validation_cannot_admit_attacker_content() {
        let (outcome, reason, admitted) =
            run_scenario("legit-branch", build_admin_swap, |worktree| {
                admitted_from_metadata(super::resolve_git_metadata(worktree))
            });
        assert_provenance_fail_closed(
            "admin-swap via private resolve_git_metadata",
            outcome,
            &reason,
            &admitted,
        );

        // Public-API observation: `resolve_git_branch` is the real entry point
        // exported as `engram::db::workspace::resolve_git_branch`.
        let (outcome, reason, admitted) =
            run_scenario("legit-branch", build_admin_swap, |worktree| {
                admitted_from_branch(super::resolve_git_branch(worktree))
            });
        assert_provenance_fail_closed(
            "admin-swap via public resolve_git_branch",
            outcome,
            &reason,
            &admitted,
        );
    }

    // ── T1: whole-common-dir swap while the common handle is held ────────────

    /// After the common authority is established the whole common `.git` is
    /// renamed aside and a full attacker copy (admin `HEAD` = `attacker-branch`)
    /// is moved into its place. Because the common handle is HELD across the
    /// checkpoint, every child value is still read from the legitimate object:
    /// the result must never carry `attacker-branch`. Asserted through both the
    /// private struct and the public `resolve_git_branch` API.
    #[test]
    fn ancestor_swap_after_common_validation_cannot_admit_attacker_content() {
        let (outcome, reason, admitted) =
            run_scenario("legit-branch", build_common_swap, |worktree| {
                admitted_from_metadata(super::resolve_git_metadata(worktree))
            });
        assert_provenance_fail_closed(
            "common-swap via private resolve_git_metadata",
            outcome,
            &reason,
            &admitted,
        );

        let (outcome, reason, admitted) =
            run_scenario("legit-branch", build_common_swap, |worktree| {
                admitted_from_branch(super::resolve_git_branch(worktree))
            });
        assert_provenance_fail_closed(
            "common-swap via public resolve_git_branch",
            outcome,
            &reason,
            &admitted,
        );
    }
}
