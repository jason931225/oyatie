use std::collections::BTreeSet;

use crate::{ListEntry, PageRequest, ResourceProvider};

use super::{ConformanceFixture, ConformanceViolation, MAX_PAGE_WALK, violation};

pub(super) async fn list_all<P: ResourceProvider>(
    check: &'static str,
    provider: &P,
    collection: &str,
    page_size: u32,
) -> Result<Vec<ListEntry<P::Resource>>, ConformanceViolation> {
    let mut request =
        PageRequest::first(page_size).map_err(|error| violation(check, error.to_string()))?;
    let mut all = Vec::new();
    for _ in 0..MAX_PAGE_WALK {
        let page = provider
            .list(collection, &request)
            .await
            .map_err(|e| violation(check, format!("list failed: {e}")))?;
        if page.items.len() > page_size as usize {
            return Err(violation(
                check,
                format!(
                    "page returned {} items for page_size {page_size}",
                    page.items.len()
                ),
            ));
        }
        all.extend(page.items);
        match page.next_page_token {
            Some(token) => request = request.after(token),
            None => return Ok(all),
        }
    }
    Err(violation(
        check,
        format!("pagination did not terminate within {MAX_PAGE_WALK} pages"),
    ))
}

pub async fn check_stable_pagination<F: ConformanceFixture>(
    fixture: &F,
) -> Result<(), ConformanceViolation> {
    const CHECK: &str = "stable_pagination";
    const TOTAL: u32 = 7;
    let mut provider = fixture.fresh_provider();
    let mut expected_names = BTreeSet::new();
    for ordinal in 0..TOTAL {
        let name = fixture.resource_name(ordinal)?;
        provider
            .create(
                &name,
                fixture.resource_payload(ordinal),
                &fixture.idempotency_key(10 + ordinal)?,
            )
            .await
            .map_err(|e| violation(CHECK, format!("seeding create failed: {e}")))?;
        expected_names.insert(name.to_string());
    }

    let first_walk = list_all(CHECK, &provider, fixture.collection(), 3).await?;
    let first_names: Vec<String> = first_walk.iter().map(|e| e.name.to_string()).collect();
    if first_names.len() as u32 != TOTAL {
        return Err(violation(
            CHECK,
            format!(
                "walk yielded {} entries, expected {TOTAL}",
                first_names.len()
            ),
        ));
    }
    let unique: BTreeSet<&String> = first_names.iter().collect();
    if unique.len() != first_names.len() {
        return Err(violation(
            CHECK,
            "walk yielded duplicate entries".to_owned(),
        ));
    }
    let walked: BTreeSet<String> = first_names.iter().cloned().collect();
    if walked != expected_names {
        return Err(violation(
            CHECK,
            format!("walk yielded {walked:?}, expected {expected_names:?}"),
        ));
    }

    let second_walk = list_all(CHECK, &provider, fixture.collection(), 3).await?;
    let second_names: Vec<String> = second_walk.iter().map(|e| e.name.to_string()).collect();
    if second_names != first_names {
        return Err(violation(
            CHECK,
            format!("ordering is unstable across walks: {first_names:?} vs {second_names:?}"),
        ));
    }
    Ok(())
}
