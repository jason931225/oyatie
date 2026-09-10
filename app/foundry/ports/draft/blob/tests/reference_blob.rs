//! Reference in-memory blob store, driven through the port's conformance
//! suite. Volatile on purpose; it proves the contract and declines to prove
//! durability.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use foundry_blob_draft::conformance::{
    BlobFixture, check_content_address_integrity, check_durability_across_reopen,
    check_empty_blob_is_a_valid_blob, check_missing_blob_reads_none,
    check_put_is_idempotent_by_content, check_round_trip_preserves_bytes,
    check_tenants_never_share_blobs,
};
use foundry_blob_draft::{BlobRef, BlobStore, BlobStoreError};

#[derive(Default)]
struct InMemoryStore {
    blobs: BTreeMap<(String, BlobRef), Vec<u8>>,
}

impl BlobStore for InMemoryStore {
    fn put(&mut self, tenant_id: &str, bytes: &[u8]) -> Result<BlobRef, BlobStoreError> {
        let reference = BlobRef::for_bytes(bytes);
        self.blobs
            .insert((tenant_id.to_owned(), reference.clone()), bytes.to_vec());
        Ok(reference)
    }

    fn get(&self, tenant_id: &str, reference: &BlobRef) -> Result<Option<Vec<u8>>, BlobStoreError> {
        Ok(self
            .blobs
            .get(&(tenant_id.to_owned(), reference.clone()))
            .cloned())
    }
}

#[derive(Default)]
struct InMemoryFixture {
    store: InMemoryStore,
}

impl BlobFixture for InMemoryFixture {
    type Store = InMemoryStore;

    fn store(&mut self) -> &mut Self::Store {
        &mut self.store
    }

    fn reopen(&mut self) -> bool {
        false
    }
}

type Check = fn(&mut InMemoryFixture) -> Result<(), String>;

#[test]
fn reference_store_satisfies_every_conformance_check() {
    let checks: [(&str, Check); 7] = [
        ("round trip", check_round_trip_preserves_bytes),
        ("content address", check_content_address_integrity),
        ("idempotent put", check_put_is_idempotent_by_content),
        ("tenant isolation", check_tenants_never_share_blobs),
        ("missing is none", check_missing_blob_reads_none),
        ("empty blob", check_empty_blob_is_a_valid_blob),
        ("durability", check_durability_across_reopen),
    ];
    for (name, check) in checks {
        let mut fixture = InMemoryFixture::default();
        check(&mut fixture).unwrap_or_else(|violation| panic!("{name}: {violation}"));
    }
}

#[test]
fn blob_refs_render_and_parse_the_same_digest_form() {
    let reference = BlobRef::for_bytes(b"hello");
    let rendered = reference.to_string();
    assert!(rendered.starts_with("sha256:"), "{rendered}");
    assert_eq!(BlobRef::parse(&rendered), Ok(reference));
    assert!(BlobRef::parse("sha256:zz").is_err());
    assert!(BlobRef::parse("md5:00").is_err());
}
