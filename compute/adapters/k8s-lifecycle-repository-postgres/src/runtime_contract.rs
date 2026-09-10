use std::collections::BTreeSet;

use crate::RUNTIME_ROLE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PgK8sLifecycleRuntimeContractError {
    Empty,
    InvalidRoleName,
    DuplicateRole { role: String },
    PolicyRoleCannotServe,
}

impl core::fmt::Display for PgK8sLifecycleRuntimeContractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Empty => f.write_str("at least one lifecycle serving role is required"),
            Self::InvalidRoleName => f.write_str("lifecycle serving role name is invalid"),
            Self::DuplicateRole { role } => {
                write!(
                    f,
                    "lifecycle serving role '{role}' is declared more than once"
                )
            }
            Self::PolicyRoleCannotServe => {
                f.write_str("the non-login lifecycle policy role cannot be a serving principal")
            }
        }
    }
}

impl std::error::Error for PgK8sLifecycleRuntimeContractError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgK8sLifecycleRuntimeContract {
    serving_roles: BTreeSet<String>,
}

impl PgK8sLifecycleRuntimeContract {
    pub fn new<I, S>(serving_roles: I) -> Result<Self, PgK8sLifecycleRuntimeContractError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut roles = BTreeSet::new();
        for role in serving_roles {
            let role = role.into();
            if role.is_empty() || role.trim() != role || role.contains('\0') {
                return Err(PgK8sLifecycleRuntimeContractError::InvalidRoleName);
            }
            if role == RUNTIME_ROLE {
                return Err(PgK8sLifecycleRuntimeContractError::PolicyRoleCannotServe);
            }
            if !roles.insert(role.clone()) {
                return Err(PgK8sLifecycleRuntimeContractError::DuplicateRole { role });
            }
        }
        if roles.is_empty() {
            return Err(PgK8sLifecycleRuntimeContractError::Empty);
        }
        Ok(Self {
            serving_roles: roles,
        })
    }

    pub fn serving_roles(&self) -> impl ExactSizeIterator<Item = &str> {
        self.serving_roles.iter().map(String::as_str)
    }

    pub(crate) fn contains(&self, role: &str) -> bool {
        self.serving_roles.contains(role)
    }

    pub(crate) fn owned_role_names(&self) -> Vec<String> {
        self.serving_roles.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serving_role_contract_is_nonempty_unique_and_excludes_policy_role() {
        assert_eq!(
            PgK8sLifecycleRuntimeContract::new(Vec::<String>::new()),
            Err(PgK8sLifecycleRuntimeContractError::Empty)
        );
        assert_eq!(
            PgK8sLifecycleRuntimeContract::new(["app", "app"]),
            Err(PgK8sLifecycleRuntimeContractError::DuplicateRole {
                role: "app".to_owned()
            })
        );
        assert_eq!(
            PgK8sLifecycleRuntimeContract::new([RUNTIME_ROLE]),
            Err(PgK8sLifecycleRuntimeContractError::PolicyRoleCannotServe)
        );
        let contract = PgK8sLifecycleRuntimeContract::new(["blue", "green"]).unwrap();
        assert_eq!(
            contract.serving_roles().collect::<Vec<_>>(),
            ["blue", "green"]
        );
    }
}
