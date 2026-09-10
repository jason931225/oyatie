//! Decision-audit emission onto the tracing pipeline.

use iam_pdp_kernel::DecisionAuditSink;
use shared_pdp_kernel::DecisionAuditRecord;
use shared_platform_contracts_kernel::pdp::Decision;

/// One structured tracing event per sealed record. Tracing is fire-and-forget,
/// which is what satisfies the port's requirement that emission never fail the
/// decision path.
#[derive(Clone, Copy, Debug, Default)]
pub struct TracingDecisionAuditSink;

impl TracingDecisionAuditSink {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DecisionAuditSink for TracingDecisionAuditSink {
    fn record(&self, record: &DecisionAuditRecord) {
        let decision = match record.decision {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        };
        tracing::info!(
            target: "cloud_iam_pdp::audit",
            decision_id = %record.decision_id,
            request_id = %record.request_id,
            tenant_id = %record.tenant_id,
            principal_type = %record.principal.entity_type,
            principal_id = %record.principal.entity_id,
            action = %record.action,
            resource_type = %record.resource.entity_type,
            resource_id = %record.resource.entity_id,
            decision,
            policy_version = %record.policy_version.as_str(),
            determining_policy_ids = %record.determining_policy_ids.join(","),
            cache_hit = record.cache_hit,
            "decision-audit-record",
        );
    }
}
