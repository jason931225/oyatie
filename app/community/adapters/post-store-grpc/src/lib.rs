//! Framework-free gRPC write-plan boundary for community post-store.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use community_post_store_api::{
    AuthorizedCommunityContext, CastVoteRequest, CreatePostRequest, ModeratePostRequest,
};
use community_post_store_app::{
    CommunityAppError, CommunityModerationPlan, CommunityPostPlan, CommunityVotePlan,
    plan_cast_vote, plan_create_post, plan_moderation_action,
};
use community_post_store_domain::{CommunityPost, VoteLedger};
use shared_postgres_command_kernel::TenantSqlContext;
use shared_protocol_transport_kernel::GrpcUnaryPlan;

pub const COMMUNITY_PROTO_PACKAGE: &str = "oya.community.v1";
pub const POST_STORE_SERVICE: &str = "PostStoreService";
pub const VOTING_ENGINE_SERVICE: &str = "VotingEngineService";
pub const MODERATION_QUEUE_SERVICE: &str = "ModerationQueueService";
pub const CREATE_POST_RPC: &str = "CreatePost";
pub const CAST_VOTE_RPC: &str = "CastVote";
pub const APPLY_ACTION_RPC: &str = "ApplyAction";

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
pub enum CommunityGrpcError {
    App(CommunityAppError),
    RpcMismatch {
        expected_package: &'static str,
        expected_service: &'static str,
        expected_rpc: &'static str,
        actual_package: &'static str,
        actual_service: &'static str,
        actual_rpc: &'static str,
    },
}

pub fn create_post_write_plan(
    tenant: TenantSqlContext,
    context: AuthorizedCommunityContext,
    space_id: impl Into<String>,
    request: CreatePostRequest,
) -> Result<GrpcResponse<CommunityPostPlan>, CommunityGrpcError> {
    let plan =
        plan_create_post(tenant, context, space_id, request).map_err(CommunityGrpcError::App)?;
    let rpc = require_rpc(
        &plan.transport.grpc_unary,
        POST_STORE_SERVICE,
        CREATE_POST_RPC,
    )?;
    Ok(GrpcResponse {
        status: GrpcStatusCode::Ok,
        rpc,
        body: plan,
    })
}

pub fn cast_vote_write_plan(
    tenant: TenantSqlContext,
    context: AuthorizedCommunityContext,
    post: &CommunityPost,
    ledger: &mut VoteLedger,
    request: CastVoteRequest,
) -> Result<GrpcResponse<CommunityVotePlan>, CommunityGrpcError> {
    let plan =
        plan_cast_vote(tenant, context, post, ledger, request).map_err(CommunityGrpcError::App)?;
    let rpc = require_rpc(
        &plan.transport.grpc_unary,
        VOTING_ENGINE_SERVICE,
        CAST_VOTE_RPC,
    )?;
    Ok(GrpcResponse {
        status: GrpcStatusCode::Ok,
        rpc,
        body: plan,
    })
}

pub fn apply_moderation_action_write_plan(
    tenant: TenantSqlContext,
    context: AuthorizedCommunityContext,
    post: &CommunityPost,
    request: ModeratePostRequest,
) -> Result<GrpcResponse<CommunityModerationPlan>, CommunityGrpcError> {
    let plan =
        plan_moderation_action(tenant, context, post, request).map_err(CommunityGrpcError::App)?;
    let rpc = require_rpc(
        &plan.transport.grpc_unary,
        MODERATION_QUEUE_SERVICE,
        APPLY_ACTION_RPC,
    )?;
    Ok(GrpcResponse {
        status: GrpcStatusCode::Ok,
        rpc,
        body: plan,
    })
}

fn require_rpc(
    rpc: &GrpcUnaryPlan,
    expected_service: &'static str,
    expected_rpc: &'static str,
) -> Result<GrpcUnaryPlan, CommunityGrpcError> {
    if rpc.package == COMMUNITY_PROTO_PACKAGE
        && rpc.service == expected_service
        && rpc.rpc == expected_rpc
    {
        Ok(rpc.clone())
    } else {
        Err(CommunityGrpcError::RpcMismatch {
            expected_package: COMMUNITY_PROTO_PACKAGE,
            expected_service,
            expected_rpc,
            actual_package: rpc.package,
            actual_service: rpc.service,
            actual_rpc: rpc.rpc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use community_post_store_api::{CommunityApiMode, ModerationVerb, VoteDirection};

    fn tenant() -> TenantSqlContext {
        TenantSqlContext::new("tenant:t", "cell-a", "tenant:t#cell-a", "US").unwrap()
    }

    fn context() -> AuthorizedCommunityContext {
        AuthorizedCommunityContext {
            tenant_scope_ref: "tenant:t".into(),
            principal_ref: "user:u".into(),
            idempotency_key: "idem".into(),
            policy_decision_ref: "policy".into(),
            audit_correlation_id: "audit".into(),
        }
    }

    fn post_request() -> CreatePostRequest {
        CreatePostRequest {
            post_id: "post:p".into(),
            thread_id: "thread:t".into(),
            mode: CommunityApiMode::Teamblind,
            routine_display_ref: "anon".into(),
            audit_author_ref: "user:u".into(),
            disclosure_policy_ref: Some("disclosure".into()),
            body_ref: "body:b".into(),
            retention_policy_id: "retain".into(),
        }
    }

    #[test]
    fn grpc_create_vote_and_moderate_return_matching_proto_methods() {
        let created =
            create_post_write_plan(tenant(), context(), "space:s", post_request()).unwrap();
        assert_eq!(created.status, GrpcStatusCode::Ok);
        assert_eq!(created.rpc.service, POST_STORE_SERVICE);
        assert_eq!(created.rpc.rpc, CREATE_POST_RPC);
        assert_eq!(
            created.body.transport.broker_publish.event_kind,
            "oya.community.post.created.v1"
        );

        let mut vote_context = context();
        vote_context.principal_ref = "user:voter".into();
        vote_context.idempotency_key = "vote:v".into();
        let mut ledger = VoteLedger::new(&created.body.post);
        let vote = cast_vote_write_plan(
            tenant(),
            vote_context,
            &created.body.post,
            &mut ledger,
            CastVoteRequest {
                post_id: "post:p".into(),
                voter_ref: "user:voter".into(),
                direction: VoteDirection::Up,
            },
        )
        .unwrap();
        assert_eq!(vote.rpc.service, VOTING_ENGINE_SERVICE);
        assert_eq!(vote.rpc.rpc, CAST_VOTE_RPC);
        assert_eq!(
            vote.body.transport.broker_publish.channel_address,
            "community.vote.cast"
        );

        let moderation = apply_moderation_action_write_plan(
            tenant(),
            context(),
            &created.body.post,
            ModeratePostRequest {
                policy_ref: "policy:moderation".into(),
                evidence_ref: "evidence:e".into(),
                verb: ModerationVerb::Hide,
            },
        )
        .unwrap();
        assert_eq!(moderation.rpc.service, MODERATION_QUEUE_SERVICE);
        assert_eq!(moderation.rpc.rpc, APPLY_ACTION_RPC);
        assert_eq!(
            moderation.body.transport.broker_publish.message_name,
            "ModerationActioned"
        );
    }
}
