//! Decrypt-only key-version rotation: version rotation, never re-encryption.

use secrets_kms_enclave::{DekId, EnclaveError, KekId, KekMaterial, KekVersion, KekVersionChain};

fn chain(kek_id: &str) -> KekVersionChain {
    KekVersionChain::new(
        KekMaterial::generate(KekId::new(kek_id).expect("kek id"), KekVersion::INITIAL)
            .expect("kek generate"),
    )
}

#[test]
fn rotation_advances_version_and_retires_previous() {
    let mut versions = chain("kek/ten_alpha");
    assert_eq!(versions.current_version().value(), 1);
    assert_eq!(versions.retired_versions().count(), 0);

    let new_version = versions.rotate().expect("rotate");
    assert_eq!(new_version.value(), 2);
    assert_eq!(versions.current_version().value(), 2);
    let retired: Vec<u32> = versions.retired_versions().map(KekVersion::value).collect();
    assert_eq!(retired, vec![1]);
}

#[test]
fn old_ciphertext_decrypts_after_rotation_without_reencryption() {
    let mut versions = chain("kek/ten_alpha");
    let (dek_v1, wrapped_v1) = versions
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .expect("generate dek");
    let payload = b"wrapped before rotation";
    let blob = dek_v1.seal(b"ctx", payload).expect("seal");
    let wrapped_bytes_before = wrapped_v1.encode();

    versions.rotate().expect("rotate");
    versions.rotate().expect("rotate again");

    assert_eq!(wrapped_v1.encode(), wrapped_bytes_before);
    let recovered = versions
        .unwrap_dek(&wrapped_v1)
        .expect("unwrap via retired version");
    let plaintext = recovered.open(b"ctx", &blob).expect("open");
    assert_eq!(plaintext.as_slice(), payload);
}

#[test]
fn new_wraps_carry_current_version_only() {
    let mut versions = chain("kek/ten_alpha");
    versions.rotate().expect("rotate");
    let (_, wrapped) = versions
        .generate_dek(DekId::new("dek/obj_2").unwrap())
        .expect("dek");
    assert_eq!(wrapped.kek_version(), 2);
}

#[test]
fn unknown_version_fails_closed() {
    // A chain that has rotated ahead wraps at a version the stale chain
    // (still at v1) does not hold: routing must fail closed, not guess.
    let mut rotated_ahead = chain("kek/ten_alpha");
    rotated_ahead.rotate().expect("rotate to v2");
    rotated_ahead.rotate().expect("rotate to v3");
    let (_, wrapped_v3) = rotated_ahead
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .expect("dek");

    let stale = chain("kek/ten_alpha");
    assert!(matches!(
        stale.unwrap_dek(&wrapped_v3),
        Err(EnclaveError::UnknownKekVersion { version: 3 })
    ));

    let mut same_shape = chain("kek/ten_alpha");
    same_shape.rotate().expect("rotate to v2");
    same_shape.rotate().expect("rotate to v3");
    assert!(matches!(
        same_shape.unwrap_dek(&wrapped_v3),
        Err(EnclaveError::CryptoRejected)
    ));
}

#[test]
fn cross_kek_unwrap_rejected_before_crypto() {
    let alpha = chain("kek/ten_alpha");
    let beta = chain("kek/ten_beta");
    let (_, wrapped) = alpha
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .expect("dek");
    assert!(matches!(
        beta.unwrap_dek(&wrapped),
        Err(EnclaveError::KeyBindingMismatch { .. })
    ));
}

#[test]
fn deep_rotation_keeps_every_retired_version_serving() {
    let mut versions = chain("kek/ten_alpha");
    let mut wrapped_per_version = Vec::new();
    for round in 0..8 {
        let dek_id = DekId::new(format!("dek/obj_{round}")).expect("dek id");
        let (dek, wrapped) = versions.generate_dek(dek_id).expect("dek");
        let blob = dek.seal(b"ctx", b"payload").expect("seal");
        wrapped_per_version.push((wrapped, blob));
        versions.rotate().expect("rotate");
    }
    assert_eq!(versions.current_version().value(), 9);
    assert_eq!(versions.retired_versions().count(), 8);
    for (wrapped, blob) in &wrapped_per_version {
        let dek = versions.unwrap_dek(wrapped).expect("retired unwrap");
        assert_eq!(dek.open(b"ctx", blob).expect("open").as_slice(), b"payload");
    }
}

#[test]
fn version_arithmetic_guards() {
    assert!(matches!(KekVersion::new(0), Err(EnclaveError::ZeroVersion)));
    let max = KekVersion::new(u32::MAX).expect("max version");
    assert!(matches!(max.next(), Err(EnclaveError::VersionOverflow)));
}
