//! Machine type primitive, mirroring Talos `machine.Type`.

use crate::error::{Error, Result};
use alloc::string::String;
use core::fmt;

/// The role a node plays in a cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MachineType {
    /// Legacy bootstrap control plane, kept only for compatibility.
    Init,
    ControlPlane,
    Worker,
    Unknown,
}

impl MachineType {
    /// Fixed by the Talos protobuf enum ordering; renumbering breaks the wire.
    pub fn as_i32(self) -> i32 {
        match self {
            MachineType::Unknown => 0,
            MachineType::Init => 1,
            MachineType::ControlPlane => 2,
            MachineType::Worker => 3,
        }
    }

    pub fn from_i32(v: i32) -> Result<Self> {
        match v {
            0 => Ok(MachineType::Unknown),
            1 => Ok(MachineType::Init),
            2 => Ok(MachineType::ControlPlane),
            3 => Ok(MachineType::Worker),
            other => Err(Error::parse(alloc::format!("unknown machine type {other}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MachineType::Init => "init",
            MachineType::ControlPlane => "controlplane",
            MachineType::Worker => "worker",
            MachineType::Unknown => "unknown",
        }
    }

    pub fn is_control_plane(self) -> bool {
        matches!(self, MachineType::Init | MachineType::ControlPlane)
    }
}

impl fmt::Display for MachineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for MachineType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        // Where an arm carries two spellings the second is a historical Talos
        // config alias. Blank input is not an alias but a default to `Unknown`,
        // which `NodeIdentity::validate` then refuses.
        let normalized: String = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "init" => Ok(MachineType::Init),
            "controlplane" | "control-plane" => Ok(MachineType::ControlPlane),
            "worker" | "join" => Ok(MachineType::Worker),
            "unknown" | "" => Ok(MachineType::Unknown),
            other => Err(Error::parse(alloc::format!(
                "invalid machine type '{other}'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn roundtrip_i32() {
        for t in [
            MachineType::Unknown,
            MachineType::Init,
            MachineType::ControlPlane,
            MachineType::Worker,
        ] {
            assert_eq!(MachineType::from_i32(t.as_i32()).unwrap(), t);
        }
        assert!(MachineType::from_i32(99).is_err());
    }

    #[test]
    fn parses_aliases() {
        assert_eq!(
            MachineType::from_str("controlplane").unwrap(),
            MachineType::ControlPlane
        );
        assert_eq!(
            MachineType::from_str("control-plane").unwrap(),
            MachineType::ControlPlane
        );
        assert_eq!(MachineType::from_str("JOIN").unwrap(), MachineType::Worker);
        assert_eq!(
            MachineType::from_str("  worker ").unwrap(),
            MachineType::Worker
        );
        assert!(MachineType::from_str("nonsense").is_err());
    }

    #[test]
    fn blank_input_defaults_to_unknown() {
        assert_eq!(MachineType::from_str("").unwrap(), MachineType::Unknown);
        assert_eq!(MachineType::from_str("   ").unwrap(), MachineType::Unknown);
    }

    #[test]
    fn control_plane_membership() {
        assert!(MachineType::Init.is_control_plane());
        assert!(MachineType::ControlPlane.is_control_plane());
        assert!(!MachineType::Worker.is_control_plane());
        assert!(!MachineType::Unknown.is_control_plane());
    }
}
