use shared_resource_provider_contract_kernel::{
    CreateOutcome, IdempotencyKey, ProviderError, ProviderFuture, PutOutcome, ResourceName,
    WriteDisposition,
};

use super::support::{AppliedWrite, Document, ReferenceProvider};

pub(super) fn create<'a>(
    provider: &'a mut ReferenceProvider,
    name: &'a ResourceName,
    resource: Document,
    idempotency_key: &'a IdempotencyKey,
) -> ProviderFuture<'a, CreateOutcome<Document>> {
    Box::pin(async move {
        let key = idempotency_key.as_str().to_owned();
        if let Some(applied) = provider.dedup_log.get(&key) {
            return match applied {
                AppliedWrite::Create { name: n, payload }
                    if *n == name.to_string() && *payload == resource =>
                {
                    Ok(CreateOutcome {
                        resource: payload.clone(),
                        replayed: true,
                    })
                }
                _ => Err(ProviderError::IdempotencyKeyReuse { key }),
            };
        }
        if provider.items.contains_key(&name.to_string()) {
            return Err(ProviderError::AlreadyExists {
                name: name.to_string(),
            });
        }
        provider.items.insert(name.to_string(), resource.clone());
        provider.dedup_log.insert(
            key,
            AppliedWrite::Create {
                name: name.to_string(),
                payload: resource.clone(),
            },
        );
        Ok(CreateOutcome {
            resource,
            replayed: false,
        })
    })
}

pub(super) fn put<'a>(
    provider: &'a mut ReferenceProvider,
    name: &'a ResourceName,
    resource: Document,
    idempotency_key: &'a IdempotencyKey,
) -> ProviderFuture<'a, PutOutcome<Document>> {
    Box::pin(async move {
        let key = idempotency_key.as_str().to_owned();
        if let Some(applied) = provider.dedup_log.get(&key) {
            return match applied {
                AppliedWrite::Put { name: n, payload }
                    if *n == name.to_string() && *payload == resource =>
                {
                    Ok(PutOutcome {
                        resource: payload.clone(),
                        disposition: WriteDisposition::Replayed,
                    })
                }
                _ => Err(ProviderError::IdempotencyKeyReuse { key }),
            };
        }
        let disposition = if provider.items.contains_key(&name.to_string()) {
            WriteDisposition::Replaced
        } else {
            WriteDisposition::Created
        };
        provider.items.insert(name.to_string(), resource.clone());
        provider.dedup_log.insert(
            key,
            AppliedWrite::Put {
                name: name.to_string(),
                payload: resource.clone(),
            },
        );
        Ok(PutOutcome {
            resource,
            disposition,
        })
    })
}
