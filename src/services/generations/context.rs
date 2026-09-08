//! Shared read context for a published generation opened through a runtime copy.
//!
//! The context layer sits above the database open path and below server state:
//! it owns a single opened generation handle behind an [`Arc`] so callers can
//! cheaply clone read access without reopening or re-copying the database.

use std::sync::Arc;

use crate::db::cozo_backend::OpenedGeneration;

use super::{GenerationId, GenerationIdError};

/// Shared read context for one published generation.
#[derive(Debug, Clone)]
pub struct GenerationReadContext {
    generation_id: GenerationId,
    opened_generation: Arc<OpenedGeneration>,
}

impl GenerationReadContext {
    /// Construct a shared read context from an already-opened generation.
    ///
    /// The identifier is derived from `opened_generation`'s own runtime copy
    /// (`OpenedGeneration::runtime_copy().generation_id()`), not accepted as an
    /// independent parameter: a caller-supplied identifier could otherwise be
    /// paired with an unrelated `OpenedGeneration`, so the context would
    /// report one generation while actually serving a different database.
    /// Deriving the identifier from the same value that was already opened
    /// closes that gap.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationIdError`] when the runtime copy's carried
    /// identifier is not a valid [`GenerationId`] (the lower `db` layer uses
    /// a looser, platform-dependent single-path-component check; this
    /// constructor re-validates against this module's stricter rule before a
    /// context is ever handed to a caller).
    pub fn new(opened_generation: OpenedGeneration) -> Result<Self, GenerationIdError> {
        let generation_id = GenerationId::new(opened_generation.runtime_copy().generation_id())?;
        Ok(Self {
            generation_id,
            opened_generation: Arc::new(opened_generation),
        })
    }

    /// Return the generation identifier represented by this context.
    #[must_use]
    pub const fn generation_id(&self) -> &GenerationId {
        &self.generation_id
    }

    /// Borrow the opened generation handle shared by this context.
    #[must_use]
    pub fn opened_generation(&self) -> &OpenedGeneration {
        self.opened_generation.as_ref()
    }

    /// Borrow the shared opened-generation ownership handle.
    #[must_use]
    pub fn shared_opened_generation(&self) -> &Arc<OpenedGeneration> {
        &self.opened_generation
    }
}
