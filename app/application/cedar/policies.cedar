permit(
  principal == ServicePrincipal::"oyatie.foundry.cell-orchestrator",
  action in [Action::"ComputeAutoshardingPlacement", Action::"ExecuteAutoRebalance", Action::"ExecuteDynamicSharding"],
  resource is ShardingAutomation
) when {
  resource.microservice == "application" &&
  context.doctrine.adr_0348 == true &&
  context.autosharding_mode == "control_plane_driven" &&
  context.honors_residency == true &&
  context.compliance_pack_check_result == "pass" &&
  context.residency_check_result == "pass" &&
  context.audit_chain_emit == true
};

forbid(
  principal == ServicePrincipal::"oyatie.foundry.cell-orchestrator",
  action in [Action::"ExecuteAutoRebalance", Action::"ExecuteDynamicSharding"],
  resource is ShardingAutomation
) when {
  resource.microservice == "application" &&
  (
    context.honors_residency == false ||
    context.compliance_pack_check_result != "pass" ||
    context.residency_check_result != "pass" ||
    context.audit_chain_emit != true
  )
};

forbid(
  principal == ServicePrincipal::"oyatie.foundry.cell-orchestrator",
  action == Action::"MigrateTenantCrossJurisdiction",
  resource is TenantMigration
) unless {
  resource.microservice == "application" &&
  context.cedar_permit_id != "" &&
  context.cross_jurisdiction_permit_id != "" &&
  context.audit_chain_emit == true
};

permit(
  principal == ServicePrincipal::"oyatie.jenkins.ci",
  action == Action::"RunCiMirror",
  resource is ServiceCiSurface
) when {
  resource.microservice == "application" &&
  context.doctrine.adr_0346 == true &&
  context.verify_ci_required == true &&
  context.steps.cargo_fmt_all_check == true &&
  context.steps.cargo_check_workspace_all_targets == true &&
  context.steps.cargo_clippy_workspace_all_targets_d_warnings == true &&
  context.steps.cargo_nextest_workspace_no_fail_fast == true &&
  context.steps.gate_run_all_ci_required == true &&
  context.step_exit_semantics == "block_on_exit_0_each_step" &&
  context.exit_code_contract in [0, 1, 2]
};

permit(
  principal == ServicePrincipal::"oyatie.jenkins.ci",
  action == Action::"RunSelfHostedCi",
  resource is ServiceCiSurface
) when {
  resource.microservice == "application" &&
  context.doctrine.adr_0349 == true &&
  context.github_actions_parity_ok == true &&
  context.jcasc_declared == true &&
  context.lane_prefix in ["governance", "check"] &&
  context.audit_chain_emit == true
};

forbid(
  principal == ServicePrincipal::"oyatie.jenkins.ci",
  action in [Action::"RunCiMirror", Action::"RunSelfHostedCi"],
  resource is ServiceCiSurface
) when {
  resource.microservice == "application" &&
  (
    context.github_actions_parity_ok == false ||
    context.jcasc_declared == false ||
    context.lane_id.matches("^governance-.*$")
  )
};

permit(
  principal == ServicePrincipal::"oyatie.argocd.gitops",
  action == Action::"SyncApplication",
  resource is GitOpsApplication
) when {
  resource.microservice == "application" &&
  context.doctrine.adr_0349 == true &&
  context.cosign_signature_verified == true &&
  context.tenant_namespace_isolated == true &&
  context.cedar_policy_gate_granted == true &&
  context.audit_chain_emit == true
};

forbid(
  principal == ServicePrincipal::"oyatie.argocd.gitops",
  action == Action::"SyncApplication",
  resource is GitOpsApplication
) when {
  resource.microservice == "application" &&
  (
    context.cosign_signature_verified == false ||
    context.tenant_namespace_isolated == false ||
    context.audit_chain_emit != true
  )
};

forbid(
  principal,
  action == Action::"AuthorGovernanceLane",
  resource is ServiceCiSurface
) when {
  resource.microservice == "application" &&
  context.doctrine.adr_0347 == true &&
  context.lane_id.matches("^governance-.*$") &&
  context.historical_reference != true
};
