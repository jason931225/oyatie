//! Envelope round-trip + tamper rejection for the enclave kernel.
//!
//! The property sweep uses a deterministic xorshift generator: proptest is
//! not buckified, so the sweep is reproducible by construction instead.

use secrets_kms_enclave::{
    DekId, EnclaveError, EnclaveRoot, KekId, KekMaterial, KekVersion, SealingRootId, TokenError,
    WrappedDek, WrappedKekToken,
};

fn root(id: &str) -> EnclaveRoot {
    EnclaveRoot::generate(SealingRootId::new(id).expect("root id")).expect("root generate")
}

fn kek(id: &str, version: u32) -> KekMaterial {
    KekMaterial::generate(
        KekId::new(id).expect("kek id"),
        KekVersion::new(version).expect("version"),
    )
    .expect("kek generate")
}

struct XorShift(u64);

impl XorShift {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }
}

#[test]
fn kek_wrap_unwrap_round_trip_preserves_dek_crypto() {
    let sealing_root = root("cell-1-root");
    let original = kek("kek/ten_alpha", 1);
    let (dek, wrapped_dek) = original
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .unwrap();

    let token = sealing_root.wrap_kek(&original).unwrap();
    let recovered = sealing_root.unwrap_kek(&token).unwrap();
    assert_eq!(recovered.kek_id().value(), "kek/ten_alpha");
    assert_eq!(recovered.version().value(), 1);

    let payload = b"static-stability payload";
    let blob = dek.seal(b"ctx", payload).unwrap();
    let reopened_dek = recovered.unwrap_dek(&wrapped_dek).unwrap();
    let plaintext = reopened_dek.open(b"ctx", &blob).unwrap();
    assert_eq!(plaintext.as_slice(), payload);
}

#[test]
fn token_encode_decode_round_trip() {
    let sealing_root = root("cell-1-root");
    let material = kek("kek/ten_alpha", 3);
    let token = sealing_root.wrap_kek(&material).unwrap();

    let decoded = WrappedKekToken::decode(&token.encode()).unwrap();
    assert_eq!(decoded, token);
    assert_eq!(decoded.root_id(), "cell-1-root");
    assert_eq!(decoded.kek_id().value(), "kek/ten_alpha");
    assert_eq!(decoded.kek_version(), 3);

    let recovered = sealing_root.unwrap_kek(&decoded).unwrap();
    assert_eq!(recovered.version().value(), 3);
}

#[test]
fn wrapped_dek_encode_decode_round_trip() {
    let material = kek("kek/ten_alpha", 2);
    let (_, wrapped) = material
        .generate_dek(DekId::new("dek/obj_7").unwrap())
        .unwrap();
    let decoded = WrappedDek::decode(&wrapped.encode()).unwrap();
    assert_eq!(decoded, wrapped);
    assert!(material.unwrap_dek(&decoded).is_ok());
}

#[test]
fn property_sweep_payload_round_trips() {
    // Property: for arbitrary payload sizes and AADs, seal∘open = identity
    // and any AAD mismatch is rejected.
    let material = kek("kek/ten_prop", 1);
    let (dek, _) = material
        .generate_dek(DekId::new("dek/prop").unwrap())
        .unwrap();
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    for round in 0..64 {
        let payload_len = (rng.next_u64() % 1024) as usize;
        let payload = rng.bytes(payload_len);
        let aad_len = (rng.next_u64() % 32) as usize;
        let aad = rng.bytes(aad_len);
        let blob = dek.seal(&aad, &payload).expect("seal");
        let opened = dek.open(&aad, &blob).expect("open");
        assert_eq!(opened.as_slice(), payload.as_slice(), "round {round}");

        let mut wrong_aad = aad.clone();
        wrong_aad.push(0x01);
        assert!(
            matches!(
                dek.open(&wrong_aad, &blob),
                Err(EnclaveError::CryptoRejected)
            ),
            "aad mismatch must reject (round {round})"
        );
    }
}

#[test]
fn property_sweep_token_tamper_never_authenticates() {
    // Property: flipping any single byte of an encoded token must fail
    // decode or fail authentication — never yield a usable KEK.
    let sealing_root = root("cell-1-root");
    let material = kek("kek/ten_tamper", 1);
    let token_bytes = sealing_root.wrap_kek(&material).unwrap().encode();

    for index in 0..token_bytes.len() {
        let mut tampered = token_bytes.clone();
        tampered[index] ^= 0x01;
        match WrappedKekToken::decode(&tampered) {
            Err(_) => {}
            Ok(decoded) => {
                assert!(
                    sealing_root.unwrap_kek(&decoded).is_err(),
                    "byte {index}: tampered token must not unwrap"
                );
            }
        }
    }
}

#[test]
fn wrong_root_rejects_by_binding_then_crypto() {
    let root_a = root("cell-a-root");
    let root_b = root("cell-b-root");
    let material = kek("kek/ten_alpha", 1);
    let token = root_a.wrap_kek(&material).unwrap();

    assert!(matches!(
        root_b.unwrap_kek(&token),
        Err(EnclaveError::KeyBindingMismatch { .. })
    ));

    let impostor = root("cell-a-root");
    assert!(matches!(
        impostor.unwrap_kek(&token),
        Err(EnclaveError::CryptoRejected)
    ));
}

#[test]
fn wrong_kek_and_wrong_version_reject_dek_unwrap() {
    let kek_alpha = kek("kek/ten_alpha", 1);
    let kek_beta = kek("kek/ten_beta", 1);
    let kek_alpha_v2 = kek("kek/ten_alpha", 2);
    let (_, wrapped) = kek_alpha
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .unwrap();

    assert!(matches!(
        kek_beta.unwrap_dek(&wrapped),
        Err(EnclaveError::KeyBindingMismatch { .. })
    ));
    assert!(matches!(
        kek_alpha_v2.unwrap_dek(&wrapped),
        Err(EnclaveError::UnknownKekVersion { version: 1 })
    ));
}

#[test]
fn nonce_uniqueness_across_wraps() {
    let sealing_root = root("cell-1-root");
    let material = kek("kek/ten_alpha", 1);
    let token_one = sealing_root.wrap_kek(&material).unwrap();
    let token_two = sealing_root.wrap_kek(&material).unwrap();
    assert_ne!(token_one.encode(), token_two.encode());
    assert!(sealing_root.unwrap_kek(&token_one).is_ok());
    assert!(sealing_root.unwrap_kek(&token_two).is_ok());
}

#[test]
fn strict_decode_rejects_malformed_inputs() {
    let sealing_root = root("cell-1-root");
    let material = kek("kek/ten_alpha", 1);
    let valid = sealing_root.wrap_kek(&material).unwrap().encode();

    for cut in 0..valid.len() {
        assert!(WrappedKekToken::decode(&valid[..cut]).is_err(), "cut {cut}");
    }

    let mut trailing = valid.clone();
    trailing.push(0x00);
    assert_eq!(
        WrappedKekToken::decode(&trailing),
        Err(TokenError::TrailingBytes)
    );

    let (_, wrapped_dek) = material
        .generate_dek(DekId::new("dek/obj_1").unwrap())
        .unwrap();
    assert_eq!(
        WrappedKekToken::decode(&wrapped_dek.encode()),
        Err(TokenError::WrongKind)
    );

    let mut bad_magic = valid;
    bad_magic[0] = b'X';
    assert_eq!(
        WrappedKekToken::decode(&bad_magic),
        Err(TokenError::BadMagic)
    );

    assert_eq!(WrappedKekToken::decode(&[]), Err(TokenError::Truncated));
}
