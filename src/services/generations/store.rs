//! Filesystem containment and sealed target resolution for generation storage.
//!
//! [`GenerationStore`] canonicalizes a configured generation root once at
//! construction, then mints sealed [`IndexTarget`] values only for contained
//! regular files or exclusive candidate directories. Candidate exclusivity uses
//! `std::fs::create_dir`: an already-existing directory means another candidate
//! already claimed that target path.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use super::GenerationId;

/// Filename of the durable active-generation manifest within a
/// [`GenerationStore`]'s root. This is the ONLY destination
/// `publish_generation_manifest` accepts (via [`GenerationStore::active_manifest_path`]),
/// so publication can never target an arbitrary, unsealed path.
const ACTIVE_MANIFEST_FILE_NAME: &str = "active.json";

/// Top-level names within a [`GenerationStore`] root that are exclusively
/// owned by store/publication infrastructure and can never be claimed by a
/// candidate directory or resolved as an arbitrary sealed target. Sourced
/// from the concrete constants that mint each name (this module's own
/// [`ACTIVE_MANIFEST_FILE_NAME`] and [`super::publish`]'s advisory lock file
/// name) so the reservation cannot silently drift out of sync with the name
/// each infrastructure path actually writes.
///
/// Copilot review round 4 found that sealing a candidate directory named
/// `active.json` mints a directory at the manifest authority path, after
/// which every publication fails to read the current manifest (recovery
/// would require deleting the invalid candidate). Reserving both names here
/// closes that gap at the same layer that already enforces containment.
const RESERVED_ROOT_NAMES: &[&str] = &[
    ACTIVE_MANIFEST_FILE_NAME,
    super::publish::PUBLISHER_LOCK_FILE_NAME,
];

/// Whether `name` is one of this store's reserved root-level infrastructure
/// names (see [`RESERVED_ROOT_NAMES`]).
fn is_reserved_root_name(name: &std::ffi::OsStr) -> bool {
    RESERVED_ROOT_NAMES
        .iter()
        .any(|reserved| name == std::ffi::OsStr::new(reserved))
}

/// Canonical generation-root wrapper that seals contained indexing targets.
#[derive(Debug, Clone)]
pub struct GenerationStore {
    root: PathBuf,
}

impl GenerationStore {
    /// Construct a store rooted at an existing generation directory.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when `root` cannot be canonicalized or is not a
    /// directory.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let requested_root = root.into();
        let canonical_root =
            requested_root
                .canonicalize()
                .map_err(|source| StoreError::RootCanonicalization {
                    path: requested_root.clone(),
                    source,
                })?;

        if !canonical_root.is_dir() {
            return Err(StoreError::RootNotDirectory {
                path: canonical_root,
            });
        }

        Ok(Self {
            root: canonical_root,
        })
    }

    /// Return the canonical path to the durable active-generation manifest
    /// within this store's root.
    ///
    /// This is the sole sealed destination `publish_generation_manifest`
    /// accepts: a publication caller obtains it only by holding a
    /// `GenerationStore` for the correct, already-validated root, so
    /// publication can never target an arbitrary, unsealed path.
    #[must_use]
    pub fn active_manifest_path(&self) -> PathBuf {
        self.root.join(ACTIVE_MANIFEST_FILE_NAME)
    }

    /// Seal an existing regular file inside the configured generation root.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when `relative_path` is invalid, escapes the
    /// configured root, or does not resolve to a regular file.
    pub fn seal_legacy_direct(
        &self,
        generation_id: GenerationId,
        relative_path: impl AsRef<Path>,
    ) -> Result<IndexTarget, StoreError> {
        let path = self.resolve_regular_file(relative_path.as_ref())?;
        Ok(IndexTarget::legacy_direct(generation_id, path))
    }

    /// Create and seal an exclusive candidate directory inside the generation root.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when `relative_path` is invalid, escapes the
    /// configured root, its parent is not a directory, or the candidate path is
    /// already claimed.
    pub fn seal_candidate(
        &self,
        generation_id: GenerationId,
        relative_path: impl AsRef<Path>,
    ) -> Result<IndexTarget, StoreError> {
        let path = self.create_candidate_directory(relative_path.as_ref(), &generation_id)?;
        Ok(IndexTarget::candidate(generation_id, path))
    }

    fn resolve_regular_file(&self, relative_path: &Path) -> Result<PathBuf, StoreError> {
        validate_relative_path(relative_path)?;

        let candidate = self.root.join(relative_path);
        let canonical = self.canonicalize_contained(&candidate)?;
        self.reject_reserved_root_name(&canonical)?;
        let metadata = fs::metadata(&canonical).map_err(|source| StoreError::Io {
            operation: "read metadata for",
            path: canonical.clone(),
            source,
        })?;

        if !metadata.is_file() {
            return Err(StoreError::NonRegularFile { path: canonical });
        }

        Ok(canonical)
    }

    fn create_candidate_directory(
        &self,
        relative_path: &Path,
        generation_id: &GenerationId,
    ) -> Result<PathBuf, StoreError> {
        let components = validated_components(relative_path)?;
        let (parent, leaf) = split_parent_and_leaf(&components, relative_path)?;
        let parent_directory = if parent.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.canonicalize_contained(&self.root.join(parent))?
        };
        let parent_metadata = fs::metadata(&parent_directory).map_err(|source| StoreError::Io {
            operation: "read metadata for",
            path: parent_directory.clone(),
            source,
        })?;

        if !parent_metadata.is_dir() {
            return Err(StoreError::ParentNotDirectory {
                path: parent_directory,
            });
        }

        let candidate_directory = parent_directory.join(leaf);
        if parent_directory == self.root && is_reserved_root_name(leaf) {
            return Err(StoreError::ReservedName {
                name: leaf.to_string_lossy().into_owned(),
            });
        }
        match fs::create_dir(&candidate_directory) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                return Err(StoreError::CandidateAlreadyExists {
                    generation_id: generation_id.clone(),
                    path: candidate_directory,
                });
            }
            Err(source) => {
                return Err(StoreError::Io {
                    operation: "create directory",
                    path: candidate_directory,
                    source,
                });
            }
        }

        self.canonicalize_contained(&parent_directory.join(leaf))
    }

    fn canonicalize_contained(&self, path: &Path) -> Result<PathBuf, StoreError> {
        let canonical = path.canonicalize().map_err(|source| StoreError::Io {
            operation: "canonicalize",
            path: path.to_path_buf(),
            source,
        })?;

        if canonical.starts_with(&self.root) {
            Ok(canonical)
        } else {
            Err(StoreError::PathEscape {
                root: self.root.clone(),
                path: canonical,
            })
        }
    }

    /// Reject a canonicalized target whose parent is exactly this store's
    /// root AND whose file name matches a [`RESERVED_ROOT_NAMES`] entry.
    ///
    /// Only root-level occurrences are reserved: a nested file that merely
    /// shares a leaf name (e.g. `candidates/active.json`) is unaffected.
    fn reject_reserved_root_name(&self, canonical: &Path) -> Result<(), StoreError> {
        let (Some(parent), Some(name)) = (canonical.parent(), canonical.file_name()) else {
            return Ok(());
        };
        if parent == self.root && is_reserved_root_name(name) {
            return Err(StoreError::ReservedName {
                name: name.to_string_lossy().into_owned(),
            });
        }
        Ok(())
    }
}

/// Sealed indexing target minted only by [`GenerationStore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexTarget {
    kind: IndexTargetKind,
    path: PathBuf,
    generation_id: GenerationId,
}

impl IndexTarget {
    fn legacy_direct(generation_id: GenerationId, path: PathBuf) -> Self {
        Self {
            kind: IndexTargetKind::LegacyDirect,
            path,
            generation_id,
        }
    }

    fn candidate(generation_id: GenerationId, path: PathBuf) -> Self {
        Self {
            kind: IndexTargetKind::Candidate,
            path,
            generation_id,
        }
    }

    /// Return the sealed target kind.
    #[must_use]
    pub const fn kind(&self) -> IndexTargetKind {
        self.kind
    }

    /// Return the sealed canonical path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the generation identifier carried by this target.
    #[must_use]
    pub const fn generation_id(&self) -> &GenerationId {
        &self.generation_id
    }
}

/// Kind discriminator for a sealed indexing target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexTargetKind {
    /// Existing legacy direct-file target.
    LegacyDirect,
    /// Exclusive candidate directory target.
    Candidate,
}

/// Errors returned by [`GenerationStore`] containment and sealing operations.
#[derive(Debug, Error)]
pub enum StoreError {
    /// The configured generation root could not be canonicalized.
    #[error("generation root {path:?} could not be canonicalized: {source}")]
    RootCanonicalization {
        /// The requested generation root path.
        path: PathBuf,
        /// The underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// The configured generation root is not a directory.
    #[error("generation root {path:?} must be a directory")]
    RootNotDirectory {
        /// The canonicalized non-directory path.
        path: PathBuf,
    },
    /// The requested relative path is syntactically unsafe.
    #[error("generation target path {path:?} must be a relative path of normal components")]
    InvalidRelativePath {
        /// The rejected raw path.
        path: PathBuf,
    },
    /// The resolved path escaped the configured generation root.
    #[error("generation target path {path:?} escapes generation root {root:?}")]
    PathEscape {
        /// The canonical generation root.
        root: PathBuf,
        /// The canonicalized escaped path.
        path: PathBuf,
    },
    /// A regular file was required but a different filesystem object was found.
    #[error("generation target {path:?} must be a regular file")]
    NonRegularFile {
        /// The rejected canonical path.
        path: PathBuf,
    },
    /// A directory was required but the resolved parent path was not a directory.
    #[error("generation target parent {path:?} must be a directory")]
    ParentNotDirectory {
        /// The rejected canonical parent path.
        path: PathBuf,
    },
    /// An exclusive candidate directory already exists for the requested target.
    #[error("candidate target for generation {generation_id} already exists at {path:?}")]
    CandidateAlreadyExists {
        /// The generation that attempted the conflicting claim.
        generation_id: GenerationId,
        /// The existing candidate directory.
        path: PathBuf,
    },
    /// The requested target name collides with a name exclusively owned by
    /// store/publication infrastructure (e.g. the active manifest or
    /// publisher lock file) at the generation root.
    #[error(
        "generation target name {name:?} is reserved for store/publication infrastructure at the generation root"
    )]
    ReservedName {
        /// The rejected reserved name.
        name: String,
    },
    /// A filesystem operation failed.
    #[error("failed to {operation} {path:?}: {source}")]
    Io {
        /// The failed filesystem operation.
        operation: &'static str,
        /// The path used for the operation.
        path: PathBuf,
        /// The underlying filesystem error.
        #[source]
        source: io::Error,
    },
}

fn validate_relative_path(path: &Path) -> Result<(), StoreError> {
    validated_components(path).map(|_| ())
}

fn validated_components(path: &Path) -> Result<Vec<OsString>, StoreError> {
    if path.as_os_str().is_empty() {
        return Err(StoreError::InvalidRelativePath {
            path: path.to_path_buf(),
        });
    }

    let components: Result<Vec<_>, _> = path
        .components()
        .map(|component| match component {
            Component::Normal(segment) => Ok(segment.to_os_string()),
            _ => Err(StoreError::InvalidRelativePath {
                path: path.to_path_buf(),
            }),
        })
        .collect();

    components
}

fn split_parent_and_leaf<'a>(
    components: &'a [OsString],
    raw_path: &Path,
) -> Result<(PathBuf, &'a OsString), StoreError> {
    match components.split_last() {
        Some((leaf, parent_components)) => {
            let parent = parent_components
                .iter()
                .fold(PathBuf::new(), |mut acc, component| {
                    acc.push(component);
                    acc
                });
            Ok((parent, leaf))
        }
        None => Err(StoreError::InvalidRelativePath {
            path: raw_path.to_path_buf(),
        }),
    }
}
