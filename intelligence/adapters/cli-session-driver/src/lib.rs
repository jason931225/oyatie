//! Provider-CLI session driver: one [`SessionDriver`] for every provider
//! driven by spawning a vendor CLI as a subprocess, differing only in the
//! values on [`CliDriverSpec`].
//!
//! Spawning a CLI contradicts the cloud-native-API doctrine for this
//! capability; one implementation means one `spawn_for_message` to replace.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::process::Stdio;
use tokio::process::Command;

use intelligence_account_domain::SecretStorePort;
use intelligence_supervisor_kernel::{
    DriverHealth, ProviderFamily, SessionDriver, SessionTicket, SpawnedSession, SupervisorError,
};

/// Everything that differs between one provider CLI and the next.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CliDriverSpec {
    pub family: ProviderFamily,
    /// Executable spawned once per message.
    pub program: &'static str,
    /// Environment variable the resolved secret is handed to the child through.
    pub api_key_env: &'static str,
    /// Prefix of the synthesised session id, and of the placeholder drain payload.
    pub session_prefix: &'static str,
    /// When false the flag is omitted, so two calls carrying the same
    /// request id are two independent invocations.
    pub idempotency_key_flag: bool,
}

impl CliDriverSpec {
    pub const CLAUDE: Self = Self {
        family: ProviderFamily::Claude,
        program: "claude-code",
        api_key_env: "ANTHROPIC_API_KEY",
        session_prefix: "claude",
        idempotency_key_flag: true,
    };

    pub const CODEX: Self = Self {
        family: ProviderFamily::OpenAiOrCodex,
        program: "codex",
        api_key_env: "OPENAI_API_KEY",
        session_prefix: "codex",
        idempotency_key_flag: true,
    };

    pub const GEMINI: Self = Self {
        family: ProviderFamily::Gemini,
        program: "gemini",
        api_key_env: "GOOGLE_API_KEY",
        session_prefix: "gemini",
        idempotency_key_flag: false,
    };
}

/// A `SessionDriver` that spawns the vendor CLI described by its [`CliDriverSpec`].
pub struct CliSessionDriver<S> {
    spec: CliDriverSpec,
    secrets: S,
}

impl<S> CliSessionDriver<S> {
    pub fn new(spec: CliDriverSpec, secrets: S) -> Self {
        Self { spec, secrets }
    }

    pub fn claude(secrets: S) -> Self {
        Self::new(CliDriverSpec::CLAUDE, secrets)
    }

    pub fn codex(secrets: S) -> Self {
        Self::new(CliDriverSpec::CODEX, secrets)
    }

    pub fn gemini(secrets: S) -> Self {
        Self::new(CliDriverSpec::GEMINI, secrets)
    }

    pub fn spec(&self) -> CliDriverSpec {
        self.spec
    }
}

impl<S: SecretStorePort + Send + Sync> SessionDriver for CliSessionDriver<S> {
    fn provider_family(&self) -> ProviderFamily {
        self.spec.family
    }

    fn spawn_for_message(&self, ticket: &SessionTicket) -> Result<SpawnedSession, SupervisorError> {
        let material = self.secrets.get(&ticket.secret_ref).map_err(|e| {
            SupervisorError::DriverError(format!("secret resolution failed: {:?}", e))
        })?;

        let mut command = Command::new(self.spec.program);
        command.arg("--message-id").arg(&ticket.message_id.0);
        if self.spec.idempotency_key_flag {
            command.arg("--idempotency-key").arg(&ticket.request_id.0);
        }
        let _child = command
            .env(
                self.spec.api_key_env,
                String::from_utf8_lossy(material.expose_for_provider_call()).into_owned(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                SupervisorError::DriverError(format!(
                    "failed to spawn {}: {}",
                    self.spec.program, e
                ))
            })?;

        Ok(SpawnedSession {
            session_id: format!("{}-{}", self.spec.session_prefix, ticket.message_id.0),
            account_id: ticket.account_id.clone(),
            message_id: ticket.message_id.clone(),
        })
    }

    /// Discards `_msg` and reports success; nothing reaches the child.
    fn inject_message(
        &self,
        _session: &SpawnedSession,
        _msg: &[u8],
    ) -> Result<(), SupervisorError> {
        Ok(())
    }

    /// Returns a synthesised payload; the child's stdout is never read.
    fn drain_response(&self, _session: &SpawnedSession) -> Result<Vec<u8>, SupervisorError> {
        Ok(format!("{} response placeholder", self.spec.session_prefix).into_bytes())
    }

    /// Reports success without signalling the child, which keeps running.
    fn kill(&self, _session: &SpawnedSession) -> Result<(), SupervisorError> {
        Ok(())
    }

    fn health_check(&self) -> DriverHealth {
        DriverHealth::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guards against one provider's behaviour being adopted for all three.
    #[test]
    fn spec_table_preserves_per_provider_behaviour() {
        for (spec, family, program, env, prefix, idempotent) in [
            (
                CliDriverSpec::CLAUDE,
                ProviderFamily::Claude,
                "claude-code",
                "ANTHROPIC_API_KEY",
                "claude",
                true,
            ),
            (
                CliDriverSpec::CODEX,
                ProviderFamily::OpenAiOrCodex,
                "codex",
                "OPENAI_API_KEY",
                "codex",
                true,
            ),
            (
                CliDriverSpec::GEMINI,
                ProviderFamily::Gemini,
                "gemini",
                "GOOGLE_API_KEY",
                "gemini",
                false,
            ),
        ] {
            assert_eq!(spec.family, family, "{program}: provider family");
            assert_eq!(spec.program, program, "{program}: executable");
            assert_eq!(spec.api_key_env, env, "{program}: api key env var");
            assert_eq!(spec.session_prefix, prefix, "{program}: session prefix");
            assert_eq!(
                spec.idempotency_key_flag, idempotent,
                "{program}: idempotency-key support"
            );
        }
    }

    /// The same wrong value pasted twice passes every per-constant assertion
    /// above, and routes two providers at one CLI.
    #[test]
    fn specs_are_pairwise_distinct() {
        let specs = [
            CliDriverSpec::CLAUDE,
            CliDriverSpec::CODEX,
            CliDriverSpec::GEMINI,
        ];
        for (i, a) in specs.iter().enumerate() {
            for b in &specs[i + 1..] {
                assert_ne!(a.family, b.family, "duplicate provider family");
                assert_ne!(a.program, b.program, "duplicate executable");
                assert_ne!(a.api_key_env, b.api_key_env, "duplicate api key env var");
                assert_ne!(a.session_prefix, b.session_prefix, "duplicate prefix");
            }
        }
    }
}
