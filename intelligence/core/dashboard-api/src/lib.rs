//! Typed request/response shapes for the read-only dashboard endpoints. The
//! HTTP runtime lives in a separate adapter crate (ADR-0090).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use intelligence_dashboard_kernel::{AccountHealthView, RoutingView, SessionView, UsageView};

/// One variant, so no API on this surface can be handed a write method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
}

/// Deliberately disjoint from [`HttpMethod`]: no conversion exists in either
/// direction, so a rejected method can never become an accepted one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectedWriteMethod {
    Post,
    Put,
    Delete,
    Patch,
}

impl RejectedWriteMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyRequest {
    method: HttpMethod,
    path: String,
}

impl ReadOnlyRequest {
    pub fn new_get(path: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            path: path.into(),
        }
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountHealthResponse {
    pub status: u16,
    pub view: AccountHealthView, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountHealthListResponse {
    pub status: u16,
    pub views: Vec<AccountHealthView>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionResponse {
    pub status: u16,       // data_class: INTERNAL_ONLY
    pub view: SessionView, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageResponse {
    pub status: u16,     // data_class: INTERNAL_ONLY
    pub view: UsageView, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutingResponse {
    pub status: u16,       // data_class: INTERNAL_ONLY
    pub view: RoutingView, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodNotAllowedResponse {
    pub status: u16,
    pub allow_header: &'static str,
    pub audit_event: &'static str,
    pub rejected_method: RejectedWriteMethod, // data_class: INTERNAL_ONLY
}

impl MethodNotAllowedResponse {
    pub fn reject(method: RejectedWriteMethod) -> Self {
        Self {
            status: 405,
            allow_header: "GET",
            audit_event: "forbidden_write_attempt",
            rejected_method: method,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadOnlyEndpoint {
    AccountHealth,
    AccountHealthList,
    Session,
    Usage,
    Routing,
}

impl ReadOnlyEndpoint {
    pub fn path(&self) -> &'static str {
        match self {
            Self::AccountHealth => "/v1/accounts/{id}/health",
            Self::AccountHealthList => "/v1/accounts/health",
            Self::Session => "/v1/sessions/{id}",
            Self::Usage => "/v1/accounts/{id}/usage",
            Self::Routing => "/v1/routing/explain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intelligence_account_domain::{
        AccountHealth, AccountId, AccountState, ProviderAccount, ProviderFamily, RouteExplanation,
        UsageWindow, UsageWindowKind,
    };

    fn aid(s: &str) -> AccountId {
        AccountId(s.to_owned())
    }

    fn active() -> ProviderAccount {
        let mut a = ProviderAccount::new(aid("a1"), ProviderFamily::Claude);
        a.state = AccountState::Active;
        a
    }

    #[test]
    fn request_constructor_pins_method_to_get() {
        let r = ReadOnlyRequest::new_get("/v1/accounts/a1/health");
        assert_eq!(r.method(), HttpMethod::Get);
        assert_eq!(r.path(), "/v1/accounts/a1/health");
    }

    #[test]
    fn account_health_response_ok_status() {
        let a = active();
        let h = AccountHealth {
            account_id: aid("a1"),
            is_healthy: true,
            reason: None,
            last_check_at_epoch_secs: 0,
        };
        let resp = AccountHealthResponse {
            status: 200,
            view: AccountHealthView::new(&a, &h),
        };
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn account_health_list_response_ok_status() {
        let a = active();
        let h = AccountHealth {
            account_id: aid("a1"),
            is_healthy: true,
            reason: None,
            last_check_at_epoch_secs: 0,
        };
        let resp = AccountHealthListResponse {
            status: 200,
            views: vec![AccountHealthView::new(&a, &h)],
        };
        assert_eq!(resp.status, 200);
        assert_eq!(resp.views.len(), 1);
    }

    #[test]
    fn usage_response_ok_status() {
        let w = UsageWindow::new(UsageWindowKind::FiveHour, 0, 18000, 80, 20).unwrap();
        let resp = UsageResponse {
            status: 200,
            view: UsageView::new(aid("a1"), &w),
        };
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn routing_response_ok_status() {
        let e = RouteExplanation {
            chosen_provider: ProviderFamily::Claude,
            chosen_account_id: aid("a1"),
            chosen_model: "claude-sonnet-4-6".to_owned(),
            reason: "cost".to_owned(),
        };
        let resp = RoutingResponse {
            status: 200,
            view: RoutingView::new(&e),
        };
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn endpoint_paths_are_stable() {
        assert_eq!(
            ReadOnlyEndpoint::AccountHealth.path(),
            "/v1/accounts/{id}/health"
        );
        assert_eq!(
            ReadOnlyEndpoint::AccountHealthList.path(),
            "/v1/accounts/health"
        );
        assert_eq!(ReadOnlyEndpoint::Session.path(), "/v1/sessions/{id}");
        assert_eq!(ReadOnlyEndpoint::Usage.path(), "/v1/accounts/{id}/usage");
        assert_eq!(ReadOnlyEndpoint::Routing.path(), "/v1/routing/explain");
    }

    #[test]
    fn negative_write_methods_rejected_at_type_level() {
        for m in [
            RejectedWriteMethod::Post,
            RejectedWriteMethod::Put,
            RejectedWriteMethod::Delete,
            RejectedWriteMethod::Patch,
        ] {
            let r = MethodNotAllowedResponse::reject(m);
            assert_eq!(r.status, 405);
            assert_eq!(r.allow_header, "GET");
            assert_eq!(r.audit_event, "forbidden_write_attempt");
            assert_eq!(r.rejected_method, m);
            assert!(!m.as_str().is_empty());
        }
        let only = HttpMethod::Get;
        assert_eq!(only, HttpMethod::Get);
    }
}
