//! Hand-written `Serialize` for `PolicyRuleInput`, kept next to the diff
//! because the wire shape and the diff identity key must stay in step.

use serde::{Deserialize, Serialize};

use crate::policy::PolicyRuleInput;

impl Serialize for PolicyRuleInput {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PolicyRuleInput", 5)?;
        s.serialize_field("effect", &self.effect)?;
        s.serialize_field("principal_role", &self.principal_role)?;
        s.serialize_field("action", &self.action)?;
        s.serialize_field("resource_prefix", &self.resource_prefix)?;
        s.serialize_field("required_attribute", &self.required_attribute)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for PolicyRuleInput {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct PolicyRuleInputVisitor;

        impl<'de> Visitor<'de> for PolicyRuleInputVisitor {
            type Value = PolicyRuleInput;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct PolicyRuleInput")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<PolicyRuleInput, V::Error> {
                let mut effect = None;
                let mut principal_role = None;
                let mut action = None;
                let mut resource_prefix = None;
                let mut required_attribute = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "effect" => {
                            effect = Some(map.next_value()?);
                        }
                        "principal_role" => {
                            principal_role = Some(map.next_value()?);
                        }
                        "action" => {
                            action = Some(map.next_value()?);
                        }
                        "resource_prefix" => {
                            resource_prefix = Some(map.next_value()?);
                        }
                        "required_attribute" => {
                            required_attribute = map.next_value()?;
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(PolicyRuleInput {
                    effect: effect.ok_or_else(|| de::Error::missing_field("effect"))?,
                    principal_role: principal_role
                        .ok_or_else(|| de::Error::missing_field("principal_role"))?,
                    action: action.ok_or_else(|| de::Error::missing_field("action"))?,
                    resource_prefix: resource_prefix
                        .ok_or_else(|| de::Error::missing_field("resource_prefix"))?,
                    required_attribute,
                    annotations: Vec::new(),
                })
            }
        }

        const FIELDS: &[&str] = &[
            "effect",
            "principal_role",
            "action",
            "resource_prefix",
            "required_attribute",
        ];
        deserializer.deserialize_struct("PolicyRuleInput", FIELDS, PolicyRuleInputVisitor)
    }
}
