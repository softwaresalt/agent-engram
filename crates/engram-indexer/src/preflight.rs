//! Typed preflight state machine for the indexing supervisor (plan unit F50a).
//!
//! The machine walks Build, Seal, Publish, `DaemonVerified`, `HealthVerified`,
//! `CliProbeVerified`, `McpProbeVerified`, then `Succeeded`. Transitions are
//! forward only and encoded in the type system, so skipping a stage is
//! unrepresentable. One deadline and one expected generation are carried by
//! [`Preflight`] itself. Verification is fail-closed: any failure yields a
//! typed [`Failure`] naming the stage, never a degraded success.

use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Instant;

use engram::models::config::CodeGraphConfig;
use engram::services::generations::candidate_build::{
    BuiltCandidate, CandidateBuildError, ReadServerLayout, SealedCandidate,
    build_candidate_generation, open_generation_store, publish_sealed_candidate,
    seal_candidate_inventory,
};
use engram::services::generations::{GenerationId, GenerationStore};
use tokio::runtime::{Builder, Handle, Runtime};

/// The preflight stage currently being verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageKind {
    /// The index build completed.
    Build,
    /// The built index was sealed.
    Seal,
    /// The sealed index was published.
    Publish,
    /// The daemon verified the published generation.
    DaemonVerified,
    /// The daemon health check passed.
    HealthVerified,
    /// The CLI probe passed.
    CliProbeVerified,
    /// The MCP probe passed.
    McpProbeVerified,
}

/// The stage-specific failure returned by preflight verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Failure {
    /// Build verification failed.
    Build,
    /// Seal verification failed.
    Seal,
    /// Publish verification failed.
    Publish,
    /// Daemon verification failed.
    DaemonVerified,
    /// Health verification failed.
    HealthVerified,
    /// CLI probe verification failed.
    CliProbeVerified,
    /// MCP probe verification failed.
    McpProbeVerified,
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = match self {
            Self::Build => "build",
            Self::Seal => "seal",
            Self::Publish => "publish",
            Self::DaemonVerified => "daemon verification",
            Self::HealthVerified => "health verification",
            Self::CliProbeVerified => "CLI probe verification",
            Self::McpProbeVerified => "MCP probe verification",
        };
        write!(formatter, "preflight failed at stage: {stage}")
    }
}

impl std::error::Error for Failure {}

/// Type-state marker for the build stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build;

/// Type-state marker for the seal stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seal;

/// Type-state marker for the publish stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Publish;

/// Type-state marker for daemon verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonVerified;

/// Type-state marker for health verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthVerified;

/// Type-state marker for CLI probe verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliProbeVerified;

/// Type-state marker for MCP probe verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpProbeVerified;

/// Type-state marker for a fully verified preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Succeeded;

mod private {
    pub trait Sealed {}
}

/// A valid stage in the ordered preflight sequence (sealed).
pub trait Stage: private::Sealed {
    /// The state reached after this stage verifies.
    type Next;

    /// The stage represented by this state.
    const KIND: StageKind;

    /// The typed failure associated with this stage.
    const FAILURE: Failure;
}

macro_rules! stage {
    ($stage:ty, $kind:ident, $next:ty) => {
        impl private::Sealed for $stage {}

        impl Stage for $stage {
            type Next = $next;

            const KIND: StageKind = StageKind::$kind;
            const FAILURE: Failure = Failure::$kind;
        }
    };
}

stage!(Build, Build, Seal);
stage!(Seal, Seal, Publish);
stage!(Publish, Publish, DaemonVerified);
stage!(DaemonVerified, DaemonVerified, HealthVerified);
stage!(HealthVerified, HealthVerified, CliProbeVerified);
stage!(CliProbeVerified, CliProbeVerified, McpProbeVerified);
stage!(McpProbeVerified, McpProbeVerified, Succeeded);

/// Performs the underlying verification for a preflight stage.
///
/// Implementations must enforce the deadline themselves (for example via
/// timeouts on I/O). The gate also post-checks the deadline after each call
/// and fails closed if it was exceeded, but it cannot interrupt a verifier.
pub trait Verifier {
    /// Verifies a stage using the shared deadline and expected generation.
    ///
    /// # Errors
    ///
    /// Returns [`VerificationError`] if the stage cannot be verified.
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError>;
}

/// Classifies why a [`Verifier`] could not verify a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VerificationErrorKind {
    /// The stage failed for a reason other than the deadline.
    Failed,
    /// The shared deadline expired before or during the stage.
    DeadlineExpired,
}

/// Error returned by a [`Verifier`] when a stage cannot be verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationError {
    kind: VerificationErrorKind,
    reason: String,
}

impl VerificationError {
    /// Creates a verification error carrying a diagnostic reason.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            kind: VerificationErrorKind::Failed,
            reason: reason.into(),
        }
    }

    /// Creates a typed deadline-expiry error carrying a diagnostic reason.
    #[must_use]
    pub fn deadline_expired(reason: impl Into<String>) -> Self {
        Self {
            kind: VerificationErrorKind::DeadlineExpired,
            reason: reason.into(),
        }
    }

    /// Returns why verification failed.
    #[must_use]
    pub const fn kind(&self) -> VerificationErrorKind {
        self.kind
    }

    /// Returns the diagnostic reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "preflight verification failed: {}", self.reason)
    }
}

impl std::error::Error for VerificationError {}

/// Preflight state carrying the shared deadline and expected generation.
#[derive(Debug)]
#[must_use = "preflight state must be advanced or inspected"]
pub struct Preflight<S> {
    deadline: Instant,
    expected_generation: String,
    state: PhantomData<S>,
}

impl Preflight<Build> {
    /// Starts preflight with its single deadline and expected generation.
    pub fn start(deadline: Instant, expected_generation: String) -> Self {
        Self {
            deadline,
            expected_generation,
            state: PhantomData,
        }
    }
}

impl<S: Stage> Preflight<S> {
    /// Verifies this stage and advances to its type-defined successor.
    ///
    /// A stage that completes after the deadline is reported as failed
    /// (fail-closed) even if its side effects already completed.
    ///
    /// # Errors
    ///
    /// Returns this stage's [`Failure`] if verification fails or the shared
    /// deadline has expired.
    pub fn verify<V: Verifier>(self, verifier: &mut V) -> Result<Preflight<S::Next>, Failure> {
        // Fail closed: an already-expired deadline never reaches the verifier.
        if Instant::now() >= self.deadline {
            return Err(S::FAILURE);
        }
        verifier
            .verify(S::KIND, self.deadline, &self.expected_generation)
            .map_err(|_| S::FAILURE)?;
        // Fail closed: a verification that finished past the deadline is not trusted.
        if Instant::now() >= self.deadline {
            return Err(S::FAILURE);
        }
        Ok(Preflight {
            deadline: self.deadline,
            expected_generation: self.expected_generation,
            state: PhantomData,
        })
    }
}

impl Preflight<Succeeded> {
    /// Returns the deadline shared by all preflight stages.
    #[must_use]
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Returns the generation expected by all preflight stages.
    #[must_use]
    pub fn expected_generation(&self) -> &str {
        &self.expected_generation
    }
}

/// Production [`Verifier`] for the Build, Seal and Publish stages (plan unit F50b).
///
/// Inputs are the single layout produced by the factory's one
/// `read_server_layout` call, the engram executable and the shared deadline;
/// the expected generation arrives through [`Verifier::verify`]. The verifier
/// owns a current-thread runtime, never re-derives workspace identity, and
/// is fail-closed: any failure leaves the previously published generation
/// untouched because only the final Publish stage mutates the manifest.
///
/// The four probe stages belong to a later slot and fail closed here.
pub struct ProductionVerifier {
    layout: ReadServerLayout,
    engram_executable: PathBuf,
    deadline: Instant,
    runtime: Option<Runtime>,
    store: Option<GenerationStore>,
    built: Option<BuiltCandidate>,
    sealed: Option<SealedCandidate>,
}

impl Drop for ProductionVerifier {
    fn drop(&mut self) {
        // Dropping a runtime inside an async context panics; shut it down
        // without blocking so the verifier is safe to drop anywhere.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

impl fmt::Debug for ProductionVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductionVerifier")
            .field("engram_executable", &self.engram_executable)
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

impl ProductionVerifier {
    /// Creates a verifier from the single layout, executable and deadline.
    ///
    /// # Errors
    ///
    /// Returns [`VerificationError`] when called inside another tokio runtime
    /// (the verifier owns its own and never nests) or when its runtime cannot
    /// be created.
    pub fn new(
        layout: ReadServerLayout,
        engram_executable: PathBuf,
        deadline: Instant,
    ) -> Result<Self, VerificationError> {
        if Handle::try_current().is_ok() {
            return Err(VerificationError::new(
                "production verifier must not run inside another runtime",
            ));
        }
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| {
                VerificationError::new(format!("runtime creation failed: {:?}", error.kind()))
            })?;
        Ok(Self {
            layout,
            engram_executable,
            deadline,
            runtime: Some(runtime),
            store: None,
            built: None,
            sealed: None,
        })
    }

    /// Returns the engram executable the probe stages will use.
    #[must_use]
    pub fn engram_executable(&self) -> &Path {
        &self.engram_executable
    }

    /// Returns the shared deadline.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
}

/// Producer label recorded in the manifest by the Publish stage.
const PREFLIGHT_PRODUCER: &str = "engram-preflight";

/// Fails typed when the deadline has already expired.
fn ensure_time_remaining(stage: StageKind, deadline: Instant) -> Result<(), VerificationError> {
    if Instant::now() >= deadline {
        Err(VerificationError::deadline_expired(format!(
            "deadline expired before {stage:?}"
        )))
    } else {
        Ok(())
    }
}

/// Maps a candidate-facade failure; its messages carry no paths or payloads.
fn stage_failure(stage: StageKind, error: &CandidateBuildError) -> VerificationError {
    VerificationError::new(format!("{stage:?} failed: {error}"))
}

fn out_of_order(stage: StageKind, missing: &str) -> VerificationError {
    VerificationError::new(format!("{stage:?} requires {missing}"))
}

impl ProductionVerifier {
    /// Build: opens the store (first write), mints nothing, builds the
    /// candidate under the deadline.
    fn build_stage(
        &mut self,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError> {
        // `block_on` panics inside a runtime; fail closed with a typed error
        // instead (the constructor guard only covers construction time).
        if Handle::try_current().is_ok() {
            return Err(VerificationError::new(
                "production verifier must not run inside another runtime",
            ));
        }
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| VerificationError::new("verifier runtime is unavailable"))?;
        // A fresh attempt discards any state from an earlier one.
        self.store = None;
        self.built = None;
        self.sealed = None;
        let id = GenerationId::new(expected_generation).map_err(|error| {
            VerificationError::new(format!("Build rejected expected generation: {error}"))
        })?;
        let store = open_generation_store(&self.layout, true)
            .map_err(|error| stage_failure(StageKind::Build, &error))?;
        // Indexing configuration is the facade's default; the daemon and the
        // preflight must index identically, so no override exists here.
        let config = CodeGraphConfig::default();
        let layout = &self.layout;
        let pending = build_candidate_generation(layout, &store, &id, &config);
        let limit = tokio::time::Instant::from_std(deadline);
        // The timeout preempts only at `.await` points; synchronous work inside
        // the facade is caught late by the machine's post-deadline check. A
        // timed-out build may leave a partial candidate directory, which is
        // never published (a fresh generation ID is used per attempt).
        let built = runtime
            .block_on(async { tokio::time::timeout_at(limit, pending).await })
            .map_err(|_| VerificationError::deadline_expired("deadline expired during Build"))?
            .map_err(|error| stage_failure(StageKind::Build, &error))?;
        self.store = Some(store);
        self.built = Some(built);
        Ok(())
    }

    /// Seal: computes the sealed inventory of the built candidate.
    fn seal_stage(&mut self, expected_generation: &str) -> Result<(), VerificationError> {
        let built = self
            .built
            .take()
            .ok_or_else(|| out_of_order(StageKind::Seal, "a built candidate"))?;
        // One expected generation spans every stage; never seal another one.
        if built.id().as_str() != expected_generation {
            return Err(VerificationError::new(
                "Seal expected generation differs from the built candidate",
            ));
        }
        let sealed = seal_candidate_inventory(built)
            .map_err(|error| stage_failure(StageKind::Seal, &error))?;
        self.sealed = Some(sealed);
        Ok(())
    }

    /// Publish: atomic publication of the sealed candidate under the
    /// publisher lock.
    fn publish_stage(&mut self, expected_generation: &str) -> Result<(), VerificationError> {
        let store = self
            .store
            .as_ref()
            .ok_or_else(|| out_of_order(StageKind::Publish, "an open store"))?;
        let sealed = self
            .sealed
            .take()
            .ok_or_else(|| out_of_order(StageKind::Publish, "a sealed candidate"))?;
        // Only the candidate this machine built and sealed may be published.
        if sealed.id().as_str() != expected_generation {
            return Err(VerificationError::new(
                "Publish expected generation differs from the sealed candidate",
            ));
        }
        publish_sealed_candidate(&self.layout, store, sealed, PREFLIGHT_PRODUCER)
            .map(|_| ())
            .map_err(|error| stage_failure(StageKind::Publish, &error))
    }
}

impl Verifier for ProductionVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError> {
        // The earlier of the machine's and the constructor's deadline governs.
        let deadline = deadline.min(self.deadline);
        ensure_time_remaining(stage, deadline)?;
        match stage {
            StageKind::Build => self.build_stage(deadline, expected_generation),
            StageKind::Seal => self.seal_stage(expected_generation),
            StageKind::Publish => self.publish_stage(expected_generation),
            StageKind::DaemonVerified
            | StageKind::HealthVerified
            | StageKind::CliProbeVerified
            | StageKind::McpProbeVerified => Err(VerificationError::new(format!(
                "{stage:?} is not implemented in this slice"
            ))),
        }
    }
}
