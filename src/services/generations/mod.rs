//! Generation-domain types for immutable published read snapshots.
//!
//! The module exposes a small safe facade: strict generation identity,
//! monotonic publication revision, and the typed manifest payload consumed by
//! sibling generation services.

mod activation;
mod context;
mod manifest;
mod publish;
mod read_inputs;
mod store;

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

pub use self::activation::{
    ExpectedIdentity, GENERATION_DATABASE_FILE_NAME, GenerationActivator, RejectionClass,
    SUPPORTED_MANIFEST_SCHEMA_VERSION, ValidatedManifest, backoff_delay, classify, parse_manifest,
};
pub use self::context::GenerationReadContext;
pub use self::manifest::{
    BranchIdentity, GenerationManifest, GenerationProvenance, ManifestFileDigest, SealedInventory,
    WorkspaceIdentity,
};
pub use self::publish::{
    PublishError, PublisherLock, PublisherLockGuard, list_orphaned_staging_files,
    publish_generation_manifest,
};
pub use self::read_inputs::{
    ENUMERATED_READ_INPUTS, ReadInput, ReadInputKind, descriptors_without_enumerated_inputs,
    duplicate_enumerated_ids, read_mode_descriptor_names,
};
pub use self::store::{GenerationStore, IndexTarget, IndexTargetKind, StoreError};
/// Strict single-component identifier for a published generation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenerationId(String);

impl GenerationId {
    /// Construct a generation identifier after validating it is a single safe component.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationIdError`] when `value` is empty or contains `/`, `\\`,
    /// `..`, or `:` (the last rejected so a Windows drive-prefix such as `C:`
    /// can never be mistaken for a single safe path component).
    pub fn new(value: impl Into<String>) -> Result<Self, GenerationIdError> {
        let value = value.into();
        validate_generation_id(&value)?;
        Ok(Self(value))
    }

    /// Borrow the underlying generation identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for GenerationId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for GenerationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for GenerationId {
    type Error = GenerationIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for GenerationId {
    type Error = GenerationIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for GenerationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for GenerationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Publication revision for the durable active-generation manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GenerationRevision(u64);

impl GenerationRevision {
    /// Construct a revision value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the numeric revision.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Advance to `next` only when it is strictly greater than `self`.
    ///
    /// # Errors
    ///
    /// Returns [`RevisionError::NonIncreasing`] when `next` is less than or equal
    /// to the current revision.
    pub fn advance_to(self, next: Self) -> Result<Self, RevisionError> {
        if next <= self {
            return Err(RevisionError::NonIncreasing {
                current: self,
                attempted: next,
            });
        }

        Ok(next)
    }
}

impl fmt::Display for GenerationRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Error returned when a generation identifier is invalid.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenerationIdError {
    /// The identifier was empty.
    #[error("generation ID must not be empty")]
    Empty,
    /// The identifier was not a strict single path component.
    #[error("generation ID {value:?} must be a single component without '/', '\\\\', '..', or ':'")]
    InvalidComponent {
        /// The rejected raw identifier.
        value: String,
    },
}

/// Error returned when a publication revision does not advance monotonically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RevisionError {
    /// The attempted revision was not strictly greater than the current value.
    #[error(
        "generation revision must increase strictly (current: {current}, attempted: {attempted})"
    )]
    NonIncreasing {
        /// The current published revision.
        current: GenerationRevision,
        /// The attempted successor revision.
        attempted: GenerationRevision,
    },
}

fn validate_generation_id(value: &str) -> Result<(), GenerationIdError> {
    if value.is_empty() {
        return Err(GenerationIdError::Empty);
    }

    // `:` is rejected outright rather than relying on `Path::components()` to
    // catch a Windows drive-prefix like `C:` (`Component::Prefix`, not a
    // `Component::Normal` component): `Path::components()` parsing is
    // platform-dependent (e.g. `\\` is a separator only on Windows), so
    // switching this whole check to a components-based one would silently
    // change the portable, cross-platform behavior of the existing `/`/`\\`
    // rejections below. A plain substring check keeps every rejection
    // identical on every target platform. Generation IDs are simple
    // identifiers (hashes/UUIDs/slugs) that never legitimately need a colon,
    // so this is purely additive hardening, not a behavior narrowing for any
    // legitimate value.
    if value == "."
        || value.contains('/')
        || value.contains('\\')
        || value.contains("..")
        || value.contains(':')
    {
        return Err(GenerationIdError::InvalidComponent {
            value: value.to_owned(),
        });
    }

    Ok(())
}
