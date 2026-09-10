use shared_platform_contracts_kernel::tenancy::{IsolationPosture, Tenant, TenantLifecycleState};
use shared_resource_provider_contract_kernel::{
    OperationResult, ProviderError, ResourceName, ResourceProvider,
};
use tenancy_tenant_lifecycle_domain::{DesiredTenantState, Plan, derive_step_key};
use tenancy_tenant_lifecycle_kernel::TenantLifecycleStore;

use crate::TenantLifecycleProvider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantSpec {
    pub display_name: String,
    pub isolation_posture: IsolationPosture,
    pub cell_id: String,
    pub residency_zone: Option<String>,
    pub desired: DesiredTenantState,
}

#[derive(Debug, Clone, Copy)]
pub struct ReconcileContext<'a> {
    /// The CR's stable uid (never reused, survives spec edits).
    pub cr_uid: &'a str,
    pub generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// Observed equals desired (observed `None` means retired-and-gone).
    Converged {
        observed: Option<TenantLifecycleState>,
    },
    Progressing {
        detail: String,
    },
    /// Terminal for this spec: the plan is unreachable or the requested
    /// operation failed its precondition. A spec change (new generation)
    /// is required to make progress.
    Blocked {
        reason: String,
    },
}

fn tenant_from_spec(name: &ResourceName, spec: &TenantSpec) -> Tenant {
    Tenant {
        tenant_id: name.resource_id().to_owned(),
        display_name: spec.display_name.clone(),
        state: TenantLifecycleState::initial(),
        isolation_posture: spec.isolation_posture,
        cell_id: spec.cell_id.clone(),
        residency_zone: spec.residency_zone.clone(),
    }
}

fn state_name(state: TenantLifecycleState) -> &'static str {
    match state {
        TenantLifecycleState::Provisioning => "provisioning",
        TenantLifecycleState::Active => "active",
        TenantLifecycleState::Suspended => "suspended",
        TenantLifecycleState::Retired => "retired",
    }
}

impl<S: TenantLifecycleStore + Send + Sync> TenantLifecycleProvider<S> {
    /// One level-triggered reconcile pass: observe, plan ONE step via the
    /// domain planner, request it through the AIP-151 ledger, and report.
    /// Pure function of (stored state, spec, ctx) — safe to re-run after a
    /// controller restart because every mutation key is derived, not drawn.
    pub async fn reconcile(
        &mut self,
        name: &ResourceName,
        spec: &TenantSpec,
        ctx: ReconcileContext<'_>,
    ) -> Result<ReconcileOutcome, ProviderError> {
        let observed = match self.observe_stored(name).await? {
            Some(tenant) => tenant,
            None => {
                if spec.desired == DesiredTenantState::Retired {
                    // Never existed and meant to be retired: nothing to do.
                    return Ok(ReconcileOutcome::Converged { observed: None });
                }
                let key = derive_step_key(ctx.cr_uid, ctx.generation, "create").map_err(|e| {
                    ProviderError::Internal {
                        message: e.to_string(),
                    }
                })?;
                self.create(name, tenant_from_spec(name, spec), &key)
                    .await?;
                return Ok(ReconcileOutcome::Progressing {
                    detail: "created".to_owned(),
                });
            }
        };

        match tenancy_tenant_lifecycle_domain::plan_next_operation(observed.state, spec.desired) {
            Plan::Unreachable => Ok(ReconcileOutcome::Blocked {
                reason: format!(
                    "desired {:?} is unreachable from {:?} (retired tenants are terminal)",
                    spec.desired, observed.state
                ),
            }),
            Plan::Converged => {
                // State converged; reconcile declarative metadata drift
                // through the same idempotent put surface.
                let declared = Tenant {
                    state: observed.state,
                    ..tenant_from_spec(name, spec)
                };
                if declared != observed {
                    let key = derive_step_key(ctx.cr_uid, ctx.generation, "put-metadata").map_err(
                        |e| ProviderError::Internal {
                            message: e.to_string(),
                        },
                    )?;
                    self.put(name, declared, &key).await?;
                    return Ok(ReconcileOutcome::Progressing {
                        detail: "metadata-updated".to_owned(),
                    });
                }
                Ok(ReconcileOutcome::Converged {
                    observed: Some(observed.state),
                })
            }
            Plan::Step(operation) => {
                let step = format!("{operation:?}-from-{}", state_name(observed.state));
                let key = derive_step_key(ctx.cr_uid, ctx.generation, &step).map_err(|e| {
                    ProviderError::Internal {
                        message: e.to_string(),
                    }
                })?;
                let requested = self.apply_lifecycle(name, operation, &key).await?;
                let polled = self.poll_operation(&requested.name).await?;
                if !polled.done {
                    return Ok(ReconcileOutcome::Progressing {
                        detail: format!("operation {} pending", polled.name),
                    });
                }
                match polled.result {
                    Some(OperationResult::Response(_)) | None => {
                        Ok(ReconcileOutcome::Progressing {
                            detail: format!("operation {} completed", polled.name),
                        })
                    }
                    Some(OperationResult::Error(error)) => Ok(ReconcileOutcome::Blocked {
                        reason: format!(
                            "operation {} failed: {} ({})",
                            polled.name, error.message, error.code
                        ),
                    }),
                }
            }
        }
    }
}
