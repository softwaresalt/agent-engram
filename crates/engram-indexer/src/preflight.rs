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
use std::time::Instant;

/// The preflight stage currently being verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Error returned by a [`Verifier`] when a stage cannot be verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationError {
    reason: String,
}

impl VerificationError {
    /// Creates a verification error carrying a diagnostic reason.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
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
pub struct Preflight<S> {
    deadline: Instant,
    expected_generation: String,
    state: PhantomData<S>,
}

impl Preflight<Build> {
    /// Starts preflight with its single deadline and expected generation.
    #[must_use]
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
    /// # Errors
    ///
    /// Returns this stage's [`Failure`] if verification fails or the shared
    /// deadline has expired.
    ///
    /// # Panics
    ///
    /// STUB (RED phase): always panics until Worker task 142.078-T.
    pub fn verify<V: Verifier>(self, _verifier: &mut V) -> Result<Preflight<S::Next>, Failure> {
        unimplemented!("Worker: 142.078-T")
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
