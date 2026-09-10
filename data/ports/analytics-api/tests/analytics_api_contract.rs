#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use data_analytics_api::{
    ANALYTICS_ASYNCAPI_CONTRACT, ANALYTICS_OPENAPI_CONTRACT, ANALYTICS_PROTO_CONTRACT, ApiError,
};
use data_analytics_usecase::UseCaseError;
use shared_olap_client_kernel::KernelError;

/// The runtime constants must equal the paths the CATALOG declares, not a second copy of
/// them written here. Hard-coding the expected strings made this test vacuous against the
/// defect it exists to catch: retargeting the catalog without a matching constant change
/// left the test green while the constants pointed at the old location. Reading the catalog
/// is what makes a retarget fail here.
#[test]
fn analytics_api_contract_runtime_constants_match_the_catalog() {
    let catalog_path = repo_root().join("data/ports/analytics-api/contracts/contracts.json");
    let catalog = std::fs::read_to_string(&catalog_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", catalog_path.display()));

    // Dependency-free extraction of every `"path": "..."` value: this crate's test target must
    // not grow a JSON dependency to assert a handful of strings.
    let declared: Vec<String> = catalog
        .split("\"path\"")
        .skip(1)
        .filter_map(|rest| {
            let rest = rest.trim_start().strip_prefix(':')?.trim_start();
            let rest = rest.strip_prefix('"')?;
            rest.find('"').map(|end| rest[..end].to_owned())
        })
        .collect();
    assert!(
        !declared.is_empty(),
        "catalog {} declared no contract paths — a vacuous pass is the failure this test \
         exists to prevent",
        catalog_path.display()
    );

    for constant in [
        ANALYTICS_OPENAPI_CONTRACT,
        ANALYTICS_ASYNCAPI_CONTRACT,
        ANALYTICS_PROTO_CONTRACT,
    ] {
        assert!(
            declared.iter().any(|path| path == constant),
            "runtime constant {constant:?} is not declared by {}; the catalog declares \
             {declared:?}. Retarget the constant and the catalog together.",
            catalog_path.display()
        );
    }
}

/// Walk up from a start dir to the repo root (buck2-safe; avoid sole reliance on
/// `CARGO_MANIFEST_DIR`, which is absent or non-source under buck).
fn repo_root() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(PathBuf::from(manifest));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    for start in candidates {
        let mut dir = start;
        for _ in 0..12 {
            if dir
                .join("data/ports/analytics-api/contracts/openapi-v1.yaml")
                .is_file()
                && dir.join("AGENTS.md").is_file()
            {
                return dir;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    panic!("could not locate repo root containing data/ports/analytics-api contracts");
}

/// Resolve one declared contract.
///
/// Under Buck2 the checkout tree is unreachable, so the file arrives as a DECLARED input and its
/// sandbox path is handed over in an environment variable by the target's `env`. Under Cargo there
/// is no such binding and the repository walk is correct. Preferring the binding keeps both engines
/// on the same assertion instead of making the Buck target a weaker variant of it.
fn declared_contract(env_key: &str, relative: &str) -> PathBuf {
    match std::env::var(env_key) {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => repo_root().join(relative),
    }
}

#[test]
fn analytics_api_contract_files_exist_under_data_root() {
    for (env_key, rel) in [
        ("DATA_ANALYTICS_OPENAPI", ANALYTICS_OPENAPI_CONTRACT),
        ("DATA_ANALYTICS_ASYNCAPI", ANALYTICS_ASYNCAPI_CONTRACT),
        ("DATA_ANALYTICS_PROTO", ANALYTICS_PROTO_CONTRACT),
    ] {
        let abs = declared_contract(env_key, rel);
        assert!(
            abs.is_file(),
            "missing analytics contract under data/ root: {rel} (resolved {abs:?})"
        );
        // The declared path must still be the data/ capability root spelling, so a Buck binding
        // cannot quietly satisfy this test with a contract from the retired location.
        assert!(
            rel.starts_with("data/ports/analytics-api/contracts/"),
            "contract constant must name the data/ capability root: {rel}"
        );
    }
    let _: &Path = repo_root().as_path();
}

#[test]
fn api_error_maps_cross_tenant_kernel_error_to_forbidden() {
    let err = ApiError::from(UseCaseError::Kernel(KernelError::CrossTenantAccessDenied));
    assert!(matches!(err, ApiError::Forbidden(_)));
}
