use super::McpServerConfig;
use crate::error::Result;
use crate::status::McpServerToolPolicy;
use serde::Deserialize;
use serde::Serialize;
use serde::de;
use serde::de::Deserializer;
use serde::ser::SerializeMap;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;

impl Serialize for McpServerConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Stdio {
                command,
                args,
                env,
                timeout,
                always_load,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", "stdio")?;
                map.serialize_entry("command", command)?;
                if !args.is_empty() {
                    map.serialize_entry("args", args)?;
                }
                if !env.is_empty() {
                    map.serialize_entry("env", env)?;
                }
                if let Some(timeout) = effective_mcp_timeout(timeout) {
                    map.serialize_entry("timeout", &timeout)?;
                }
                if let Some(always_load) = always_load {
                    map.serialize_entry("alwaysLoad", always_load)?;
                }
                map.end()
            }
            Self::Sse {
                url,
                headers,
                tools,
                timeout,
                always_load,
            } => serialize_remote_mcp_server(
                serializer,
                "sse",
                url,
                headers,
                tools,
                timeout,
                always_load,
            ),
            Self::Http {
                url,
                headers,
                tools,
                timeout,
                always_load,
            } => serialize_remote_mcp_server(
                serializer,
                "http",
                url,
                headers,
                tools,
                timeout,
                always_load,
            ),
            Self::Sdk { name } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "sdk")?;
                map.serialize_entry("name", name)?;
                map.end()
            }
            Self::Raw(value) => value.serialize(serializer),
            Self::Unknown => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "unknown")?;
                map.end()
            }
        }
    }
}

fn serialize_remote_mcp_server<S>(
    serializer: S,
    transport_type: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    tools: &[McpServerToolPolicy],
    timeout: &Option<u64>,
    always_load: &Option<bool>,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut map = serializer.serialize_map(None)?;
    map.serialize_entry("type", transport_type)?;
    map.serialize_entry("url", url)?;
    if !headers.is_empty() {
        map.serialize_entry("headers", headers)?;
    }
    if !tools.is_empty() {
        map.serialize_entry("tools", tools)?;
    }
    if let Some(timeout) = effective_mcp_timeout(timeout) {
        map.serialize_entry("timeout", &timeout)?;
    }
    if let Some(always_load) = always_load {
        map.serialize_entry("alwaysLoad", always_load)?;
    }
    map.end()
}

fn effective_mcp_timeout(timeout: &Option<u64>) -> Option<u64> {
    timeout.filter(|timeout| *timeout >= 1000)
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct StdioWire {
            command: String,
            #[serde(default)]
            args: Vec<String>,
            #[serde(default)]
            env: BTreeMap<String, String>,
            timeout: Option<u64>,
            #[serde(alias = "always_load")]
            always_load: Option<bool>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RemoteWire {
            url: String,
            #[serde(default)]
            headers: BTreeMap<String, String>,
            #[serde(default)]
            tools: Vec<McpServerToolPolicy>,
            timeout: Option<u64>,
            #[serde(alias = "always_load")]
            always_load: Option<bool>,
        }

        #[derive(Deserialize)]
        struct SdkWire {
            name: String,
        }

        let value = Value::deserialize(deserializer)?;
        let object = value.as_object().ok_or_else(|| {
            de::Error::custom("MCP server config must be an object with type or command")
        })?;
        let transport_type = object.get("type").and_then(Value::as_str);
        match transport_type {
            Some("stdio") => {
                let wire: StdioWire = serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(Self::Stdio {
                    command: wire.command,
                    args: wire.args,
                    env: wire.env,
                    timeout: wire.timeout,
                    always_load: wire.always_load,
                })
            }
            None if object.contains_key("command") => {
                let wire: StdioWire = serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(Self::Stdio {
                    command: wire.command,
                    args: wire.args,
                    env: wire.env,
                    timeout: wire.timeout,
                    always_load: wire.always_load,
                })
            }
            Some("sse") => {
                let wire: RemoteWire = serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(Self::Sse {
                    url: wire.url,
                    headers: wire.headers,
                    tools: wire.tools,
                    timeout: wire.timeout,
                    always_load: wire.always_load,
                })
            }
            Some("http") => {
                let wire: RemoteWire = serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(Self::Http {
                    url: wire.url,
                    headers: wire.headers,
                    tools: wire.tools,
                    timeout: wire.timeout,
                    always_load: wire.always_load,
                })
            }
            Some("sdk") => {
                let wire: SdkWire = serde_json::from_value(value).map_err(de::Error::custom)?;
                Ok(Self::Sdk { name: wire.name })
            }
            Some(_) => Ok(Self::Raw(value)),
            None => Err(de::Error::custom(
                "MCP server config without type must include command for stdio",
            )),
        }
    }
}
