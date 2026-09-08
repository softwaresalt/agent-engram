//! Serializable generation-manifest domain types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{GenerationId, GenerationRevision};

/// Identity of the indexed branch captured by a generation manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchIdentity {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_revision: Option<String>,
}

impl BranchIdentity {
    /// Construct branch identity for a manifest.
    #[must_use]
    pub fn new(name: impl Into<String>, source_revision: Option<String>) -> Self {
        Self {
            name: name.into(),
            source_revision,
        }
    }

    /// Return the branch name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the indexed source revision when one is recorded.
    #[must_use]
    pub fn source_revision(&self) -> Option<&str> {
        self.source_revision.as_deref()
    }
}

/// Opaque workspace identity captured by a generation manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentity {
    workspace_id: String,
}

impl WorkspaceIdentity {
    /// Construct workspace identity for a manifest.
    #[must_use]
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
        }
    }

    /// Return the stored workspace identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.workspace_id
    }
}

/// Digest for one file sealed into the published generation inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFileDigest {
    path: String,
    sha256: String,
}

impl ManifestFileDigest {
    /// Construct a single inventory entry.
    #[must_use]
    pub fn new(path: impl Into<String>, sha256: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
        }
    }

    /// Return the file path recorded in the inventory.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Return the recorded SHA-256 digest.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// The sealed set of files and digests that define a published generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedInventory {
    files: Vec<ManifestFileDigest>,
}

impl SealedInventory {
    /// Construct a sealed inventory from concrete file digests.
    #[must_use]
    pub fn new(files: Vec<ManifestFileDigest>) -> Self {
        Self { files }
    }

    /// Return the ordered inventory entries.
    #[must_use]
    pub fn files(&self) -> &[ManifestFileDigest] {
        &self.files
    }
}

/// Minimal provenance for a published generation manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationProvenance {
    producer: String,
    created_at: DateTime<Utc>,
}

impl GenerationProvenance {
    /// Construct manifest provenance.
    #[must_use]
    pub fn new(producer: impl Into<String>, created_at: DateTime<Utc>) -> Self {
        Self {
            producer: producer.into(),
            created_at,
        }
    }

    /// Return the producer identifier that built or published the generation.
    #[must_use]
    pub fn producer(&self) -> &str {
        &self.producer
    }

    /// Return the manifest creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

/// Durable manifest naming the published generation and its validation facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationManifest {
    generation_id: GenerationId,
    revision: GenerationRevision,
    schema_version: String,
    branch: BranchIdentity,
    workspace: WorkspaceIdentity,
    inventory: SealedInventory,
    provenance: GenerationProvenance,
}

impl GenerationManifest {
    /// Construct a generation manifest.
    #[must_use]
    pub fn new(
        generation_id: GenerationId,
        revision: GenerationRevision,
        schema_version: impl Into<String>,
        branch: BranchIdentity,
        workspace: WorkspaceIdentity,
        inventory: SealedInventory,
        provenance: GenerationProvenance,
    ) -> Self {
        Self {
            generation_id,
            revision,
            schema_version: schema_version.into(),
            branch,
            workspace,
            inventory,
            provenance,
        }
    }

    /// Return the published generation identifier.
    #[must_use]
    pub const fn generation_id(&self) -> &GenerationId {
        &self.generation_id
    }

    /// Return the monotonic publication revision.
    #[must_use]
    pub const fn revision(&self) -> GenerationRevision {
        self.revision
    }

    /// Return the snapshot schema version.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Return the recorded branch identity.
    #[must_use]
    pub const fn branch(&self) -> &BranchIdentity {
        &self.branch
    }

    /// Return the recorded workspace identity.
    #[must_use]
    pub const fn workspace(&self) -> &WorkspaceIdentity {
        &self.workspace
    }

    /// Return the sealed inventory.
    #[must_use]
    pub const fn inventory(&self) -> &SealedInventory {
        &self.inventory
    }

    /// Return the manifest provenance.
    #[must_use]
    pub const fn provenance(&self) -> &GenerationProvenance {
        &self.provenance
    }
}
