// ADR-0083 Tier 3 exemption: integration tests assert with unwrap/expect/unwrap_err.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use iac_domain::{
    CellDefinition, CellIsolationTier, CellTopologyPlan, IacPlanDiffReport, IacPlanDiffVerdict,
    OpenTofuModuleRef, PlanAction, PlanDiffEntry, compute_iac_plan_diff,
};

fn make_ref(name: &str, version: &str) -> OpenTofuModuleRef {
    // OpenTofuModuleRef has no public constructor; mint one via OpenTofuModuleRelease.
    use iac_domain::OpenTofuModuleRelease;
    let source = format!(
        "git::https://git.oyatie.internal/oyatie/oyatie.git//modules/{name}?ref=v{version}"
    );
    let digest = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let evidence = "evidence://iac/test/ref";
    OpenTofuModuleRelease::new(
        "oyatie", name, "opentofu", version, source, digest, evidence,
    )
    .expect("valid test module release")
    .module_ref()
}

fn cell(cell_id: &str, module_refs: Vec<OpenTofuModuleRef>) -> CellDefinition {
    CellDefinition::new(
        "ten_oyatie",
        "us-east-1",
        cell_id,
        CellIsolationTier::Foundation,
        module_refs,
        false,
    )
    .expect("valid test cell")
}

fn topology(topology_id: &str, cells: Vec<CellDefinition>) -> CellTopologyPlan {
    let mut plan = CellTopologyPlan::new(topology_id, "us-east-1", "evidence://iac/test/topology")
        .expect("valid topology");
    for c in cells {
        plan = plan.add_cell(c).expect("add cell");
    }
    plan
}

#[test]
fn all_converged() {
    let r = make_ref("tenant-namespace", "1.0.0");
    let desired = topology("topo-1", vec![cell("cell-us-east", vec![r.clone()])]);
    let observed = topology("topo-1", vec![cell("cell-us-east", vec![r.clone()])]);

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::Converged);
    assert!(
        report
            .entries
            .iter()
            .all(|e| e.action == PlanAction::NoChange)
    );
}

#[test]
fn desired_only_module() {
    let r = make_ref("tenant-namespace", "1.0.0");
    let desired = topology("topo-1", vec![cell("cell-us-east", vec![r.clone()])]);
    // Observed needs its own ref: CellDefinition::new rejects an empty module_refs.
    let r_obs = make_ref("storage-bucket", "1.0.0");
    let observed = topology("topo-1", vec![cell("cell-us-east", vec![r_obs.clone()])]);

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::HasChanges);
    let creates: Vec<_> = report
        .entries
        .iter()
        .filter(|e| e.action == PlanAction::Create)
        .collect();
    assert_eq!(creates.len(), 1);
    assert_eq!(creates[0].module_ref, r);
    assert_eq!(creates[0].cell_id, "cell-us-east");
}

#[test]
fn observed_only_module() {
    let r_des = make_ref("tenant-namespace", "1.0.0");
    let r_obs_extra = make_ref("storage-bucket", "1.0.0");
    let desired = topology("topo-1", vec![cell("cell-us-east", vec![r_des.clone()])]);
    let observed = topology(
        "topo-1",
        vec![cell(
            "cell-us-east",
            vec![r_des.clone(), r_obs_extra.clone()],
        )],
    );

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::HasChanges);
    let destroys: Vec<_> = report
        .entries
        .iter()
        .filter(|e| e.action == PlanAction::Destroy)
        .collect();
    assert_eq!(destroys.len(), 1);
    assert_eq!(destroys[0].module_ref, r_obs_extra);
}

#[test]
fn version_update() {
    let r_v1 = make_ref("tenant-namespace", "1.0.0");
    let r_v2 = make_ref("tenant-namespace", "2.0.0");
    let desired = topology("topo-1", vec![cell("cell-us-east", vec![r_v2.clone()])]);
    let observed = topology("topo-1", vec![cell("cell-us-east", vec![r_v1.clone()])]);

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::HasChanges);
    let updates: Vec<_> = report
        .entries
        .iter()
        .filter(|e| e.action == PlanAction::Update)
        .collect();
    assert_eq!(updates.len(), 1);
    assert_eq!(
        updates[0].module_ref, r_v2,
        "Update entry carries desired ref"
    );
}

#[test]
fn identity_mismatch_topology_id() {
    let r = make_ref("tenant-namespace", "1.0.0");
    let desired = topology("topo-alpha", vec![cell("cell-us-east", vec![r.clone()])]);
    let observed = topology("topo-beta", vec![cell("cell-us-east", vec![r.clone()])]);

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::IdentityMismatch);
    assert!(report.entries.is_empty());
}

#[test]
fn identity_mismatch_region() {
    // Each cell must match its own topology's region; the mismatch is at topology level.
    let r = make_ref("tenant-namespace", "1.0.0");
    let desired = {
        let desired_cell = CellDefinition::new(
            "ten_oyatie",
            "us-east-1",
            "cell-us-east",
            CellIsolationTier::Foundation,
            vec![r.clone()],
            false,
        )
        .expect("valid desired cell");
        let mut plan = CellTopologyPlan::new("topo-1", "us-east-1", "evidence://iac/test/topology")
            .expect("valid topology");
        plan = plan.add_cell(desired_cell).expect("add cell");
        plan
    };
    let observed = {
        let observed_cell = CellDefinition::new(
            "ten_oyatie",
            "eu-west-1",
            "cell-eu-west",
            CellIsolationTier::Foundation,
            vec![r.clone()],
            false,
        )
        .expect("valid observed cell");
        let mut plan = CellTopologyPlan::new("topo-1", "eu-west-1", "evidence://iac/test/topology")
            .expect("valid topology");
        plan = plan.add_cell(observed_cell).expect("add cell");
        plan
    };

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::IdentityMismatch);
    assert!(report.entries.is_empty());
}

#[test]
fn identity_mismatch_cell_tenant_id() {
    let r = make_ref("tenant-namespace", "1.0.0");
    // cell_id format requires "cell-<suffix>" where suffix must contain a hyphen.
    let desired_cell = CellDefinition::new(
        "ten_desired",
        "us-east-1",
        "cell-shared-01",
        CellIsolationTier::Foundation,
        vec![r.clone()],
        false,
    )
    .expect("valid desired cell");
    let observed_cell = CellDefinition::new(
        "ten_observed",
        "us-east-1",
        "cell-shared-01",
        CellIsolationTier::Foundation,
        vec![r.clone()],
        false,
    )
    .expect("valid observed cell");

    let desired = {
        let mut plan = CellTopologyPlan::new("topo-1", "us-east-1", "evidence://iac/test/topology")
            .expect("valid topology");
        plan = plan.add_cell(desired_cell).expect("add cell");
        plan
    };
    let observed = {
        let mut plan = CellTopologyPlan::new("topo-1", "us-east-1", "evidence://iac/test/topology")
            .expect("valid topology");
        plan = plan.add_cell(observed_cell).expect("add cell");
        plan
    };

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::IdentityMismatch);
    assert!(report.entries.is_empty());
}

#[test]
fn determinism() {
    let r1 = make_ref("tenant-namespace", "1.0.0");
    let r2 = make_ref("storage-bucket", "2.0.0");
    let desired = topology(
        "topo-1",
        vec![cell("cell-us-east", vec![r1.clone(), r2.clone()])],
    );
    let r2_v1 = make_ref("storage-bucket", "1.0.0");
    let observed = topology(
        "topo-1",
        vec![cell("cell-us-east", vec![r1.clone(), r2_v1.clone()])],
    );

    let report_a = compute_iac_plan_diff(&desired, &observed);
    let report_b = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(
        report_a, report_b,
        "compute_iac_plan_diff must be deterministic"
    );
    let mut sorted = report_a.entries.clone();
    sorted.sort();
    assert_eq!(report_a.entries, sorted, "entries must already be sorted");
}

#[test]
fn all_action_variants() {
    let r_keep = make_ref("tenant-namespace", "1.0.0");
    let r_new = make_ref("network-policy", "1.0.0");
    let r_upd_v2 = make_ref("storage-bucket", "2.0.0");
    let r_old = make_ref("legacy-module", "1.0.0");
    let r_upd_v1 = make_ref("storage-bucket", "1.0.0");

    let desired = topology(
        "topo-1",
        vec![cell(
            "cell-us-east",
            vec![r_keep.clone(), r_new.clone(), r_upd_v2.clone()],
        )],
    );
    let observed = topology(
        "topo-1",
        vec![cell(
            "cell-us-east",
            vec![r_keep.clone(), r_old.clone(), r_upd_v1.clone()],
        )],
    );

    let report = compute_iac_plan_diff(&desired, &observed);
    assert_eq!(report.verdict, IacPlanDiffVerdict::HasChanges);

    let actions: std::collections::BTreeMap<&str, &PlanAction> = report
        .entries
        .iter()
        .map(|e| (e.module_ref.name(), &e.action))
        .collect();

    assert_eq!(actions["tenant-namespace"], &PlanAction::NoChange);
    assert_eq!(actions["network-policy"], &PlanAction::Create);
    assert_eq!(actions["storage-bucket"], &PlanAction::Update);
    assert_eq!(actions["legacy-module"], &PlanAction::Destroy);
}
