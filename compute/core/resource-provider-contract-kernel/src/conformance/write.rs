use crate::{ProviderError, ResourceProvider, WriteDisposition};

use super::listing::list_all;
use super::{ConformanceFixture, ConformanceViolation, violation};

pub async fn check_idempotent_put<F: ConformanceFixture>(
    fixture: &F,
) -> Result<(), ConformanceViolation> {
    const CHECK: &str = "idempotent_put";
    let mut provider = fixture.fresh_provider();
    let name = fixture.resource_name(1)?;
    let first_payload = fixture.resource_payload(1);
    let second_payload = fixture.resource_payload(2);
    let key_a = fixture.idempotency_key(1)?;
    let key_b = fixture.idempotency_key(2)?;

    let first = provider
        .put(&name, first_payload.clone(), &key_a)
        .await
        .map_err(|e| violation(CHECK, format!("initial put failed: {e}")))?;
    if first.disposition != WriteDisposition::Created {
        return Err(violation(
            CHECK,
            format!(
                "initial put must report created, got {:?}",
                first.disposition
            ),
        ));
    }

    let replay = provider
        .put(&name, first_payload.clone(), &key_a)
        .await
        .map_err(|e| violation(CHECK, format!("replayed put failed: {e}")))?;
    if replay.disposition != WriteDisposition::Replayed {
        return Err(violation(
            CHECK,
            format!(
                "replay under the same key must report replayed, got {:?}",
                replay.disposition
            ),
        ));
    }
    if replay.resource != first_payload {
        return Err(violation(
            CHECK,
            format!(
                "replay must return the original resource, got {:?}",
                replay.resource
            ),
        ));
    }
    let read = provider
        .get(&name)
        .await
        .map_err(|e| violation(CHECK, format!("get after replay failed: {e}")))?;
    if read != first_payload {
        return Err(violation(CHECK, "replay mutated stored state".to_owned()));
    }

    let replaced = provider
        .put(&name, second_payload.clone(), &key_b)
        .await
        .map_err(|e| violation(CHECK, format!("replacing put failed: {e}")))?;
    if replaced.disposition != WriteDisposition::Replaced {
        return Err(violation(
            CHECK,
            format!(
                "new-key put over an existing name must report replaced, got {:?}",
                replaced.disposition
            ),
        ));
    }
    let read = provider
        .get(&name)
        .await
        .map_err(|e| violation(CHECK, format!("get after replace failed: {e}")))?;
    if read != second_payload {
        return Err(violation(
            CHECK,
            "replace did not become visible".to_owned(),
        ));
    }
    Ok(())
}

pub async fn check_create_idempotency<F: ConformanceFixture>(
    fixture: &F,
) -> Result<(), ConformanceViolation> {
    const CHECK: &str = "create_idempotency";
    let mut provider = fixture.fresh_provider();
    let name = fixture.resource_name(1)?;
    let payload = fixture.resource_payload(1);
    let other_payload = fixture.resource_payload(2);
    let key = fixture.idempotency_key(1)?;
    let other_key = fixture.idempotency_key(2)?;

    let created = provider
        .create(&name, payload.clone(), &key)
        .await
        .map_err(|e| violation(CHECK, format!("initial create failed: {e}")))?;
    if created.replayed {
        return Err(violation(
            CHECK,
            "initial create must not be a replay".to_owned(),
        ));
    }

    let replay = provider
        .create(&name, payload.clone(), &key)
        .await
        .map_err(|e| {
            violation(
                CHECK,
                format!("create retry under the same key failed: {e}"),
            )
        })?;
    if !replay.replayed || replay.resource != payload {
        return Err(violation(
            CHECK,
            "create retry under the same key must replay the original resource".to_owned(),
        ));
    }

    match provider.create(&name, other_payload, &key).await {
        Err(ProviderError::IdempotencyKeyReuse { .. }) => {}
        other => {
            return Err(violation(
                CHECK,
                format!(
                    "key reuse with different parameters must fail with idempotency_key_reuse, got {other:?}"
                ),
            ));
        }
    }

    match provider.create(&name, payload, &other_key).await {
        Err(ProviderError::AlreadyExists { .. }) => {}
        other => {
            return Err(violation(
                CHECK,
                format!(
                    "creating an existing name under a new key must fail with already_exists, got {other:?}"
                ),
            ));
        }
    }

    let all = list_all(CHECK, &provider, fixture.collection(), 10).await?;
    if all.len() != 1 {
        return Err(violation(
            CHECK,
            format!(
                "exactly one resource must exist after retries, found {}",
                all.len()
            ),
        ));
    }
    Ok(())
}

pub async fn check_read_after_write<F: ConformanceFixture>(
    fixture: &F,
) -> Result<(), ConformanceViolation> {
    const CHECK: &str = "read_after_write";
    let mut provider = fixture.fresh_provider();
    let created_name = fixture.resource_name(1)?;
    let put_name = fixture.resource_name(2)?;
    let created_payload = fixture.resource_payload(1);
    let put_payload = fixture.resource_payload(2);

    provider
        .create(
            &created_name,
            created_payload.clone(),
            &fixture.idempotency_key(1)?,
        )
        .await
        .map_err(|e| violation(CHECK, format!("create failed: {e}")))?;
    let read = provider
        .get(&created_name)
        .await
        .map_err(|e| violation(CHECK, format!("get after create failed: {e}")))?;
    if read != created_payload {
        return Err(violation(
            CHECK,
            format!("get after create returned {read:?}, expected {created_payload:?}"),
        ));
    }

    provider
        .put(&put_name, put_payload.clone(), &fixture.idempotency_key(2)?)
        .await
        .map_err(|e| violation(CHECK, format!("put failed: {e}")))?;
    let read = provider
        .get(&put_name)
        .await
        .map_err(|e| violation(CHECK, format!("get after put failed: {e}")))?;
    if read != put_payload {
        return Err(violation(
            CHECK,
            format!("get after put returned {read:?}, expected {put_payload:?}"),
        ));
    }

    match provider.get(&fixture.resource_name(99)?).await {
        Err(ProviderError::NotFound { .. }) => Ok(()),
        other => Err(violation(
            CHECK,
            format!("get of an unknown name must fail with not_found, got {other:?}"),
        )),
    }
}
