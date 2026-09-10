//! Refusal of a deletion that something surviving still points at.
//!
//! An owner migration deletes its prose in one candidate. That is only safe if
//! nothing left standing still names what went away, so this decides whether a
//! proposed deletion leaves a dangling pointer behind.
//!
//! The judgment that matters is what counts as naming a path, and it is not a
//! single-byte test at each end. `.` and `/` continue a path in
//! `docs/a.json.bak` and `vendor/docs/a.json`, and end one in `See docs/a.md.`
//! and `./docs/a.md` — the same byte, opposite answers. Treating either as
//! always-continuing makes the gate fail OPEN on the most ordinary way prose
//! names a file: at the end of a sentence.

use std::collections::BTreeSet;

/// A surviving file still naming a path the candidate deletes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RetainedReference {
    /// The surviving file that carries the mention.
    pub referrer: String,
    /// The deleted path it names.
    pub deleted: String,
    /// 1-indexed line of the first mention.
    pub line: usize,
}

impl RetainedReference {
    pub fn reason(&self) -> String {
        let Self {
            referrer,
            deleted,
            line,
        } = self;
        format!("{referrer}:{line} still references deleted `{deleted}`")
    }
}

/// Why a candidate may not proceed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The deletion set held a blank path. What it names cannot be decided,
    /// and a check that cannot reach its subject reports that rather than
    /// passing.
    UndecidablePath,
    /// Surviving files still name deleted paths.
    Retained(Vec<RetainedReference>),
}

impl Refusal {
    pub fn reason(&self) -> String {
        match self {
            Self::UndecidablePath => "deletion set contains a blank path".to_owned(),
            Self::Retained(found) => found
                .iter()
                .map(RetainedReference::reason)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// A byte that can occur inside a path.
fn path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/')
}

/// A byte that can begin a path segment, so a `/` before it joins two parts of
/// one name rather than ending a sentence or opening a root-relative one.
fn segment_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

/// Whether the match is the tail of a longer name.
///
/// `.` and `/` extend the name only when something namelike follows them:
/// `docs/a.json.bak` and `docs/a/nested` continue, while the period closing
/// `See docs/a.md.` is punctuation and the path ended before it.
fn continues_right(bytes: &[u8], at: usize) -> bool {
    match bytes.get(at).copied() {
        None => false,
        Some(b'.' | b'/') => bytes.get(at + 1).copied().is_some_and(path_byte),
        Some(byte) => path_byte(byte),
    }
}

/// Whether the match is the head of a longer name.
///
/// A `/` before it separates a parent directory only when something namelike
/// precedes the slash. `./docs/a.md` and `/docs/a.md` name `docs/a.md`
/// itself; `vendor/docs/a.md` does not.
fn continues_left(bytes: &[u8], start: usize) -> bool {
    let Some(before) = start.checked_sub(1).map(|index| bytes[index]) else {
        return false;
    };
    if before != b'/' {
        return path_byte(before);
    }
    start
        .checked_sub(2)
        .map(|index| bytes[index])
        .is_some_and(segment_byte)
}

/// Whether `line` names `path` rather than merely containing its characters.
fn names_path(line: &str, path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    let bytes = line.as_bytes();
    line.match_indices(path).any(|(start, matched)| {
        !continues_left(bytes, start) && !continues_right(bytes, start + matched.len())
    })
}

/// Every surviving file that still names a deleted path, first mention only,
/// in a deterministic order.
///
/// A referrer that is itself being deleted is not retained, so prose that
/// points at its own neighbours does not block the candidate that removes both.
///
/// ponytail: matches a full path, not a bare basename, and cannot tell a URL
/// under another host from a local path. Both need an ambiguity rule for names
/// that repeat, worth adding when a real migration produces one.
pub fn retained_references<'a, I>(deleted: &BTreeSet<String>, live: I) -> Vec<RetainedReference>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut found = Vec::new();
    for (referrer, contents) in live {
        if deleted.contains(referrer) {
            continue;
        }
        for path in deleted {
            let mention = contents
                .lines()
                .enumerate()
                .find(|(_, line)| names_path(line, path));
            if let Some((index, _)) = mention {
                found.push(RetainedReference {
                    referrer: referrer.to_owned(),
                    deleted: path.clone(),
                    line: index + 1,
                });
            }
        }
    }
    found.sort();
    found
}

/// Whether the candidate may proceed.
///
/// # Errors
/// [`Refusal::UndecidablePath`] when the deletion set is not answerable, and
/// [`Refusal::Retained`] carrying every retained reference, so an operator
/// repairs the whole set rather than rediscovering them one deletion at a time.
pub fn refuse_retained<'a, I>(deleted: &BTreeSet<String>, live: I) -> Result<(), Refusal>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    if deleted.iter().any(|path| path.trim().is_empty()) {
        return Err(Refusal::UndecidablePath);
    }
    let found = retained_references(deleted, live);
    if found.is_empty() {
        return Ok(());
    }
    Err(Refusal::Retained(found))
}
