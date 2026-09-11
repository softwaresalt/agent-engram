//! Read-mode input inventory (plan unit F24).
//!
//! A ReadServer's whole promise is that every byte it serves comes from one
//! sealed generation. That promise is only as good as the list of inputs it
//! was checked against, so this module keeps that list as code rather than as
//! prose in a plan document: an input that is not enumerated here has never
//! been reviewed, and an enumerated input that is not classified is a gap the
//! contract test refuses to let pass silently.
//!
//! The inventory is deliberately split into two independent tables:
//!
//! * [`ENUMERATED_READ_INPUTS`] — the breadth pass (142.033.001-ST). Every
//!   input observable from a Read-mode descriptor, with no judgement attached.
//! * [`CLASSIFIED_READ_INPUTS`] — the ownership verdict (142.033.002-ST).
//!
//! Keeping them separate is what makes the guard meaningful. If enumeration
//! and classification were one table, adding a row would classify it by
//! construction and the guard could never fire. Because they are two tables
//! reconciled by [`unclassified_inputs`] and [`unenumerated_classifications`],
//! a newly discovered input is RED until somebody writes down what owns it.

use std::collections::BTreeSet;

use crate::tools::capabilities::{self, CapabilityClass};

// ── Enumeration (142.033.001-ST) ─────────────────────────────────────────────

/// The category of a Read-mode-observable input.
///
/// The categories map to the distinct trust boundaries a read request can
/// cross, not to implementation modules: a relation inside the generation
/// database and a file under the live workspace root have very different
/// mutability guarantees even when the same handler touches both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadInputKind {
    /// A named relation inside the generation database.
    DatabaseRelation,
    /// A field of the in-memory workspace snapshot held by the daemon.
    WorkspaceSnapshotField,
    /// A path under the live (indexer-mutable) workspace root.
    WorkspaceRootPath,
    /// A path under the workspace's `.engram/` control directory.
    EngramPath,
    /// A process environment variable.
    EnvironmentValue,
}

impl ReadInputKind {
    /// Canonical setting string for this kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DatabaseRelation => "database_relation",
            Self::WorkspaceSnapshotField => "workspace_snapshot_field",
            Self::WorkspaceRootPath => "workspace_root_path",
            Self::EngramPath => "engram_path",
            Self::EnvironmentValue => "environment_value",
        }
    }

    /// Every kind, so the breadth pass can assert it covered all of them.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::DatabaseRelation,
            Self::WorkspaceSnapshotField,
            Self::WorkspaceRootPath,
            Self::EngramPath,
            Self::EnvironmentValue,
        ]
    }
}

impl std::fmt::Display for ReadInputKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One input observable from Read-mode dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadInput {
    /// Stable identifier, unique across the whole inventory.
    pub id: &'static str,
    /// Which trust boundary this input sits behind.
    pub kind: ReadInputKind,
    /// How a read request reaches it. `"*"` means "any Read descriptor".
    pub reached_via: &'static str,
}

const fn input(id: &'static str, kind: ReadInputKind, reached_via: &'static str) -> ReadInput {
    ReadInput {
        id,
        kind,
        reached_via,
    }
}

/// Every input reachable from a Read-mode descriptor.
///
/// This is a breadth pass on purpose: entries are recorded even when the
/// answer is obviously "generation-owned", because the value of the inventory
/// is completeness, not novelty. The database relations are listed
/// individually rather than as "the database" because `query_graph` accepts
/// caller-authored read queries, so the reachable surface really is every
/// relation the schema declares.
pub const ENUMERATED_READ_INPUTS: &[ReadInput] = &[
    // -- Generation database relations -------------------------------------
    input(
        "db.backlog_content_record",
        ReadInputKind::DatabaseRelation,
        "query_memory, unified_search, query_graph",
    ),
    input(
        "db.backlog_edge",
        ReadInputKind::DatabaseRelation,
        "query_graph",
    ),
    input(
        "db.backlog_node",
        ReadInputKind::DatabaseRelation,
        "query_memory, query_graph",
    ),
    input(
        "db.calls_edge",
        ReadInputKind::DatabaseRelation,
        "map_code, impact_analysis, query_graph",
    ),
    input(
        "db.class_code",
        ReadInputKind::DatabaseRelation,
        "list_symbols, unified_search, query_graph",
    ),
    input(
        "db.class_embedding",
        ReadInputKind::DatabaseRelation,
        "unified_search, query_graph",
    ),
    input(
        "db.class_meta",
        ReadInputKind::DatabaseRelation,
        "list_symbols, map_code, query_graph",
    ),
    input(
        "db.commit_node",
        ReadInputKind::DatabaseRelation,
        "query_changes, query_graph",
    ),
    input(
        "db.concerns_edge",
        ReadInputKind::DatabaseRelation,
        "query_changes, query_graph",
    ),
    input(
        "db.content_record",
        ReadInputKind::DatabaseRelation,
        "query_memory, unified_search, query_graph",
    ),
    input(
        "db.dataset_node",
        ReadInputKind::DatabaseRelation,
        "query_graph, lint_dax",
    ),
    input(
        "db.defines_edge",
        ReadInputKind::DatabaseRelation,
        "map_code, impact_analysis, query_graph",
    ),
    input(
        "db.file_hash",
        ReadInputKind::DatabaseRelation,
        "get_workspace_statistics, query_graph",
    ),
    input(
        "db.file_node",
        ReadInputKind::DatabaseRelation,
        "map_code, list_symbols, query_graph",
    ),
    input(
        "db.function_code",
        ReadInputKind::DatabaseRelation,
        "list_symbols, unified_search, query_graph",
    ),
    input(
        "db.function_embedding",
        ReadInputKind::DatabaseRelation,
        "unified_search, query_graph",
    ),
    input(
        "db.function_meta",
        ReadInputKind::DatabaseRelation,
        "list_symbols, map_code, impact_analysis, query_graph",
    ),
    input(
        "db.import_node",
        ReadInputKind::DatabaseRelation,
        "map_code, query_graph",
    ),
    input(
        "db.imports_edge",
        ReadInputKind::DatabaseRelation,
        "map_code, impact_analysis, query_graph",
    ),
    input(
        "db.index_canonical_workspace_snapshot",
        ReadInputKind::DatabaseRelation,
        "get_workspace_status, get_workspace_statistics",
    ),
    input(
        "db.inherits_from_edge",
        ReadInputKind::DatabaseRelation,
        "map_code, impact_analysis, query_graph",
    ),
    input(
        "db.interface_code",
        ReadInputKind::DatabaseRelation,
        "list_symbols, unified_search, query_graph",
    ),
    input(
        "db.interface_embedding",
        ReadInputKind::DatabaseRelation,
        "unified_search, query_graph",
    ),
    input(
        "db.interface_meta",
        ReadInputKind::DatabaseRelation,
        "list_symbols, map_code, query_graph",
    ),
    input(
        "db.lineage_edge",
        ReadInputKind::DatabaseRelation,
        "query_graph, lint_dax",
    ),
    input(
        "db.lineage_edge_evidence",
        ReadInputKind::DatabaseRelation,
        "query_graph",
    ),
    input(
        "db.lineage_index_state",
        ReadInputKind::DatabaseRelation,
        "get_health_report, query_graph",
    ),
    input(
        "db.powerbi_edge",
        ReadInputKind::DatabaseRelation,
        "query_graph, lint_dax",
    ),
    input(
        "db.powerbi_file_index_state",
        ReadInputKind::DatabaseRelation,
        "get_health_report, query_graph",
    ),
    input(
        "db.powerbi_node",
        ReadInputKind::DatabaseRelation,
        "query_graph, lint_dax",
    ),
    input(
        "db.references_edge",
        ReadInputKind::DatabaseRelation,
        "map_code, impact_analysis, query_graph",
    ),
    input("db.rewrite", ReadInputKind::DatabaseRelation, "query_graph"),
    input(
        "db.schema_meta",
        ReadInputKind::DatabaseRelation,
        "get_health_report, query_graph",
    ),
    input(
        "db.staged_call",
        ReadInputKind::DatabaseRelation,
        "map_code, query_graph",
    ),
    // -- Workspace snapshot fields -----------------------------------------
    input(
        "snapshot.workspace_id",
        ReadInputKind::WorkspaceSnapshotField,
        "*",
    ),
    input(
        "snapshot.workspace_uuid",
        ReadInputKind::WorkspaceSnapshotField,
        "get_workspace_status",
    ),
    input(
        "snapshot.branch",
        ReadInputKind::WorkspaceSnapshotField,
        "*",
    ),
    input(
        "snapshot.data_dir",
        ReadInputKind::WorkspaceSnapshotField,
        "*",
    ),
    input(
        "snapshot.path",
        ReadInputKind::WorkspaceSnapshotField,
        "get_workspace_status, unified_search",
    ),
    input(
        "snapshot.last_flush",
        ReadInputKind::WorkspaceSnapshotField,
        "get_workspace_status, get_daemon_status",
    ),
    input(
        "snapshot.stale_files",
        ReadInputKind::WorkspaceSnapshotField,
        "get_workspace_status, get_health_report",
    ),
    input(
        "snapshot.connection_count",
        ReadInputKind::WorkspaceSnapshotField,
        "get_daemon_status",
    ),
    input(
        "snapshot.file_mtimes",
        ReadInputKind::WorkspaceSnapshotField,
        "get_workspace_status, get_health_report",
    ),
    // -- Live workspace-root paths -----------------------------------------
    input(
        "workspace_root.source_file_bytes",
        ReadInputKind::WorkspaceRootPath,
        "unified_search, map_code, list_symbols",
    ),
    input(
        "workspace_root.git_directory",
        ReadInputKind::WorkspaceRootPath,
        "query_changes",
    ),
    input(
        "workspace_root.directory_listing",
        ReadInputKind::WorkspaceRootPath,
        "get_workspace_statistics",
    ),
    // -- `.engram/` control paths -------------------------------------------
    input(
        "engram.generations_active_manifest",
        ReadInputKind::EngramPath,
        "* (via request-entry reconciliation)",
    ),
    input(
        "engram.generation_database_file",
        ReadInputKind::EngramPath,
        "*",
    ),
    input(
        "engram.generation_runtime_copy",
        ReadInputKind::EngramPath,
        "*",
    ),
    input(
        "engram.legacy_managed_database",
        ReadInputKind::EngramPath,
        "* (managed mode only)",
    ),
    input(
        "engram.run_socket",
        ReadInputKind::EngramPath,
        "* (transport)",
    ),
    input(
        "engram.run_lock",
        ReadInputKind::EngramPath,
        "* (transport)",
    ),
    input(
        "engram.config_toml",
        ReadInputKind::EngramPath,
        "* (startup configuration)",
    ),
    // -- Environment values --------------------------------------------------
    input(
        "env.ENGRAM_WORKSPACE",
        ReadInputKind::EnvironmentValue,
        "* (startup binding)",
    ),
    input(
        "env.ENGRAM_DATA_DIR",
        ReadInputKind::EnvironmentValue,
        "* (startup binding)",
    ),
    input(
        "env.ENGRAM_LOG_FORMAT",
        ReadInputKind::EnvironmentValue,
        "* (startup logging)",
    ),
    input(
        "env.ENGRAM_IDLE_TIMEOUT_MS",
        ReadInputKind::EnvironmentValue,
        "* (startup lifecycle)",
    ),
    input(
        "env.ENGRAM_READY_TIMEOUT_MS",
        ReadInputKind::EnvironmentValue,
        "* (startup lifecycle)",
    ),
    input(
        "env.ENGRAM_AUTO_REINDEX",
        ReadInputKind::EnvironmentValue,
        "* (startup lifecycle)",
    ),
    input(
        "env.ENGRAM_TEST_STARTUP_DELAY_MS",
        ReadInputKind::EnvironmentValue,
        "* (test harness only)",
    ),
    input(
        "env.ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE",
        ReadInputKind::EnvironmentValue,
        "* (test harness only)",
    ),
];

/// Descriptor names the inventory was enumerated against.
///
/// Derived from the F19 registry rather than hand-listed so a newly declared
/// read-server method widens the inventory's obligation automatically.
#[must_use]
pub fn read_mode_descriptor_names() -> Vec<&'static str> {
    capabilities::all_descriptors()
        .into_iter()
        .filter(|descriptor| {
            descriptor.read_server_available && descriptor.capability == CapabilityClass::Read
        })
        .map(|descriptor| descriptor.name)
        .collect()
}

/// Descriptor names that no enumerated input claims to be reachable from.
///
/// A descriptor that reaches nothing is either genuinely input-free or an
/// enumeration gap; the contract test treats the empty result as the only
/// acceptable answer for descriptors that read anything at all.
#[must_use]
pub fn descriptors_without_enumerated_inputs() -> Vec<&'static str> {
    read_mode_descriptor_names()
        .into_iter()
        .filter(|name| {
            !ENUMERATED_READ_INPUTS
                .iter()
                .any(|entry| entry.reached_via.contains('*') || entry.reached_via.contains(name))
        })
        .collect()
}

/// Identifiers that appear more than once in the enumeration.
#[must_use]
pub fn duplicate_enumerated_ids() -> Vec<&'static str> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for entry in ENUMERATED_READ_INPUTS {
        if !seen.insert(entry.id) {
            duplicates.insert(entry.id);
        }
    }
    duplicates.into_iter().collect()
}

// ── Classification (142.033.002-ST) ──────────────────────────────────────────

/// Who owns an input, from the ReadServer's point of view.
///
/// The three answers are exhaustive by construction: either the sealed
/// generation supplies the bytes, or something outside the generation supplies
/// them and that exception is individually justified, or the input has no
/// business being read at all in this mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadInputClass {
    /// Supplied by the sealed generation the request was admitted against.
    ///
    /// These are the inputs the whole design is for: immutable for the life of
    /// the generation, so two reads admitted against the same generation
    /// cannot disagree.
    GenerationOwned,
    /// Supplied from outside the generation, but pinned for the process
    /// lifetime and therefore stable across every request.
    ///
    /// Every entry in this class is an exception and must carry its own
    /// justification; a pinned-operational input that is not actually pinned
    /// is a correctness bug wearing a classification.
    PinnedOperational,
    /// Must not be read in `ReadServer` mode at all.
    ///
    /// Typically a live, indexer-mutable input whose value can change
    /// underneath an in-flight read and thereby break the single-generation
    /// guarantee.
    DisallowedInReadServerMode,
}

impl ReadInputClass {
    /// Canonical setting string for this class.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        // Exhaustive with no wildcard arm on purpose: a new class cannot be
        // added without deciding how it serializes.
        match self {
            Self::GenerationOwned => "generation_owned",
            Self::PinnedOperational => "pinned_operational",
            Self::DisallowedInReadServerMode => "disallowed_in_read_server_mode",
        }
    }

    /// Whether an entry of this class must carry an individual justification.
    #[must_use]
    pub const fn requires_justification(self) -> bool {
        match self {
            Self::PinnedOperational | Self::DisallowedInReadServerMode => true,
            Self::GenerationOwned => false,
        }
    }
}

impl std::fmt::Display for ReadInputClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The ownership verdict for one enumerated input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadInputClassification {
    /// Identifier, matching a [`ReadInput::id`] in [`ENUMERATED_READ_INPUTS`].
    pub id: &'static str,
    /// The ownership verdict.
    pub class: ReadInputClass,
    /// Why the verdict is correct. Required for every non-generation class.
    pub justification: &'static str,
}

const fn owned(id: &'static str) -> ReadInputClassification {
    ReadInputClassification {
        id,
        class: ReadInputClass::GenerationOwned,
        justification: "",
    }
}

const fn pinned(id: &'static str, justification: &'static str) -> ReadInputClassification {
    ReadInputClassification {
        id,
        class: ReadInputClass::PinnedOperational,
        justification,
    }
}

const fn disallowed(id: &'static str, justification: &'static str) -> ReadInputClassification {
    ReadInputClassification {
        id,
        class: ReadInputClass::DisallowedInReadServerMode,
        justification,
    }
}

/// The ownership verdict for every enumerated input.
///
/// Deliberately a second table rather than a field on [`ReadInput`]: see the
/// module docs. Reconciling two tables is what lets a newly enumerated input
/// be RED until somebody classifies it.
pub const CLASSIFIED_READ_INPUTS: &[ReadInputClassification] = &[
    // -- Generation database relations: the sealed payload -------------------
    owned("db.backlog_content_record"),
    owned("db.backlog_edge"),
    owned("db.backlog_node"),
    owned("db.calls_edge"),
    owned("db.class_code"),
    owned("db.class_embedding"),
    owned("db.class_meta"),
    owned("db.commit_node"),
    owned("db.concerns_edge"),
    owned("db.content_record"),
    owned("db.dataset_node"),
    owned("db.defines_edge"),
    owned("db.file_hash"),
    owned("db.file_node"),
    owned("db.function_code"),
    owned("db.function_embedding"),
    owned("db.function_meta"),
    owned("db.import_node"),
    owned("db.imports_edge"),
    owned("db.index_canonical_workspace_snapshot"),
    owned("db.inherits_from_edge"),
    owned("db.interface_code"),
    owned("db.interface_embedding"),
    owned("db.interface_meta"),
    owned("db.lineage_edge"),
    owned("db.lineage_edge_evidence"),
    owned("db.lineage_index_state"),
    owned("db.powerbi_edge"),
    owned("db.powerbi_file_index_state"),
    owned("db.powerbi_node"),
    owned("db.references_edge"),
    owned("db.rewrite"),
    owned("db.schema_meta"),
    owned("db.staged_call"),
    // -- Workspace snapshot fields ------------------------------------------
    pinned(
        "snapshot.workspace_id",
        "Pinned at activation from the manifest's workspace identity. It is the identity the \
         generation was sealed for, so it cannot drift without a new generation.",
    ),
    pinned(
        "snapshot.workspace_uuid",
        "Same provenance as workspace_id: carried on the manifest's workspace identity and \
         therefore constant for the generation's lifetime.",
    ),
    pinned(
        "snapshot.branch",
        "Pinned at activation from the manifest's branch identity. A branch change produces a \
         different generation rather than mutating this value.",
    ),
    pinned(
        "snapshot.data_dir",
        "Resolved once to the generation's runtime copy root. It addresses generation-owned \
         bytes, so pinning it is what keeps reads inside the sealed payload.",
    ),
    pinned(
        "snapshot.path",
        "The workspace root path string, fixed at process start by the binding. Only the path \
         value is pinned-operational; reading the tree it points at is classified separately \
         and disallowed.",
    ),
    disallowed(
        "snapshot.last_flush",
        "An indexer-write watermark. It changes whenever the indexer flushes, so serving it \
         from a read server would report progress the served generation does not contain.",
    ),
    disallowed(
        "snapshot.stale_files",
        "Derived from live filesystem comparison against the mutable workspace tree, which is \
         exactly the mutable input generations exist to remove from the read path.",
    ),
    pinned(
        "snapshot.connection_count",
        "A daemon-local transport counter, not workspace data. It describes the server process \
         itself, so it neither belongs to nor can contradict the served generation.",
    ),
    disallowed(
        "snapshot.file_mtimes",
        "Live mtime fingerprints of the mutable workspace tree. Two reads admitted against the \
         same generation could observe different values, breaking the single-generation \
         guarantee.",
    ),
    // -- Live workspace-root paths ------------------------------------------
    disallowed(
        "workspace_root.source_file_bytes",
        "The indexer may rewrite these bytes mid-request. Snippets must come from the sealed \
         content relations instead, which is why the code relations are enumerated separately.",
    ),
    disallowed(
        "workspace_root.git_directory",
        "Git history is mutable (commits, rebases, gc). Commit-derived answers must come from \
         the sealed commit_node/concerns_edge relations captured at index time.",
    ),
    disallowed(
        "workspace_root.directory_listing",
        "A live directory walk reports files the generation never indexed, producing statistics \
         that contradict the generation being served.",
    ),
    // -- `.engram/` control paths --------------------------------------------
    pinned(
        "engram.generations_active_manifest",
        "Read only by the activation seam, never by a read handler. It is the generation \
         boundary itself: reconciling it is how a request learns which generation to pin, so \
         it sits outside the payload by definition.",
    ),
    owned("engram.generation_database_file"),
    owned("engram.generation_runtime_copy"),
    disallowed(
        "engram.legacy_managed_database",
        "The managed-mode database is written in place by the indexer. A read server must reach \
         its data only through a sealed generation, never through the live managed file.",
    ),
    pinned(
        "engram.run_socket",
        "Bound once before readiness is reported and never rebound; it is transport, not data, \
         and carries no generation-dependent content.",
    ),
    pinned(
        "engram.run_lock",
        "Acquired once at startup for single-instance enforcement. Transport/lifecycle only, \
         with no bearing on served content.",
    ),
    pinned(
        "engram.config_toml",
        "Loaded once at startup and held for the process lifetime. Because it is never re-read \
         per request, every request observes the same configuration.",
    ),
    // -- Environment values ---------------------------------------------------
    pinned(
        "env.ENGRAM_WORKSPACE",
        "Read once during startup binding. The process environment is fixed at exec time, so \
         this cannot change between requests.",
    ),
    pinned(
        "env.ENGRAM_DATA_DIR",
        "Read once during startup binding to locate the store root; fixed for the process \
         lifetime.",
    ),
    pinned(
        "env.ENGRAM_LOG_FORMAT",
        "Startup logging configuration only. It affects diagnostics, never served content.",
    ),
    pinned(
        "env.ENGRAM_IDLE_TIMEOUT_MS",
        "Startup lifecycle configuration. It governs when the process exits, not what any \
         request observes.",
    ),
    pinned(
        "env.ENGRAM_READY_TIMEOUT_MS",
        "Startup lifecycle configuration consumed before readiness is reported; never consulted \
         on the read path.",
    ),
    disallowed(
        "env.ENGRAM_AUTO_REINDEX",
        "Enables indexer writes. A read server honouring it would perform the very mutation the \
         mode exists to exclude.",
    ),
    disallowed(
        "env.ENGRAM_TEST_STARTUP_DELAY_MS",
        "Test-harness fault injection. It must have no effect on a production read server, so \
         it is excluded rather than pinned.",
    ),
    disallowed(
        "env.ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE",
        "Test-harness tracing hook. Same reasoning as the startup delay: excluded from the \
         production read path entirely.",
    ),
];

// ── Fail-on-unclassified guard ───────────────────────────────────────────────

/// Enumerated inputs that carry no ownership verdict.
///
/// This is the guard the F24 contract test exists to run. A non-empty result
/// means somebody found a new input and stopped before answering the only
/// question that matters about it.
#[must_use]
pub fn unclassified_inputs() -> Vec<&'static str> {
    let classified: BTreeSet<&str> = CLASSIFIED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .collect();
    ENUMERATED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .filter(|id| !classified.contains(id))
        .collect()
}

/// Classifications with no matching enumerated input.
///
/// The mirror of [`unclassified_inputs`]: it catches a verdict left behind
/// after the input it described was removed or renamed, which would otherwise
/// let the two tables agree on a count while disagreeing on content.
#[must_use]
pub fn unenumerated_classifications() -> Vec<&'static str> {
    let enumerated: BTreeSet<&str> = ENUMERATED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .collect();
    CLASSIFIED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .filter(|id| !enumerated.contains(id))
        .collect()
}

/// Look up the ownership verdict for an enumerated input.
#[must_use]
pub fn classification_of(id: &str) -> Option<&'static ReadInputClassification> {
    CLASSIFIED_READ_INPUTS.iter().find(|entry| entry.id == id)
}

/// Classifications that owe a justification but do not supply one.
#[must_use]
pub fn unjustified_classifications() -> Vec<&'static str> {
    CLASSIFIED_READ_INPUTS
        .iter()
        .filter(|entry| {
            entry.class.requires_justification() && entry.justification.trim().is_empty()
        })
        .map(|entry| entry.id)
        .collect()
}
