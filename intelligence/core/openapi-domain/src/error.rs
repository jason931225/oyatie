#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenApiSourceError {
    NoDocuments,
    InvalidPath {
        path: String,
        reason: String,
    },
    DuplicateDocument {
        path: String,
    },
    EmptyDocument {
        path: String,
    },
    MissingTopLevelField {
        path: String,
        field: &'static str,
    },
    UnsupportedOpenApiVersion {
        path: String,
        version: String,
    },
    MissingInfoField {
        path: String,
        field: &'static str,
    },
    InvalidInfoVersion {
        path: String,
        version: String,
    },
    VersionSuffixMismatch {
        path: String,
        path_major: u64,
        info_major: u64,
    },
    MissingPathItem {
        path: String,
    },
    MissingOperation {
        path: String,
        api_path: String,
    },
    AdditionalOperationFixedMethodCollision {
        path: String,
        api_path: String,
        method: String,
    },
    MissingOperationId {
        path: String,
        api_path: String,
        method: String,
    },
    MissingResponses {
        path: String,
        api_path: String,
        method: String,
    },
    MissingSpecMirror {
        path: String,
    },
    MissingMachineMirror {
        path: String,
    },
    StaleSpecMirror {
        path: String,
    },
    StaleMachineMirror {
        contract_id: String,
        path: String,
    },
    MissingDataClassAnnotation {
        path: String,
        location: String,
    },
    InvalidDataClassAnnotation {
        path: String,
        location: String,
        data_class: String,
    },
    MissingBearerSecurityScheme {
        path: String,
    },
    InvalidBearerSecurityScheme {
        path: String,
        reason: String,
    },
    MissingOperationSecurity {
        path: String,
        api_path: String,
        method: String,
        scheme: String,
    },
    ForbiddenAuthorizationParameter {
        path: String,
        api_path: String,
        method: String,
    },
    MissingRequiredHeaderParameter {
        path: String,
        api_path: String,
        method: String,
        header: String,
    },
    InvalidHeaderParameter {
        path: String,
        api_path: String,
        method: String,
        header: String,
        reason: String,
    },
    MissingPathTemplateParameter {
        path: String,
        api_path: String,
        method: String,
        parameter: String,
    },
    InvalidPathTemplateParameter {
        path: String,
        api_path: String,
        method: String,
        parameter: String,
        reason: String,
    },
    DuplicateOperationId {
        operation_id: String,
        first_path: String,
        second_path: String,
    },
    MissingRuntimeBinding {
        operation_id: String,
        contract_path: String,
    },
    DuplicateRuntimeBinding {
        operation_id: String,
    },
    StaleRuntimeBinding {
        operation_id: String,
        contract_path: String,
    },
    NonExplicitRuntimeResponseKey {
        operation_id: String,
        contract_path: String,
        response_key: String,
    },
    InvalidRuntimeBinding {
        operation_id: String,
        field: &'static str,
        reason: String,
    },
    DuplicateRuntimeSource {
        path: String,
    },
    DuplicateRuntimeTest {
        path: String,
    },
    MissingRuntimeSource {
        operation_id: String,
        path: String,
    },
    MissingRuntimeTest {
        operation_id: String,
        path: String,
    },
    MissingRuntimeSymbol {
        operation_id: String,
        path: String,
        symbol: String,
    },
    MissingRuntimeEvidenceSurface {
        operation_id: String,
        path: String,
        evidence_surface: String,
    },
    MissingRuntimeStatusType {
        operation_id: String,
        path: String,
        status_type: String,
    },
    InvalidRuntimeStatusType {
        operation_id: String,
        path: String,
        status_type: String,
        reason: String,
    },
    MissingRuntimeTestCoverage {
        operation_id: String,
        test_path: String,
        symbol: String,
        evidence_surface: String,
    },
    MissingRuntimeResponseStatus {
        operation_id: String,
        path: String,
        status_type: String,
        status: String,
    },
    UndocumentedRuntimeResponseStatus {
        operation_id: String,
        path: String,
        status_type: String,
        status: String,
    },
    MissingRuntimeTestResponseStatus {
        operation_id: String,
        test_path: String,
        status_type: String,
        status: String,
    },
    MissingRuntimeResponseSchema {
        operation_id: String,
        contract_path: String,
        status: String,
    },
    RuntimeResponseSchemaMismatch {
        operation_id: String,
        contract_path: String,
        status: String,
        expected_schema: String,
        actual_schema: String,
    },
    MissingSchemaBinding {
        schema_name: String,
        contract_path: String,
    },
    DuplicateSchemaBinding {
        schema_name: String,
        contract_path: String,
    },
    StaleSchemaBinding {
        schema_name: String,
        contract_path: String,
    },
    InvalidSchemaBinding {
        schema_name: String,
        field: &'static str,
        reason: String,
    },
    MissingSchemaRuntimeSource {
        schema_name: String,
        path: String,
    },
    MissingRuntimeStruct {
        schema_name: String,
        path: String,
        rust_struct: String,
    },
    InvalidRuntimeStruct {
        schema_name: String,
        path: String,
        rust_struct: String,
        reason: String,
    },
    SchemaFieldMismatch {
        schema_name: String,
        contract_path: String,
        missing_properties: Vec<String>,
        extra_properties: Vec<String>,
    },
    SchemaRequiredMismatch {
        schema_name: String,
        contract_path: String,
        missing_required: Vec<String>,
        extra_required: Vec<String>,
    },
    SchemaTypeMismatch {
        schema_name: String,
        contract_path: String,
        mismatches: Vec<String>,
    },
}

pub(crate) fn invalid_path<T>(path: &str, reason: &str) -> Result<T, OpenApiSourceError> {
    Err(OpenApiSourceError::InvalidPath {
        path: path.into(),
        reason: reason.into(),
    })
}
