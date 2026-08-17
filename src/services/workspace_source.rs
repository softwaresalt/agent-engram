//! Capability-rooted source reads for code-graph indexing.

use std::ffi::OsStr;
use std::fmt;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

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
        if raw_path.is_empty()
            || raw_path
                .split(['/', '\\'])
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
        {
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

    pub(crate) fn starts_with(&self, prefix: &Self) -> bool {
        self.path.starts_with(&prefix.path)
    }
}

fn validate_component(path: &Path, component: &str) -> Result<(), SourceRejection> {
    if component.contains([':', '\\', '\0']) {
        return Err(SourceRejection::invalid_path(
            path,
            "path contains an unsupported stream, separator, or NUL spelling",
        ));
    }
    if component.chars().any(char::is_control) {
        return Err(SourceRejection::invalid_path(
            path,
            "path contains control characters",
        ));
    }
    if component.ends_with(['.', ' ']) || is_reserved_windows_name(component) {
        return Err(SourceRejection::invalid_path(
            path,
            "path contains a Windows-ambiguous component",
        ));
    }
    Ok(())
}

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
}

impl WorkspaceSourceReader {
    pub(crate) async fn open(workspace_root: &Path) -> Result<Self, EngramError> {
        let workspace_root = workspace_root.to_path_buf();
        let display = workspace_root.display().to_string();
        let root = tokio::task::spawn_blocking(move || {
            Dir::open_ambient_dir(workspace_root, ambient_authority())
        })
        .await
        .map_err(|error| source_access_error(&display, format!("root-open task failed: {error}")))?
        .map_err(|error| source_access_error(&display, format!("failed to open root: {error}")))?;
        Ok(Self {
            root: Arc::new(root),
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
        let mut file = match self.open_file_nofollow(relative_path) {
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

    fn open_file_nofollow(&self, relative_path: &ValidatedRelativePath) -> std::io::Result<File> {
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
                let base = opened_directory.as_ref().unwrap_or(self.root.as_ref());
                opened_directory = Some(open_directory_component_nofollow(base, component)?);
            }
        }
        let base = opened_directory.as_ref().unwrap_or(self.root.as_ref());
        open_file_component_nofollow(base, file_name)
    }

    fn open_directory_nofollow(
        &self,
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
            let base = opened_directory.as_ref().unwrap_or(self.root.as_ref());
            opened_directory = Some(open_directory_component_nofollow(base, component)?);
        }
        opened_directory.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "validated directory path is empty",
            )
        })
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

#[cfg(unix)]
fn open_directory_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<Dir> {
    use cap_std::fs::OpenOptionsExt as _;
    use rustix::fs::OFlags;

    let flags = OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::NONBLOCK;
    let custom_flags = i32::try_from(flags.bits()).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("directory no-follow flags are not representable: {error}"),
        )
    })?;
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(custom_flags);
    let file = base.open_with(Path::new(component), &options)?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "intermediate component is not a regular directory",
        ));
    }
    Ok(Dir::from_std_file(file.into_std()))
}

#[cfg(unix)]
fn open_file_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<File> {
    use cap_std::fs::OpenOptionsExt as _;
    use rustix::fs::OFlags;

    let flags = OFlags::NOFOLLOW | OFlags::NONBLOCK;
    let custom_flags = i32::try_from(flags.bits()).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("file no-follow flags are not representable: {error}"),
        )
    })?;
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(custom_flags);
    base.open_with(Path::new(component), &options)
}

#[cfg(windows)]
fn open_directory_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<Dir> {
    use cap_std::fs::{MetadataExt as _, OpenOptionsExt as _};

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    let file = base.open_with(Path::new(component), &options)?;
    let metadata = file.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "intermediate component is a reparse point or is not a directory",
        ));
    }
    Ok(Dir::from_std_file(file.into_std()))
}

#[cfg(windows)]
fn open_file_component_nofollow(base: &Dir, component: &OsStr) -> std::io::Result<File> {
    use cap_std::fs::{MetadataExt as _, OpenOptionsExt as _};

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = base.open_with(Path::new(component), &options)?;
    let metadata = file.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "final component is a reparse point",
        ));
    }
    Ok(file)
}

fn source_access_error(path: &str, reason: String) -> EngramError {
    CodeGraphError::SourceAccess {
        file_path: path.to_owned(),
        reason,
    }
    .into()
}
