//! Touched-file budget. Provenance: ADR-0719 file-budget decision (D-35).

use crate::layout::{APP_PRODUCT_DIRS, is_capability_root};

const MAX_LINES: usize = 300;
const MAX_COMMENT_RUN: usize = 20;
const OWNER_LAW: &[&str] = &["ADR.md", "PRD.md", "SPEC.md", "PLAN.md"];

/// Count physical newline characters exactly as `wc -l`; the closed exempt
/// set is path-derived and cannot be expanded by file contents.
pub fn file_budget_violations(path: &str, contents: &[u8]) -> Vec<String> {
    if exempt(path) {
        return Vec::new();
    }
    let lines = contents.iter().filter(|byte| **byte == b'\n').count();
    if lines <= MAX_LINES {
        Vec::new()
    } else {
        vec![format!(
            "{path}: {lines} physical lines exceeds the repository {MAX_LINES}-line file budget"
        )]
    }
}

/// Refuse a contiguous run of comment-only lines longer than
/// `MAX_COMMENT_RUN` in a Rust source. A blank line bridges a run instead of
/// breaking it, so a blob cannot be split into two passing halves by adding
/// whitespace; only real code ends a run. Line comments, both doc spellings
/// and block-comment bodies all count, and the exempt set is the same closed,
/// path-derived one the file budget uses: as with that budget, it cannot be
/// expanded by file contents, so no comment can switch this rule off. The
/// remedy for a refusal is to move the argument into the commit message,
/// where it is addressed to a reader who asked for it.
pub fn comment_run_violations(path: &str, contents: &[u8]) -> Vec<String> {
    if exempt(path) || !path.ends_with(".rs") {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(contents);
    let mut violations = Vec::new();
    let mut scanner = CommentScanner::default();
    let mut run: Option<CommentRun> = None;
    for (index, line) in text.lines().enumerate() {
        match scanner.classify(line.trim()) {
            LineKind::Comment => {
                run.get_or_insert(CommentRun {
                    start: index + 1,
                    lines: 0,
                })
                .lines += 1;
            }
            LineKind::Blank => {}
            LineKind::Code => report(&mut violations, path, run.take()),
        }
    }
    report(&mut violations, path, run.take());
    violations
}

struct CommentRun {
    start: usize,
    lines: usize,
}

fn report(violations: &mut Vec<String>, path: &str, run: Option<CommentRun>) {
    if let Some(run) = run
        && run.lines > MAX_COMMENT_RUN
    {
        let (start, lines) = (run.start, run.lines);
        violations.push(format!(
            "{path}:{start}: {lines} consecutive comment lines exceed the \
             {MAX_COMMENT_RUN}-line comment-run ceiling; move the explanation \
             into the commit message"
        ));
    }
}

enum LineKind {
    Comment,
    Blank,
    Code,
}

/// Block-comment nesting carried across lines; Rust nests `/* /* */ */`, so a
/// depth is the only reading that closes the right block.
#[derive(Default)]
struct CommentScanner {
    depth: usize,
}

impl CommentScanner {
    fn classify(&mut self, line: &str) -> LineKind {
        if self.depth == 0 {
            if line.is_empty() {
                return LineKind::Blank;
            }
            if line.starts_with("//") {
                return LineKind::Comment;
            }
            if !line.starts_with("/*") {
                return LineKind::Code;
            }
        }
        let tail = self.advance(line);
        if self.depth == 0 && !tail.trim().is_empty() {
            LineKind::Code
        } else {
            LineKind::Comment
        }
    }

    /// Walk `line`, tracking block-comment depth, and return whatever follows
    /// the outermost close when the block ends on this line.
    fn advance<'line>(&mut self, line: &'line str) -> &'line str {
        let bytes = line.as_bytes();
        let mut index = 0;
        while index + 1 < bytes.len() {
            match (bytes[index], bytes[index + 1]) {
                (b'/', b'*') => {
                    self.depth += 1;
                    index += 2;
                }
                (b'*', b'/') => {
                    self.depth = self.depth.saturating_sub(1);
                    index += 2;
                    if self.depth == 0 {
                        return &line[index..];
                    }
                }
                _ => index += 1,
            }
        }
        ""
    }
}

fn exempt(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let name = parts.last().copied().unwrap_or_default();
    path == "Cargo.lock"
        || path.starts_with("third-party/")
        || matches!(name, "AGENTS.md" | "CLAUDE.md")
        || name.contains(".generated.")
        || vendored_lock_step_snapshot(path)
        || live_apex_adr(path)
        || owner_law(&parts)
}

fn vendored_lock_step_snapshot(path: &str) -> bool {
    const SNAPSHOT_PREFIX: &str = "build/port-engine/adapters/snapshot/src/fixture-snapshot-";
    const PORT_GO_PREFIX: &str = "build/port-engine/facade/app/src/port-go-golden-v";
    path.strip_prefix(SNAPSHOT_PREFIX)
        .and_then(|name| name.strip_suffix(".json"))
        .is_some_and(lowercase_versioned_name)
        || path
            .strip_prefix(PORT_GO_PREFIX)
            .and_then(|version| version.strip_suffix(".txt"))
            .is_some_and(|version| {
                !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit())
            })
}

fn lowercase_versioned_name(name: &str) -> bool {
    let version = name.strip_prefix('v').or_else(|| {
        name.rsplit_once("-v")
            .and_then(|(stem, version)| (!stem.is_empty()).then_some(version))
    });
    !name.is_empty()
        && !name.contains('/')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && version.is_some_and(|version| {
            !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn live_apex_adr(path: &str) -> bool {
    path.strip_prefix("docs/decisions/ADR-07")
        .and_then(|rest| rest.strip_suffix(".md"))
        .is_some_and(|rest| {
            rest.len() > 3
                && rest.as_bytes()[..2].iter().all(u8::is_ascii_digit)
                && rest.as_bytes()[2] == b'-'
        })
}

fn owner_law(parts: &[&str]) -> bool {
    matches!(parts, [owner, name] if is_capability_root(owner) && OWNER_LAW.contains(name))
        || matches!(parts, ["app", product, name]
            if APP_PRODUCT_DIRS.contains(product) && OWNER_LAW.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touched_text_over_three_hundred_lines_is_red() {
        let text = "line\n".repeat(301);
        assert!(!file_budget_violations("network/core/x/src/lib.rs", text.as_bytes()).is_empty());
        let mut non_utf8 = text.into_bytes();
        non_utf8.push(0xff);
        assert!(!file_budget_violations("network/core/x/blob", &non_utf8).is_empty());
    }

    #[test]
    fn closed_exempt_set_is_honored() {
        let text = "line\n".repeat(301);
        for path in [
            "Cargo.lock",
            "third-party/vendor/source.rs",
            "docs/decisions/ADR-0719-example.md",
            "network/ADR.md",
            "app/payroll/SPEC.md",
            "network/observability/slos/a.generated.openslo.yaml",
            "build/port-engine/adapters/snapshot/src/fixture-snapshot-v1.json",
            "build/port-engine/adapters/snapshot/src/fixture-snapshot-interface-v1.json",
            "build/port-engine/facade/app/src/port-go-golden-v1.txt",
        ] {
            assert!(
                file_budget_violations(path, text.as_bytes()).is_empty(),
                "{path}"
            );
        }
    }

    #[test]
    fn exemption_spellings_are_closed() {
        let text = "line\n".repeat(301);
        for path in [
            "base/ADR.md",
            "app/not-a-product/PLAN.md",
            "docs/decisions/ADR-0719evil.md",
            "build/port-engine/adapters/other/src/fixture-snapshot-v1.json",
            "build/port-engine/adapters/snapshot/src/not-a-snapshot-v1.json",
            "build/port-engine/facade/app/src/port-go-golden-vnext.txt",
        ] {
            assert!(
                !file_budget_violations(path, text.as_bytes()).is_empty(),
                "{path}"
            );
        }
    }
}
