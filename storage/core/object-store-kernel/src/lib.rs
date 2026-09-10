//! Deprecated source-compatibility crate for the historical CAS package
//! identity. New storage code imports [`storage_domain::cas`].

#![forbid(unsafe_code)]

pub use storage_domain::cas::*;

#[cfg(test)]
mod tests {
    use super::TenantId;

    #[test]
    fn legacy_package_reexports_the_cas_contract() {
        let tenant = TenantId::parse("ten_compat").expect("canonical tenant id parses");
        assert_eq!(tenant.as_str(), "ten_compat");
    }
}
