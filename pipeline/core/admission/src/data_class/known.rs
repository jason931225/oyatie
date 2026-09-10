//! The closed, path-keyed sets this rule grandfathers.
//!
//! Both are keyed by declaration site, so relocating an entry drops its
//! grandfather and goes red rather than laundering it into a fresh path.

pub(super) const CANONICAL_CRATE: &str = "data/core/data-boundary-kernel/";

/// Every `DataClass`-shaped enum that already stood outside the canonical
/// crate when this rule landed. The set is closed and keyed by declaration
/// site, so relocating one of these drops its grandfather and goes red rather
/// than laundering the declaration into a fresh path.
pub(super) const GRANDFATHERED: &[(&str, &str)] = &[
    ("app/foundry/core/edits/src/property.rs", "WireDataClass"),
    (
        "audit/core/retention-cascade-domain/src/lib.rs",
        "DataClass",
    ),
    (
        "cell/core/regional-pack/src/kr_regulatory.rs",
        "PipaDataClassification",
    ),
    (
        "data/facade/pipeline-lineage-replay-service/src/domain/mod.rs",
        "DataClass",
    ),
    (
        "data/facade/warehouse-tenant-olap-service/src/domain/mod.rs",
        "DataClass",
    ),
    ("iam/ports/tenant-rbac-api/src/lib.rs", "DataClassDto"),
    (
        "intelligence/core/assist-draft-kernel/src/lib.rs",
        "AssistDraftDataClass",
    ),
    (
        "intelligence/core/attribution-kernel/src/lib.rs",
        "AttributionDataClass",
    ),
    (
        "intelligence/core/context-aware-retrieval-kernel/src/lib.rs",
        "ContextDataClass",
    ),
    (
        "intelligence/core/guardrails-domain/src/lib.rs",
        "GuardrailDataClass",
    ),
    (
        "intelligence/core/kernel/src/safety.rs",
        "EvidenceDataClass",
    ),
    (
        "intelligence/core/model-routing-kernel/src/lib.rs",
        "IntelligenceDataClass",
    ),
    (
        "secrets/core/kms-operator-kernel/src/lib.rs",
        "DataClassLabel",
    ),
    (
        "tenancy/core/cell-assignment/src/transfer_authority.rs",
        "TransferDataClassV1",
    ),
];

/// Silent primitives that already stood inside a classified struct, as
/// `(path, "Struct.field")`. Every one outside `cell/core/regional-pack` sits
/// in a file past the 300-line budget, so touching it to add the missing class
/// costs the lane a budget refusal. Recorded here so the live set stays empty
/// and a NEW hole is refused.
pub(super) const GRANDFATHERED_HOLES: &[(&str, &str)] = &[
    (
        "app/application/facade/application-app/src/lib.rs",
        "ObjectPropertyInput.name",
    ),
    (
        "app/application/facade/application-app/src/lib.rs",
        "ObjectPropertyInput.value",
    ),
    (
        "app/application/facade/application-app/src/lib.rs",
        "CapabilityRegistration.capability_id",
    ),
    (
        "app/application/facade/application-app/src/lib.rs",
        "CapabilityRegistration.namespace",
    ),
    (
        "app/application/facade/application-app/src/lib.rs",
        "CapabilityRegistration.evidence_topic",
    ),
    (
        "cell/core/regional-pack/src/kr_regulatory.rs",
        "KrRegulatoryBinding.pack_id",
    ),
    (
        "cell/core/regional-pack/src/kr_regulatory.rs",
        "KrRegulatoryBinding.csap_evidence_ref",
    ),
    (
        "intelligence/core/adapter-kernel/src/lib.rs",
        "InvocationPolicy.max_latency_ms",
    ),
    (
        "intelligence/core/capability-domain/src/lib.rs",
        "Capability.id",
    ),
    (
        "intelligence/core/evidence-domain/src/lib.rs",
        "EvidenceHashInput.timestamp_epoch_seconds",
    ),
    (
        "intelligence/core/mcp-gateway-domain/src/lib.rs",
        "McpAuthorizationChallenge.status_code",
    ),
];
