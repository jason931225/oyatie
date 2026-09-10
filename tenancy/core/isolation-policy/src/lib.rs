#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

/// A tenant-bound table that MUST have RLS FORCE applied.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TenantBoundTable {
    pub schema: String,
    pub table: String,
    pub tenant_column: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RlsPolicy {
    pub table: TenantBoundTable,
    pub policy_name: String,
    pub using_expr: String,
    pub check_expr: String,
}

pub trait RlsInstaller {
    fn install(&self, policy: &RlsPolicy) -> Result<(), IsolationKernelError>;
    fn verify(&self, policy: &RlsPolicy) -> Result<bool, IsolationKernelError>;
}

pub trait JwtIssuer {
    fn issue(&self, claims: &[(String, String)]) -> Result<String, IsolationKernelError>;
}

pub trait JwtVerifier {
    fn verify(&self, token: &str) -> Result<Vec<(String, String)>, IsolationKernelError>;
}

pub trait SigningKeyStore {
    fn current_key_fingerprint(&self) -> Result<String, IsolationKernelError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IsolationKernelError {
    PolicyMalformed,
    InstallFailed,
    VerifyFailed,
    JwtSignFailed,
    JwtVerifyFailed,
    KeyStoreUnavailable,
}
