//! Shared read context for a published generation opened through a runtime copy.
//!
//! The context layer sits above the database open path and below server state:
//! it owns a single opened generation handle behind an [`Arc`] so callers can
//! cheaply clone read access without reopening or re-copying the database.

use std::sync::Arc;

use crate::db::cozo_backend::OpenedGeneration;

use super::GenerationId;

/// Shared read context for one published generation.
#[derive(Debug, Clone)]
pub struct GenerationReadContext {
    generation_id: GenerationId,
    opened_generation: Arc<OpenedGeneration>,
}

impl GenerationReadContext {
    /// Construct a shared read context from an already-opened generation.
    #[must_use]
    pub fn new(generation_id: GenerationId, opened_generation: OpenedGeneration) -> Self {
        Self {
            generation_id,
            opened_generation: Arc::new(opened_generation),
        }
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
