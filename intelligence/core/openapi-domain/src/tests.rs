use std::collections::BTreeMap;

use super::*;

mod additional_operation;
mod contract_mirror;
mod data_class;
mod document;
mod fixture;
mod ingress;
mod parameter;
mod runtime_fixture;
mod runtime_parity;
mod runtime_source;
mod runtime_status;
mod schema_annotation;
mod schema_composition;
mod schema_parity;
mod schema_serde_attribute;
mod schema_serde_rename;

use self::fixture::*;
use self::runtime_fixture::*;

fn document(path: &str, contents: &str) -> OpenApiDocument {
    OpenApiDocument {
        path: path.into(),
        contents: contents.into(),
    }
}

fn mirror_location(contract_id: &str, location: &str) -> OpenApiContractMirrorLocation {
    OpenApiContractMirrorLocation {
        contract_id: contract_id.into(),
        location: location.into(),
    }
}

fn runtime_binding() -> OpenApiRuntimeBinding {
    OpenApiRuntimeBinding {
        operation_id: "invokeCapability".into(),
        contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
        runtime_crate: "intelligence-api".into(),
        source_path: "crates/intelligence-api/src/lib.rs".into(),
        symbol: "invoke_capability_from_api".into(),
        status_type: "CapabilityInvokeApiStatus".into(),
        evidence_surface: "foundry.capability.invoke".into(),
        test_path: "crates/intelligence-api/tests/capability_invoke_api.rs".into(),
        response_schemas: response_schemas(),
    }
}

fn response_schemas() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("202".into(), "CapabilityInvokeApiSuccessResponse".into()),
        ("400".into(), "CapabilityInvokeApiErrorResponse".into()),
        ("403".into(), "CapabilityInvokeApiErrorResponse".into()),
    ])
}

fn copy_runtime_binding(response_schema: &str) -> OpenApiRuntimeBinding {
    OpenApiRuntimeBinding {
        operation_id: "copyResource".into(),
        contract_path: "contracts/openapi/foundry/copy-v1.yaml".into(),
        runtime_crate: "intelligence-api".into(),
        source_path: "crates/intelligence-api/src/lib.rs".into(),
        symbol: "copy_resource_from_api".into(),
        status_type: "CopyApiStatus".into(),
        evidence_surface: "foundry.copy".into(),
        test_path: "crates/intelligence-api/tests/copy_api.rs".into(),
        response_schemas: BTreeMap::from([("200".into(), response_schema.into())]),
    }
}

fn schema_bindings() -> Vec<OpenApiSchemaBinding> {
    vec![
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvocationRequest".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "foundation-app".into(),
            source_path: "crates/foundation-app/src/lib.rs".into(),
            rust_struct: "CapabilityInvocationRequest".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvocationReceipt".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "foundation-app".into(),
            source_path: "crates/foundation-app/src/lib.rs".into(),
            rust_struct: "InvocationReceipt".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvokeApiSuccessResponse".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "intelligence-api".into(),
            source_path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiSuccessResponse".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvokeApiResponseMetadata".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "intelligence-api".into(),
            source_path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiResponseMetadata".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvokeApiErrorResponse".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "intelligence-api".into(),
            source_path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiErrorResponse".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvokeApiErrorBody".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "intelligence-api".into(),
            source_path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiErrorBody".into(),
        },
        OpenApiSchemaBinding {
            schema_name: "CapabilityInvokeApiErrorDetail".into(),
            contract_path: "contracts/openapi/foundry/capability-v1.yaml".into(),
            runtime_crate: "intelligence-api".into(),
            source_path: "crates/intelligence-api/src/lib.rs".into(),
            rust_struct: "CapabilityInvokeApiErrorDetail".into(),
        },
    ]
}

fn runtime_source(path: &str, contents: &str) -> OpenApiRuntimeSource {
    OpenApiRuntimeSource {
        path: path.into(),
        contents: contents.into(),
    }
}

fn schema_runtime_sources() -> Vec<OpenApiRuntimeSource> {
    vec![
        runtime_source("crates/foundation-app/src/lib.rs", RUNTIME_STRUCTS),
        runtime_source("crates/intelligence-api/src/lib.rs", RUNTIME_API),
    ]
}

fn invalid_runtime_status_type_reason(source: &str) -> String {
    match validate_openapi_runtime_parity(
        [document(
            "contracts/openapi/foundry/capability-v1.yaml",
            VALID,
        )],
        [runtime_binding()],
        [runtime_source("crates/intelligence-api/src/lib.rs", source)],
        [runtime_source(
            "crates/intelligence-api/tests/capability_invoke_api.rs",
            RUNTIME_API_TEST,
        )],
    ) {
        Err(OpenApiSourceError::InvalidRuntimeStatusType { reason, .. }) => reason,
        other => panic!("expected InvalidRuntimeStatusType, got {other:?}"),
    }
}

fn replace_response_block(replacement: &str) -> String {
    let start = VALID
        .find("      responses:\n")
        .expect("fixture has responses block");
    let end = VALID
        .find("components:\n")
        .expect("fixture has components block");
    format!("{}{}{}", &VALID[..start], replacement, &VALID[end..])
}

const VALID: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/capability-v1.yaml"));
