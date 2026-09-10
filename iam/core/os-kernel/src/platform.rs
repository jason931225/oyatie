//! Supported install platforms, mirroring Talos `pkg/machinery` platform names.

use crate::error::{Error, Result};
use core::fmt;

/// Where the node runs, which decides how it discovers its config and network
/// metadata at boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Metal,
    Aws,
    Gcp,
    Azure,
    Qemu,
    VMware,
    Container,
    Unknown,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Metal => "metal",
            Platform::Aws => "aws",
            Platform::Gcp => "gcp",
            Platform::Azure => "azure",
            Platform::Qemu => "qemu",
            Platform::VMware => "vmware",
            Platform::Container => "container",
            Platform::Unknown => "unknown",
        }
    }

    /// Where an arm carries two spellings the second is an accepted alias.
    /// Blank input is not an alias but a default to `Unknown`, which
    /// `NodeIdentity::validate` then refuses.
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "metal" | "bare-metal" => Ok(Platform::Metal),
            "aws" => Ok(Platform::Aws),
            "gcp" | "gce" => Ok(Platform::Gcp),
            "azure" => Ok(Platform::Azure),
            "qemu" | "kvm" => Ok(Platform::Qemu),
            "vmware" => Ok(Platform::VMware),
            "container" | "docker" => Ok(Platform::Container),
            "unknown" | "" => Ok(Platform::Unknown),
            other => Err(Error::parse(alloc::format!("unknown platform '{other}'"))),
        }
    }

    /// Cloud means: config is discoverable from an instance metadata service.
    pub fn is_cloud(self) -> bool {
        matches!(self, Platform::Aws | Platform::Gcp | Platform::Azure)
    }

    /// Config arrives on a block device or ISO instead of a metadata service.
    ///
    /// `Unknown` is in neither set: an unidentified platform has no known
    /// config source, and `NodeIdentity::validate` refuses it outright. The
    /// match is exhaustive so a new variant must be classified here.
    pub fn uses_local_config(self) -> bool {
        match self {
            Platform::Metal | Platform::Qemu | Platform::VMware | Platform::Container => true,
            Platform::Aws | Platform::Gcp | Platform::Azure | Platform::Unknown => false,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Platform {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Platform::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_and_aliases() {
        assert_eq!(Platform::parse("aws").unwrap(), Platform::Aws);
        assert_eq!(Platform::parse("GCE").unwrap(), Platform::Gcp);
        assert_eq!(Platform::parse("bare-metal").unwrap(), Platform::Metal);
        assert_eq!(Platform::parse("docker").unwrap(), Platform::Container);
        assert!(Platform::parse("heroku").is_err());
    }

    #[test]
    fn blank_input_defaults_to_unknown() {
        assert_eq!(Platform::parse("").unwrap(), Platform::Unknown);
        assert_eq!(Platform::parse("   ").unwrap(), Platform::Unknown);
    }

    #[test]
    fn every_variant_but_unknown_is_cloud_or_local_and_never_both() {
        for platform in [
            Platform::Metal,
            Platform::Aws,
            Platform::Gcp,
            Platform::Azure,
            Platform::Qemu,
            Platform::VMware,
            Platform::Container,
        ] {
            assert_ne!(
                platform.is_cloud(),
                platform.uses_local_config(),
                "{platform}"
            );
        }
        assert!(!Platform::Unknown.is_cloud());
        assert!(!Platform::Unknown.uses_local_config());
    }

    #[test]
    fn cloud_classification() {
        assert!(Platform::Aws.is_cloud());
        assert!(Platform::Azure.is_cloud());
        assert!(!Platform::Metal.is_cloud());
        assert!(!Platform::Qemu.is_cloud());
    }

    #[test]
    fn local_config_classification() {
        assert!(Platform::Metal.uses_local_config());
        assert!(!Platform::Aws.uses_local_config());
        assert_eq!(Platform::VMware.to_string(), "vmware");
    }
}
