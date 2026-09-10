use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ContractShapeError;

pub(crate) fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_'))
}

/// A client-supplied idempotency key: canonical RFC 4122 textual UUID,
/// normalized to lowercase.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ContractShapeError> {
        let value = value.into();
        let normalized = value.to_ascii_lowercase();
        let bytes = normalized.as_bytes();
        let well_formed = bytes.len() == 36
            && normalized.char_indices().all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            });
        if well_formed {
            Ok(Self(normalized))
        } else {
            Err(ContractShapeError::MalformedIdempotencyKey { value })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = ContractShapeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(key: IdempotencyKey) -> Self {
        key.0
    }
}

/// A relative resource name in AIP-122 shape: `collection/resource-id`,
/// both segments slug-form.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResourceName {
    collection: String,
    resource_id: String,
}

impl ResourceName {
    pub fn new(
        collection: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Result<Self, ContractShapeError> {
        let collection = collection.into();
        let resource_id = resource_id.into();
        if is_slug(&collection) && is_slug(&resource_id) {
            Ok(Self {
                collection,
                resource_id,
            })
        } else {
            Err(ContractShapeError::MalformedResourceName {
                value: format!("{collection}/{resource_id}"),
            })
        }
    }

    #[must_use]
    pub fn collection(&self) -> &str {
        &self.collection
    }

    #[must_use]
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }
}

impl fmt::Display for ResourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.collection, self.resource_id)
    }
}

impl TryFrom<String> for ResourceName {
    type Error = ContractShapeError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.split_once('/') {
            Some((collection, resource_id)) if !resource_id.contains('/') => {
                Self::new(collection, resource_id)
            }
            _ => Err(ContractShapeError::MalformedResourceName { value }),
        }
    }
}

impl From<ResourceName> for String {
    fn from(name: ResourceName) -> Self {
        name.to_string()
    }
}
