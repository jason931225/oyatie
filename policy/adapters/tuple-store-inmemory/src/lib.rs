//! In-memory tuple store. Writes are totally ordered and a read at an earlier
//! snapshot cannot observe a later write, so "deny before the grant was
//! written, allow after" is a real property here and not a simulation of one.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use policy_cedar_domain::rebac::{
    RebacReadSnapshot, RebacTenantScope, RebacTuple, RebacTuplePage, RebacTupleQuery,
    RebacTupleStore, RebacTupleStoreError, ResolvedRebacSnapshot, SnapshotToken, Zookie,
};

/// Default tuples per page. Small on purpose: a reader that stops at the first
/// page is wrong, and a store that only ever returns one page never exercises
/// that path.
const DEFAULT_PAGE_SIZE: usize = 50;

#[derive(Clone, Debug)]
struct WrittenTuple {
    version_written: u64,
    tuple: RebacTuple,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryTupleStore {
    written: Vec<WrittenTuple>,
    version: u64,
    page_size: usize,
}

impl InMemoryTupleStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
            version: 0,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }

    /// # Panics
    /// When `page_size` is zero: a zero page can never make progress, and a
    /// reader looping on it would hang rather than fail.
    #[must_use]
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        assert!(page_size > 0, "page size must be positive");
        self.page_size = page_size;
        self
    }

    /// # Errors
    /// When the version cannot be rendered as a valid token.
    pub fn head(&self) -> Result<Zookie, RebacTupleStoreError> {
        Zookie::new(self.version.to_string()).map_err(RebacTupleStoreError::InvalidZookie)
    }

    fn visible_at(&self, token: &SnapshotToken) -> Result<u64, RebacTupleStoreError> {
        let requested = token.as_str().parse::<u64>().map_err(|_| {
            RebacTupleStoreError::Backend(format!(
                "snapshot token {:?} was not minted by this store",
                token.as_str()
            ))
        })?;
        if requested > self.version {
            return Err(RebacTupleStoreError::StaleSnapshot {
                requested: token.clone(),
                current: SnapshotToken::new(self.version.to_string())
                    .map_err(RebacTupleStoreError::InvalidZookie)?,
            });
        }
        Ok(requested)
    }
}

impl RebacTupleStore for InMemoryTupleStore {
    fn write_tuple(&mut self, tuple: RebacTuple) -> Result<Zookie, RebacTupleStoreError> {
        self.version = self.version.saturating_add(1);
        self.written.push(WrittenTuple {
            version_written: self.version,
            tuple,
        });
        self.head()
    }

    fn resolve_snapshot(
        &self,
        tenant: &RebacTenantScope,
        requested: RebacReadSnapshot,
    ) -> Result<ResolvedRebacSnapshot, RebacTupleStoreError> {
        let version = match requested {
            RebacReadSnapshot::Latest => self.version,
            RebacReadSnapshot::At { snapshot } => self.visible_at(&snapshot)?,
        };
        let token =
            SnapshotToken::new(version.to_string()).map_err(RebacTupleStoreError::InvalidZookie)?;
        Ok(ResolvedRebacSnapshot::new(tenant.clone(), token))
    }

    fn read_tuples(
        &self,
        query: &RebacTupleQuery,
        snapshot: &ResolvedRebacSnapshot,
    ) -> Result<RebacTuplePage, RebacTupleStoreError> {
        if &query.tenant != snapshot.tenant() {
            return Err(RebacTupleStoreError::SnapshotScopeMismatch {
                query_tenant: query.tenant.clone(),
                snapshot_tenant: snapshot.tenant().clone(),
            });
        }
        let ceiling = self.visible_at(snapshot.token())?;
        let matched: Vec<RebacTuple> = self
            .written
            .iter()
            .filter(|written| written.version_written <= ceiling && query.matches(&written.tuple))
            .map(|written| written.tuple.clone())
            .collect();

        let start = match query.page_token.as_deref() {
            None => 0,
            Some(token) => token.parse::<usize>().map_err(|_| {
                RebacTupleStoreError::Backend(format!("page token {token:?} is not one of ours"))
            })?,
        };
        let end = start.saturating_add(self.page_size).min(matched.len());
        let tuples = matched.get(start..end).unwrap_or_default().to_vec();
        let next_page_token = (end < matched.len()).then(|| end.to_string());

        Ok(RebacTuplePage {
            tuples,
            snapshot: snapshot.clone(),
            next_page_token,
        })
    }
}
