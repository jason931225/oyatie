//! Framework-free gRPC write-plan boundary for social post composition.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use community_social_app::{SocialAppError, SocialPublishPlan, plan_publish_post};
use community_social_post_composition_api::{AuthorizedSocialContext, ComposePostRequest};
use shared_postgres_command_kernel::TenantSqlContext;
use shared_protocol_transport_kernel::GrpcUnaryPlan;

pub const SOCIAL_PROTO_PACKAGE: &str = "oya.social.v1";
pub const SOCIAL_GRPC_SERVICE: &str = "PostComposition";
pub const PUBLISH_POST_RPC: &str = "PublishPost";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrpcStatusCode {
    Ok,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrpcResponse<T> {
    pub status: GrpcStatusCode,
    pub rpc: GrpcUnaryPlan,
    pub body: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SocialGrpcError {
    App(SocialAppError),
    RpcMismatch {
        expected_package: &'static str,
        expected_service: &'static str,
        expected_rpc: &'static str,
        actual_package: &'static str,
        actual_service: &'static str,
        actual_rpc: &'static str,
    },
}

pub fn publish_post_write_plan(
    tenant: TenantSqlContext,
    context: AuthorizedSocialContext,
    request: ComposePostRequest,
    story_purge_now: Option<u64>,
) -> Result<GrpcResponse<SocialPublishPlan>, SocialGrpcError> {
    let plan = plan_publish_post(tenant, context, request, story_purge_now)
        .map_err(SocialGrpcError::App)?;
    let rpc = require_rpc(&plan.transport.grpc_unary)?;
    Ok(GrpcResponse {
        status: GrpcStatusCode::Ok,
        rpc,
        body: plan,
    })
}

fn require_rpc(rpc: &GrpcUnaryPlan) -> Result<GrpcUnaryPlan, SocialGrpcError> {
    if rpc.package == SOCIAL_PROTO_PACKAGE
        && rpc.service == SOCIAL_GRPC_SERVICE
        && rpc.rpc == PUBLISH_POST_RPC
    {
        Ok(rpc.clone())
    } else {
        Err(SocialGrpcError::RpcMismatch {
            expected_package: SOCIAL_PROTO_PACKAGE,
            expected_service: SOCIAL_GRPC_SERVICE,
            expected_rpc: PUBLISH_POST_RPC,
            actual_package: rpc.package,
            actual_service: rpc.service,
            actual_rpc: rpc.rpc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use community_social_post_composition_api::{SocialApiArtifactKind, SocialApiContext};

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

    fn request() -> ComposePostRequest {
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
    fn grpc_publish_post_returns_transport_and_story_purge_plan() {
        let response = publish_post_write_plan(tenant(), context(), request(), Some(11)).unwrap();

        assert_eq!(response.status, GrpcStatusCode::Ok);
        assert_eq!(response.rpc.rpc, PUBLISH_POST_RPC);
        assert_eq!(response.body.story_purge_targets.len(), 3);
        assert_eq!(
            response.body.transport.broker_publish.event_kind,
            "oya.social.post.published.v1"
        );
    }
}
