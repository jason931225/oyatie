//! `rust-toolchain.toml` is the sole authority for the execution channel;
//! every toolchain-naming workflow input is checked against it.

/// Inputs by which a hosted action is told which Rust toolchain to use.
///
/// `RUSTUP_TOOLCHAIN` outranks the DECLARATION: rustup reads it before it
/// looks for a `rust-toolchain.toml` in the cwd ancestry, so a job setting it
/// in `env:` overrides the channel this gate exists to enforce. It is spelled
/// in upper case and `str::find` is case-sensitive, so it matched neither of
/// the other keys and was invisible here.
///
/// It is NOT the highest-precedence mechanism, and this scanner is not a
/// complete account of toolchain selection. An explicit `+<channel>` argument
/// beats it -- measured on the build host, `RUSTUP_TOOLCHAIN=1.98.0 rustc
/// +1.97.1` resolves to 1.97.1 -- and a `cargo +<channel>` inside a `run:`
/// body is not read here at all.
pub const TOOLCHAIN_PIN_KEYS: [&str; 3] = ["toolchain:", "rust-version:", "RUSTUP_TOOLCHAIN:"];

/// One toolchain-naming input found in one hosted workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolchainPin {
    pub workflow: String,
    pub line: usize,
    pub key: &'static str,
    pub job: String,
    pub step: usize,
    /// `None` when the key is named but its value is not on this line.
    pub value: Option<String>,
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

/// YAML structure only; values keep their `#` because a channel may not.
fn without_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line).trim_end()
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
    let mut step = 0;
    for (index, line) in contents.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let structure = without_comment(line);
        if structure == "jobs:" {
            reached_jobs = true;
            continue;
        }
        if reached_jobs && let Some(name) = job_named_by(structure) {
            job = name.to_owned();
        }
        let item = structure.trim_start();
        if item == "-" || item.starts_with("- ") {
            step += 1;
        }
        for key in TOOLCHAIN_PIN_KEYS {
            // Searched on the comment-stripped line. Against the raw line, a
            // key named inside a trailing comment became a pin whose value
            // parsed to whatever followed it, and the gate refused a workflow
            // for a sentence about a toolchain.
            let Some(at) = structure.find(key) else {
                continue;
            };
            pins.push(ToolchainPin {
                workflow: workflow.to_owned(),
                line: index + 1,
                key,
                job: job.clone(),
                step,
                value: pin_value(&structure[at + key.len()..]),
            });
        }
    }
    pins
}

/// Rust sources that spell the declared channel as a literal, by file and
/// line. A literal equal to the live channel is a second place the channel is
/// named, so it goes silently wrong at the next bump; a literal that differs
/// is an oracle for version comparison and is never matched here.
pub fn channel_literal_violations(channel: &str, path: &str, contents: &str) -> Vec<String> {
    // Both quote spellings. Matching only the double-quoted form left a
    // one-character bypass: TOML treats `'1.98.0'` as the same string, and the
    // tree already restates the channel that way -- so the rule read clean over
    // files that name the channel, which is the precise failure it exists to
    // prevent.
    let spellings = [format!("\"{channel}\""), format!("'{channel}'")];
    contents
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let unescaped = line.replace('\\', "");
            spellings
                .iter()
                .any(|spelling| unescaped.contains(spelling))
        })
        .map(|(index, _)| {
            format!(
                "{path}:{}: spells the declared channel {channel:?} as a literal; \
                 derive it from rust-toolchain.toml or choose a value that differs",
                index + 1
            )
        })
        .collect()
}

/// Only an install in the very next step replaces this one unused. Any step
/// in between may have run cargo on the earlier compiler.
fn supersedes(later: &ToolchainPin, earlier: &ToolchainPin) -> bool {
    // An unattributed pin is never shadowed. `step` is file-global and job
    // attribution needs the exact two-space `  name:` shape, so legal YAML this
    // scanner cannot parse -- four-space indentation, or a quoted `"jobs":` --
    // gives every pin the same empty job and consecutive step numbers across a
    // job boundary. Two installs in different jobs would then shadow each
    // other, and a drifted pin would be admitted as replaced.
    !earlier.job.is_empty()
        && later.key == earlier.key
        && later.workflow == earlier.workflow
        && later.job == earlier.job
        && later.step == earlier.step + 1
}

/// Pins that disagree with the declared channel, named by file and line. A
/// shadowed install belongs to whichever foreign source that job qualifies,
/// so it is not judged here.
pub fn execution_channel_violations(channel: &str, pins: &[ToolchainPin]) -> Vec<String> {
    let mut violations = Vec::new();
    for (index, pin) in pins.iter().enumerate() {
        let Some(value) = pin.value.as_deref() else {
            violations.push(format!(
                "{}:{}: job `{}` names {} with no value this scanner can read; a pin it \
                 cannot resolve is not a pin it may ignore",
                pin.workflow, pin.line, pin.job, pin.key
            ));
            continue;
        };
        let shadowed =
            pin.can_be_shadowed() && pins[index + 1..].iter().any(|later| supersedes(later, pin));
        if shadowed || value == channel {
            continue;
        }
        violations.push(format!(
            "{}:{}: job `{}` pins {} {:?}; rust-toolchain.toml declares the channel {:?}",
            pin.workflow, pin.line, pin.job, pin.key, value, channel
        ));
    }
    violations
}

#[cfg(test)]
mod channel_literal_tests;
#[cfg(test)]
mod live_tree_tests;
#[cfg(test)]
mod workflow_pin_tests;
