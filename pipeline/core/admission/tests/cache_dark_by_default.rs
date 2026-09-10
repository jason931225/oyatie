//! The root build config never selects the warm cache, and the licence cannot exist.
//!
//! `build/toolchains/cache/defs.bzl` states the invariant and names a
//! conformance gate at `ci/facade/build-cache-policy` that asserts it. That
//! path does not exist -- retired with the canary by ADR-0716 D3 -- so the
//! invariant has been a comment since.
//!
//! That same paragraph, and `build/toolchains/cache/BUCK`, name overlays at
//! `infra/ci/buckconfig/warm-cache-{rw,ro}.buckconfig` as the platform's only
//! selector. Those files were never created and `infra/` is not a root here.
//! Both sites are byte-mirrored into the port-engine toolchain corpus, so this
//! is the only admissible home for the correction.
//!
//! 1. The root `.buckconfig` selects the prelude platform and carries no
//!    `[cache]` section. That is all these tests read; `.buckconfig.d/` is
//!    closed off by `ALLOWED_DOT_ROOT_DIRS`, a different gate.
//! 2. `specs/cache-warm-license.json` does not exist and cannot: ADR-0716 D2
//!    and the g004 mapping name that path, and `specs` is in FORBIDDEN_NAMES.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn the_root_build_config_does_not_select_the_cache_platform() {
    let text = std::fs::read_to_string(repo_root().join(".buckconfig"))
        .expect(".buckconfig must exist at the repository root");
    // Every occurrence, not the first. Which of two `execution_platforms`
    // lines a parser keeps is its business; requiring exactly one leaves this
    // test no reason to care.
    let selected: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix("execution_platforms"))
        .map(|rest| rest.trim_start_matches([' ', '=']).trim().to_string())
        .collect();

    assert_eq!(
        selected,
        ["prelude//platforms:default"],
        "the root config must declare `execution_platforms` exactly once, at \
         the prelude platform. Selecting toolchains//cache:cache-platform \
         here -- or adding a second line a parser may prefer -- would put \
         every ordinary build on the warm cache, which only the opt-in CI \
         lane may do."
    );
}

#[test]
fn the_root_build_config_carries_no_cache_knobs() {
    let text = std::fs::read_to_string(repo_root().join(".buckconfig"))
        .expect(".buckconfig must exist at the repository root");
    // `cache_execution_platform` reads these with `read_root_config`, so they
    // are only ever set by the lane that opts in. In the root config they
    // would arm the cache for everyone.
    for knob in ["remote_cache_enabled", "allow_cache_uploads"] {
        assert!(
            !text.contains(knob),
            "the root .buckconfig must not set `{knob}`; the cache knobs belong \
             to the opt-in lane, and setting them here is the bypass the \
             dark-by-default invariant exists to prevent"
        );
    }
    assert!(
        !text.contains("[cache]"),
        "the root .buckconfig must carry no [cache] section"
    );
}

#[test]
fn the_admission_control_has_no_admissible_home() {
    // Not a test of the licence -- a test of the contradiction that stops it
    // existing. ADR-0716 D2 and the g004 mapping name
    // `specs/cache-warm-license.json` (AGENTS.md:209 names only the
    // `warm_reads_licensed` key), and `specs` is in FORBIDDEN_NAMES, so
    // the repository refuses the path its own ADR specifies. Restoring the
    // file was tried and the layout gate rejected it: "forbidden root `specs`".
    //
    // This asserts the conflict is still live, so that whoever resolves it
    // finds a failing test rather than a stale comment. When `specs` stops
    // being forbidden, or the ADR names an admissible path, this fails and
    // says so.
    assert!(
        pipeline_admission::FORBIDDEN_NAMES.contains(&"specs"),
        "`specs` is no longer forbidden, so the path ADR-0716 D2 names for the \
         warm-read licence is now admissible. Restore \
         specs/cache-warm-license.json at warm_reads_licensed: false and make \
         the consumers read it."
    );
    assert!(
        !repo_root().join("specs/cache-warm-license.json").exists(),
        "the licence exists at a path the layout gate forbids; one of the two \
         must give and it should be decided rather than discovered"
    );
}
