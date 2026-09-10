#![allow(dead_code)]

pub trait RetentionPolicySource {
    type Policy;
    type Error;
    fn load(&self) -> Result<Self::Policy, Self::Error>;
}

pub trait DsrCascadeSource {
    type Request;
    type Error;
    fn next(&self) -> Result<Option<Self::Request>, Self::Error>;
}

/// Port: write redaction markers that preserve proof of original existence.
pub trait RedactionWriter {
    type Marker;
    type Error;
    fn redact(&self, marker: &Self::Marker) -> Result<(), Self::Error>;
}
