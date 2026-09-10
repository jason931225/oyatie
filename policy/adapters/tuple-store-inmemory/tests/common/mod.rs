use policy_cedar_domain::rebac::{
    RebacObjectRef, RebacReadSnapshot, RebacRelation, RebacSubjectRef, RebacTenantScope,
    RebacTuple, RebacTupleStore, UsersetRewrite, Zookie,
};
use policy_rebac_domain::{Expander, ExpansionBounds, NamespaceConfig, ValidatedNamespace};
use policy_tuple_store_inmemory::InMemoryTupleStore;

pub fn tenant() -> RebacTenantScope {
    RebacTenantScope::new("ten_conformance").expect("tenant scope is valid")
}

pub fn relation(name: &str) -> RebacRelation {
    RebacRelation::new(name).expect("relation is valid")
}

pub fn object(reference: &str) -> RebacObjectRef {
    RebacObjectRef::parse(reference).expect("object reference is valid")
}

pub fn user(id: &str) -> RebacSubjectRef {
    RebacSubjectRef::object(object(id))
}

pub fn write(store: &mut InMemoryTupleStore, tuple: &str) -> Zookie {
    let parsed = RebacTuple::parse(tenant(), tuple).expect("canonical tuple parses");
    store.write_tuple(parsed).expect("write succeeds")
}

pub fn at(zookie: Zookie) -> RebacReadSnapshot {
    RebacReadSnapshot::at_zookie(zookie)
}

pub fn new_expander<'a, S: RebacTupleStore>(
    store: &'a S,
    namespace: &'a ValidatedNamespace,
    requested: RebacReadSnapshot,
) -> Expander<'a, S> {
    bounded_expander(store, namespace, requested, ExpansionBounds::DEFAULT)
}

pub fn bounded_expander<'a, S: RebacTupleStore>(
    store: &'a S,
    namespace: &'a ValidatedNamespace,
    requested: RebacReadSnapshot,
    bounds: ExpansionBounds,
) -> Expander<'a, S> {
    Expander::new(store, namespace, tenant(), requested).with_bounds(bounds)
}

pub fn document_model() -> ValidatedNamespace {
    NamespaceConfig::new()
        .define("folder", &relation("viewer"), UsersetRewrite::this())
        .define(
            "document",
            &relation("viewer"),
            UsersetRewrite::union(vec![
                UsersetRewrite::this(),
                UsersetRewrite::tuple_to_userset(relation("parent"), relation("viewer")),
            ])
            .expect("a two-child union is valid"),
        )
        .define("document", &relation("parent"), UsersetRewrite::this())
        .validated()
        .expect("the document model is stratified")
}
