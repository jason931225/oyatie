use std::collections::BTreeMap;

use policy_cedar_domain::rebac::UsersetRewrite;

use crate::NamespaceCompileError;
use crate::namespace::RelationKey;

pub(crate) fn check_references(
    relations: &BTreeMap<RelationKey, UsersetRewrite>,
) -> Result<(), NamespaceCompileError> {
    for (
        RelationKey {
            object_type,
            relation,
        },
        rewrite,
    ) in relations
    {
        let mut pending = vec![rewrite];
        while let Some(node) = pending.pop() {
            let referenced = match node {
                UsersetRewrite::This => None,
                UsersetRewrite::ComputedUserset { relation } => Some(relation),
                UsersetRewrite::TupleToUserset {
                    tupleset_relation, ..
                } => Some(tupleset_relation),
                UsersetRewrite::Union { children } | UsersetRewrite::Intersection { children } => {
                    if children.is_empty() {
                        return Err(NamespaceCompileError::EmptyRewrite {
                            object_type: object_type.clone(),
                            relation: relation.clone(),
                            kind: if matches!(node, UsersetRewrite::Union { .. }) {
                                "union"
                            } else {
                                "intersection"
                            },
                        });
                    }
                    pending.extend(children.iter().rev());
                    None
                }
                UsersetRewrite::Difference { base, subtract } => {
                    pending.push(subtract);
                    pending.push(base);
                    None
                }
            };
            if let Some(referenced) = referenced {
                let key = RelationKey::new(object_type.clone(), referenced);
                if !relations.contains_key(&key) {
                    return Err(NamespaceCompileError::UnknownRelationReference {
                        object_type: object_type.clone(),
                        relation: relation.clone(),
                        referenced_relation: key.relation,
                    });
                }
            }
        }
    }
    Ok(())
}
