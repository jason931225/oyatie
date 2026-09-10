use std::collections::BTreeSet;
use std::fmt;

use semver::Version;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolchainSide {
    Protected,
    Candidate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionToolchainProfile {
    Minimal,
    Default,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionToolchainState {
    execution: ExecutionChannel,
    msrv: Version,
    profile: ExecutionToolchainProfile,
    components: BTreeSet<String>,
    targets: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclarationRefusal {
    MalformedToml(String),
    Missing(&'static str),
    Unknown(String),
    WrongType(&'static str, &'static str),
    Duplicate(&'static str, String),
    InvalidStableVersion(&'static str, String),
    InvalidNightlyDate(&'static str, String),
    FloatingNightlyChannel(&'static str, String),
    UnsupportedValue(&'static str, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionToolchainAnalysisRefusal {
    InvalidToolchain(ToolchainSide, DeclarationRefusal),
    InvalidMsrv(ToolchainSide, DeclarationRefusal),
}

struct ParsedToolchain {
    execution: ExecutionChannel,
    profile: ExecutionToolchainProfile,
    components: BTreeSet<String>,
    targets: BTreeSet<String>,
}

impl ExecutionToolchainState {
    pub fn execution(&self) -> &ExecutionChannel {
        &self.execution
    }

    pub fn msrv(&self) -> &Version {
        &self.msrv
    }

    pub fn profile(&self) -> ExecutionToolchainProfile {
        self.profile
    }

    pub fn components(&self) -> &BTreeSet<String> {
        &self.components
    }

    pub fn targets(&self) -> &BTreeSet<String> {
        &self.targets
    }
}

fn parse_toolchain(source: &str) -> Result<ParsedToolchain, DeclarationRefusal> {
    let mut root = parse_table(source)?;
    let toolchain = root
        .remove("toolchain")
        .ok_or(DeclarationRefusal::Missing("toolchain"))?;
    if let Some(field) = root.keys().min() {
        return Err(DeclarationRefusal::Unknown(field.clone()));
    }
    let mut table = toolchain
        .as_table()
        .cloned()
        .ok_or(DeclarationRefusal::WrongType("toolchain", "table"))?;
    if let Some(field) = table
        .keys()
        .filter(|field| {
            !matches!(
                field.as_str(),
                "channel" | "components" | "profile" | "targets"
            )
        })
        .min()
    {
        return Err(DeclarationRefusal::Unknown(format!("toolchain.{field}")));
    }
    let channel = take_string(&mut table, "channel", "toolchain.channel")?;
    let components = unique_set(
        table
            .remove("components")
            .ok_or(DeclarationRefusal::Missing("toolchain.components"))?,
        "toolchain.components",
    )?;
    let profile = parse_profile(take_string(&mut table, "profile", "toolchain.profile")?)?;
    let targets = table
        .remove("targets")
        .map(|value| unique_set(value, "toolchain.targets"))
        .transpose()?
        .unwrap_or_default();
    Ok(ParsedToolchain {
        execution: parse_execution_channel("toolchain.channel", &channel)?,
        profile,
        components,
        targets,
    })
}

fn parse_msrv(source: &str) -> Result<Version, DeclarationRefusal> {
    let root = parse_table(source)?;
    let workspace = table_field(&root, "workspace")?;
    let package = table_field(workspace, "workspace.package")?;
    let value = package
        .get("rust-version")
        .ok_or(DeclarationRefusal::Missing(
            "workspace.package.rust-version",
        ))?
        .as_str()
        .ok_or(DeclarationRefusal::WrongType(
            "workspace.package.rust-version",
            "string",
        ))?;
    stable_version("workspace.package.rust-version", value)
}

fn parse_table(source: &str) -> Result<toml::Table, DeclarationRefusal> {
    toml::from_str(source).map_err(|error| DeclarationRefusal::MalformedToml(error.to_string()))
}

fn table_field<'a>(
    table: &'a toml::Table,
    field: &'static str,
) -> Result<&'a toml::Table, DeclarationRefusal> {
    let key = field.rsplit('.').next().expect("static field path");
    table
        .get(key)
        .ok_or(DeclarationRefusal::Missing(field))?
        .as_table()
        .ok_or(DeclarationRefusal::WrongType(field, "table"))
}

fn take_string(
    table: &mut toml::Table,
    key: &str,
    field: &'static str,
) -> Result<String, DeclarationRefusal> {
    table
        .remove(key)
        .ok_or(DeclarationRefusal::Missing(field))?
        .as_str()
        .map(str::to_owned)
        .ok_or(DeclarationRefusal::WrongType(field, "string"))
}

fn unique_set(
    value: toml::Value,
    field: &'static str,
) -> Result<BTreeSet<String>, DeclarationRefusal> {
    let values = value
        .as_array()
        .ok_or(DeclarationRefusal::WrongType(field, "array of strings"))?;
    let mut unique = BTreeSet::new();
    for value in values {
        let value = value.as_str().filter(|value| !value.is_empty()).ok_or(
            DeclarationRefusal::WrongType(field, "array of non-empty strings"),
        )?;
        if !unique.insert(value.to_owned()) {
            return Err(DeclarationRefusal::Duplicate(field, value.to_owned()));
        }
    }
    Ok(unique)
}

fn parse_profile(value: String) -> Result<ExecutionToolchainProfile, DeclarationRefusal> {
    match value.as_str() {
        "minimal" => Ok(ExecutionToolchainProfile::Minimal),
        "default" => Ok(ExecutionToolchainProfile::Default),
        "complete" => Ok(ExecutionToolchainProfile::Complete),
        _ => Err(DeclarationRefusal::UnsupportedValue(
            "toolchain.profile",
            value,
        )),
    }
}

fn stable_version(field: &'static str, value: &str) -> Result<Version, DeclarationRefusal> {
    let invalid = || DeclarationRefusal::InvalidStableVersion(field, value.to_owned());
    let version = Version::parse(value).map_err(|_| invalid())?;
    if version.pre.is_empty() && version.build.is_empty() && version.to_string() == value {
        Ok(version)
    } else {
        Err(invalid())
    }
}
