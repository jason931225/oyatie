use super::*;

#[test]
fn accepts_contract_mirror_when_spec_and_machine_reference_source() {
    assert_eq!(
        validate_openapi_contract_mirror(
            ["contracts/openapi/foundry/capability-v1.yaml"],
            "Reference: contracts/openapi/foundry/capability-v1.yaml",
            [mirror_location(
                "CAPABILITY_INVOCATION",
                "crates/foundation-app + contracts/openapi/foundry/capability-v1.yaml",
            )],
        ),
        Ok(OpenApiContractMirrorReport {
            contracts_checked: 1,
            spec_references_checked: 1,
            mirror_references_checked: 1,
        })
    );
}

#[test]
fn rejects_contract_mirror_missing_spec_or_machine_reference() {
    assert_eq!(
        validate_openapi_contract_mirror(
            ["contracts/openapi/foundry/capability-v1.yaml"],
            "No exact contract path here.",
            [mirror_location(
                "CAPABILITY_INVOCATION",
                "contracts/openapi/foundry/capability-v1.yaml",
            )],
        ),
        Err(OpenApiSourceError::MissingSpecMirror {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        })
    );

    assert_eq!(
        validate_openapi_contract_mirror(
            ["contracts/openapi/foundry/capability-v1.yaml"],
            "contracts/openapi/foundry/capability-v1.yaml",
            [mirror_location(
                "CAPABILITY_INVOCATION",
                "crates/foundation-app"
            )],
        ),
        Err(OpenApiSourceError::MissingMachineMirror {
            path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        })
    );
}

#[test]
fn rejects_stale_exact_spec_or_machine_contract_paths() {
    assert_eq!(
        validate_openapi_contract_mirror(
            ["contracts/openapi/foundry/capability-v1.yaml"],
            "contracts/openapi/foundry/capability-v1.yaml contracts/openapi/foundry/missing-v1.yaml",
            [mirror_location(
                "CAPABILITY_INVOCATION",
                "contracts/openapi/foundry/capability-v1.yaml",
            )],
        ),
        Err(OpenApiSourceError::StaleSpecMirror {
            path: "contracts/openapi/foundry/missing-v1.yaml".into(),
        })
    );

    assert_eq!(
        validate_openapi_contract_mirror(
            ["contracts/openapi/foundry/capability-v1.yaml"],
            "contracts/openapi/foundry/capability-v1.yaml",
            [
                mirror_location(
                    "CAPABILITY_INVOCATION",
                    "contracts/openapi/foundry/capability-v1.yaml",
                ),
                mirror_location("STALE", "contracts/openapi/foundry/missing-v1.yaml"),
            ],
        ),
        Err(OpenApiSourceError::StaleMachineMirror {
            contract_id: "STALE".into(),
            path: "contracts/openapi/foundry/missing-v1.yaml".into(),
        })
    );
}
