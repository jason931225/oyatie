//! Tenant isolation on the read plane, held by three independent walls.
//!
//! A caller bearing tenant B's credential must learn nothing about tenant
//! A — not the contents of an object, and not whether one exists. The
//! walls are: the credential binds the tenant server-side, so the request
//! cannot address another; the Cedar seed's structural forbid covers every
//! action, so even a permitted role is refused across tenants; and the
//! fold poisons a cross-tenant envelope, so the projection could not serve
//! one even if the first two were bypassed.
#[path = "facade_support/mod.rs"]
mod support;

use axum::http::StatusCode;
use support::{Fixture, get, write_a_record};

#[tokio::test]
async fn a_foreign_credential_cannot_read_another_tenants_object() {
    let fixture = Fixture::new("iso-read");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (status, body) = get(
        &fixture,
        Some(fixture.foreign_token()),
        "/v1/objects/ent_alpha?revision=1",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(
        !body.contains("Ada"),
        "a refusal must not leak the value it refused: {body}"
    );
}

#[tokio::test]
async fn a_foreign_credential_cannot_distinguish_present_from_absent() {
    let fixture = Fixture::new("iso-oracle");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    let (present, _) = get(
        &fixture,
        Some(fixture.foreign_token()),
        "/v1/objects/ent_alpha?revision=1",
    )
    .await;
    let (absent, _) = get(
        &fixture,
        Some(fixture.foreign_token()),
        "/v1/objects/ent_ghost?revision=1",
    )
    .await;
    assert_eq!(
        present, absent,
        "an existing and a non-existing object must be indistinguishable across tenants, \
         or the surface is an existence oracle"
    );
    assert_eq!(present, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_foreign_credential_reads_neither_history_nor_audit() {
    let fixture = Fixture::new("iso-views");
    write_a_record(&fixture, "ent_alpha", "idem_1").await;
    for path in ["/v1/objects/ent_alpha/history", "/v1/audit"] {
        let (status, body) = get(&fixture, Some(fixture.foreign_token()), path).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
        assert!(
            !body.contains("prn_alice"),
            "{path} must not leak the other tenant's actor: {body}"
        );
    }
}
