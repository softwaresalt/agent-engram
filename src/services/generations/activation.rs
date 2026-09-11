//! Generation activation service (plan unit F17, 142.018-T).
//!
//! Activation is the only path that turns the durable active-generation
//! manifest into a usable [`GenerationReadContext`]. It is deliberately narrow:
//!
//! * the manifest is parsed straight into the typed
//!   [`GenerationManifest`] shape — no `serde_json::Value` or free-form map
//!   access ever reaches activation logic, so a field can only be read after
//!   it has already been proven to have the right type;
//! * every typed field is then bounds-checked and identity-checked before any
//!   filesystem or database work happens, so a malformed or foreign manifest
//!   is rejected before it can cost anything;
//! * resolution goes through the F07 [`GenerationStore`], which is the only
//!   component allowed to turn a manifest-relative path into a sealed,
//!   contained absolute path;
//! * digests are revalidated against the bytes actually on disk before the
//!   F09 open path runs, so a generation whose bytes drifted from what the
//!   publisher attested to is never opened.
//!
//! # Why activation has no control endpoint
//!
//! Per plan rule R48 the only two activation triggers are startup
//! reconciliation ([`GenerationActivator::activate_initial`]) and request-entry
//! reconciliation ([`GenerationActivator::maybe_activate_newer`]). There is no
//! notification path and no control endpoint, so activation freshness can
//! never depend on delivery ordering or on a retry policy living outside this
//! module.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::db::cozo_backend::{ExistingDbLocation, open_existing_generation_via_runtime_copy};
use crate::errors::ActivationError;

use super::{
    GenerationId, GenerationManifest, GenerationReadContext, GenerationRevision, GenerationStore,
    ManifestFileDigest,
};

/// The snapshot schema version this build of the activation service accepts.
///
/// A manifest declaring any other value is rejected up front rather than
/// interpreted optimistically: a schema this build does not know may place
/// different meaning on the very fields activation is about to trust.
pub const SUPPORTED_MANIFEST_SCHEMA_VERSION: &str = "1.0.0";

/// File name of the published generation database inside a generation
/// directory. The sealed inventory must carry a digest entry for
/// `<generation-id>/<GENERATION_DATABASE_FILE_NAME>`, which is the file the
/// F09 open path copies into its private runtime location.
pub const GENERATION_DATABASE_FILE_NAME: &str = "engram.db";

/// Maximum number of entries a sealed inventory may declare.
///
/// Every entry costs one containment resolution plus one full file re-digest
/// during activation, so an unbounded inventory is an unbounded startup cost
/// driven entirely by durable bytes this process did not write.
const MAX_INVENTORY_ENTRIES: usize = 4_096;

/// Length of a lowercase hex-encoded SHA-256 digest.
const SHA256_HEX_LEN: usize = 64;

/// Maximum accepted length for a manifest identity string (branch name,
/// workspace identity, schema version, producer).
const MAX_IDENTITY_LEN: usize = 512;

/// Maximum accepted length for an inventory-relative path.
const MAX_INVENTORY_PATH_LEN: usize = 1_024;

/// Individual sealed artifact size cap (plan unit F17, separate-indexer/read-server plan).
///
/// Checked from filesystem metadata alone, before [`file_digest`] ever opens
/// the file: a corrupt or hostile manifest must not be able to force
/// unbounded blocking-thread I/O by pointing at an implausibly large sealed
/// artifact.
const MAX_SEALED_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024; // 4 GiB

/// Cumulative size cap across the whole sealed inventory for one activation.
///
/// Bounds the total bytes [`resolve_and_open`] will hash and later copy into
/// the runtime root during a single activation attempt, independent of how
/// many individual entries stay under [`MAX_SEALED_ARTIFACT_BYTES`].
const MAX_TOTAL_RUNTIME_COPY_BYTES: u64 = 16 * 1024 * 1024 * 1024; // 16 GiB

/// Base delay applied after the first transient activation failure.
const TRANSIENT_BACKOFF_BASE: Duration = Duration::from_millis(250);

/// Ceiling for the exponential transient backoff.
const TRANSIENT_BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Maximum number of rejection records retained at once.
///
/// The cache is keyed by revision and pruned on every successful activation
/// (see [`RejectionCache::prune_through`]); this cap is the second, absolute
/// bound that holds even if no activation ever succeeds.
const MAX_REJECTION_ENTRIES: usize = 256;

// ── Expected identity ────────────────────────────────────────────────────────

/// The branch and workspace identity a daemon will accept a generation for.
///
/// Held separately from the manifest so the comparison is always
/// "what this process is serving" against "what the publisher attested to".
/// Without it, a manifest published for another branch or another workspace
/// would activate silently and serve the wrong data under the right name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedIdentity {
    branch: String,
    workspace_id: String,
}

impl ExpectedIdentity {
    /// Construct the identity activation must match before opening anything.
    #[must_use]
    pub fn new(branch: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
            workspace_id: workspace_id.into(),
        }
    }

    /// Return the branch name this daemon serves.
    #[must_use]
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// Return the workspace identity this daemon serves.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
}

// ── Typed parse and validation ───────────────────────────────────────────────

/// Parse durable manifest bytes into the typed [`GenerationManifest`].
///
/// This is the only place activation turns bytes into a manifest. It
/// deserializes directly into the typed shape rather than into an intermediate
/// `serde_json::Value`, so activation logic can never reach a field through
/// free-form map access, and a structurally wrong document is rejected here
/// instead of surfacing as a surprising `None` deeper in the pipeline.
///
/// # Errors
///
/// Returns [`ActivationError::ManifestMalformed`] when `bytes` are not a
/// well-formed manifest document. The error deliberately carries no revision:
/// the revision itself lives inside bytes that could not be parsed.
pub fn parse_manifest(bytes: &[u8]) -> Result<GenerationManifest, ActivationError> {
    serde_json::from_slice(bytes).map_err(|source| ActivationError::ManifestMalformed {
        reason: source.to_string(),
    })
}

/// A manifest that has passed typed parsing, bounds enforcement, and identity
/// verification, and is therefore safe to resolve and open.
///
/// The type exists so the "validated" fact is carried in the type system
/// rather than in a convention: a function that takes a `ValidatedManifest`
/// cannot be handed a manifest that skipped validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedManifest {
    manifest: GenerationManifest,
    database_path: String,
}

impl ValidatedManifest {
    /// Parse and fully validate durable manifest bytes against `expected`.
    ///
    /// # Errors
    ///
    /// Returns [`ActivationError::ManifestMalformed`] for unparseable bytes,
    /// [`ActivationError::ManifestSchemaMismatch`] for an unsupported schema
    /// version, [`ActivationError::ManifestFieldOutOfBounds`] for a typed but
    /// out-of-range field, and [`ActivationError::IdentityMismatch`] when the
    /// manifest was published for a different branch or workspace.
    pub fn parse(bytes: &[u8], expected: &ExpectedIdentity) -> Result<Self, ActivationError> {
        Self::validate(parse_manifest(bytes)?, expected)
    }

    /// Validate an already-parsed manifest against `expected`.
    ///
    /// Checks run cheapest-first and reject on the first failure: schema
    /// version, then field bounds, then identity. Ordering matters only for
    /// which error a caller sees, never for whether an invalid manifest can
    /// slip through.
    ///
    /// # Errors
    ///
    /// See [`ValidatedManifest::parse`].
    pub fn validate(
        manifest: GenerationManifest,
        expected: &ExpectedIdentity,
    ) -> Result<Self, ActivationError> {
        validate_schema_version(&manifest)?;
        validate_bounds(&manifest)?;
        validate_identity(&manifest, expected)?;
        let database_path = database_entry_path(manifest.generation_id());
        require_database_entry(&manifest, &database_path)?;

        Ok(Self {
            manifest,
            database_path,
        })
    }

    /// Borrow the validated manifest.
    #[must_use]
    pub const fn manifest(&self) -> &GenerationManifest {
        &self.manifest
    }

    /// Return the validated publication revision.
    #[must_use]
    pub const fn revision(&self) -> GenerationRevision {
        self.manifest.revision()
    }

    /// Return the validated generation identifier.
    #[must_use]
    pub const fn generation_id(&self) -> &GenerationId {
        self.manifest.generation_id()
    }

    /// Return the inventory-relative path of the published generation
    /// database, which validation already proved is present in the sealed
    /// inventory.
    #[must_use]
    pub fn database_path(&self) -> &str {
        &self.database_path
    }
}

/// Inventory-relative path of the published database for `generation_id`.
fn database_entry_path(generation_id: &GenerationId) -> String {
    format!("{generation_id}/{GENERATION_DATABASE_FILE_NAME}")
}

fn validate_schema_version(manifest: &GenerationManifest) -> Result<(), ActivationError> {
    if manifest.schema_version() == SUPPORTED_MANIFEST_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(ActivationError::ManifestSchemaMismatch {
            expected: SUPPORTED_MANIFEST_SCHEMA_VERSION.to_owned(),
            found: manifest.schema_version().to_owned(),
        })
    }
}

fn out_of_bounds(field: &str, reason: impl Into<String>) -> ActivationError {
    ActivationError::ManifestFieldOutOfBounds {
        field: field.to_owned(),
        reason: reason.into(),
    }
}

fn validate_identity_string(field: &str, value: &str) -> Result<(), ActivationError> {
    if value.is_empty() {
        return Err(out_of_bounds(field, "must not be empty"));
    }
    if value.len() > MAX_IDENTITY_LEN {
        return Err(out_of_bounds(
            field,
            format!("must be at most {MAX_IDENTITY_LEN} bytes"),
        ));
    }
    Ok(())
}

fn validate_bounds(manifest: &GenerationManifest) -> Result<(), ActivationError> {
    validate_identity_string("schema_version", manifest.schema_version())?;
    validate_identity_string("branch.name", manifest.branch().name())?;
    if let Some(source_revision) = manifest.branch().source_revision() {
        validate_identity_string("branch.source_revision", source_revision)?;
    }
    validate_identity_string("workspace.workspace_id", manifest.workspace().id())?;
    validate_identity_string("provenance.producer", manifest.provenance().producer())?;

    // Revision 0 is the sentinel `publish_generation_manifest` treats as
    // "nothing published yet", so a manifest can never legitimately declare
    // it; accepting it would make "no generation" and "generation 0"
    // indistinguishable at the activation boundary.
    if manifest.revision().value() == 0 {
        return Err(out_of_bounds(
            "revision",
            "must be greater than 0 (0 means no generation is published)",
        ));
    }

    let files = manifest.inventory().files();
    if files.is_empty() {
        return Err(out_of_bounds("inventory.files", "must not be empty"));
    }
    if files.len() > MAX_INVENTORY_ENTRIES {
        return Err(out_of_bounds(
            "inventory.files",
            format!("must declare at most {MAX_INVENTORY_ENTRIES} entries"),
        ));
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for entry in files {
        validate_inventory_entry(entry)?;
        if !seen.insert(entry.path()) {
            return Err(out_of_bounds(
                "inventory.files[].path",
                format!("duplicate inventory path {:?}", entry.path()),
            ));
        }
    }

    Ok(())
}

fn validate_inventory_entry(entry: &ManifestFileDigest) -> Result<(), ActivationError> {
    let path = entry.path();
    if path.is_empty() {
        return Err(out_of_bounds("inventory.files[].path", "must not be empty"));
    }
    if path.len() > MAX_INVENTORY_PATH_LEN {
        return Err(out_of_bounds(
            "inventory.files[].path",
            format!("must be at most {MAX_INVENTORY_PATH_LEN} bytes"),
        ));
    }
    // Containment is ultimately enforced by `GenerationStore`, but rejecting
    // the obvious escapes here means a hostile manifest never even reaches
    // the resolution step, and the caller gets a manifest-shaped error rather
    // than a filesystem-shaped one.
    if path.starts_with('/') || path.starts_with('\\') || path.contains("..") {
        return Err(out_of_bounds(
            "inventory.files[].path",
            format!("must be a relative path without '..' components (found {path:?})"),
        ));
    }

    let digest = entry.sha256();
    if digest.len() != SHA256_HEX_LEN
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(out_of_bounds(
            "inventory.files[].sha256",
            format!("must be {SHA256_HEX_LEN} lowercase hex characters (found {digest:?})"),
        ));
    }

    Ok(())
}

fn validate_identity(
    manifest: &GenerationManifest,
    expected: &ExpectedIdentity,
) -> Result<(), ActivationError> {
    if manifest.branch().name() != expected.branch() {
        return Err(ActivationError::IdentityMismatch {
            field: "branch".to_owned(),
            expected: expected.branch().to_owned(),
            found: manifest.branch().name().to_owned(),
        });
    }
    if manifest.workspace().id() != expected.workspace_id() {
        return Err(ActivationError::IdentityMismatch {
            field: "workspace".to_owned(),
            expected: expected.workspace_id().to_owned(),
            found: manifest.workspace().id().to_owned(),
        });
    }
    Ok(())
}

fn require_database_entry(
    manifest: &GenerationManifest,
    database_path: &str,
) -> Result<(), ActivationError> {
    if manifest
        .inventory()
        .files()
        .iter()
        .any(|entry| entry.path() == database_path)
    {
        Ok(())
    } else {
        Err(out_of_bounds(
            "inventory.files",
            format!("must seal the published generation database {database_path:?}"),
        ))
    }
}

// ── Rejection cache ──────────────────────────────────────────────────────────

/// Why a revision was rejected, and therefore whether it may ever be retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionClass {
    /// The revision can never succeed: the durable manifest bytes are
    /// immutable for a given revision, so a schema, bounds, identity, or
    /// digest rejection will reproduce identically forever. Recorded once and
    /// never retried.
    Permanent,
    /// The revision failed for a reason that may resolve on its own (a file
    /// not yet visible, a busy database, an I/O hiccup). Retried, but only
    /// after an exponential backoff so a broken publish cannot turn every
    /// inbound request into a fresh activation attempt.
    Transient,
}

/// One recorded rejection for a revision.
#[derive(Debug, Clone)]
struct RejectionRecord {
    class: RejectionClass,
    reason: String,
    attempts: u32,
    last_attempt: Instant,
}

/// Revision-keyed record of rejected activations.
#[derive(Debug, Default)]
struct RejectionCache {
    records: BTreeMap<GenerationRevision, RejectionRecord>,
}

impl RejectionCache {
    /// Whether a fresh activation attempt for `revision` is allowed `now`.
    fn may_attempt(&self, revision: GenerationRevision, now: Instant) -> bool {
        match self.records.get(&revision) {
            None => true,
            Some(record) => match record.class {
                RejectionClass::Permanent => false,
                RejectionClass::Transient => {
                    now.saturating_duration_since(record.last_attempt)
                        >= backoff_delay(record.attempts)
                }
            },
        }
    }

    fn record(
        &mut self,
        revision: GenerationRevision,
        class: RejectionClass,
        reason: String,
        now: Instant,
    ) {
        let attempts = self
            .records
            .get(&revision)
            .map_or(1, |record| record.attempts.saturating_add(1));
        self.records.insert(
            revision,
            RejectionRecord {
                class,
                reason,
                attempts,
                last_attempt: now,
            },
        );
        self.enforce_capacity();
    }

    /// Drop every record at or below `revision`.
    ///
    /// [`GenerationActivator::maybe_activate_newer`] only ever considers a
    /// revision strictly greater than the active one, so once `revision` is
    /// serving, no record at or below it can ever be consulted again. Pruning
    /// them keeps the cache proportional to the number of *future* bad
    /// revisions rather than to uptime.
    fn prune_through(&mut self, revision: GenerationRevision) {
        self.records = self
            .records
            .split_off(&GenerationRevision::new(revision.value().saturating_add(1)));
    }

    /// Enforce the absolute cap by evicting the lowest revisions first.
    ///
    /// The lowest revisions are the ones most likely to already be
    /// unreachable (a newer revision has since been published), so evicting
    /// them loses the least information. Eviction can only ever cause a
    /// redundant re-attempt, never an incorrect activation.
    fn enforce_capacity(&mut self) {
        while self.records.len() > MAX_REJECTION_ENTRIES {
            let Some(lowest) = self.records.keys().next().copied() else {
                break;
            };
            self.records.remove(&lowest);
        }
    }

    fn class_of(&self, revision: GenerationRevision) -> Option<RejectionClass> {
        self.records.get(&revision).map(|record| record.class)
    }

    fn reason_of(&self, revision: GenerationRevision) -> Option<String> {
        self.records
            .get(&revision)
            .map(|record| record.reason.clone())
    }

    fn len(&self) -> usize {
        self.records.len()
    }
}

/// Exponential backoff for the `attempts`-th consecutive transient failure.
///
/// Exposed so the bound is assertable rather than implied by timing: attempt 1
/// waits [`TRANSIENT_BACKOFF_BASE`], each subsequent attempt doubles, and the
/// delay saturates at [`TRANSIENT_BACKOFF_MAX`].
#[must_use]
pub fn backoff_delay(attempts: u32) -> Duration {
    if attempts == 0 {
        return Duration::ZERO;
    }
    let shift = attempts.saturating_sub(1).min(16);
    TRANSIENT_BACKOFF_BASE
        .saturating_mul(1_u32 << shift)
        .min(TRANSIENT_BACKOFF_MAX)
}

/// Classify an activation failure as permanently or transiently rejected.
///
/// Permanence follows from immutability: a manifest revision's durable bytes
/// never change, so any rejection derived purely from those bytes (schema,
/// bounds, identity) or from a digest attestation they carry can only ever
/// reproduce. Everything else — missing files, I/O errors, database open
/// failures — may be a mid-publish race or a transient environment problem.
#[must_use]
pub fn classify(error: &ActivationError) -> RejectionClass {
    match error {
        ActivationError::ManifestSchemaMismatch { .. }
        | ActivationError::ManifestMalformed { .. }
        | ActivationError::ManifestFieldOutOfBounds { .. }
        | ActivationError::IdentityMismatch { .. }
        | ActivationError::DigestMismatch { .. } => RejectionClass::Permanent,
        ActivationError::GenerationNotYetActivated { .. }
        | ActivationError::ActivationDeadlineExceeded { .. }
        | ActivationError::TransientActivationFailure { .. } => RejectionClass::Transient,
    }
}

fn transient(reason: impl Into<String>) -> ActivationError {
    ActivationError::TransientActivationFailure {
        reason: reason.into(),
    }
}

/// Content-based fingerprint of the durable manifest file.
///
/// Used by [`GenerationActivator::maybe_activate_newer`] to decide whether a
/// freshly-read manifest is the same one it already parsed (F17, Fix5), so a
/// request that repeats against unchanged bytes -- most importantly, one
/// this activator has already permanently rejected -- pays for a checksum
/// comparison, not a second JSON parse.
///
/// mtime and length alone are not a reliable change detector (F17, Fix4):
/// two manifests published in quick succession can share both within
/// filesystem timestamp granularity, especially on platforms with coarse
/// mtime resolution or atomic-replace publish patterns, which could leave a
/// same-length replacement undetected indefinitely. The manifest is bounded
/// to [`MAX_MANIFEST_BYTES`] (F17, Fix1), so hashing the full bytes here is
/// cheap and bounded -- far cheaper than the JSON parse and validate/open
/// pipeline this fingerprint lets the activator skip -- and checksum
/// equality reliably implies byte-for-byte content equality within that
/// bound. `len` is kept as a fast pre-filter before the checksum comparison;
/// it is never trusted on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestFingerprint {
    len: u64,
    checksum: String,
}

/// Result of the last full manifest read attempt for a given
/// [`ManifestFingerprint`] (F17, Fix5).
///
/// Caching only the successful case would mean a permanently malformed
/// manifest repeats its full read + JSON parse on every subsequent
/// reconciliation attempt, even though the fingerprint proves nothing about
/// it has changed. Caching the malformed outcome too lets a repeat probe
/// against the same unchanged bytes return the identical typed error without
/// re-invoking [`parse_manifest`].
#[derive(Debug, Clone)]
enum ProbeOutcome {
    /// The manifest parsed successfully into this typed value.
    Parsed(GenerationManifest),
    /// The manifest failed to parse; `reason` is the [`ActivationError::ManifestMalformed`]
    /// reason string to reconstruct the same typed error from, without
    /// re-parsing.
    Malformed(String),
}

// ── Activator ────────────────────────────────────────────────────────────────

/// The currently-serving generation, paired with the revision that produced it.
#[derive(Debug, Clone)]
struct ActiveGeneration {
    revision: GenerationRevision,
    context: GenerationReadContext,
}

/// Owns generation activation for one daemon process.
///
/// A single activator instance is the whole activation surface: it holds the
/// currently-serving context, the single-flight gate that coalesces concurrent
/// reconciliations, and the rejection cache that keeps a bad published
/// revision from re-costing work on every request.
#[derive(Debug)]
pub struct GenerationActivator {
    store: GenerationStore,
    runtime_root: PathBuf,
    expected: ExpectedIdentity,
    deadline: Duration,
    /// The serving context. Held behind an `RwLock` that is taken only to read
    /// or swap the handle — never across the open itself — so activation can
    /// never block a read that is already in flight.
    active: RwLock<Option<ActiveGeneration>>,
    /// Single-flight gate. Concurrent `maybe_activate_newer` callers serialize
    /// here and then re-check the active revision, so only the first caller
    /// performs the open and the rest observe the result.
    single_flight: tokio::sync::Mutex<()>,
    rejections: Mutex<RejectionCache>,
    /// Cached fingerprint of the manifest bytes last actually read, paired
    /// with the outcome of attempting to parse them. `None` until the first
    /// full read. See [`ManifestFingerprint`] and [`ProbeOutcome`].
    probe: Mutex<Option<(ManifestFingerprint, ProbeOutcome)>>,
    validation_attempts: AtomicUsize,
    open_attempts: AtomicUsize,
    /// Number of times the manifest has actually been JSON-parsed, as
    /// opposed to short-circuited by the Fix5 fingerprint probe.
    manifest_read_attempts: AtomicUsize,
}

impl GenerationActivator {
    /// Construct an activator for one daemon process.
    ///
    /// `runtime_root` must be an absolute path: the F09 open path refuses a
    /// relative runtime root outright, and failing here would otherwise be
    /// deferred to the first activation attempt.
    #[must_use]
    pub fn new(
        store: GenerationStore,
        runtime_root: impl Into<PathBuf>,
        expected: ExpectedIdentity,
        deadline: Duration,
    ) -> Self {
        Self {
            store,
            runtime_root: runtime_root.into(),
            expected,
            deadline,
            active: RwLock::new(None),
            single_flight: tokio::sync::Mutex::new(()),
            rejections: Mutex::new(RejectionCache::default()),
            probe: Mutex::new(None),
            validation_attempts: AtomicUsize::new(0),
            open_attempts: AtomicUsize::new(0),
            manifest_read_attempts: AtomicUsize::new(0),
        }
    }

    /// The identity this activator was constructed to serve.
    ///
    /// Exposed so callers that wrap an activator (for example
    /// [`crate::daemon::startup_activation::ReadServerStartupGate`]) can
    /// derive their own identity from this single source of truth instead of
    /// accepting a second, independently-suppliable copy that could diverge
    /// from what this activator actually validates against.
    #[must_use]
    pub fn expected_identity(&self) -> &ExpectedIdentity {
        &self.expected
    }

    /// Return the currently-serving read context, if one has been activated.
    ///
    /// The returned context is a cheap `Arc`-backed clone: once a caller holds
    /// it, a subsequent activation cannot invalidate it, and the underlying
    /// generation stays open until that last holder drops.
    pub async fn active_context(&self) -> Option<GenerationReadContext> {
        self.active
            .read()
            .await
            .as_ref()
            .map(|active| active.context.clone())
    }

    /// Return the revision currently being served, if any.
    pub async fn active_revision(&self) -> Option<GenerationRevision> {
        self.active
            .read()
            .await
            .as_ref()
            .map(|active| active.revision)
    }

    /// Number of full validate-and-resolve passes performed so far.
    ///
    /// Exposed so "performs no work" can be asserted directly rather than
    /// inferred from timing.
    #[must_use]
    pub fn validation_attempt_count(&self) -> usize {
        self.validation_attempts.load(Ordering::SeqCst)
    }

    /// Number of times the F09 open path has actually been entered.
    #[must_use]
    pub fn open_attempt_count(&self) -> usize {
        self.open_attempts.load(Ordering::SeqCst)
    }

    /// Number of times the manifest has actually been read and parsed.
    ///
    /// Exposed so a repeat call against an already-rejected, unchanged
    /// revision can be proven to skip the full read + parse (F17, Fix5)
    /// rather than merely inferred from timing.
    #[must_use]
    pub fn manifest_read_attempt_count(&self) -> usize {
        self.manifest_read_attempts.load(Ordering::SeqCst)
    }

    /// Rejection class recorded for `revision`, if it has been rejected.
    #[must_use]
    pub fn rejection_class(&self, revision: GenerationRevision) -> Option<RejectionClass> {
        self.with_rejections(|cache| cache.class_of(revision))
    }

    /// Recorded rejection reason for `revision`, if it has been rejected.
    #[must_use]
    pub fn rejection_reason(&self, revision: GenerationRevision) -> Option<String> {
        self.with_rejections(|cache| cache.reason_of(revision))
    }

    /// Number of rejection records currently retained.
    #[must_use]
    pub fn rejection_cache_len(&self) -> usize {
        self.with_rejections(RejectionCache::len)
    }

    /// Run `action` against the rejection cache, recovering from poisoning.
    ///
    /// The cache is advisory bookkeeping: a panic elsewhere must not make the
    /// read path permanently unable to consult it, so a poisoned lock is
    /// recovered rather than propagated.
    fn with_rejections<T>(&self, action: impl FnOnce(&RejectionCache) -> T) -> T {
        match self.rejections.lock() {
            Ok(guard) => action(&guard),
            Err(poisoned) => action(&poisoned.into_inner()),
        }
    }

    fn with_rejections_mut<T>(&self, action: impl FnOnce(&mut RejectionCache) -> T) -> T {
        match self.rejections.lock() {
            Ok(mut guard) => action(&mut guard),
            Err(poisoned) => action(&mut poisoned.into_inner()),
        }
    }

    /// Cached fingerprint/outcome pair from the last full manifest read, if
    /// there has been one. Recovers from poisoning for the same reason
    /// [`GenerationActivator::with_rejections`] does: this cache is advisory,
    /// and a panic elsewhere must not permanently disable the probe
    /// short-circuit.
    fn cached_probe(&self) -> Option<(ManifestFingerprint, ProbeOutcome)> {
        match self.probe.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    fn set_probe(&self, fingerprint: ManifestFingerprint, outcome: ProbeOutcome) {
        let recorded = Some((fingerprint, outcome));
        match self.probe.lock() {
            Ok(mut guard) => *guard = recorded,
            Err(poisoned) => *poisoned.into_inner() = recorded,
        }
    }

    /// Perform the initial startup activation.
    ///
    /// The startup gate (F18) calls this before publishing readiness, so the
    /// deadline is what keeps a failed startup bounded rather than hung: an
    /// activation that cannot finish within [`GenerationActivator::new`]'s
    /// `deadline` is abandoned with a typed timeout instead of holding the
    /// daemon in `starting` forever. The deadline bounds the *complete*
    /// attempt -- single-flight lock acquisition, the manifest read and
    /// parse, and validate-resolve-open -- not just the final open step (F17,
    /// Fix4): see [`GenerationActivator::run_bounded`].
    ///
    /// Re-entrant by design: if a context is already active this returns it
    /// without re-opening anything, so a retried startup never produces a
    /// second open of the same generation.
    ///
    /// # Errors
    ///
    /// Returns a typed [`ActivationError`] when the durable manifest cannot be
    /// read, parsed, validated, resolved, digest-revalidated, or opened within
    /// the configured deadline. On every error path nothing is published, so a
    /// failed activation leaves no partially-opened state behind.
    pub async fn activate_initial(&self) -> Result<GenerationReadContext, ActivationError> {
        self.run_bounded(self.activate_initial_attempt()).await
    }

    async fn activate_initial_attempt(&self) -> Result<GenerationReadContext, ActivationError> {
        let _single_flight = self.single_flight.lock().await;
        if let Some(context) = self.active_context().await {
            return Ok(context);
        }

        self.manifest_read_attempts.fetch_add(1, Ordering::SeqCst);
        let manifest = parse_manifest(&read_manifest_bytes_blocking(self.store.clone()).await?)?;
        let revision = manifest.revision();
        match self.validate_and_open(manifest).await {
            Ok(context) => {
                self.publish_active(revision, context.clone()).await;
                Ok(context)
            }
            Err(error) => {
                self.record_rejection(revision, &error);
                Err(error)
            }
        }
    }

    /// Reconcile the durable manifest and activate a strictly newer revision.
    ///
    /// This is the request-entry reconciliation trigger (plan rule R48). It is
    /// single-flight: concurrent callers serialize on one gate and then
    /// re-check the active revision, so N simultaneous callers produce exactly
    /// one open rather than N. The open itself runs without holding the
    /// context lock, and the previously-active context is swapped only after
    /// the new one is fully open, so a read already in flight is never blocked
    /// and never loses the generation it captured.
    ///
    /// Before doing any JSON parse, a fingerprint probe checks whether the
    /// manifest's content is unchanged since the last time it was actually
    /// parsed (F17, Fix5). When it is -- most importantly, when the cached
    /// outcome is a revision this activator has already permanently
    /// rejected, or a manifest that failed to parse -- this call reuses the
    /// cached outcome without repeating the JSON parse on every request. The
    /// manifest is still read every call (it is bounded to
    /// [`MAX_MANIFEST_BYTES`], so this is cheap), because computing the
    /// content checksum the fingerprint relies on requires the bytes; what is
    /// skipped is the more expensive JSON parse and, transitively, the
    /// validate/resolve/open pipeline (F17, Fix4).
    ///
    /// Returns `Ok(None)` when there is nothing to do: no newer revision was
    /// published, another caller already activated it, or the published
    /// revision is one this activator has already rejected (permanently, or
    /// transiently and still inside its backoff window). `Ok(None)` therefore
    /// means "keep serving what you are serving", which is exactly what the
    /// read path should do.
    ///
    /// # Errors
    ///
    /// Returns a typed [`ActivationError`] when a strictly newer revision was
    /// found but could not be activated, or when the manifest bytes are
    /// oversized or malformed. The currently-active generation is left
    /// untouched and still serving in every error case. As with
    /// [`GenerationActivator::activate_initial`], the configured deadline
    /// bounds the complete attempt, not just the final open step.
    pub async fn maybe_activate_newer(
        &self,
    ) -> Result<Option<GenerationReadContext>, ActivationError> {
        self.run_bounded(self.maybe_activate_newer_attempt()).await
    }

    async fn maybe_activate_newer_attempt(
        &self,
    ) -> Result<Option<GenerationReadContext>, ActivationError> {
        let (bytes, fingerprint) =
            read_manifest_with_fingerprint_blocking(self.store.clone()).await?;

        let manifest = match self.probe_outcome_if_unchanged(&fingerprint) {
            Some(ProbeOutcome::Malformed(reason)) => {
                return Err(ActivationError::ManifestMalformed { reason });
            }
            Some(ProbeOutcome::Parsed(manifest)) => manifest,
            None => {
                self.manifest_read_attempts.fetch_add(1, Ordering::SeqCst);
                match parse_manifest(&bytes) {
                    Ok(manifest) => {
                        self.set_probe(fingerprint, ProbeOutcome::Parsed(manifest.clone()));
                        manifest
                    }
                    Err(error) => {
                        let ActivationError::ManifestMalformed { reason } = &error else {
                            return Err(error);
                        };
                        self.set_probe(fingerprint, ProbeOutcome::Malformed(reason.clone()));
                        return Err(error);
                    }
                }
            }
        };
        let revision = manifest.revision();

        if !self.is_strictly_newer(revision).await || !self.may_attempt(revision) {
            return Ok(None);
        }

        let _single_flight = self.single_flight.lock().await;
        // Re-check under the gate: a caller that was ahead of us may have
        // already activated this revision (or already rejected it), in which
        // case this call must perform no further work at all.
        if !self.is_strictly_newer(revision).await || !self.may_attempt(revision) {
            return Ok(None);
        }

        match self.validate_and_open(manifest).await {
            Ok(context) => {
                self.publish_active(revision, context.clone()).await;
                Ok(Some(context))
            }
            Err(error) => {
                self.record_rejection(revision, &error);
                Err(error)
            }
        }
    }

    /// Whether `revision` is strictly greater than the revision being served.
    ///
    /// Nothing active means any published revision is newer.
    async fn is_strictly_newer(&self, revision: GenerationRevision) -> bool {
        match self.active_revision().await {
            None => true,
            Some(active) => active.advance_to(revision).is_ok(),
        }
    }

    /// Whether the rejection cache currently permits an attempt on `revision`.
    fn may_attempt(&self, revision: GenerationRevision) -> bool {
        let now = Instant::now();
        self.with_rejections(|cache| cache.may_attempt(revision, now))
    }

    fn record_rejection(&self, revision: GenerationRevision, error: &ActivationError) {
        let class = classify(error);
        let reason = error.to_string();
        let now = Instant::now();
        self.with_rejections_mut(|cache| cache.record(revision, class, reason, now));
    }

    /// Pre-parse check for [`GenerationActivator::maybe_activate_newer`]: if
    /// `current`'s content checksum matches the fingerprint recorded the
    /// last time the manifest was actually parsed, reuse the cached outcome
    /// instead of repeating the JSON parse.
    ///
    /// Returns `None` when there is no cached fingerprint yet, or the current
    /// fingerprint does not match the cached one -- in either case the
    /// caller must parse the freshly-read bytes.
    fn probe_outcome_if_unchanged(&self, current: &ManifestFingerprint) -> Option<ProbeOutcome> {
        let (cached_fingerprint, outcome) = self.cached_probe()?;
        (cached_fingerprint == *current).then_some(outcome)
    }

    /// Run `attempt` bounded by the configured activation deadline.
    ///
    /// The deadline bounds the *complete* attempt -- single-flight lock
    /// acquisition, the manifest read and parse, and validate-resolve-open --
    /// not just the tail portion that used to be wrapped inside
    /// `validate_and_open` alone (F17, Fix4): a stalled manifest read or a
    /// long single-flight wait must not keep the whole operation pending
    /// indefinitely.
    ///
    /// A non-positive deadline is rejected before `attempt` is ever polled --
    /// futures are lazy, so this check runs before any filesystem work -- so
    /// a misconfigured deadline cannot silently degrade into "unbounded".
    ///
    /// On timeout the wrapped future (and anything it was awaiting, such as
    /// an open's join handle) is dropped. A blocking task already spawned
    /// from within it still runs to completion, but its result -- including
    /// any opened database handle -- is dropped with it, so an abandoned
    /// attempt closes what it opened instead of leaking a half-activated
    /// generation.
    async fn run_bounded<T>(
        &self,
        attempt: impl Future<Output = Result<T, ActivationError>>,
    ) -> Result<T, ActivationError> {
        let deadline_ms = u64::try_from(self.deadline.as_millis()).unwrap_or(u64::MAX);
        if self.deadline.is_zero() {
            return Err(ActivationError::ActivationDeadlineExceeded { deadline_ms });
        }

        match tokio::time::timeout(self.deadline, attempt).await {
            Ok(result) => result,
            Err(_elapsed) => Err(ActivationError::ActivationDeadlineExceeded { deadline_ms }),
        }
    }

    /// Validate, resolve, revalidate digests, and open.
    ///
    /// Bounded by the caller: [`GenerationActivator::run_bounded`] wraps the
    /// whole attempt this is one step of, so this method performs no deadline
    /// bookkeeping of its own.
    async fn validate_and_open(
        &self,
        manifest: GenerationManifest,
    ) -> Result<GenerationReadContext, ActivationError> {
        self.validation_attempts.fetch_add(1, Ordering::SeqCst);
        let validated = ValidatedManifest::validate(manifest, &self.expected)?;

        self.open_attempts.fetch_add(1, Ordering::SeqCst);
        let store = self.store.clone();
        let runtime_root = self.runtime_root.clone();
        let opened = tokio::task::spawn_blocking(move || {
            resolve_and_open(&store, &runtime_root, &validated)
        });

        match opened.await {
            Ok(result) => result,
            Err(join_error) => Err(transient(format!(
                "generation activation task did not complete: {join_error}"
            ))),
        }
    }

    /// Publish `context` as the serving generation and prune superseded
    /// rejection records.
    ///
    /// The write lock is held only for the swap itself -- never across the
    /// open -- so a concurrent reader waits at most for one pointer
    /// assignment.
    async fn publish_active(&self, revision: GenerationRevision, context: GenerationReadContext) {
        {
            let mut active = self.active.write().await;
            *active = Some(ActiveGeneration { revision, context });
        }
        self.with_rejections_mut(|cache| cache.prune_through(revision));
    }
}

/// Enforce the per-artifact and cumulative sealed-inventory size caps for one
/// entry, from filesystem metadata alone.
///
/// `per_artifact_cap` and `total_cap` are taken as parameters rather than
/// always reading [`MAX_SEALED_ARTIFACT_BYTES`] / [`MAX_TOTAL_RUNTIME_COPY_BYTES`]
/// directly, so this cap logic can be regression-tested with small caps
/// without ever writing multi-gigabyte files to disk in CI.
///
/// # Errors
///
/// Returns [`ActivationError::ManifestFieldOutOfBounds`] when `size` alone
/// exceeds `per_artifact_cap`, or when `size` added to the running total would
/// exceed `total_cap`. On success, `running_total` is updated to include
/// `size`.
fn check_artifact_size(
    path: &Path,
    size: u64,
    running_total: &mut u64,
    per_artifact_cap: u64,
    total_cap: u64,
) -> Result<(), ActivationError> {
    if size > per_artifact_cap {
        return Err(out_of_bounds(
            "inventory.files[].size",
            format!(
                "sealed artifact {} is {size} bytes, exceeds the {per_artifact_cap} byte per-artifact cap",
                path.display()
            ),
        ));
    }
    let candidate_total = running_total.checked_add(size).ok_or_else(|| {
        out_of_bounds(
            "inventory.files",
            format!(
                "cumulative sealed inventory size overflowed while accumulating {}",
                path.display()
            ),
        )
    })?;
    if candidate_total > total_cap {
        return Err(out_of_bounds(
            "inventory.files",
            format!(
                "cumulative sealed inventory size {candidate_total} bytes exceeds the {total_cap} byte total cap"
            ),
        ));
    }
    *running_total = candidate_total;
    Ok(())
}

/// Resolve every sealed inventory entry, revalidate its digest, and open the
/// generation database through the F09 runtime-copy path.
///
/// Runs on a blocking thread: every step here is synchronous filesystem or
/// database work, and running it on the async runtime would stall unrelated
/// tasks for the whole duration of a database copy.
///
/// Digest revalidation covers the *whole* sealed inventory, not just the
/// database: the manifest attests to a set of bytes, and activating a
/// generation whose sidecar files drifted would serve a snapshot the publisher
/// never sealed.
///
/// Each entry's size is checked against [`MAX_SEALED_ARTIFACT_BYTES`] and the
/// running total against [`MAX_TOTAL_RUNTIME_COPY_BYTES`] from filesystem
/// metadata alone, before [`file_digest`] ever opens the file: neither the
/// async activation deadline nor `spawn_blocking` can interrupt a hash or copy
/// already in progress, so an oversized artifact must be rejected before any
/// bytes are read.
fn resolve_and_open(
    store: &GenerationStore,
    runtime_root: &Path,
    validated: &ValidatedManifest,
) -> Result<GenerationReadContext, ActivationError> {
    let manifest = validated.manifest();
    let generation_id = manifest.generation_id();
    // Paired so the manifest-attested digest for the database entry can
    // never be forgotten alongside its resolved path: see the
    // `ExistingDbLocation` digest re-check below for why this pairing
    // matters.
    let mut database_entry: Option<(PathBuf, String)> = None;
    let mut cumulative_bytes: u64 = 0;

    for entry in manifest.inventory().files() {
        // Containment is the store's job: it is the only component allowed to
        // turn a manifest-relative path into an absolute one, so a hostile
        // inventory path cannot reach outside the generation root here.
        let target = store
            .seal_legacy_direct(generation_id.clone(), entry.path())
            .map_err(|source| {
                transient(format!(
                    "failed to resolve sealed inventory file {:?}: {source}",
                    entry.path()
                ))
            })?;
        let metadata = std::fs::metadata(target.path()).map_err(|source| {
            transient(format!(
                "failed to stat sealed inventory file {}: {source}",
                target.path().display()
            ))
        })?;
        check_artifact_size(
            target.path(),
            metadata.len(),
            &mut cumulative_bytes,
            MAX_SEALED_ARTIFACT_BYTES,
            MAX_TOTAL_RUNTIME_COPY_BYTES,
        )?;
        let found = file_digest(target.path(), MAX_SEALED_ARTIFACT_BYTES)?;
        if found != entry.sha256() {
            return Err(ActivationError::DigestMismatch {
                path: entry.path().to_owned(),
                expected: entry.sha256().to_owned(),
                found,
            });
        }
        if entry.path() == validated.database_path() {
            database_entry = Some((target.path().to_path_buf(), entry.sha256().to_owned()));
        }
    }

    let (database_target, expected_database_digest) = database_entry.ok_or_else(|| {
        out_of_bounds(
            "inventory.files",
            format!(
                "must seal the published generation database {:?}",
                validated.database_path()
            ),
        )
    })?;

    let location = ExistingDbLocation::new(store.root(), &database_target).map_err(|source| {
        transient(format!(
            "failed to validate published generation database {}: {source}",
            database_target.display()
        ))
    })?;
    // `ExistingDbLocation::new` re-reads and re-hashes `database_target`
    // rather than reusing the `found` digest already checked against
    // `entry.sha256()` above: without this second explicit compare, a
    // replacement of the file in the window between that check and this
    // constructor call would be sealed under its own (self-consistent but
    // manifest-diverging) digest and opened as if it were the attested
    // content (F17, Fix2).
    let found_database_digest = location.published_db_digest_hex();
    if found_database_digest != expected_database_digest {
        return Err(ActivationError::DigestMismatch {
            path: validated.database_path().to_owned(),
            expected: expected_database_digest,
            found: found_database_digest,
        });
    }
    let opened =
        open_existing_generation_via_runtime_copy(&location, runtime_root, generation_id.as_str())
            .map_err(|source| {
                transient(format!(
                    "failed to open generation {generation_id} via runtime copy: {source}"
                ))
            })?;

    GenerationReadContext::new(opened).map_err(|source| {
        transient(format!(
            "opened generation {generation_id} did not yield a valid read context: {source}"
        ))
    })
}

/// Upper bound on the durable manifest file size.
///
/// A corrupt or oversized `active.json` must not be able to consume
/// unbounded memory (F17, Fix1): the deadline that bounds a complete
/// activation attempt (see [`GenerationActivator::run_bounded`]) does not
/// stop a blocking read already spawned from it, so the read itself must
/// refuse to start once the file is implausibly large rather than relying on
/// the deadline alone. 1 MiB matches the bound already used for a
/// similarly-purposed manifest read in
/// [`crate::services::code_graph::discover_workspace_crates_from_reader`].
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

/// Build the typed out-of-bounds error for a manifest exceeding
/// [`MAX_MANIFEST_BYTES`].
fn manifest_too_large(len: u64) -> ActivationError {
    ActivationError::ManifestFieldOutOfBounds {
        field: "manifest.bytes_len".to_owned(),
        reason: format!("manifest is {len} bytes, exceeding the {MAX_MANIFEST_BYTES} byte limit"),
    }
}

/// Read at most [`MAX_MANIFEST_BYTES`] `+ 1` bytes from an already-opened
/// manifest `file` handle.
///
/// The metadata length check the caller performs before calling this helper
/// only bounds the read at the moment of the `stat`: a manifest file that
/// grows after that check but before this read runs would otherwise let
/// `read_to_end` consume unbounded memory regardless of the earlier check
/// (F17, Fix2b). Capping the read itself at one byte past the limit lets the
/// caller detect "grew past the limit during the read" the same way it
/// detects "was already past the limit at stat time", without ever
/// buffering more than `MAX_MANIFEST_BYTES + 1` bytes.
fn read_manifest_bytes_bounded(file: &mut File, path: &Path) -> Result<Vec<u8>, ActivationError> {
    use std::io::Read as _;

    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| {
            transient(format!(
                "failed to read active generation manifest at {}: {source}",
                path.display()
            ))
        })?;
    let observed_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if observed_len > MAX_MANIFEST_BYTES {
        return Err(manifest_too_large(observed_len));
    }
    Ok(bytes)
}

/// Read the durable active-generation manifest bytes from `store`.
///
/// # Errors
///
/// Returns [`ActivationError::TransientActivationFailure`] when the manifest is
/// absent or unreadable. Absence is transient rather than permanent: a
/// publisher that has not published yet may publish at any moment, and the
/// read path must stay willing to notice. Returns
/// [`ActivationError::ManifestFieldOutOfBounds`] when the file exceeds
/// [`MAX_MANIFEST_BYTES`], checked both from metadata alone before any bytes
/// are read and again from the actual bytes read (see
/// [`read_manifest_bytes_bounded`]).
fn read_manifest_bytes(store: &GenerationStore) -> Result<Vec<u8>, ActivationError> {
    let path = store.active_manifest_path();
    let mut file = File::open(&path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            transient(format!(
                "no active generation manifest published at {}",
                path.display()
            ))
        } else {
            transient(format!(
                "failed to open active generation manifest at {}: {source}",
                path.display()
            ))
        }
    })?;
    let metadata = file.metadata().map_err(|source| {
        transient(format!(
            "failed to stat active generation manifest at {}: {source}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(manifest_too_large(metadata.len()));
    }
    read_manifest_bytes_bounded(&mut file, &path)
}

/// Read the active manifest on a blocking thread.
///
/// Manifest reconciliation runs on every admitted read request once wired
/// into request-entry (F20), so this filesystem read must never run directly
/// on the async runtime -- the same reasoning [`resolve_and_open`] already
/// documents for the heavier post-parse work.
async fn read_manifest_bytes_blocking(store: GenerationStore) -> Result<Vec<u8>, ActivationError> {
    match tokio::task::spawn_blocking(move || read_manifest_bytes(&store)).await {
        Ok(result) => result,
        Err(join_error) => Err(transient(format!(
            "generation manifest read task did not complete: {join_error}"
        ))),
    }
}

/// Read the durable active-generation manifest bytes and capture its content
/// fingerprint from the same open file handle.
///
/// Capturing both from one handle means the recorded fingerprint always
/// corresponds to exactly the bytes that were parsed, with no separate
/// stat-then-read window for the two to drift apart.
///
/// # Errors
///
/// Returns [`ActivationError::TransientActivationFailure`] when the manifest is
/// absent or unreadable, for the same reason [`read_manifest_bytes`] does.
/// Returns [`ActivationError::ManifestFieldOutOfBounds`] when the file
/// exceeds [`MAX_MANIFEST_BYTES`], checked both from metadata alone before
/// any bytes are read and again from the actual bytes read (see
/// [`read_manifest_bytes_bounded`]).
fn read_manifest_with_fingerprint(
    store: &GenerationStore,
) -> Result<(Vec<u8>, ManifestFingerprint), ActivationError> {
    let path = store.active_manifest_path();
    let mut file = File::open(&path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            transient(format!(
                "no active generation manifest published at {}",
                path.display()
            ))
        } else {
            transient(format!(
                "failed to open active generation manifest at {}: {source}",
                path.display()
            ))
        }
    })?;
    let metadata = file.metadata().map_err(|source| {
        transient(format!(
            "failed to stat active generation manifest at {}: {source}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(manifest_too_large(metadata.len()));
    }
    let bytes = read_manifest_bytes_bounded(&mut file, &path)?;
    let checksum = hex_lower(&Sha256::digest(&bytes));
    let fingerprint = ManifestFingerprint {
        len: metadata.len(),
        checksum,
    };
    Ok((bytes, fingerprint))
}

/// Read the active manifest and its fingerprint on a blocking thread. See
/// [`read_manifest_with_fingerprint`] and [`read_manifest_bytes_blocking`].
async fn read_manifest_with_fingerprint_blocking(
    store: GenerationStore,
) -> Result<(Vec<u8>, ManifestFingerprint), ActivationError> {
    match tokio::task::spawn_blocking(move || read_manifest_with_fingerprint(&store)).await {
        Ok(result) => result,
        Err(join_error) => Err(transient(format!(
            "generation manifest read task did not complete: {join_error}"
        ))),
    }
}

/// Recompute the SHA-256 digest of `path` as lowercase hex.
///
/// The read is bounded at `per_artifact_cap + 1` bytes rather than trusting
/// the metadata-time size the caller already checked: a sealed artifact that
/// grows or is replaced after that stat but before this digest read would
/// otherwise let `io::copy` stream an unbounded number of bytes through the
/// hasher, consuming unbounded blocking-thread I/O and bypassing both the
/// per-artifact and cumulative caps [`resolve_and_open`] is meant to enforce
/// (F17, Fix3, round-7 review). Capping the read itself lets the cap be
/// re-checked against the bytes actually read, the same pattern already used
/// by [`read_manifest_bytes_bounded`] for the manifest file.
///
/// `per_artifact_cap` is a parameter (mirroring [`check_artifact_size`])
/// specifically so this can be regression-tested with a small cap instead of
/// writing multi-gigabyte files to disk.
///
/// # Errors
///
/// Returns [`ActivationError::ManifestFieldOutOfBounds`] when the file
/// actually contains more than `per_artifact_cap` bytes at read time, even if
/// it was within the cap at the earlier metadata check.
fn file_digest(path: &Path, per_artifact_cap: u64) -> Result<String, ActivationError> {
    use std::io::Read as _;

    let mut file = File::open(path).map_err(|source| {
        transient(format!(
            "failed to open sealed inventory file {}: {source}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut bounded = (&mut file).take(per_artifact_cap.saturating_add(1));
    let bytes_read = io::copy(&mut bounded, &mut hasher).map_err(|source| {
        transient(format!(
            "failed to read sealed inventory file {}: {source}",
            path.display()
        ))
    })?;
    if bytes_read > per_artifact_cap {
        return Err(out_of_bounds(
            "inventory.files[].size",
            format!(
                "sealed artifact {} grew past the {per_artifact_cap} byte per-artifact cap while being read",
                path.display()
            ),
        ));
    }
    Ok(hex_lower(&hasher.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        // Writing into a `String` is infallible, so the result is discarded
        // rather than propagated through a digest helper that has no other
        // failure mode.
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

#[cfg(test)]
mod size_cap_tests {
    //! Regression coverage for the F17 sealed-artifact size caps
    //! (142.018-T). `check_artifact_size` takes its caps as parameters
    //! specifically so these tests can exercise the per-artifact and
    //! cumulative-total rejection paths with small caps, without ever
    //! writing multi-gigabyte files to disk.

    use super::{ActivationError, Path, check_artifact_size};

    #[test]
    fn a_single_artifact_over_the_per_artifact_cap_is_rejected() {
        let mut running_total: u64 = 0;

        let error = check_artifact_size(
            Path::new("gen-a/oversized.bin"),
            101,
            &mut running_total,
            100,
            1_000,
        )
        .expect_err("a single artifact over the per-artifact cap must be rejected");

        assert!(matches!(
            error,
            ActivationError::ManifestFieldOutOfBounds { .. }
        ));
        // Rejection must happen before any accumulation occurs.
        assert_eq!(running_total, 0);
    }

    #[test]
    fn entries_individually_under_cap_whose_sum_exceeds_the_total_cap_are_rejected() {
        let per_artifact_cap = 100;
        let total_cap = 150;
        let mut running_total: u64 = 0;

        check_artifact_size(
            Path::new("gen-a/first.bin"),
            90,
            &mut running_total,
            per_artifact_cap,
            total_cap,
        )
        .expect("the first entry alone is under both caps");
        assert_eq!(running_total, 90);

        let error = check_artifact_size(
            Path::new("gen-a/second.bin"),
            90,
            &mut running_total,
            per_artifact_cap,
            total_cap,
        )
        .expect_err("the cumulative total must be rejected even though each entry is individually under the per-artifact cap");

        assert!(matches!(
            error,
            ActivationError::ManifestFieldOutOfBounds { .. }
        ));
        // A rejected entry must not be folded into the running total.
        assert_eq!(running_total, 90);
    }
}

#[cfg(test)]
mod bounded_manifest_read_tests {
    //! Regression coverage for F17 Fix2b (142.018-T round-6 review):
    //! `read_manifest_bytes_bounded` must reject a manifest exceeding
    //! [`MAX_MANIFEST_BYTES`] from the actual bytes read, not only from a
    //! separate metadata check that a growing file could outrun.

    use std::io::Write as _;

    use super::{ActivationError, File, MAX_MANIFEST_BYTES, read_manifest_bytes_bounded};

    #[test]
    fn a_manifest_within_the_cap_is_read_back_in_full() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("active.json");
        let contents = b"{\"generation_id\":\"gen-1\"}";
        std::fs::write(&path, contents).expect("write manifest");

        let mut file = File::open(&path).expect("open manifest");
        let bytes =
            read_manifest_bytes_bounded(&mut file, &path).expect("within-cap read must succeed");

        assert_eq!(bytes, contents);
    }

    #[test]
    fn a_manifest_whose_actual_bytes_exceed_the_cap_is_rejected_by_the_read_itself() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("active.json");

        // Write a file whose content genuinely exceeds MAX_MANIFEST_BYTES.
        // This directly exercises the read-time bound: a caller that skipped
        // (or raced past) the metadata pre-check must still be protected by
        // the `.take(cap + 1)` in `read_manifest_bytes_bounded` itself.
        let mut file = std::fs::File::create(&path).expect("create manifest");
        let oversized_len = usize::try_from(MAX_MANIFEST_BYTES).unwrap_or(usize::MAX) + 1;
        let chunk = vec![b'a'; oversized_len];
        file.write_all(&chunk).expect("write oversized manifest");
        drop(file);

        let mut file = File::open(&path).expect("open manifest");
        let error = read_manifest_bytes_bounded(&mut file, &path)
            .expect_err("a manifest whose real bytes exceed the cap must be rejected");

        assert!(matches!(
            error,
            ActivationError::ManifestFieldOutOfBounds { .. }
        ));
    }
}

#[cfg(test)]
mod bounded_digest_read_tests {
    //! Regression coverage for F17 Fix3 (142.018-T round-7 review):
    //! `file_digest` must bound its own read at `per_artifact_cap + 1`
    //! bytes and re-check the cap against bytes actually read, rather than
    //! trusting the caller's earlier metadata-time size check. A sealed
    //! artifact that grows after that stat but before this read would
    //! otherwise let `io::copy` stream it unbounded and bypass the cap.

    use std::io::Write as _;

    use super::{ActivationError, file_digest};

    #[test]
    fn a_file_within_the_cap_is_digested_normally() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("artifact.bin");
        std::fs::write(&path, b"small sealed artifact").expect("write artifact");

        let digest =
            file_digest(&path, 1_000).expect("a file within the cap must digest successfully");

        assert_eq!(digest.len(), 64, "SHA-256 hex digest must be 64 characters");
    }

    #[test]
    fn a_file_whose_actual_bytes_exceed_the_cap_is_rejected_by_the_read_itself() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("artifact.bin");

        // Write a file whose content genuinely exceeds a small per-artifact
        // cap, simulating an artifact that grew after the caller's earlier
        // metadata-time stat. This must be caught by the bounded read in
        // `file_digest` itself, not merely by a separate metadata check.
        let per_artifact_cap: u64 = 100;
        let oversized_len = usize::try_from(per_artifact_cap).unwrap_or(usize::MAX) + 1;
        let mut file = std::fs::File::create(&path).expect("create artifact");
        file.write_all(&vec![b'a'; oversized_len])
            .expect("write oversized artifact");
        drop(file);

        let error = file_digest(&path, per_artifact_cap)
            .expect_err("a file whose real bytes exceed the cap must be rejected");

        assert!(matches!(
            error,
            ActivationError::ManifestFieldOutOfBounds { .. }
        ));
    }
}
