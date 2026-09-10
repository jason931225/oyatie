use super::*;

pub(super) const RUNTIME_API: &str = r#"
pub const CAPABILITY_INVOKE_SURFACE: &str = "foundry.capability.invoke";

	pub enum CapabilityInvokeApiStatus {
	    Accepted,
	    BadRequest,
	    Forbidden,
	}

	impl CapabilityInvokeApiStatus {
	    pub const fn code(self) -> u16 {
	        match self {
		            Self::Accepted => 202,
		            Self::BadRequest => 400,
		            Self::Forbidden => 403,
		        }
	    }
	}

	pub struct CapabilityInvokeApiSuccessResponse {
	    pub data: CapabilityInvocationReceipt, // data_class: INTERNAL_ONLY
	    pub metadata: CapabilityInvokeApiResponseMetadata, // data_class: INTERNAL_ONLY
	}

	pub struct CapabilityInvokeApiResponseMetadata {
	    pub request_id: String, // data_class: INTERNAL_ONLY
	}

	pub struct CapabilityInvokeApiErrorResponse {
	    pub error: CapabilityInvokeApiErrorBody, // data_class: INTERNAL_ONLY
	}

	pub struct CapabilityInvokeApiErrorBody {
	    pub code: String, // data_class: INTERNAL_ONLY
	    pub message: String, // data_class: INTERNAL_ONLY
	    pub message_localized: Option<String>, // data_class: INTERNAL_ONLY
	    pub request_id: String, // data_class: INTERNAL_ONLY
	    pub details: Vec<CapabilityInvokeApiErrorDetail>, // data_class: INTERNAL_ONLY
	    pub retry_after_seconds: Option<u64>, // data_class: INTERNAL_ONLY
	}

	pub struct CapabilityInvokeApiErrorDetail {
	    pub field: String, // data_class: INTERNAL_ONLY
	    pub issue: String, // data_class: INTERNAL_ONLY
	}

	pub fn invoke_capability_from_api() {
	    let _surface = CAPABILITY_INVOKE_SURFACE;
	}
"#;

pub(super) const RUNTIME_API_TEST: &str = r#"
invoke_capability_from_api(foundation, request);
assert_eq!(CAPABILITY_INVOKE_SURFACE, "foundry.capability.invoke");
assert_eq!(CapabilityInvokeApiStatus::Accepted.code(), 202);
assert_eq!(CapabilityInvokeApiStatus::BadRequest.code(), 400);
assert_eq!(CapabilityInvokeApiStatus::Forbidden.code(), 403);

"#;

pub(super) const COPY_RUNTIME_API: &str = r#"
pub const COPY_SURFACE: &str = "foundry.copy";
pub enum CopyApiStatus { Ok }
impl CopyApiStatus { pub const fn code(self) -> u16 { match self { Self::Ok => 200 } } }
pub fn copy_resource_from_api() { let _surface = COPY_SURFACE; }
"#;

pub(super) const COPY_RUNTIME_API_TEST: &str = r#"
copy_resource_from_api(foundation, request);
assert_eq!(COPY_SURFACE, "foundry.copy");
assert_eq!(CopyApiStatus::Ok.code(), 200);
"#;

pub(super) const RUNTIME_STRUCTS: &str = r#"
pub struct CapabilityInvocationRequest {
    pub tenant_id: String, // data_class: INTERNAL_ONLY
    pub user_id: String, // data_class: INTERNAL_ONLY
    pub capability_id: String, // data_class: INTERNAL_ONLY
    pub purpose: Purpose, // data_class: INTERNAL_ONLY
    pub subject_class: SubjectClass, // data_class: INTERNAL_ONLY
    pub budget_window_id: String, // data_class: INTERNAL_ONLY
    pub projected_cost_micros: u64, // data_class: INTERNAL_ONLY
    pub started_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

pub struct InvocationReceipt {
    pub tenant_id: String, // data_class: INTERNAL_ONLY
    pub user_id: String, // data_class: INTERNAL_ONLY
    pub capability_id: String, // data_class: INTERNAL_ONLY
    pub evidence_event_hash: String, // data_class: INTERNAL_ONLY
    pub cost_reservation_id: Option<String>, // data_class: INTERNAL_ONLY
    pub cost_budget_warning: Option<BudgetWarning>, // data_class: INTERNAL_ONLY
    pub run_id: Option<String>, // data_class: INTERNAL_ONLY
    pub foundry_step_id: Option<String>, // data_class: INTERNAL_ONLY
    pub foundry_evidence_id: Option<String>, // data_class: INTERNAL_ONLY
}
"#;
