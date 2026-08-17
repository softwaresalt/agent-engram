//! Capability-rooted source reads for code-graph indexing.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use sha2::{Digest, Sha256};

use crate::errors::{CodeGraphError, EngramError};

/// A validated, non-empty path relative to a captured workspace capability.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ValidatedRelativePath {
    path: PathBuf,
    normalized: String,
}

impl ValidatedRelativePath {
    pub(crate) fn new(path: &Path) -> Result<Self, SourceRejection> {
        let Some(raw_path) = path.to_str() else {
            return Err(SourceRejection::invalid_path(
                path,
                "path is not valid UTF-8",
            ));
        };
        if raw_path.is_empty() || has_invalid_raw_component(raw_path) {
            return Err(SourceRejection::invalid_path(
                path,
                "path must contain non-empty normal components only",
            ));
        }

        let mut validated = PathBuf::new();
        let mut normalized = Vec::new();

        for component in path.components() {
            let Component::Normal(component) = component else {
                return Err(SourceRejection::invalid_path(
                    path,
                    "only normal relative path components are allowed",
                ));
            };
            let Some(component) = component.to_str() else {
                return Err(SourceRejection::invalid_path(
                    path,
                    "path is not valid UTF-8",
                ));
            };
            validate_component(path, component)?;
            validated.push(component);
            normalized.push(component);
        }

        if normalized.is_empty() {
            return Err(SourceRejection::invalid_path(
                path,
                "relative path must not be empty",
            ));
        }

        Ok(Self {
            path: validated,
            normalized: normalized.join("/"),
        })
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.normalized
    }

    #[cfg(not(windows))]
    pub(crate) fn starts_with(&self, prefix: &Self) -> bool {
        self.path.starts_with(&prefix.path)
    }

    #[cfg(windows)]
    pub(crate) fn starts_with(&self, prefix: &Self) -> bool {
        let mut components = self.normalized.split('/');
        prefix.normalized.split('/').all(|prefix_component| {
            components
                .next()
                .is_some_and(|component| unicode_lowercase_eq(component, prefix_component))
        })
    }
}

#[cfg(windows)]
fn unicode_lowercase_eq(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_lowercase)
        .eq(right.chars().flat_map(char::to_lowercase))
}

#[cfg(not(windows))]
fn has_invalid_raw_component(path: &str) -> bool {
    path.split('/')
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
}

#[cfg(windows)]
fn has_invalid_raw_component(path: &str) -> bool {
    path.split(['/', '\\'])
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
}

fn validate_component(path: &Path, component: &str) -> Result<(), SourceRejection> {
    if component.contains('\0') {
        return Err(SourceRejection::invalid_path(path, "path contains a NUL"));
    }
    if component.chars().any(char::is_control) {
        return Err(SourceRejection::invalid_path(
            path,
            "path contains control characters",
        ));
    }
    #[cfg(windows)]
    if component.contains([':', '\\'])
        || component.ends_with(['.', ' '])
        || is_reserved_windows_name(component)
    {
        return Err(SourceRejection::invalid_path(
            path,
            "path contains a Windows-ambiguous component",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reserved_windows_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(['.', ' '])
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}

/// Exact bounded UTF-8 bytes accepted for one code-graph source snapshot.
#[derive(Debug)]
pub(crate) struct SourceSnapshot {
    pub(crate) source: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
}

/// Result of trying to read one candidate beneath the workspace capability.
#[derive(Debug)]
pub(crate) enum SourceRead {
    Snapshot(SourceSnapshot),
    Oversized { size_bytes: u64 },
    Rejected(SourceRejection),
}

/// No-follow classification of one direct child of the workspace capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RootChildKind {
    Absent,
    Directory,
    Other,
}

/// Structured rejection of an unsafe or unreadable source candidate.
#[derive(Debug)]
pub(crate) struct SourceRejection {
    relative_path: String,
    reason: String,
    not_found: bool,
    capability_boundary: bool,
}

impl SourceRejection {
    fn invalid_path(path: &Path, reason: &str) -> Self {
        Self {
            relative_path: path.display().to_string(),
            reason: reason.to_owned(),
            not_found: false,
            capability_boundary: true,
        }
    }

    fn io(path: &ValidatedRelativePath, operation: &str, error: &std::io::Error) -> Self {
        Self {
            relative_path: path.as_str().to_owned(),
            reason: format!("{operation}: {error}"),
            not_found: error.kind() == std::io::ErrorKind::NotFound,
            capability_boundary: true,
        }
    }

    fn other(path: &ValidatedRelativePath, reason: impl Into<String>) -> Self {
        Self {
            relative_path: path.as_str().to_owned(),
            reason: reason.into(),
            not_found: false,
            capability_boundary: true,
        }
    }

    fn content(path: &ValidatedRelativePath, reason: impl Into<String>) -> Self {
        Self {
            relative_path: path.as_str().to_owned(),
            reason: reason.into(),
            not_found: false,
            capability_boundary: false,
        }
    }

    pub(crate) fn is_not_found(&self) -> bool {
        self.not_found
    }

    pub(crate) fn is_capability_boundary(&self) -> bool {
        self.capability_boundary
    }
}

impl fmt::Display for SourceRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "source access rejected for '{}': {}",
            self.relative_path, self.reason
        )
    }
}

/// One retained workspace-directory capability for a complete index or sync pass.
#[derive(Clone)]
pub(crate) struct WorkspaceSourceReader {
    root: Arc<Dir>,
    root_path: Arc<PathBuf>,
}

/// A distinct read-only capability for Git metadata outside the workspace.
///
/// This type intentionally exposes only bounded metadata reads and opening a
/// referenced metadata directory. It cannot be used for workspace source reads.
pub(crate) struct GitMetadataReader {
    root: Dir,
    root_path: PathBuf,
}

impl WorkspaceSourceReader {
    pub(crate) async fn open(workspace_root: &Path) -> Result<Self, EngramError> {
        let workspace_root = std::path::absolute(workspace_root).map_err(|error| {
            source_access_error(
                &workspace_root.display().to_string(),
                format!("failed to make workspace root absolute: {error}"),
            )
        })?;
        let display = workspace_root.display().to_string();
        let root_path = workspace_root.clone();
        let root = tokio::task::spawn_blocking(move || {
            Dir::open_ambient_dir(workspace_root, ambient_authority())
        })
        .await
        .map_err(|error| source_access_error(&display, format!("root-open task failed: {error}")))?
        .map_err(|error| source_access_error(&display, format!("failed to open root: {error}")))?;
        Ok(Self {
            root: Arc::new(root),
            root_path: Arc::new(root_path),
        })
    }

    pub(crate) async fn read(
        &self,
        relative_path: &Path,
        max_bytes: u64,
    ) -> Result<SourceRead, EngramError> {
        let validated = match ValidatedRelativePath::new(relative_path) {
            Ok(path) => path,
            Err(rejection) => return Ok(SourceRead::Rejected(rejection)),
        };
        self.read_validated(&validated, max_bytes).await
    }

    pub(crate) async fn read_validated(
        &self,
        relative_path: &ValidatedRelativePath,
        max_bytes: u64,
    ) -> Result<SourceRead, EngramError> {
        let reader = self.clone();
        let relative_path = relative_path.clone();
        let display = relative_path.as_str().to_owned();
        tokio::task::spawn_blocking(move || reader.read_blocking(&relative_path, max_bytes))
            .await
            .map_err(|error| {
                source_access_error(&display, format!("source-read task failed: {error}"))
            })
    }

    pub(crate) fn read_blocking(
        &self,
        relative_path: &ValidatedRelativePath,
        max_bytes: u64,
    ) -> SourceRead {
        read_blocking_from_root(self.root.as_ref(), relative_path, max_bytes)
    }

    pub(crate) fn open_git_metadata_directory_blocking(
        &self,
        path: &Path,
    ) -> std::io::Result<GitMetadataReader> {
        GitMetadataReader::open(resolve_metadata_path(self.root_path.as_ref(), path)?)
    }

    pub(crate) fn classify_root_child_nofollow_blocking(
        &self,
        relative_path: &ValidatedRelativePath,
    ) -> Result<RootChildKind, SourceRejection> {
        if relative_path.as_path().components().count() != 1 {
            return Err(SourceRejection::other(
                relative_path,
                "root-child classification requires exactly one path component",
            ));
        }
        let metadata = match self.root.symlink_metadata(relative_path.as_path()) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(RootChildKind::Absent);
            }
            Err(error) => {
                return Err(SourceRejection::io(
                    relative_path,
                    "capability root-child classification failed",
                    &error,
                ));
            }
        };
        if metadata.file_type().is_symlink() {
            return Ok(RootChildKind::Other);
        }
        #[cfg(windows)]
        {
            use cap_std::fs::MetadataExt as _;

            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Ok(RootChildKind::Other);
            }
        }
        if metadata.is_dir() {
            Ok(RootChildKind::Directory)
        } else {
            Ok(RootChildKind::Other)
        }
    }

    fn open_directory_nofollow(
        &self,
        relative_path: &ValidatedRelativePath,
    ) -> std::io::Result<Dir> {
        open_directory_nofollow_from_root(self.root.as_ref(), relative_path)
    }

    pub(crate) fn list_child_directories_blocking(
        &self,
        relative_path: &ValidatedRelativePath,
    ) -> Result<Vec<String>, SourceRejection> {
        let directory = self
            .open_directory_nofollow(relative_path)
            .map_err(|error| {
                SourceRejection::io(
                    relative_path,
                    "capability no-follow directory open failed",
                    &error,
                )
            })?;
        let entries = directory.entries().map_err(|error| {
            SourceRejection::io(relative_path, "capability directory read failed", &error)
        })?;
        let mut children = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                SourceRejection::io(relative_path, "capability directory entry failed", &error)
            })?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return Err(SourceRejection::other(
                    relative_path,
                    "manifest directory entry is not valid UTF-8",
                ));
            };
            let child = ValidatedRelativePath::new(Path::new(name))?;
            let file_type = entry.file_type().map_err(|error| {
                SourceRejection::io(relative_path, "manifest entry type failed", &error)
            })?;
            if file_type.is_symlink() {
                return Err(SourceRejection::other(
                    relative_path,
                    format!("manifest directory child '{}' is a link", child.as_str()),
                ));
            }
            if file_type.is_dir() {
                children.push(child.as_str().to_owned());
            }
        }
        children.sort();
        Ok(children)
    }
}

impl GitMetadataReader {
    fn open(root_path: PathBuf) -> std::io::Result<Self> {
        let root = open_absolute_directory_nofollow(&root_path)?;
        Ok(Self { root, root_path })
    }

    pub(crate) fn open_directory_blocking(&self, path: &Path) -> std::io::Result<Self> {
        Self::open(resolve_metadata_path(&self.root_path, path)?)
    }

    pub(crate) fn read_blocking(
        &self,
        relative_path: &ValidatedRelativePath,
        max_bytes: u64,
    ) -> SourceRead {
        read_blocking_from_root(&self.root, relative_path, max_bytes)
    }
}

fn read_blocking_from_root(
    root: &Dir,
    relative_path: &ValidatedRelativePath,
    max_bytes: u64,
) -> SourceRead {
    let mut file = match open_file_nofollow_from_root(root, relative_path) {
        Ok(file) => file,
        Err(error) => {
            return SourceRead::Rejected(SourceRejection::io(
                relative_path,
                "capability no-follow open failed",
                &error,
            ));
        }
    };
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            return SourceRead::Rejected(SourceRejection::io(
                relative_path,
                "opened-file metadata failed",
                &error,
            ));
        }
    };
    if !metadata.is_file() {
        return SourceRead::Rejected(SourceRejection::other(
            relative_path,
            "opened object is not a regular file",
        ));
    }
    if metadata.len() > max_bytes {
        return SourceRead::Oversized {
            size_bytes: metadata.len(),
        };
    }

    let read_limit = max_bytes.saturating_add(1);
    let initial_capacity =
        usize::try_from(metadata.len().min(max_bytes).min(64 * 1024)).unwrap_or(64 * 1024);
    let mut bytes = Vec::with_capacity(initial_capacity);
    if let Err(error) = file.by_ref().take(read_limit).read_to_end(&mut bytes) {
        return SourceRead::Rejected(SourceRejection::io(
            relative_path,
            "opened-file read failed",
            &error,
        ));
    }
    let size_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size_bytes > max_bytes {
        return SourceRead::Oversized { size_bytes };
    }
    let content_hash = hex::encode(Sha256::digest(&bytes));
    match String::from_utf8(bytes) {
        Ok(source) => SourceRead::Snapshot(SourceSnapshot {
            source,
            size_bytes,
            content_hash,
        }),
        Err(error) => SourceRead::Rejected(SourceRejection::content(
            relative_path,
            format!("source is not valid UTF-8: {error}"),
        )),
    }
}

fn open_directory_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<Dir> {
    base.open_dir_nofollow(Path::new(component))
}

fn open_file_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt as _;
        use rustix::fs::OFlags;

        let flags = OFlags::NOFOLLOW | OFlags::NONBLOCK;
        let custom_flags = i32::try_from(flags.bits()).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("file no-follow flags are not representable: {error}"),
            )
        })?;
        options.custom_flags(custom_flags);
    }
    base.open_with(Path::new(component), &options)
}

fn open_file_nofollow_from_root(
    root: &Dir,
    relative_path: &ValidatedRelativePath,
) -> std::io::Result<File> {
    let path = relative_path.as_path();
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "validated source path has no final component",
        )
    })?;
    let mut opened_directory = None;
    if let Some(parent) = path.parent() {
        for component in parent.components() {
            let Component::Normal(component) = component else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "validated source parent contains a non-normal component",
                ));
            };
            let base = opened_directory.as_ref().unwrap_or(root);
            opened_directory = Some(open_directory_component_nofollow(base, component)?);
        }
    }
    let base = opened_directory.as_ref().unwrap_or(root);
    open_file_component_nofollow(base, file_name)
}

fn open_directory_nofollow_from_root(
    root: &Dir,
    relative_path: &ValidatedRelativePath,
) -> std::io::Result<Dir> {
    let mut opened_directory = None;
    for component in relative_path.as_path().components() {
        let Component::Normal(component) = component else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "validated directory path contains a non-normal component",
            ));
        };
        let base = opened_directory.as_ref().unwrap_or(root);
        opened_directory = Some(open_directory_component_nofollow(base, component)?);
    }
    opened_directory.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "validated directory path is empty",
        )
    })
}

fn resolve_metadata_path(base: &Path, referenced: &Path) -> std::io::Result<PathBuf> {
    let candidate = if referenced.is_absolute() {
        referenced.to_path_buf()
    } else {
        base.join(referenced)
    };
    if !candidate.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Git metadata path did not resolve to an absolute path",
        ));
    }
    let mut normalized = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Git metadata path traverses above its filesystem root",
                    ));
                }
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    if !normalized.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "normalized Git metadata path is not absolute",
        ));
    }
    Ok(normalized)
}

fn open_absolute_directory_nofollow(path: &Path) -> std::io::Result<Dir> {
    let mut anchor = PathBuf::new();
    let mut children: Vec<OsString> = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => anchor.push(prefix.as_os_str()),
            Component::RootDir => anchor.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::Normal(component) => children.push(component.to_os_string()),
            Component::CurDir | Component::ParentDir => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Git metadata path was not lexically normalized",
                ));
            }
        }
    }
    if !anchor.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Git metadata path has no absolute filesystem anchor",
        ));
    }

    let mut directory = Dir::open_ambient_dir(anchor, ambient_authority())?;
    for component in children {
        directory = open_directory_component_nofollow(&directory, &component)?;
    }
    Ok(directory)
}

fn source_access_error(path: &str, reason: String) -> EngramError {
    CodeGraphError::SourceAccess {
        file_path: path.to_owned(),
        reason,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn symlink_file(source: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(source, link)
    }

    #[cfg(windows)]
    fn symlink_file(source: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(source, link)
    }

    #[cfg(unix)]
    fn symlink_dir(source: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(source, link)
    }

    #[cfg(windows)]
    fn symlink_dir(source: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(source, link)
    }

    #[test]
    fn portable_path_validation_rejects_traversal_nul_and_controls() {
        assert!(ValidatedRelativePath::new(Path::new("../escape.rs")).is_err());
        assert!(ValidatedRelativePath::new(Path::new("nested/../escape.rs")).is_err());
        assert!(ValidatedRelativePath::new(Path::new("nul\0name.rs")).is_err());
        assert!(ValidatedRelativePath::new(Path::new("line\nbreak.rs")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_path_validation_accepts_windows_specific_spellings() {
        for path in [
            "CON",
            "name:stream",
            "trailing.",
            "trailing ",
            r"name\part.rs",
        ] {
            assert!(
                ValidatedRelativePath::new(Path::new(path)).is_ok(),
                "Unix must preserve its native byte/case-sensitive path semantics for {path:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_path_validation_rejects_ambiguous_spellings() {
        for path in ["CON", "name:stream", "trailing.", "trailing "] {
            assert!(
                ValidatedRelativePath::new(Path::new(path)).is_err(),
                "Windows-ambiguous path must be rejected: {path:?}"
            );
        }
        assert!(
            ValidatedRelativePath::new(Path::new(r"name\part.rs")).is_ok(),
            "native Windows separators must normalize as ordinary path separators"
        );
    }

    #[tokio::test]
    async fn semantic_nofollow_rejects_final_and_ancestor_replacements() -> anyhow::Result<()> {
        const SENTINEL: &str = "SEMANTIC_NOFOLLOW_EXTERNAL_SENTINEL";

        for external in [false, true] {
            for replace_ancestor in [false, true] {
                let workspace = tempfile::tempdir()?;
                let outside = tempfile::tempdir()?;
                std::fs::create_dir_all(workspace.path().join("nested"))?;
                std::fs::create_dir_all(workspace.path().join("replacement"))?;
                std::fs::write(workspace.path().join("nested/victim.rs"), "fn safe() {}\n")?;
                std::fs::write(
                    workspace.path().join("replacement/victim.rs"),
                    "fn internal_replacement() {}\n",
                )?;
                std::fs::write(outside.path().join("victim.rs"), SENTINEL)?;

                let reader = WorkspaceSourceReader::open(workspace.path()).await?;
                let relative = ValidatedRelativePath::new(Path::new("nested/victim.rs"))
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;

                if replace_ancestor {
                    std::fs::rename(
                        workspace.path().join("nested"),
                        workspace.path().join("nested-original"),
                    )?;
                    let target = if external {
                        outside.path()
                    } else {
                        Path::new("replacement")
                    };
                    symlink_dir(target, &workspace.path().join("nested"))?;
                } else {
                    std::fs::remove_file(workspace.path().join("nested/victim.rs"))?;
                    let target = if external {
                        outside.path().join("victim.rs")
                    } else {
                        PathBuf::from("../replacement/victim.rs")
                    };
                    symlink_file(&target, &workspace.path().join("nested/victim.rs"))?;
                }

                match reader.read_blocking(&relative, 1024) {
                    SourceRead::Rejected(_) => {}
                    SourceRead::Snapshot(snapshot) => {
                        assert!(
                            !snapshot.source.contains(SENTINEL),
                            "RED:SEMANTIC_NOFOLLOW_OPENED_EXTERNAL_BYTES: \
                             final={}; external={}",
                            !replace_ancestor,
                            external
                        );
                        panic!(
                            "RED:SEMANTIC_NOFOLLOW_ACCEPTED_LINK: final={}; external={}",
                            !replace_ancestor, external
                        );
                    }
                    SourceRead::Oversized { .. } => {
                        panic!("replacement link must be rejected, not classified as oversized");
                    }
                }
            }
        }
        Ok(())
    }
}
