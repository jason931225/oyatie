//! The request/response envelopes and the audit event that every transport
//! must project byte-identically.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::BTreeMap;

/// Request envelope projected by every transport.
pub trait UseCaseRequest {
    fn use_case_id(&self) -> &str;
    fn tenant_id(&self) -> &str;
    fn payload(&self) -> &BTreeMap<String, String>;
}

/// Response envelope projected by every transport.
pub trait UseCaseResponse {
    fn use_case_id(&self) -> &str;
    fn status(&self) -> ResponseStatus;
    fn body(&self) -> &BTreeMap<String, String>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseStatus {
    Ok,
    Denied,
    NotFound,
    Error,
}

/// Carries no transport field, so two events built from the same request
/// through different transports are `Eq` or the parity contract is broken.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEvent {
    pub use_case_id: String,                      // data_class: INTERNAL_ONLY
    pub tenant_id: String,                        // data_class: INTERNAL_ONLY
    pub status: ResponseStatus,                   // data_class: INTERNAL_ONLY
    pub canonical_payload: Vec<(String, String)>, // data_class: INTERNAL_ONLY
}

impl AuditEvent {
    /// Sorts `canonical_payload`, so two transports that build the payload in
    /// different orders still encode to the same bytes.
    pub fn canonical<Req: UseCaseRequest, Res: UseCaseResponse>(req: &Req, res: &Res) -> Self {
        let mut payload: Vec<(String, String)> = req
            .payload()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        payload.sort();
        Self {
            use_case_id: req.use_case_id().to_string(),
            tenant_id: req.tenant_id().to_string(),
            status: res.status(),
            canonical_payload: payload,
        }
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut s = String::new();
        s.push_str("use_case_id=");
        s.push_str(&self.use_case_id);
        s.push_str(";tenant_id=");
        s.push_str(&self.tenant_id);
        s.push_str(";status=");
        s.push_str(match self.status {
            ResponseStatus::Ok => "ok",
            ResponseStatus::Denied => "denied",
            ResponseStatus::NotFound => "not_found",
            ResponseStatus::Error => "error",
        });
        s.push_str(";payload=");
        for (k, v) in &self.canonical_payload {
            s.push_str(k);
            s.push('=');
            s.push_str(v);
            s.push(',');
        }
        s.into_bytes()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestRequest {
    pub method: String,                    // data_class: INTERNAL_ONLY
    pub path: String,                      // data_class: INTERNAL_ONLY
    pub use_case_id: String,               // data_class: INTERNAL_ONLY
    pub tenant_id: String,                 // data_class: INTERNAL_ONLY
    pub payload: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
}

impl UseCaseRequest for RestRequest {
    fn use_case_id(&self) -> &str {
        &self.use_case_id
    }
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    fn payload(&self) -> &BTreeMap<String, String> {
        &self.payload
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestResponse {
    pub status_code: u16,               // data_class: INTERNAL_ONLY
    pub use_case_id: String,            // data_class: INTERNAL_ONLY
    pub status: ResponseStatus,         // data_class: INTERNAL_ONLY
    pub body: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
}

impl UseCaseResponse for RestResponse {
    fn use_case_id(&self) -> &str {
        &self.use_case_id
    }
    fn status(&self) -> ResponseStatus {
        self.status
    }
    fn body(&self) -> &BTreeMap<String, String> {
        &self.body
    }
}

/// Shared by every transport's parity test, so a difference between two
/// transports' events comes from the transport and not from its input.
pub fn fixture_request_payload() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("account_id".into(), "acct-001".into());
    m.insert("session_id".into(), "sess-042".into());
    m
}

pub fn fixture_use_case_id() -> &'static str {
    "foundry.account.view"
}

pub fn fixture_tenant_id() -> &'static str {
    "tenant-alpha"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_request_implements_use_case_request() {
        let req = RestRequest {
            method: "GET".into(),
            path: "/v1/accounts/acct-001".into(),
            use_case_id: fixture_use_case_id().into(),
            tenant_id: fixture_tenant_id().into(),
            payload: fixture_request_payload(),
        };
        assert_eq!(req.use_case_id(), "foundry.account.view");
        assert_eq!(req.tenant_id(), "tenant-alpha");
        assert_eq!(req.payload().len(), 2);
    }

    #[test]
    fn audit_event_is_deterministic() {
        let req = RestRequest {
            method: "GET".into(),
            path: "/v1/accounts/acct-001".into(),
            use_case_id: fixture_use_case_id().into(),
            tenant_id: fixture_tenant_id().into(),
            payload: fixture_request_payload(),
        };
        let res = RestResponse {
            status_code: 200,
            use_case_id: fixture_use_case_id().into(),
            status: ResponseStatus::Ok,
            body: BTreeMap::new(),
        };
        let ev1 = AuditEvent::canonical(&req, &res);
        let ev2 = AuditEvent::canonical(&req, &res);
        assert_eq!(ev1, ev2);
        assert_eq!(ev1.canonical_bytes(), ev2.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_sort_payload() {
        let req = RestRequest {
            method: "GET".into(),
            path: "/v1/accounts/acct-001".into(),
            use_case_id: fixture_use_case_id().into(),
            tenant_id: fixture_tenant_id().into(),
            payload: fixture_request_payload(),
        };
        let res = RestResponse {
            status_code: 200,
            use_case_id: fixture_use_case_id().into(),
            status: ResponseStatus::Ok,
            body: BTreeMap::new(),
        };
        let ev = AuditEvent::canonical(&req, &res);
        let bytes = ev.canonical_bytes();
        let s = String::from_utf8(bytes).unwrap();
        // account_id sorts before session_id
        let i = s.find("account_id").unwrap();
        let j = s.find("session_id").unwrap();
        assert!(i < j);
    }
}
