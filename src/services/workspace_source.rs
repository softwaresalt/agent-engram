//! Capability-rooted source reads for code-graph indexing.

use std::fmt;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use cap_std::ambient_authority;
use cap_std::fs::Dir;
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
}

impl SourceRejection {
    fn invalid_path(path: &Path, reason: &str) -> Self {
        Self {
            relative_path: path.display().to_string(),
            reason: reason.to_owned(),
            not_found: false,
        }
    }

    fn io(path: &ValidatedRelativePath, operation: &str, error: &std::io::Error) -> Self {
        Self {
            relative_path: path.as_str().to_owned(),
            reason: format!("{operation}: {error}"),
            not_found: error.kind() == std::io::ErrorKind::NotFound,
        }
    }

    fn other(path: &ValidatedRelativePath, reason: impl Into<String>) -> Self {
        Self {
            relative_path: path.as_str().to_owned(),
            reason: reason.into(),
            not_found: false,
        }
    }

    pub(crate) fn is_not_found(&self) -> bool {
        self.not_found
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
        let mut file = match self.root.open(relative_path.as_path()) {
            Ok(file) => file,
            Err(error) => {
                return SourceRead::Rejected(SourceRejection::io(
                    relative_path,
                    "capability open failed",
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
            Err(error) => SourceRead::Rejected(SourceRejection::other(
                relative_path,
                format!("source is not valid UTF-8: {error}"),
            )),
        }
    }

    pub(crate) fn list_child_directories_blocking(
        &self,
        relative_path: &ValidatedRelativePath,
    ) -> Result<Vec<String>, SourceRejection> {
        let entries = self
            .root
            .read_dir(relative_path.as_path())
            .map_err(|error| {
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

fn source_access_error(path: &str, reason: String) -> EngramError {
    CodeGraphError::SourceAccess {
        file_path: path.to_owned(),
        reason,
    }
    .into()
}
