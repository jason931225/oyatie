use super::*;

pub type HyperHttpsConnector = hyper_rustls::HttpsConnector<HttpConnector>;

pub type HyperHttpsClient = Client<HyperHttpsConnector, Full<Bytes>>;

/// X25519MLKEM768 is explicitly first so Buck2 and Cargo cannot diverge on
/// feature unification; X25519 remains present as the classical fallback.
#[must_use]
pub fn pqc_hybrid_aws_lc_provider() -> rustls::crypto::CryptoProvider {
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    provider.kx_groups = vec![
        rustls::crypto::aws_lc_rs::kx_group::X25519MLKEM768,
        rustls::crypto::aws_lc_rs::kx_group::X25519,
        rustls::crypto::aws_lc_rs::kx_group::SECP256R1,
        rustls::crypto::aws_lc_rs::kx_group::SECP384R1,
    ];
    provider
}

#[must_use]
pub fn pqc_hybrid_kx_group_names() -> Vec<rustls::NamedGroup> {
    pqc_hybrid_aws_lc_provider()
        .kx_groups
        .iter()
        .map(|group| group.name())
        .collect()
}

#[must_use]
pub fn pqc_hybrid_tls13_client_config_builder()
-> rustls::ConfigBuilder<rustls::ClientConfig, rustls::WantsVerifier> {
    rustls::ClientConfig::builder_with_provider(Arc::new(pqc_hybrid_aws_lc_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("static aws-lc-rs TLS 1.3 PQC-hybrid client provider must be valid")
}

#[must_use]
pub fn pqc_hybrid_tls13_server_config_builder()
-> rustls::ConfigBuilder<rustls::ServerConfig, rustls::WantsVerifier> {
    rustls::ServerConfig::builder_with_provider(Arc::new(pqc_hybrid_aws_lc_provider()))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("static aws-lc-rs TLS 1.3 PQC-hybrid server provider must be valid")
}

#[must_use]
pub fn pqc_hybrid_tls13_client_config() -> rustls::ClientConfig {
    pqc_hybrid_tls13_client_config_builder()
        .with_webpki_roots()
        .with_no_client_auth()
}

#[must_use]
pub fn build_pqc_hybrid_https_connector() -> HyperHttpsConnector {
    hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(pqc_hybrid_tls13_client_config())
        .https_only()
        .enable_http1()
        .enable_http2()
        .build()
}

#[must_use]
pub fn build_pqc_hybrid_https_client() -> HyperHttpsClient {
    Client::builder(TokioExecutor::new()).build(build_pqc_hybrid_https_connector())
}

/// HTTP traffic through this connector is not PQC protected and must never be
/// used as production external-endpoint evidence.
#[doc(hidden)]
#[must_use]
pub fn build_loopback_http_or_pqc_hybrid_https_connector_for_tests() -> HyperHttpsConnector {
    hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(pqc_hybrid_tls13_client_config())
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build()
}

#[doc(hidden)]
#[must_use]
pub fn build_loopback_http_or_pqc_hybrid_https_client_for_tests() -> HyperHttpsClient {
    Client::builder(TokioExecutor::new())
        .build(build_loopback_http_or_pqc_hybrid_https_connector_for_tests())
}
