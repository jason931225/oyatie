//! Partitions capability entries into their discoverable and invocable
//! subsets.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;

use crate::{CapabilityId, CapabilityStatus};

/// A capability can land in both maps, one, or neither.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryViews {
    pub discoverable: BTreeMap<CapabilityId, CapabilityStatus>,
    pub invocable: BTreeMap<CapabilityId, CapabilityStatus>,
}

/// A repeated [`CapabilityId`] resolves to its last entry, silently.
pub fn partition_views(
    entries: impl IntoIterator<Item = (CapabilityId, CapabilityStatus)>,
) -> RegistryViews {
    let deduped: BTreeMap<CapabilityId, CapabilityStatus> = entries.into_iter().collect();

    let mut discoverable = BTreeMap::new();
    let mut invocable = BTreeMap::new();

    for (id, status) in deduped {
        if status.is_discoverable() {
            discoverable.insert(id.clone(), status);
        }
        if status.is_invocable() {
            invocable.insert(id, status);
        }
    }

    RegistryViews {
        discoverable,
        invocable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> CapabilityId {
        CapabilityId::new(s)
    }

    #[test]
    fn active_appears_in_both_views() {
        let views = partition_views([(id("svc.op.active"), CapabilityStatus::Active)]);
        assert!(views.discoverable.contains_key(&id("svc.op.active")));
        assert!(views.invocable.contains_key(&id("svc.op.active")));
    }

    #[test]
    fn deprecated_invocable_only() {
        let views = partition_views([(id("svc.op.deprecated"), CapabilityStatus::Deprecated)]);
        assert!(!views.discoverable.contains_key(&id("svc.op.deprecated")));
        assert!(views.invocable.contains_key(&id("svc.op.deprecated")));
    }

    #[test]
    fn disabled_in_neither_view() {
        let views = partition_views([(id("svc.op.disabled"), CapabilityStatus::Disabled)]);
        assert!(!views.discoverable.contains_key(&id("svc.op.disabled")));
        assert!(!views.invocable.contains_key(&id("svc.op.disabled")));
    }

    #[test]
    fn ordering_is_stable_and_lexicographic() {
        let entries = vec![
            (id("z.op"), CapabilityStatus::Active),
            (id("a.op"), CapabilityStatus::Active),
            (id("m.op"), CapabilityStatus::Active),
        ];
        let views = partition_views(entries);
        let keys: Vec<_> = views.discoverable.keys().map(|k| k.0.as_str()).collect();
        assert_eq!(keys, ["a.op", "m.op", "z.op"]);
        let inv_keys: Vec<_> = views.invocable.keys().map(|k| k.0.as_str()).collect();
        assert_eq!(inv_keys, ["a.op", "m.op", "z.op"]);
    }

    #[test]
    fn mixed_collection_partitioned_correctly() {
        let entries = vec![
            (id("cap.active"), CapabilityStatus::Active),
            (id("cap.deprecated"), CapabilityStatus::Deprecated),
            (id("cap.disabled"), CapabilityStatus::Disabled),
        ];
        let views = partition_views(entries);

        assert_eq!(views.discoverable.len(), 1);
        assert!(views.discoverable.contains_key(&id("cap.active")));

        assert_eq!(views.invocable.len(), 2);
        assert!(views.invocable.contains_key(&id("cap.active")));
        assert!(views.invocable.contains_key(&id("cap.deprecated")));
        assert!(!views.invocable.contains_key(&id("cap.disabled")));
    }

    #[test]
    fn empty_input_yields_empty_views() {
        let views = partition_views(std::iter::empty());
        assert!(views.discoverable.is_empty());
        assert!(views.invocable.is_empty());
    }
}
