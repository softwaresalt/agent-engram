//! Typed preflight state-machine scaffold for the indexing supervisor.

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

/// A valid stage in the ordered preflight sequence.
pub trait Stage: private::Sealed {
    /// The state reached after this stage verifies.
    type Next;

    /// The stage represented by this state.
    const KIND: StageKind;

    /// The typed failure associated with this stage.
    const FAILURE: Failure;
}

macro_rules! stage {
    ($stage:ty, $kind:ident, $failure:ident, $next:ty) => {
        impl private::Sealed for $stage {}

        impl Stage for $stage {
            type Next = $next;

            const KIND: StageKind = StageKind::$kind;
            const FAILURE: Failure = Failure::$failure;
        }
    };
}

stage!(Build, Build, Build, Seal);
stage!(Seal, Seal, Seal, Publish);
stage!(Publish, Publish, Publish, DaemonVerified);
stage!(
    DaemonVerified,
    DaemonVerified,
    DaemonVerified,
    HealthVerified
);
stage!(
    HealthVerified,
    HealthVerified,
    HealthVerified,
    CliProbeVerified
);
stage!(
    CliProbeVerified,
    CliProbeVerified,
    CliProbeVerified,
    McpProbeVerified
);
stage!(
    McpProbeVerified,
    McpProbeVerified,
    McpProbeVerified,
    Succeeded
);

/// Performs the underlying verification for a preflight stage.
pub trait Verifier {
    /// Verifies a stage using the shared deadline and expected generation.
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), ()>;
}

/// Preflight state carrying the shared deadline and expected generation.
#[derive(Debug)]
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
    /// The stage fails if verification fails or if the shared deadline has
    /// expired before or during verification.
    pub fn verify<V: Verifier>(self, verifier: &mut V) -> Result<Preflight<S::Next>, Failure> {
        if Instant::now() >= self.deadline {
            return Err(S::FAILURE);
        }

        verifier
            .verify(S::KIND, self.deadline, &self.expected_generation)
            .map_err(|()| S::FAILURE)?;

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
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Returns the generation expected by all preflight stages.
    pub fn expected_generation(&self) -> &str {
        &self.expected_generation
    }
}
