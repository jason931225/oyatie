//! The root build config never selects the warm cache, and the licence exists.
//!
//! `build/toolchains/cache/defs.bzl` states the invariant and names a
//! conformance gate at `ci/facade/build-cache-policy` that asserts it. That
//! path does not exist -- retired with the canary by ADR-0716 D3 -- so the
//! invariant has been a comment since. A stated invariant with no detector is
//! the shape this repository keeps finding: it reports protection while
//! measuring nothing.
//!
//! Two facts, both cheap and both currently true:
//!
//! 1. The root `.buckconfig` selects the prelude platform and carries no
//!    `[cache]` section, so an ordinary build cannot reach the cache however
//!    the substrate is configured.
//! 2. `specs/cache-warm-license.json` exists and parses, because ADR-0716 D2,
//!    AGENTS.md and the g004 mapping all name it as the admission control and
//!    it was absent from the tree until it was restored.

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
    let selected = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.strip_prefix("execution_platforms"))
        .map(|rest| rest.trim_start_matches([' ', '=']).trim().to_string())
        .expect(".buckconfig must declare [build] execution_platforms");

    assert_eq!(
        selected, "prelude//platforms:default",
        "the root config must keep the prelude platform. Selecting \
         toolchains//cache:cache-platform here would put every ordinary build \
         on the warm cache, which only the opt-in CI lane may do."
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
fn the_warm_read_admission_control_exists_and_is_readable() {
    // Named by ADR-0716 D2, AGENTS.md and the g004 enforcement-debt mapping,
    // and absent from the tree until restored. A control three documents cite
    // and no file implements is worse than no control: readers believe they
    // are covered by it.
    let path = repo_root().join("specs/cache-warm-license.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} must exist: {error}", path.display());
    });
    // serde_yaml, not a new dependency: YAML is a superset of JSON, and this
    // crate already carries serde_yaml for the workflow tests.
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&text).expect("the licence must be readable JSON");
    assert!(
        parsed
            .get("warm_reads_licensed")
            .and_then(serde_yaml::Value::as_bool)
            .is_some(),
        "the licence must declare a boolean `warm_reads_licensed`; a consumer \
         that cannot read it cannot honour it. Got: {parsed:?}"
    );
}
