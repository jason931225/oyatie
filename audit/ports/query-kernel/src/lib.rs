#![allow(dead_code)]

pub trait AuditQueryRepository {
    type Query;
    type Page;
    type Error;
    fn query(&self, q: &Self::Query) -> Result<Self::Page, Self::Error>;
}

pub trait ExportBuilder {
    type Query;
    type Bundle;
    type Error;
    fn build(&self, q: &Self::Query) -> Result<Self::Bundle, Self::Error>;
}

pub trait AuditorEngagementResolver {
    type Engagement;
    type Error;
    fn resolve(&self, engagement_id: &str) -> Result<Self::Engagement, Self::Error>;
}
