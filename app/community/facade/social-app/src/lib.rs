#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use community_social_post_composition_api::{
    AuthorizedSocialContext, ComposePostRequest, SocialApiArtifactKind, SocialApiContext,
    SocialPostReceipt, post_published_event_envelope,
};
use community_social_post_composition_postgres::{PersistSocialPostRecord, build_post_write_batch};
use community_social_post_composition_usecase::{
    SocialUsecaseError, compose_post, plan_story_purge,
};
use shared_postgres_command_kernel::{
    PostgresPoolConfig, SqlCommandError, SqlExecutionPlan, SqlWriteBatch, TenantSqlContext,
};
use shared_protocol_parity_kernel::{ProtocolEventEnvelope, ProtocolParityError};
use shared_protocol_transport_kernel::{
    ProtocolTransportBudget, ProtocolTransportBundle, ProtocolTransportError,
    plan_transport_from_envelope,
};
use shared_transactional_outbox_kernel::{
    BackboneOutboxTable, TransactionalOutboxError, append_outbox_to_batch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SocialAppError {
    ScopeMismatch {
        context_scope_ref: String,
        tenant_id: String,
    },
    Usecase(SocialUsecaseError),
    Sql(SqlCommandError),
    Protocol(ProtocolParityError),
    Transport(ProtocolTransportError),
    Outbox(TransactionalOutboxError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SocialPublishPlan {
    pub receipt: SocialPostReceipt,
    pub persistence: SqlWriteBatch,
    pub sql_execution: SqlExecutionPlan,
    pub protocol_event: ProtocolEventEnvelope,
    pub transport: ProtocolTransportBundle,
    pub story_purge_targets: Vec<String>,
}

pub fn plan_publish_post(
    tenant: TenantSqlContext,
    context: AuthorizedSocialContext,
    request: ComposePostRequest,
    story_purge_now: Option<u64>,
) -> Result<SocialPublishPlan, SocialAppError> {
    require_scope_match(&tenant, &context.scope_ref)?;
    let record_template = SocialRecordTemplate::from(&context, &request, tenant);
    let outbox_tenant = record_template.tenant.clone();
    let (post, receipt) = compose_post(&context, request).map_err(SocialAppError::Usecase)?;
    let story_purge_targets = match (record_template.story_expires_at_value, story_purge_now) {
        (Some(expires_at), Some(now)) if now >= expires_at => plan_story_purge(&post, now)
            .map_err(SocialAppError::Usecase)?
            .purge_targets
            .into_iter()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    let persistence_record = record_template.into_record(story_purge_targets.clone());
    let persistence = build_post_write_batch(&persistence_record).map_err(SocialAppError::Sql)?;
    let protocol_event =
        post_published_event_envelope(&context, &receipt).map_err(SocialAppError::Protocol)?;
    let persistence = append_outbox_to_batch(
        BackboneOutboxTable::SocialPostComposition,
        &outbox_tenant,
        persistence,
        &protocol_event,
    )
    .map_err(SocialAppError::Outbox)?;
    let sql_execution = SqlExecutionPlan::from_batch(
        PostgresPoolConfig::for_microservice("social", 16).map_err(SocialAppError::Sql)?,
        persistence.clone(),
    )
    .map_err(SocialAppError::Sql)?;
    let transport =
        plan_transport_from_envelope(&protocol_event, ProtocolTransportBudget::default())
            .map_err(SocialAppError::Transport)?;

    Ok(SocialPublishPlan {
        receipt,
        persistence,
        sql_execution,
        protocol_event,
        transport,
        story_purge_targets,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SocialRecordTemplate {
    tenant: TenantSqlContext,
    post_id: String,
    creator_ref: String,
    context_kind: String,
    artifact_kind: String,
    media_refs: Vec<String>,
    workflow_consent_ref: Option<String>,
    policy_decision_ref: String,
    idempotency_key: String,
    audit_correlation_id: String,
    story_expires_at: Option<String>,
    story_expires_at_value: Option<u64>,
}

impl SocialRecordTemplate {
    fn from(
        context: &AuthorizedSocialContext,
        request: &ComposePostRequest,
        tenant: TenantSqlContext,
    ) -> Self {
        Self {
            tenant,
            post_id: request.post_id.clone(),
            creator_ref: request.creator_ref.clone(),
            context_kind: context_kind(context.context).to_string(),
            artifact_kind: artifact_kind(request.kind).to_string(),
            media_refs: request.media_refs.clone(),
            workflow_consent_ref: request.workflow_consent_ref.clone(),
            policy_decision_ref: context.policy_decision_ref.clone(),
            idempotency_key: context.idempotency_key.clone(),
            audit_correlation_id: context.audit_correlation_id.clone(),
            story_expires_at: request.story_expires_at.map(|value| value.to_string()),
            story_expires_at_value: request.story_expires_at,
        }
    }

    fn into_record(self, story_purge_targets: Vec<String>) -> PersistSocialPostRecord {
        PersistSocialPostRecord {
            tenant: self.tenant,
            post_id: self.post_id,
            creator_ref: self.creator_ref,
            context_kind: self.context_kind,
            artifact_kind: self.artifact_kind,
            media_refs: self.media_refs,
            workflow_consent_ref: self.workflow_consent_ref,
            policy_decision_ref: self.policy_decision_ref,
            idempotency_key: self.idempotency_key,
            audit_correlation_id: self.audit_correlation_id,
            story_expires_at: self.story_expires_at,
            story_purge_targets,
        }
    }
}

fn require_scope_match(
    tenant: &TenantSqlContext,
    context_scope_ref: &str,
) -> Result<(), SocialAppError> {
    if tenant.tenant_id == context_scope_ref {
        Ok(())
    } else {
        Err(SocialAppError::ScopeMismatch {
            context_scope_ref: context_scope_ref.to_string(),
            tenant_id: tenant.tenant_id.clone(),
        })
    }
}

fn context_kind(context: SocialApiContext) -> &'static str {
    match context {
        SocialApiContext::Personal => "personal",
        SocialApiContext::Work => "work",
    }
}

fn artifact_kind(kind: SocialApiArtifactKind) -> &'static str {
    match kind {
        SocialApiArtifactKind::FeedPost => "feed_post",
        SocialApiArtifactKind::Story => "story",
        SocialApiArtifactKind::CollaborativePost => "collaborative_post",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant() -> TenantSqlContext {
        TenantSqlContext::new("person:u", "cell-a", "person:u#cell-a", "US").unwrap()
    }

    fn context() -> AuthorizedSocialContext {
        AuthorizedSocialContext {
            context: SocialApiContext::Personal,
            scope_ref: "person:u".into(),
            principal_ref: "user:u".into(),
            idempotency_key: "idem".into(),
            policy_decision_ref: "policy".into(),
            audit_correlation_id: "audit".into(),
        }
    }

    fn story_request() -> ComposePostRequest {
        ComposePostRequest {
            post_id: "post:p".into(),
            creator_ref: "user:u".into(),
            kind: SocialApiArtifactKind::Story,
            media_refs: vec!["media:m".into()],
            story_expires_at: Some(10),
            collab_owner_refs: vec![],
            collab_consent_refs: vec![],
            workflow_consent_ref: None,
            ar_biometric_persisted: false,
        }
    }

    #[test]
    fn publish_post_plan_composes_receipt_persistence_protocol_and_story_purge() {
        let plan = plan_publish_post(tenant(), context(), story_request(), Some(11)).unwrap();

        assert_eq!(plan.receipt.event_type, "social.post.created");
        assert_eq!(plan.story_purge_targets.len(), 3);
        assert_eq!(plan.persistence.statements.len(), 5);
        assert_eq!(plan.sql_execution.total_command_count, 6);
        assert_eq!(plan.sql_execution.pool.application_name, "oyatie-social");
        assert_eq!(plan.protocol_event.binding.proto_rpc, "PublishPost");
        assert_eq!(plan.transport.broker_publish.message_name, "PostPublished");
        assert_eq!(plan.transport.grpc_unary.rpc, "PublishPost");
        assert_eq!(
            plan.persistence.statements[4].name,
            "insert_transactional_outbox_event"
        );
        assert!(
            plan.persistence.statements[4]
                .sql
                .contains("social_post_composition.protocol_outbox_events")
        );
    }

    #[test]
    fn publish_post_plan_keeps_unexpired_story_purge_out_of_write_batch() {
        let plan = plan_publish_post(tenant(), context(), story_request(), Some(9)).unwrap();

        assert!(plan.story_purge_targets.is_empty());
        assert_eq!(plan.persistence.statements.len(), 2);
        assert_eq!(plan.sql_execution.total_command_count, 3);
        assert_eq!(
            plan.persistence.statements[1].name,
            "insert_transactional_outbox_event"
        );
    }
}
