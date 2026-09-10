//! A node-identity aggregate and a label map, built from the smaller newtypes.

use crate::address::{Hostname, NodeAddress};
use crate::error::{Error, Result};
use crate::machine_type::MachineType;
use crate::platform::Platform;
use crate::version::Version;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A validated key/value label map, Kubernetes-style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    inner: BTreeMap<String, String>,
}

impl Labels {
    pub fn new() -> Self {
        Labels {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let key = key.into();
        if key.is_empty() {
            return Err(Error::invalid("label key is empty"));
        }
        for c in key.chars() {
            if !is_label_key_char(c) {
                return Err(Error::invalid(alloc::format!(
                    "invalid label key character '{c}'"
                )));
            }
        }
        self.inner.insert(key, value.into());
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(key).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// A selector matches when every one of ITS entries is present and equal
    /// here; extra labels here do not prevent a match.
    pub fn matches(&self, selector: &Labels) -> bool {
        selector
            .inner
            .iter()
            .all(|(k, v)| self.inner.get(k) == Some(v))
    }

    /// Key order is sorted and therefore stable across calls, so the result is
    /// safe to compare or hash.
    pub fn to_selector_string(&self) -> String {
        let parts: Vec<String> = self
            .inner
            .iter()
            .map(|(k, v)| alloc::format!("{k}={v}"))
            .collect();
        parts.join(",")
    }
}

fn is_label_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-')
}

/// A single node's identity: the value most subsystems thread through their
/// APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    pub hostname: Hostname,
    pub address: NodeAddress,
    pub machine_type: MachineType,
    pub platform: Platform,
    pub os_version: Version,
    pub labels: Labels,
}

impl NodeIdentity {
    /// Starts with no labels; add them with [`Labels::insert`].
    pub fn new(
        hostname: Hostname,
        address: NodeAddress,
        machine_type: MachineType,
        platform: Platform,
        os_version: Version,
    ) -> Self {
        NodeIdentity {
            hostname,
            address,
            machine_type,
            platform,
            os_version,
            labels: Labels::new(),
        }
    }

    /// A node that has actually joined can never still be `Unknown` on either
    /// axis, so either one is a refusal rather than a default.
    pub fn validate(&self) -> Result<()> {
        if self.machine_type == MachineType::Unknown {
            return Err(Error::invalid_state("node has unknown machine type"));
        }
        if self.platform == Platform::Unknown {
            return Err(Error::invalid_state("node has unknown platform"));
        }
        Ok(())
    }

    pub fn is_control_plane(&self) -> bool {
        self.machine_type.is_control_plane()
    }

    pub fn display_name(&self) -> String {
        alloc::format!(
            "{}/{}/{}",
            self.hostname.as_str(),
            self.machine_type.as_str(),
            self.address
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_identity(mt: MachineType, plat: Platform) -> NodeIdentity {
        NodeIdentity::new(
            Hostname::new("cp-1.cluster.local").unwrap(),
            NodeAddress::parse_v4("10.0.0.5").unwrap(),
            mt,
            plat,
            Version::new(1, 8, 0),
        )
    }

    #[test]
    fn labels_validate_and_match() {
        let mut l = Labels::new();
        l.insert("node-role.kubernetes.io/control-plane", "")
            .unwrap();
        l.insert("topology.kubernetes.io/zone", "us-east-1a")
            .unwrap();
        assert_eq!(l.len(), 2);
        assert!(l.insert("bad key", "x").is_err());
        // The charset is closed, not merely inclusive: a punctuation mark
        // outside it is refused even though it is neither space nor control.
        assert!(l.insert("a!b", "v").is_err());

        let mut sel = Labels::new();
        sel.insert("topology.kubernetes.io/zone", "us-east-1a")
            .unwrap();
        assert!(l.matches(&sel));

        sel.insert("topology.kubernetes.io/zone", "other").unwrap();
        assert!(!l.matches(&sel));
    }

    #[test]
    fn selector_string_is_sorted() {
        let mut l = Labels::new();
        l.insert("z", "1").unwrap();
        l.insert("a", "2").unwrap();
        assert_eq!(l.to_selector_string(), "a=2,z=1");
    }

    #[test]
    fn node_identity_validation() {
        let good = sample_identity(MachineType::ControlPlane, Platform::Aws);
        assert!(good.validate().is_ok());
        assert!(good.is_control_plane());

        let bad_type = sample_identity(MachineType::Unknown, Platform::Aws);
        assert_eq!(bad_type.validate().unwrap_err().kind(), "invalid_state");

        let bad_plat = sample_identity(MachineType::Worker, Platform::Unknown);
        assert!(bad_plat.validate().is_err());
        assert!(!bad_plat.is_control_plane());
    }

    #[test]
    fn display_name_format() {
        let id = sample_identity(MachineType::ControlPlane, Platform::Metal);
        assert_eq!(
            id.display_name(),
            "cp-1.cluster.local/controlplane/10.0.0.5"
        );
    }
}
