//! Revision builders for link and action type definitions.

use crate::definitions::{ActionTypeDefinition, LinkTypeDefinition};

impl LinkTypeDefinition {
    pub fn with_revision(mut self, revision: u32) -> Self {
        self.revision = revision;
        self
    }
}

impl ActionTypeDefinition {
    pub fn with_revision(mut self, revision: u32) -> Self {
        self.revision = revision;
        self
    }
}
