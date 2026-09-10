use super::*;

pub(super) fn archive_artifact_handler(
    artifacts: Arc<BTreeMap<String, CloudIacAppArchiveArtifact>>,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> SyncHandler {
    Arc::new(move |request: HttpRequest| {
        // SECOND PEP (supply-chain): the artifact route serves module ZIP bytes,
        // so VERIFY the caller credential and PDP-authorize the DOWNLOAD surface
        // BEFORE reading any bytes.
        let credential = CallerCredential {
            authorization: request.headers.get("authorization").cloned(),
        };
        let verified = match authz_provider.verify_principal(&credential) {
            Ok(verified) => verified,
            Err(_) => return unauthorized_artifact_response(),
        };
        if authz_provider
            .ensure_authorized(&verified, CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE)
            .is_err()
        {
            return forbidden_artifact_response();
        }
        let Some(file_name) = request.path_captures.get("archive_file") else {
            return fixed_error_response(404, "artifact_not_found");
        };
        if !is_safe_artifact_request_name(file_name) {
            return fixed_error_response(400, "invalid_artifact_name");
        }
        let Some(artifact) = artifacts.get(file_name) else {
            return fixed_error_response(404, "artifact_not_found");
        };
        match fs::read(&artifact.archive_file) {
            Ok(bytes) => {
                if sha256_hex(&bytes) != artifact.archive_sha256 {
                    return fixed_error_response(409, "artifact_digest_mismatch");
                }
                HttpResponse::new(200)
                    .with_header("content-type", artifact.media_type.clone())
                    .with_body(bytes)
            }
            Err(_) => fixed_error_response(404, "artifact_not_found"),
        }
    })
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

pub(super) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

pub(super) fn is_safe_artifact_request_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name != "."
        && file_name != ".."
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains('?')
        && !file_name.contains('#')
        && file_name.ends_with(".zip")
        && file_name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '.'))
}

pub(super) fn register_health_routes(router: &mut Router<SyncHandler>) -> Result<(), RouterError> {
    router.route(
        HttpMethod::Get,
        CLOUD_IAC_HEALTH_PATH,
        fixed_json_handler(r#"{"status":"ok","service":"iac-app","check":"healthz"}"#),
    )?;
    router.route(
        HttpMethod::Get,
        CLOUD_IAC_LIVENESS_PATH,
        fixed_json_handler(r#"{"status":"ok","service":"iac-app","check":"livez"}"#),
    )?;
    Ok(())
}

pub(super) fn fixed_error_response(status: u16, code: &'static str) -> HttpResponse {
    HttpResponse::new(status)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{"error":"{code}"}}"#).into_bytes())
}

pub(super) fn unauthorized_artifact_response() -> HttpResponse {
    HttpResponse::new(401)
        .with_header("content-type", "application/json")
        .with_header("www-authenticate", "Bearer")
        .with_body(br#"{"error":"unauthorized"}"#.to_vec())
}

pub(super) fn forbidden_artifact_response() -> HttpResponse {
    fixed_error_response(403, "forbidden")
}

pub(super) fn fixed_json_handler(body: &'static str) -> SyncHandler {
    Arc::new(move |_request: HttpRequest| {
        HttpResponse::new(200)
            .with_header("content-type", "application/json")
            .with_body(body.as_bytes().to_vec())
    })
}
