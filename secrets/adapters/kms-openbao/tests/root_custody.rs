//! Sealing-root custody via OpenBao transit export.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use secrets_kms_enclave::{KekId, KekMaterial, KekVersion, RootProvenance, SealingRootId};
use secrets_kms_openbao::root_custody::{OpenBaoRootCustody, RootCustodyError};

fn custody() -> OpenBaoRootCustody {
    OpenBaoRootCustody::new(
        "https://bao.cell-1.internal:8200",
        "transit",
        "cell-1-sealing-root",
    )
    .expect("custody config")
}

fn root_id() -> SealingRootId {
    SealingRootId::new("cell-1-root").expect("root id")
}

/// Deterministic 32-byte fixture material (test-only; never a real root).
fn fixture_material() -> Vec<u8> {
    (0u8..32)
        .map(|i| i.wrapping_mul(7).wrapping_add(13))
        .collect()
}

#[test]
fn provision_command_shape() {
    let command = custody().provision_root_command("ceremony://2026-06-10/run-1");
    assert_eq!(command.method, "POST");
    assert_eq!(command.path, "/v1/transit/keys/cell-1-sealing-root");
    assert_eq!(command.namespace, None);
    assert!(command.body_canonical.contains("type=aes256-gcm96"));
    assert!(command.body_canonical.contains("exportable=true"));
    assert!(
        command
            .body_canonical
            .contains("allow_plaintext_backup=false")
    );
    assert!(
        command
            .body_canonical
            .contains("ceremony_evidence_ref=ceremony://2026-06-10/run-1")
    );
    assert_eq!(
        command.audit_evidence_ref,
        "openbao-root-custody://bao.cell-1.internal:8200/transit/cell-1-sealing-root/provision"
    );
}

#[test]
fn fetch_command_shape_carries_no_material() {
    let command = custody().fetch_root_export_command();
    assert_eq!(command.method, "GET");
    assert_eq!(
        command.path,
        "/v1/transit/export/encryption-key/cell-1-sealing-root/1"
    );
    assert!(command.body_canonical.is_empty());
    assert_eq!(
        command.audit_evidence_ref,
        "openbao-root-custody://bao.cell-1.internal:8200/transit/cell-1-sealing-root/export"
    );
}

#[test]
fn namespace_scoping() {
    let command = custody()
        .with_namespace("cell-1")
        .expect("namespace")
        .fetch_root_export_command();
    assert_eq!(command.namespace.as_deref(), Some("cell-1"));
}

const CEREMONY_REF: &str = "ceremony://2026-06-10/run-1";

#[test]
fn ingest_round_trip_restart_survivability() {
    // Boot 1 and boot 2 ingest the SAME custodied export; a KEK wrapped by
    // boot 1's root must unwrap under boot 2's root — that interchange is
    // what makes OpenBao custody survive process restarts.
    let exported = BASE64_STANDARD.encode(fixture_material());
    let custodian = custody();

    let (boot_one_root, _) = custodian
        .ingest_exported_root(root_id(), exported.clone(), CEREMONY_REF)
        .expect("boot 1 ingest");
    let (boot_two_root, _) = custodian
        .ingest_exported_root(root_id(), exported, CEREMONY_REF)
        .expect("boot 2 ingest");

    let kek = KekMaterial::generate(
        KekId::new("kek/ten_alpha").expect("id"),
        KekVersion::INITIAL,
    )
    .expect("kek");
    let token = boot_one_root.wrap_kek(&kek).expect("wrap at boot 1");
    let recovered = boot_two_root.unwrap_kek(&token).expect("unwrap at boot 2");
    assert_eq!(recovered.kek_id().value(), "kek/ten_alpha");
}

#[test]
fn ingest_carries_typed_transitional_provenance() {
    let exported = BASE64_STANDARD.encode(fixture_material());
    let (_, provenance) = custody()
        .ingest_exported_root(root_id(), exported, CEREMONY_REF)
        .expect("ingest");
    assert!(matches!(
        provenance,
        RootProvenance::OpenBaoTransitionalSingleCustodian { .. }
    ));
    assert!(!provenance.satisfies_quorum_doctrine());
    assert_eq!(provenance.ceremony_evidence_ref(), CEREMONY_REF);

    let quorum = RootProvenance::ShamirQuorumCeremony {
        threshold: 3,
        share_count: 5,
        ceremony_evidence_ref: CEREMONY_REF.to_owned(),
    };
    assert!(quorum.satisfies_quorum_doctrine());
    let degenerate = RootProvenance::ShamirQuorumCeremony {
        threshold: 1,
        share_count: 1,
        ceremony_evidence_ref: CEREMONY_REF.to_owned(),
    };
    assert!(!degenerate.satisfies_quorum_doctrine());
}

#[test]
fn ingest_tolerates_surrounding_whitespace() {
    let exported = format!("\n{}\n", BASE64_STANDARD.encode(fixture_material()));
    assert!(
        custody()
            .ingest_exported_root(root_id(), exported, CEREMONY_REF)
            .is_ok()
    );
}

#[test]
fn red_ingest_rejects_non_base64() {
    let err = custody()
        .ingest_exported_root(root_id(), "not//valid==base64!!".to_owned(), CEREMONY_REF)
        .expect_err("garbage must be rejected");
    assert!(matches!(err, RootCustodyError::MaterialNotBase64));
}

#[test]
fn red_ingest_rejects_wrong_length() {
    for len in [0usize, 16, 31, 33, 64] {
        let exported = BASE64_STANDARD.encode(vec![0xa5u8; len]);
        let err = custody()
            .ingest_exported_root(root_id(), exported, CEREMONY_REF)
            .expect_err("non-32-byte material must be rejected");
        assert!(matches!(err, RootCustodyError::MaterialWrongLength { got } if got == len));
    }
}

#[test]
fn red_config_validation() {
    assert!(matches!(
        OpenBaoRootCustody::new("bao.internal:8200", "transit", "root"),
        Err(RootCustodyError::Config(_))
    ));
    assert!(matches!(
        OpenBaoRootCustody::new("https://bao.internal:8200", "tra/nsit", "root"),
        Err(RootCustodyError::Config(_))
    ));
    assert!(matches!(
        OpenBaoRootCustody::new("https://bao.internal:8200", "transit", ""),
        Err(RootCustodyError::Config(_))
    ));
}
