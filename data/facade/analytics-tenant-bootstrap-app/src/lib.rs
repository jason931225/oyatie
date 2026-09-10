#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

use shared_olap_client_kernel::{KernelError, OlapClient, TenantId};

#[derive(Clone, Debug)]
pub enum TenantEvent {
    Created { tenant_id: TenantId },
    Suspended { tenant_id: TenantId },
    Reactivated { tenant_id: TenantId },
    Deleted { tenant_id: TenantId },
}

impl TenantEvent {
    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        match self {
            Self::Created { tenant_id }
            | Self::Suspended { tenant_id }
            | Self::Reactivated { tenant_id }
            | Self::Deleted { tenant_id } => tenant_id,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ReconcileError {
    Kernel(KernelError),
    Unimplemented(&'static str),
}

impl fmt::Display for ReconcileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel(e) => write!(f, "kernel error: {e}"),
            Self::Unimplemented(slug) => write!(f, "unimplemented: {slug}"),
        }
    }
}

impl std::error::Error for ReconcileError {}

impl From<KernelError> for ReconcileError {
    fn from(e: KernelError) -> Self {
        Self::Kernel(e)
    }
}

pub struct TenantBootstrapController<'a> {
    olap: &'a mut dyn OlapClient,
}

impl<'a> TenantBootstrapController<'a> {
    #[must_use]
    pub fn new(olap: &'a mut dyn OlapClient) -> Self {
        Self { olap }
    }

    pub fn process(&mut self, event: &TenantEvent) -> Result<(), ReconcileError> {
        match event {
            TenantEvent::Created { tenant_id } => {
                self.olap.ensure_tenant_database(tenant_id)?;
                Ok(())
            }
            TenantEvent::Suspended { .. } | TenantEvent::Reactivated { .. } => {
                Err(ReconcileError::Unimplemented(
                    "tenant_suspended/reactivated: quota enforcement IP-002 deferred",
                ))
            }
            TenantEvent::Deleted { tenant_id } => {
                self.olap.drop_tenant_database(tenant_id)?;
                Ok(())
            }
        }
    }
}

#[derive(Default)]
pub struct InMemoryEventQueue {
    events: Vec<TenantEvent>,
}

impl InMemoryEventQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: TenantEvent) {
        self.events.push(event);
    }

    /// Returns a list of `(event, result)` pairs; processing continues even
    /// when an event fails (fail-open for test inspection).
    pub fn drain_and_process(
        &mut self,
        controller: &mut TenantBootstrapController<'_>,
    ) -> Vec<(TenantEvent, Result<(), ReconcileError>)> {
        let events = std::mem::take(&mut self.events);
        events
            .into_iter()
            .map(|e| {
                let result = controller.process(&e);
                (e, result)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_olap_client_kernel::memory_adapter::InMemoryOlapClient;
    use shared_olap_client_kernel::{QualifiedTable, Query, TableName};

    fn tid(s: &str) -> TenantId {
        TenantId::try_new(s).unwrap()
    }

    fn probe_query(tenant: &TenantId) -> Query {
        Query {
            source: QualifiedTable::new(tenant.clone(), TableName::try_new("events").unwrap()),
            columns: vec!["id".to_string()],
            aggregates: vec![],
            filter: None,
            group_by: vec![],
            order_by: vec![],
            limit: None,
        }
    }

    fn query_refusal(client: &InMemoryOlapClient, tenant: &TenantId) -> String {
        match client.query(tenant, &probe_query(tenant)).unwrap_err() {
            KernelError::AdapterError(message) => message,
            other => panic!("expected AdapterError, got {other}"),
        }
    }

    #[test]
    fn controller_creates_tenant_database_via_in_memory_adapter() {
        let mut client = InMemoryOlapClient::new();
        let tenant = tid("t1");
        assert_eq!(
            query_refusal(&client, &tenant),
            "database tenant_t1 does not exist"
        );

        let mut ctrl = TenantBootstrapController::new(&mut client);
        ctrl.process(&TenantEvent::Created {
            tenant_id: tenant.clone(),
        })
        .unwrap();

        assert_eq!(
            query_refusal(&client, &tenant),
            "table tenant_t1.events does not exist"
        );
    }

    #[test]
    fn controller_drops_tenant_database() {
        let mut client = InMemoryOlapClient::new();
        client.ensure_tenant_database(&tid("t1")).unwrap();
        let mut ctrl = TenantBootstrapController::new(&mut client);
        let event = TenantEvent::Deleted {
            tenant_id: tid("t1"),
        };
        ctrl.process(&event).unwrap();
        assert_eq!(
            query_refusal(&client, &tid("t1")),
            "database tenant_t1 does not exist"
        );
    }

    #[test]
    fn controller_surfaces_unimplemented_for_suspended() {
        let mut client = InMemoryOlapClient::new();
        let mut ctrl = TenantBootstrapController::new(&mut client);
        let event = TenantEvent::Suspended {
            tenant_id: tid("t1"),
        };
        let err = ctrl.process(&event).unwrap_err();
        match err {
            ReconcileError::Unimplemented(slug) => {
                assert!(slug.contains("IP-002"));
            }
            other => panic!("expected Unimplemented, got {other}"),
        }
    }

    #[test]
    fn in_memory_queue_drains_events() {
        let mut client = InMemoryOlapClient::new();
        let mut ctrl = TenantBootstrapController::new(&mut client);
        let mut queue = InMemoryEventQueue::new();
        queue.push(TenantEvent::Created {
            tenant_id: tid("t1"),
        });
        queue.push(TenantEvent::Deleted {
            tenant_id: tid("t1"),
        });
        let results = queue.drain_and_process(&mut ctrl);
        assert_eq!(results.len(), 2);
        assert!(results[0].1.is_ok());
        assert!(results[1].1.is_ok());
        assert!(queue.drain_and_process(&mut ctrl).is_empty());
    }

    #[test]
    fn tenant_event_exposes_tenant_id() {
        let event = TenantEvent::Suspended {
            tenant_id: tid("acme"),
        };
        assert_eq!(event.tenant_id().as_str(), "acme");
    }
}
