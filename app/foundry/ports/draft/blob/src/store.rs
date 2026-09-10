use crate::blob::BlobRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlobStoreError {
    /// Adapter-level failure (I/O, corruption); the message is diagnostic.
    Storage { detail: String },
}

/// A tenant-scoped, content-addressed blob store.
///
/// The executable meaning of this contract is [`crate::conformance`]; an
/// adapter that passes the suite implements the port.
pub trait BlobStore {
    /// Store bytes for a tenant; the returned address MUST equal
    /// [`BlobRef::for_bytes`] of those bytes. Storing identical bytes twice
    /// is idempotent by construction of the address.
    fn put(&mut self, tenant_id: &str, bytes: &[u8]) -> Result<BlobRef, BlobStoreError>;

    /// The bytes at an address for this tenant, or `None` — including when
    /// another tenant stored the identical content.
    fn get(&self, tenant_id: &str, reference: &BlobRef) -> Result<Option<Vec<u8>>, BlobStoreError>;
}
