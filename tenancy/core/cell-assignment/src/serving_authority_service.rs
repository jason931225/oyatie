use crate::{BoxTenancyFuture, ServingAuthorityStoreError, VerifiedServingAuthorityInvocation};

#[derive(Debug, Eq, PartialEq)]
pub struct InstallServingAuthorityRequestV1 {
    pub grant: crate::VerifiedServingAuthorityInstallGrant,
    pub precondition: crate::ServingAuthorityLocalPreconditionV1,
    pub restore_basis: crate::ServingAuthorityRestoreBasisV1,
    pub business: crate::ServingAuthorityBusinessIdV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct FreezeServingAuthorityRequestV1 {
    pub grant: crate::VerifiedServingAuthorityFreezeGrant,
    pub precondition: crate::ServingAuthorityLocalPreconditionV1,
    pub business: crate::ServingAuthorityBusinessIdV1,
}

pub trait CellServingAuthorityService: Send + Sync {
    fn install<'a>(
        &'a self,
        invocation: VerifiedServingAuthorityInvocation,
        request: InstallServingAuthorityRequestV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::ServingAuthorityInstallationResultV1, ServingAuthorityStoreError>,
    >;

    fn freeze<'a>(
        &'a self,
        invocation: VerifiedServingAuthorityInvocation,
        request: FreezeServingAuthorityRequestV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::ServingAuthorityFreezeResultV1, ServingAuthorityStoreError>,
    >;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NotImplementedCellServingAuthorityService;

impl CellServingAuthorityService for NotImplementedCellServingAuthorityService {
    fn install<'a>(
        &'a self,
        _: VerifiedServingAuthorityInvocation,
        _: InstallServingAuthorityRequestV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::ServingAuthorityInstallationResultV1, ServingAuthorityStoreError>,
    > {
        Box::pin(async { Err(ServingAuthorityStoreError::NotImplemented) })
    }

    fn freeze<'a>(
        &'a self,
        _: VerifiedServingAuthorityInvocation,
        _: FreezeServingAuthorityRequestV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::ServingAuthorityFreezeResultV1, ServingAuthorityStoreError>,
    > {
        Box::pin(async { Err(ServingAuthorityStoreError::NotImplemented) })
    }
}
