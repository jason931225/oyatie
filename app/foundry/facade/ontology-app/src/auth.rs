use crate::authz::Caller;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorCredential {
    pub token: String,        // data_class: SECRET
    pub tenant_id: String,    // data_class: TENANT_SCOPED
    pub principal_id: String, // data_class: TENANT_SCOPED
    /// Roles this credential carries. An empty list is meaningful: the
    /// caller is recognized but reaches no permit, so policy denies it.
    pub roles: Vec<String>, // data_class: TENANT_SCOPED
}

impl OperatorCredential {
    fn caller(&self) -> Caller {
        Caller {
            tenant_id: self.tenant_id.clone(),
            principal_id: self.principal_id.clone(),
            roles: self.roles.clone(),
        }
    }
}

pub fn bearer_token(header: Option<&str>) -> Option<&str> {
    header?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

/// Constant time with respect to the ROSTER: the loop deliberately has no
/// `break` and assigns `found` instead of returning, so the work done does
/// not reveal a matching credential's position. An early return would be a
/// timing oracle, not an optimization.
pub fn authenticate(operators: &[OperatorCredential], presented: &str) -> Option<Caller> {
    let mut found: Option<&OperatorCredential> = None;
    for operator in operators {
        if constant_time_eq(operator.token.as_bytes(), presented.as_bytes()) {
            found = Some(operator);
        }
    }
    found.map(OperatorCredential::caller)
}

/// Byte equality that always inspects the full length of the longer input.
/// Length inequality is folded into the accumulator rather than returning
/// early, so neither the length nor any prefix is observable through
/// timing.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = (left.len() ^ right.len()) as u8;
    let width = left.len().max(right.len());
    for index in 0..width {
        let l = left.get(index).copied().unwrap_or(0);
        let r = right.get(index).copied().unwrap_or(0);
        difference |= l ^ r;
    }
    difference == 0
}
