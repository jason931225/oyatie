#![allow(dead_code)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerSurface(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainCoordinate {
    pub pack: String,             // data_class: INTERNAL_ONLY
    pub tenant_partition: String, // data_class: INTERNAL_ONLY
    pub period: String,           // data_class: INTERNAL_ONLY
}

pub trait AuditEmitter {
    type Envelope;
    type Receipt;
    type Error;
    fn emit(&self, envelope: Self::Envelope) -> Result<Self::Receipt, Self::Error>;
}

pub trait WalWriter {
    type Record;
    type Error;
    fn append(&self, record: Self::Record) -> Result<(), Self::Error>;
}

pub trait PrincipalResolver {
    type Principal;
    type Error;
    fn resolve(&self, principal: Self::Principal) -> Result<ProducerSurface, Self::Error>;
}
