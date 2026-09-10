use super::ClaudeAgentOptions;
use crate::error::ClaudeAgentError;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxNetworkConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub denied_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_domains_only: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_unix_sockets: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_mach_lookup: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_terminate: Option<SandboxTlsTerminateConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxTlsTerminateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_key_path: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxFilesystemConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_write: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny_write: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny_read: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_read: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_read_paths_only: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxIgnoreViolations {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<String>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub other: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_unavailable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetworkConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<SandboxFilesystemConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<SandboxIgnoreViolations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_network_isolation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ripgrep: Option<SandboxRipgrepConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bwrap_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socat_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxRipgrepConfig {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_settings_merge_rejects_missing_settings_file() {
        let options = ClaudeAgentOptions {
            settings: Some("/definitely/missing/settings.json".into()),
            sandbox: Some(SandboxSettings {
                enabled: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(matches!(
            options.to_cli_args(),
            Err(ClaudeAgentError::InvalidOption(message))
                if message.contains("settings file does not exist")
        ));
    }

    #[test]
    fn sandbox_settings_preserve_current_upstream_fields() {
        let options: ClaudeAgentOptions = serde_json::from_value(serde_json::json!({
            "sandbox": {
                "enabled": true,
                "failIfUnavailable": true,
                "network": {
                    "tlsTerminate": {
                        "caCertPath": "/tmp/ca.crt",
                        "caKeyPath": "/tmp/ca.key"
                    }
                },
                "filesystem": {
                    "allowWrite": ["/workspace/out"],
                    "denyWrite": ["/workspace/secret"],
                    "denyRead": ["/workspace/private"],
                    "allowRead": ["/workspace/private/public"],
                    "allowManagedReadPathsOnly": true
                },
                "ignoreViolations": {
                    "file": ["/tmp/noisy-file"],
                    "network": ["example.com"],
                    "custom": ["opaque-upstream-category"]
                },
                "enableWeakerNetworkIsolation": true,
                "ripgrep": {
                    "command": "rg-custom",
                    "args": ["--json"]
                },
                "bwrapPath": "/usr/bin/bwrap",
                "socatPath": "/usr/bin/socat"
            }
        }))
        .unwrap();

        let args = options.to_cli_args().unwrap();
        let settings = args
            .windows(2)
            .find_map(|window| (window[0] == "--settings").then(|| &window[1]))
            .expect("settings arg");
        let settings: Value = serde_json::from_str(settings).unwrap();
        let sandbox = &settings["sandbox"];
        assert_eq!(sandbox["failIfUnavailable"], true);
        assert_eq!(
            sandbox["network"]["tlsTerminate"]["caCertPath"],
            "/tmp/ca.crt"
        );
        assert_eq!(
            sandbox["filesystem"]["allowWrite"],
            serde_json::json!(["/workspace/out"])
        );
        assert_eq!(
            sandbox["filesystem"]["denyRead"],
            serde_json::json!(["/workspace/private"])
        );
        assert_eq!(sandbox["filesystem"]["allowManagedReadPathsOnly"], true);
        assert_eq!(
            sandbox["ignoreViolations"]["file"],
            serde_json::json!(["/tmp/noisy-file"])
        );
        assert_eq!(
            sandbox["ignoreViolations"]["custom"],
            serde_json::json!(["opaque-upstream-category"])
        );
        assert_eq!(sandbox["enableWeakerNetworkIsolation"], true);
        assert_eq!(sandbox["ripgrep"]["command"], "rg-custom");
        assert_eq!(sandbox["ripgrep"]["args"], serde_json::json!(["--json"]));
        assert_eq!(sandbox["bwrapPath"], "/usr/bin/bwrap");
        assert_eq!(sandbox["socatPath"], "/usr/bin/socat");
    }
}
