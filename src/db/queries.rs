#![allow(dead_code)]

use crate::errors::TMemError;
use crate::models::{Context, Spec, Task};

/// Query builder placeholder. Concrete SurrealDB queries will be implemented in subsequent phases.
pub struct Queries;

impl Queries {
    pub fn new() -> Self {
        Self
    }

    pub async fn insert_task(&self, _task: &Task) -> Result<(), TMemError> {
        // TODO: implement SurrealQL INSERT for task
        Ok(())
    }

    pub async fn insert_spec(&self, _spec: &Spec) -> Result<(), TMemError> {
        Ok(())
    }

    pub async fn insert_context(&self, _ctx: &Context) -> Result<(), TMemError> {
        Ok(())
    }
}
