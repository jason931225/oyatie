use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use cedar_policy::{Entity, EntityId, EntityTypeName, EntityUid, RestrictedExpression};
use shared_pdp_kernel::{EntityRecord, PdpError};
use shared_platform_contracts_kernel::pdp::EntityRef;

pub(super) fn entity_uid(entity_ref: &EntityRef) -> Result<EntityUid, PdpError> {
    let type_name =
        EntityTypeName::from_str(&entity_ref.entity_type).map_err(|e| PdpError::Evaluation {
            detail: format!("entity type {:?} rejected: {e}", entity_ref.entity_type),
        })?;
    let id = match EntityId::from_str(&entity_ref.entity_id) {
        Ok(id) => id,
        Err(infallible) => match infallible {},
    };
    Ok(EntityUid::from_type_name_and_id(type_name, id))
}

pub(super) fn restricted_expression(
    field: &str,
    value: &serde_json::Value,
) -> Result<RestrictedExpression, PdpError> {
    match value {
        serde_json::Value::String(s) => Ok(RestrictedExpression::new_string(s.clone())),
        serde_json::Value::Bool(b) => Ok(RestrictedExpression::new_bool(*b)),
        serde_json::Value::Number(n) => {
            n.as_i64()
                .map(RestrictedExpression::new_long)
                .ok_or_else(|| PdpError::Evaluation {
                    detail: format!("{field}: non-integer numbers are not mappable to Cedar"),
                })
        }
        _ => Err(PdpError::Evaluation {
            detail: format!("{field}: only string/bool/long values are mappable to Cedar"),
        }),
    }
}

pub(super) fn cedar_entity(record: &EntityRecord) -> Result<Entity, PdpError> {
    let uid = entity_uid(&record.uid)?;
    let mut attrs = HashMap::new();
    for (key, value) in &record.attributes {
        attrs.insert(
            key.clone(),
            restricted_expression(
                &format!("entity {} attr {key}", record.uid.entity_id),
                value,
            )?,
        );
    }
    let mut parents = HashSet::new();
    for parent in &record.parents {
        parents.insert(entity_uid(parent)?);
    }
    Entity::new(uid, attrs, parents).map_err(|e| PdpError::Evaluation {
        detail: format!("entity {} rejected: {e}", record.uid.entity_id),
    })
}
