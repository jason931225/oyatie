use std::collections::{HashMap, VecDeque};

use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, Decision, Obligation};

use crate::{EntityRecord, EntitySlice};

/// Canonical fingerprint of the decision-relevant request surface: tenant,
/// principal, action, resource, ABAC context, and the entity slice. The
/// correlation fields (`request_id`) and the freshness floor
/// (`min_policy_version`) are EXCLUDED — they never change the decision.
///
/// The fingerprint is the full canonical JSON string, not a digest: cache
/// correctness must not depend on hash-collision odds, and the bounded cache
/// caps memory. Entity records are sorted by uid so PEP assembly order
/// cannot split cache entries.
#[must_use]
pub fn request_fingerprint(request: &AuthorizationRequest, entities: &EntitySlice) -> String {
    let mut records: Vec<&EntityRecord> = entities.entities.iter().collect();
    records.sort_by(|a, b| {
        (&a.uid.entity_type, &a.uid.entity_id).cmp(&(&b.uid.entity_type, &b.uid.entity_id))
    });
    let entity_payload: Vec<serde_json::Value> = records
        .iter()
        .map(|r| {
            let mut parents: Vec<(&String, &String)> = r
                .parents
                .iter()
                .map(|p| (&p.entity_type, &p.entity_id))
                .collect();
            parents.sort();
            serde_json::json!({
                "uid": [r.uid.entity_type, r.uid.entity_id],
                "attributes": r.attributes,
                "parents": parents,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "tenant_id": request.tenant_id,
        "principal": [request.principal.entity_type, request.principal.entity_id],
        "action": request.action,
        "resource": [request.resource.entity_type, request.resource.entity_id],
        "context": request.context,
        "entities": entity_payload,
    });
    payload.to_string()
}

/// The decision content a cache may replay. Correlation fields (decision id,
/// request id) are NEVER cached: every replayed decision is re-minted with a
/// fresh decision id so the audit chain stays one-record-per-decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedDecision {
    pub decision: Decision,                  // data_class: INTERNAL_ONLY
    pub determining_policy_ids: Vec<String>, // data_class: INTERNAL_ONLY
    pub obligations: Vec<Obligation>,        // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecisionCacheKey {
    pub request_fingerprint: String, // data_class: INTERNAL_ONLY
    pub policy_version: String,      // data_class: INTERNAL_ONLY
}

/// Bounded in-process decision cache (insertion-order eviction). Embedded
/// PDPs are per-process; the cache never crosses a service boundary.
#[derive(Debug)]
pub struct DecisionCache {
    capacity: usize,                                // data_class: INTERNAL_ONLY
    map: HashMap<DecisionCacheKey, CachedDecision>, // data_class: INTERNAL_ONLY
    order: VecDeque<DecisionCacheKey>,              // data_class: INTERNAL_ONLY
}

impl DecisionCache {
    /// A cache holding at most `capacity` decisions. A zero capacity
    /// disables caching (every lookup misses).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn get(&self, key: &DecisionCacheKey) -> Option<&CachedDecision> {
        self.map.get(key)
    }

    pub fn insert(&mut self, key: DecisionCacheKey, value: CachedDecision) {
        if self.capacity == 0 {
            return;
        }
        if self.map.insert(key.clone(), value).is_none() {
            self.order.push_back(key);
        }
        while self.map.len() > self.capacity {
            let Some(evicted) = self.order.pop_front() else {
                break;
            };
            self.map.remove(&evicted);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
