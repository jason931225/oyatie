//! Type-system one-way door checks: key material cannot leave the crypto
//! boundary.
//!
//! The door is enforced by what the types do NOT implement: `Clone` and
//! `serde::Serialize`-style egress. This file pins that from outside the
//! crate, so a future `#[derive(Clone)]` fails with a RED test, not silently.
//! Raw bytes are pinned separately, in `mlocked`, which is private.

use std::fmt::Write as _;
use std::marker::PhantomData;

use secrets_kms_enclave::{
    DekId, DekMaterial, EnclaveRoot, KekId, KekMaterial, KekVersion, KekVersionChain, SealingRootId,
};

/// Autoref-specialization probe: `detect()` resolves to the inherent method
/// (true) only when `T: Clone`; otherwise it falls back to the trait method
/// (false). Works on stable Rust; no compile-fail harness needed.
struct CloneProbe<T>(PhantomData<T>);

impl<T: Clone> CloneProbe<T> {
    fn detect(&self) -> bool {
        true
    }
}

trait NotCloneFallback {
    fn detect(&self) -> bool {
        false
    }
}

impl<T> NotCloneFallback for CloneProbe<T> {}

macro_rules! assert_not_clone {
    ($ty:ty) => {
        assert!(
            !CloneProbe::<$ty>(PhantomData).detect(),
            concat!(stringify!($ty), " must NOT implement Clone (one-way door)")
        );
    };
}

#[test]
fn key_material_types_are_not_clone() {
    assert_not_clone!(EnclaveRoot);
    assert_not_clone!(KekMaterial);
    assert_not_clone!(DekMaterial);
    assert_not_clone!(KekVersionChain);
    assert!(CloneProbe::<String>(PhantomData).detect());
}

#[test]
fn debug_output_never_carries_key_bytes() {
    let root = EnclaveRoot::generate(SealingRootId::new("cell-1-root").unwrap()).unwrap();
    let kek =
        KekMaterial::generate(KekId::new("kek/ten_alpha").unwrap(), KekVersion::INITIAL).unwrap();
    let (dek, _) = kek.generate_dek(DekId::new("dek/obj_1").unwrap()).unwrap();
    let chain = KekVersionChain::new(
        KekMaterial::generate(KekId::new("kek/ten_beta").unwrap(), KekVersion::INITIAL).unwrap(),
    );

    let mut rendered = String::new();
    write!(rendered, "{root:?} {kek:?} {dek:?} {chain:?}").unwrap();

    assert!(
        rendered.matches("[REDACTED]").count() >= 4,
        "every holder redacts: {rendered}"
    );
    // No hex/byte-array dump patterns: once the redaction markers are
    // removed, no bracket (Debug's byte-slice rendering) may remain.
    let without_markers = rendered.replace("[REDACTED]", "");
    assert!(!without_markers.contains('['), "{rendered}");
}

#[test]
fn wrapped_forms_are_the_only_egress() {
    let root = EnclaveRoot::generate(SealingRootId::new("cell-1-root").unwrap()).unwrap();
    let kek =
        KekMaterial::generate(KekId::new("kek/ten_alpha").unwrap(), KekVersion::INITIAL).unwrap();
    let token = root.wrap_kek(&kek).unwrap();

    assert_eq!(token.kek_id().value(), "kek/ten_alpha");
    assert_eq!(token.kek_version(), 1);
    assert!(root.unwrap_kek(&token).is_ok());
}

#[test]
fn sealing_root_id_validation() {
    assert!(SealingRootId::new("cell-1-root").is_ok());
    assert!(SealingRootId::new("").is_err());
    assert!(SealingRootId::new("has space").is_err());
    assert!(SealingRootId::new("ctl\u{7}char").is_err());
    assert!(SealingRootId::new("x".repeat(129)).is_err());
}
