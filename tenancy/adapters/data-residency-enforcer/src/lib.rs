#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidencyContext {
    pub tenant_id: String,
    pub source_region: String,
    pub destination_region: String,
    pub data_class: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidencyDecision {
    Allow,
    DenyResidency,
    DenyDataClass,
    DenyJurisdictionPack,
}

pub trait ResidencyPolicyEvaluator {
    fn evaluate(&self, ctx: &ResidencyContext) -> Result<ResidencyDecision, ResidencyAdapterError>;
}

pub trait ResidencyDenialAuditSink {
    fn emit_denial(
        &self,
        ctx: &ResidencyContext,
        decision: ResidencyDecision,
    ) -> Result<(), ResidencyAdapterError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidencyAdapterError {
    PolicyMalformed,
    EvaluationFailed,
    AuditSinkUnavailable,
}

pub fn enforce<E: ResidencyPolicyEvaluator, S: ResidencyDenialAuditSink>(
    evaluator: &E,
    sink: &S,
    ctx: &ResidencyContext,
) -> Result<ResidencyDecision, ResidencyAdapterError> {
    let decision = evaluator.evaluate(ctx)?;
    if decision != ResidencyDecision::Allow {
        sink.emit_denial(ctx, decision)?;
    }
    Ok(decision)
}
