use crate::error::OpenApiSourceError;
use crate::parameter::OperationParameter;
use crate::path_validation::operation_requires_mutating_ingress;
use crate::security::{BEARER_SECURITY_SCHEME, operation_security_requires};
use crate::yaml::LogicalLine;
const REQUIRED_MUTATING_HEADERS: [&str; 3] = ["X-Request-Id", "X-Tenant-Id", "Idempotency-Key"];

pub(crate) fn validate_mutating_operation_ingress(
    document_path: &str,
    api_path: &str,
    method: &str,
    lines: &[LogicalLine],
    range: std::ops::Range<usize>,
    parameters: &[OperationParameter],
) -> Result<(), OpenApiSourceError> {
    if !operation_requires_mutating_ingress(method) {
        return Ok(());
    }
    if !operation_security_requires(lines, range.clone(), BEARER_SECURITY_SCHEME) {
        return Err(OpenApiSourceError::MissingOperationSecurity {
            path: document_path.into(),
            api_path: api_path.into(),
            method: method.into(),
            scheme: BEARER_SECURITY_SCHEME.into(),
        });
    }

    if parameters.iter().any(|parameter| {
        parameter.name == "Authorization" && parameter.location.as_deref() == Some("header")
    }) {
        return Err(OpenApiSourceError::ForbiddenAuthorizationParameter {
            path: document_path.into(),
            api_path: api_path.into(),
            method: method.into(),
        });
    }

    for header in REQUIRED_MUTATING_HEADERS {
        let Some(parameter) = parameters.iter().find(|parameter| {
            parameter.name == header && parameter.location.as_deref() == Some("header")
        }) else {
            return Err(OpenApiSourceError::MissingRequiredHeaderParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                header: header.into(),
            });
        };
        if parameter.required.as_deref() != Some("true") {
            return Err(OpenApiSourceError::InvalidHeaderParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                header: header.into(),
                reason: "header parameter must be required: true".into(),
            });
        }
        if parameter.schema_type.as_deref() != Some("string") {
            return Err(OpenApiSourceError::InvalidHeaderParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                header: header.into(),
                reason: "header parameter schema type must be string".into(),
            });
        }
        if parameter.min_length.as_deref() != Some("1") {
            return Err(OpenApiSourceError::InvalidHeaderParameter {
                path: document_path.into(),
                api_path: api_path.into(),
                method: method.into(),
                header: header.into(),
                reason: "header parameter schema must declare minLength: 1".into(),
            });
        }
    }
    Ok(())
}
