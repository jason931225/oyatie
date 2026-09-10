//! Tracing pipeline bootstrap (structured JSON logs; K8s-native stdout).

/// Idempotent, so concurrent tests can each call it without racing.
pub fn init() {
    let _ = tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}
