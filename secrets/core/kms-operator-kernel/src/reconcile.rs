use crate::{
    Action, Clock, DesiredState, KeyRing, KeyVersionState, ObservedHealth, ObservedKeyRing,
    ObservedKeyVersion, ObservedState, ReadConsistency, SealingRoot,
};

pub fn reconcile<C: Clock>(
    observed: &ObservedState,
    desired: &DesiredState,
    clock: &C,
) -> Vec<Action> {
    let now = clock.now_epoch_seconds();
    if observed.read_consistency != ReadConsistency::Complete {
        return vec![Action::QuarantineObservedState {
            reason: "observed state was not complete".to_owned(),
            effective_at_epoch_seconds: now,
        }];
    }

    let mut actions = Vec::new();

    for sealing_root in &desired.sealing_roots {
        match observed
            .sealing_roots
            .iter()
            .find(|observed_root| same_sealing_root(&observed_root.desired, sealing_root))
        {
            Some(observed_root) => match &observed_root.health {
                ObservedHealth::Healthy => {
                    if observed_root.observed_version != sealing_root.active_version {
                        actions.push(Action::CreateSealingRoot {
                            sealing_root: sealing_root.clone(),
                        });
                    }
                }
                ObservedHealth::Ambiguous(reason) | ObservedHealth::Compromised(reason) => {
                    actions.push(Action::QuarantineObservedState {
                        reason: format!("sealing root {} unhealthy: {reason}", sealing_root.name),
                        effective_at_epoch_seconds: now,
                    });
                }
            },
            None => actions.push(Action::CreateSealingRoot {
                sealing_root: sealing_root.clone(),
            }),
        }
    }

    for key_ring in &desired.key_rings {
        match observed
            .key_rings
            .iter()
            .find(|observed_ring| same_key_ring(&observed_ring.desired, key_ring))
        {
            Some(observed_ring) => {
                reconcile_key_ring(observed_ring, key_ring, now, &mut actions);
            }
            None => actions.push(Action::CreateKeyRing {
                key_ring: key_ring.clone(),
                requested_at_epoch_seconds: now,
            }),
        }
    }

    actions
}

fn reconcile_key_ring(
    observed: &ObservedKeyRing,
    desired: &KeyRing,
    now_epoch_seconds: u64,
    actions: &mut Vec<Action>,
) {
    match &observed.health {
        ObservedHealth::Healthy => {}
        ObservedHealth::Ambiguous(reason) | ObservedHealth::Compromised(reason) => {
            if !all_versions_quarantined(&observed.versions) {
                actions.push(Action::QuarantineKeyRing {
                    key_ring_name: desired.name.clone(),
                    tenant_id: desired.tenant_id.clone(),
                    reason: reason.clone(),
                    effective_at_epoch_seconds: now_epoch_seconds,
                });
            }
            return;
        }
    }

    let active_versions: Vec<&ObservedKeyVersion> = observed
        .versions
        .iter()
        .filter(|version| version.state == KeyVersionState::Active)
        .collect();

    if active_versions.is_empty() {
        if observed.versions.is_empty() {
            actions.push(Action::CreateKeyRing {
                key_ring: desired.clone(),
                requested_at_epoch_seconds: now_epoch_seconds,
            });
        } else if !all_versions_quarantined(&observed.versions) {
            actions.push(Action::QuarantineKeyRing {
                key_ring_name: desired.name.clone(),
                tenant_id: desired.tenant_id.clone(),
                reason: "no active key version observed".to_owned(),
                effective_at_epoch_seconds: now_epoch_seconds,
            });
        }
        return;
    }

    if active_versions.len() > 1 {
        if let Some(newest_active) = newest_active_version(&active_versions) {
            for active in &active_versions {
                if active.version != newest_active.version {
                    actions.push(Action::DemoteKeyVersionToDecryptOnly {
                        key_ring_name: desired.name.clone(),
                        tenant_id: desired.tenant_id.clone(),
                        version: active.version,
                        reason: format!(
                            "newer active key version {} is present",
                            newest_active.version
                        ),
                        effective_at_epoch_seconds: now_epoch_seconds,
                    });
                }
            }
        }
        return;
    }

    if let Some(active) = active_versions.first() {
        let age_seconds = now_epoch_seconds
            .checked_sub(active.activated_at_epoch_seconds)
            .unwrap_or(u64::MAX);
        if age_seconds >= desired.rotation_policy.rotate_after_seconds
            && !has_newer_non_destroyed_version(&observed.versions, active.version)
        {
            actions.push(Action::RotateKeyVersion {
                key_ring: desired.clone(),
                observed_active_version: active.version,
                reason: format!(
                    "active key version age {age_seconds}s exceeds policy {}s",
                    desired.rotation_policy.rotate_after_seconds
                ),
                requested_at_epoch_seconds: now_epoch_seconds,
            });
        }
    }
}

fn same_key_ring(observed: &KeyRing, desired: &KeyRing) -> bool {
    observed.name == desired.name && observed.tenant_id == desired.tenant_id
}

fn same_sealing_root(observed: &SealingRoot, desired: &SealingRoot) -> bool {
    observed.name == desired.name && observed.tenant_id == desired.tenant_id
}

fn all_versions_quarantined(versions: &[ObservedKeyVersion]) -> bool {
    !versions.is_empty()
        && versions
            .iter()
            .all(|version| version.state == KeyVersionState::Quarantined)
}

fn newest_active_version<'a>(
    versions: &'a [&'a ObservedKeyVersion],
) -> Option<&'a ObservedKeyVersion> {
    versions
        .iter()
        .copied()
        .max_by_key(|version| version.version)
}

fn has_newer_non_destroyed_version(versions: &[ObservedKeyVersion], active_version: u32) -> bool {
    versions.iter().any(|version| {
        version.version > active_version && version.state != KeyVersionState::Destroyed
    })
}
