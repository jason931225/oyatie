//! The executable contract: every adapter runs this suite unchanged.

use crate::blob::BlobRef;
use crate::store::BlobStore;

/// A volatile fixture answers `false` and the durability check reports
/// that it proved nothing rather than passing against lost state.
pub trait BlobFixture {
    type Store: BlobStore;

    fn store(&mut self) -> &mut Self::Store;

    fn reopen(&mut self) -> bool;
}

fn fail(clause: &str, detail: String) -> String {
    format!("{clause}: {detail}")
}

pub fn check_round_trip_preserves_bytes<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let bytes = b"workbook bytes \x00\xff binary included".to_vec();
    let reference = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    let read = fixture
        .store()
        .get("ten_a", &reference)
        .map_err(|e| format!("{e:?}"))?;
    if read.as_deref() != Some(bytes.as_slice()) {
        return Err(fail(
            "a stored blob reads back byte-identically",
            format!("{read:?}"),
        ));
    }
    Ok(())
}

pub fn check_content_address_integrity<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let bytes = b"addressed by content".to_vec();
    let reference = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    if reference != BlobRef::for_bytes(&bytes) {
        return Err(fail(
            "the returned address is the hash of the bytes",
            reference.to_string(),
        ));
    }
    let read = fixture
        .store()
        .get("ten_a", &reference)
        .map_err(|e| format!("{e:?}"))?
        .ok_or_else(|| fail("stored blob present", String::new()))?;
    if BlobRef::for_bytes(&read) != reference {
        return Err(fail("read bytes hash back to their address", String::new()));
    }
    Ok(())
}

pub fn check_put_is_idempotent_by_content<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let bytes = b"same content twice".to_vec();
    let first = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    let second = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    if first != second {
        return Err(fail(
            "identical content has one address",
            format!("{first} vs {second}"),
        ));
    }
    Ok(())
}

pub fn check_tenants_never_share_blobs<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let bytes = b"identical bytes in two tenants".to_vec();
    let reference = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    let cross = fixture
        .store()
        .get("ten_b", &reference)
        .map_err(|e| format!("{e:?}"))?;
    if cross.is_some() {
        return Err(fail(
            "a digest stored by one tenant reads as absent to another",
            "cross-tenant read returned bytes".to_owned(),
        ));
    }
    Ok(())
}

pub fn check_missing_blob_reads_none<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let reference = BlobRef::for_bytes(b"never stored");
    let read = fixture
        .store()
        .get("ten_a", &reference)
        .map_err(|e| format!("{e:?}"))?;
    if read.is_some() {
        return Err(fail(
            "an unstored address reads None, not an error",
            String::new(),
        ));
    }
    Ok(())
}

pub fn check_empty_blob_is_a_valid_blob<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let reference = fixture
        .store()
        .put("ten_a", b"")
        .map_err(|e| format!("{e:?}"))?;
    let read = fixture
        .store()
        .get("ten_a", &reference)
        .map_err(|e| format!("{e:?}"))?;
    if read.as_deref() != Some(&b""[..]) {
        return Err(fail("the empty blob stores and reads", format!("{read:?}")));
    }
    Ok(())
}

pub fn check_durability_across_reopen<F: BlobFixture>(fixture: &mut F) -> Result<(), String> {
    let bytes = b"must survive reopen".to_vec();
    let reference = fixture
        .store()
        .put("ten_a", &bytes)
        .map_err(|e| format!("{e:?}"))?;
    if !fixture.reopen() {
        return Ok(());
    }
    let read = fixture
        .store()
        .get("ten_a", &reference)
        .map_err(|e| format!("{e:?}"))?;
    if read.as_deref() != Some(bytes.as_slice()) {
        return Err(fail(
            "a reopened store reads byte-identically",
            format!("{read:?}"),
        ));
    }
    Ok(())
}
