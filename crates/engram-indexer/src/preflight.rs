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

use engram::services::generations::candidate_build::ReadServerLayout;

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
/// STRUCTURAL STUB (harness step): every stage fails closed until implemented.
#[derive(Debug)]
pub struct ProductionVerifier {
    layout: ReadServerLayout,
    engram_executable: PathBuf,
    deadline: Instant,
}

impl ProductionVerifier {
    /// Creates a verifier from the single layout, executable and deadline.
    ///
    /// # Errors
    ///
    /// Returns [`VerificationError`] when the verifier cannot be constructed.
    pub fn new(
        layout: ReadServerLayout,
        engram_executable: PathBuf,
        deadline: Instant,
    ) -> Result<Self, VerificationError> {
        Ok(Self {
            layout,
            engram_executable,
            deadline,
        })
    }

    /// Returns the engram executable the probe stages will use.
    #[must_use]
    pub fn engram_executable(&self) -> &Path {
        &self.engram_executable
    }

    /// Returns the layout this verifier was built from.
    #[must_use]
    pub const fn layout(&self) -> &ReadServerLayout {
        &self.layout
    }

    /// Returns the shared deadline.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }
}

impl Verifier for ProductionVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        _deadline: Instant,
        _expected_generation: &str,
    ) -> Result<(), VerificationError> {
        Err(VerificationError::new(format!("{stage:?} not implemented")))
    }
}
