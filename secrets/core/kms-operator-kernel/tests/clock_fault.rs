use secrets_kms_operator_kernel::{
    Action, Clock, DataClassLabel, DesiredState, HsmValidation, KeyOrigin, KeyRing, KeyUsage,
    KeyVersionRotationPolicy, KeyVersionState, ObservedHealth, ObservedKeyRing, ObservedKeyVersion,
    ObservedSealingRoot, ObservedState, ReadConsistency, ResidencyMode, SealingRoot, reconcile,
};

const CREATED_AT: u64 = 100;
const ACTIVATED_AT: u64 = 1_000;
const ROTATE_AFTER: u64 = 900;
const DECRYPT_ONLY_GRACE: u64 = 300;

struct FixedClock {
    now: u64,
}

impl Clock for FixedClock {
    fn now_epoch_seconds(&self) -> u64 {
        self.now
    }
}

fn desired_state() -> DesiredState {
    DesiredState {
        sealing_roots: vec![SealingRoot {
            name: "tenant-a-root".to_owned(),
            tenant_id: "ten_alpha".to_owned(),
            region: "us-east-1".to_owned(),
            cell_id: "cell-us-east-1a".to_owned(),
            root_ref: "sealing-root/tenant-a".to_owned(),
            active_version: 1,
            rotate_after_seconds: 86_400,
        }],
        key_rings: vec![KeyRing {
            name: "tenant-a-ring".to_owned(),
            tenant_id: "ten_alpha".to_owned(),
            region: "us-east-1".to_owned(),
            cell_id: "cell-us-east-1a".to_owned(),
            hsm_partition_ref: "hsm/us-east-1/cell-us-east-1a".to_owned(),
            origin: KeyOrigin::OyatieManaged,
            usage: KeyUsage::EncryptDecrypt,
            hsm_validation: HsmValidation::Fips1403Level3,
            residency: ResidencyMode::StrictHomeRegion,
            data_class: DataClassLabel::InternalOnly,
            rotation_policy: KeyVersionRotationPolicy {
                rotate_after_seconds: ROTATE_AFTER,
                decrypt_only_grace_seconds: DECRYPT_ONLY_GRACE,
            },
        }],
    }
}

fn observed_single_active(desired: &DesiredState) -> ObservedState {
    ObservedState {
        read_consistency: ReadConsistency::Complete,
        sealing_roots: vec![ObservedSealingRoot {
            desired: desired.sealing_roots[0].clone(),
            observed_version: 1,
            health: ObservedHealth::Healthy,
        }],
        key_rings: vec![ObservedKeyRing {
            desired: desired.key_rings[0].clone(),
            versions: vec![ObservedKeyVersion {
                version: 1,
                state: KeyVersionState::Active,
                created_at_epoch_seconds: CREATED_AT,
                activated_at_epoch_seconds: ACTIVATED_AT,
                decrypt_only_since_epoch_seconds: None,
            }],
            health: ObservedHealth::Healthy,
        }],
    }
}

fn rotation_reason_at(now: u64) -> Option<String> {
    let desired = desired_state();
    let observed = observed_single_active(&desired);
    reconcile(&observed, &desired, &FixedClock { now })
        .into_iter()
        .find_map(|action| match action {
            Action::RotateKeyVersion { reason, .. } => Some(reason),
            _ => None,
        })
}

fn rotates_at(now: u64) -> bool {
    rotation_reason_at(now).is_some()
}

#[test]
fn a_clock_reading_before_activation_rotates_the_active_key_version() {
    assert_eq!(
        rotation_reason_at(0),
        Some(format!(
            "active key version age {}s exceeds policy {ROTATE_AFTER}s",
            u64::MAX
        ))
    );
}

#[test]
fn no_clock_reading_below_activation_leaves_the_active_key_version_in_place() {
    let suppressed: Vec<u64> = [
        0,
        1,
        ACTIVATED_AT - 1,
        ACTIVATED_AT - ROTATE_AFTER,
        ACTIVATED_AT - DECRYPT_ONLY_GRACE,
        CREATED_AT,
    ]
    .into_iter()
    .filter(|now| !rotates_at(*now))
    .collect();
    assert!(
        suppressed.is_empty(),
        "clock readings that suppressed rotation: {suppressed:?}"
    );
}

#[test]
fn the_activation_instant_itself_does_not_rotate() {
    assert!(!rotates_at(ACTIVATED_AT));
}

#[test]
fn rotation_fires_at_exactly_the_policy_age() {
    assert!(!rotates_at(ACTIVATED_AT + ROTATE_AFTER - 1));
    assert!(rotates_at(ACTIVATED_AT + ROTATE_AFTER));
}

#[test]
fn an_age_past_the_decrypt_only_grace_but_short_of_the_policy_does_not_rotate() {
    assert!(!rotates_at(ACTIVATED_AT + DECRYPT_ONLY_GRACE + 1));
}
