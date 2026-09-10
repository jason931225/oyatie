//! `rust-toolchain.toml` is the sole authority for the execution channel;
//! every toolchain-naming workflow input is checked against it.

/// Inputs by which a hosted action is told which Rust toolchain to use.
pub const TOOLCHAIN_PIN_KEYS: [&str; 2] = ["toolchain:", "rust-version:"];

/// One toolchain-naming input found in one hosted workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolchainPin {
    pub workflow: String,
    pub line: usize,
    pub key: &'static str,
    pub job: String,
    pub value: String,
}

impl ToolchainPin {
    /// `rustup default` is last-write-wins, so only the final `toolchain:` of
    /// a job selects that job's compiler.
    fn can_be_shadowed(&self) -> bool {
        self.key == "toolchain:"
    }
}

/// The channel declared by `rust-toolchain.toml`.
pub fn declared_channel(declaration: &str) -> Result<String, String> {
    let parsed = declaration
        .parse::<toml::Value>()
        .map_err(|error| format!("rust-toolchain.toml is not valid TOML: {error}"))?;
    parsed
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("channel"))
        .and_then(toml::Value::as_str)
        .filter(|channel| !channel.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "rust-toolchain.toml declares no toolchain.channel".to_owned())
}

fn job_named_by(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with([' ', '#', '-']) {
        return None;
    }
    let name = rest.strip_suffix(':')?;
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    Some(name)
}

fn pin_value(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    for quote in ['"', '\''] {
        if let Some(tail) = rest.strip_prefix(quote) {
            return tail.split(quote).next().map(str::to_owned);
        }
    }
    let end = rest.find([',', '}', '#']).unwrap_or(rest.len());
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// Every toolchain-naming input in one workflow, in file order.
pub fn workflow_toolchain_pins(workflow: &str, contents: &str) -> Vec<ToolchainPin> {
    let mut pins = Vec::new();
    let mut job = String::new();
    let mut reached_jobs = false;
    for (index, line) in contents.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line == "jobs:" {
            reached_jobs = true;
            continue;
        }
        if reached_jobs && let Some(name) = job_named_by(line) {
            job = name.to_owned();
        }
        for key in TOOLCHAIN_PIN_KEYS {
            let Some(at) = line.find(key) else {
                continue;
            };
            let Some(value) = pin_value(&line[at + key.len()..]) else {
                continue;
            };
            pins.push(ToolchainPin {
                workflow: workflow.to_owned(),
                line: index + 1,
                key,
                job: job.clone(),
                value,
            });
        }
    }
    pins
}

fn supersedes(later: &ToolchainPin, earlier: &ToolchainPin) -> bool {
    later.key == earlier.key && later.workflow == earlier.workflow && later.job == earlier.job
}

/// Pins that disagree with the declared channel, named by file and line. A
/// shadowed install belongs to whichever foreign source that job qualifies,
/// so it is not judged here.
pub fn execution_channel_violations(channel: &str, pins: &[ToolchainPin]) -> Vec<String> {
    let mut violations = Vec::new();
    for (index, pin) in pins.iter().enumerate() {
        let shadowed =
            pin.can_be_shadowed() && pins[index + 1..].iter().any(|later| supersedes(later, pin));
        if shadowed || pin.value == channel {
            continue;
        }
        violations.push(format!(
            "{}:{}: job `{}` pins {} {:?}; rust-toolchain.toml declares the channel {:?}",
            pin.workflow, pin.line, pin.job, pin.key, pin.value, channel
        ));
    }
    violations
}
