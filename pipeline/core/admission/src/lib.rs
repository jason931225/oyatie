//! Admission primitives for the protected `presubmit` graph.

pub mod cadence;
pub mod commit_range;
pub mod fanin;
pub mod git_change;
pub mod layout;
pub mod line_budget;
mod live_postgres;
pub mod occupancy;
pub mod owners;
pub mod signing_authority;

pub use cadence::{
    CadenceEvent, LIVE_POSTGRES_CRATES, LIVE_POSTGRES_JOBS, POSTSUBMIT_JOBS, PRESUBMIT_JOBS,
    PresubmitChangeGates, REINDEER_QUALIFICATION_PATH_PREFIXES, WORKFLOW_FILES,
    backbone_postgres_required, compute_lifecycle_postgres_required,
    hits_reindeer_qualification_path, live_postgres_required, presubmit_change_gates,
    reindeer_qualification_exact_paths, reindeer_source_qualification_required,
};
pub use commit_range::{CommitFact, SignatureState, signing_violations};
pub use fanin::{
    FanIn, fan_in_ok, gate_value, live_postgres_cells_ok, occupancy_ok, postgres_ok, postsubmit_ok,
    reindeer_qualification_ok, required_success,
};
pub use git_change::{
    GitChangePaths, PathSetParseError, git_change_paths_from_name_status_z,
    paths_from_name_status_z,
};
pub(crate) use layout::path_parts;
pub use layout::{
    ALLOWED_DOT_ROOT_DIRS, ALLOWED_ROOT_DIRS, ALLOWED_ROOT_FILES, APP_PRODUCT_DIRS,
    BUILD_ROOT_DIRS, CAP_EXTRAS, CARGO_CONFIG_PATHS, DATA_ROOTS, FACES, FORBIDDEN_NAMES,
    META_ROOTS, WORKSPACE_EXCLUDES, WORKSPACE_MEMBER_GLOBS, base_admission_violations,
    cap_root_file_ok, cargo_config_violations, cargo_entrypoint, cargo_entrypoints,
    cargo_manifest_for_crate_path, cargo_manifest_for_entrypoint, cargo_manifest_violations,
    changed_layout_violations, draft_dependency_violations, face_dir_ok, is_capability_root,
    layout_violations, owner_core_regression_violations, proto_package_violations,
    workspace_draft_dependency_violations, workspace_membership_violations,
};
pub use line_budget::{comment_run_violations, file_budget_violations};
pub use live_postgres::{
    BACKBONE_LIVE_POSTGRES_PATH_PREFIXES, COMPUTE_LIFECYCLE_LIVE_POSTGRES_PATH_PREFIXES,
    LIVE_POSTGRES_SELECTOR_PATH_PREFIXES, hits_backbone_postgres_path,
    hits_compute_lifecycle_postgres_path, live_postgres_exact_paths,
};
pub use occupancy::{
    OccupancyRefused, OccupiedSet, admit, admit_authored, authored_paths, declared_mergeable,
};
pub use owners::{ROOT_OCCUPANT, owners_occupant};
pub use signing_authority::{SIGNING_AUTHORITY, SigningPrincipal, allowed_signers};
