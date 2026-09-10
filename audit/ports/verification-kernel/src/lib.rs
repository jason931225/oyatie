#![allow(dead_code)]

pub trait RootRegistry {
    type Root;
    type Error;
    fn resolve_root(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
    ) -> Result<Self::Root, Self::Error>;
}

pub trait KeyResolver {
    type Key;
    type Error;
    fn resolve_key(
        &self,
        pack: &str,
        tenant_partition: &str,
        period_id: &str,
    ) -> Result<Self::Key, Self::Error>;
}

pub trait MerkleVerifier {
    type Proof;
    type Root;
    type Leaf;
    fn verify(&self, leaf: &Self::Leaf, proof: &Self::Proof, root: &Self::Root) -> bool;
}
