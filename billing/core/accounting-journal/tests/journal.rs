#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use billing_accounting_journal::{
    AccountingDomainError, JournalLineInput, PeriodState, idempotency_body_fingerprint,
    post_journal, scoped_idempotency_key,
};

mod support;
use support::journal_input;

#[test]
fn fingerprint_is_deterministic() {
    let a = idempotency_body_fingerprint(&["ten_acme", "jrn_1", "100"]);
    let b = idempotency_body_fingerprint(&["ten_acme", "jrn_1", "100"]);
    assert_eq!(a, b);
    assert_eq!(a.len(), 16, "fingerprint is 16 hex digits");
}

#[test]
fn fingerprint_detects_changed_body() {
    let original = idempotency_body_fingerprint(&["ten_acme", "jrn_1", "100"]);
    let changed = idempotency_body_fingerprint(&["ten_acme", "jrn_1", "101"]);
    assert_ne!(
        original, changed,
        "a changed body field must change the fingerprint"
    );
}

#[test]
fn fingerprint_is_length_prefixed_against_field_boundary_collisions() {
    // Without length-prefixing, ["ab", "c"] and ["a", "bc"] would concatenate
    // to the same byte stream. They must produce different fingerprints.
    let left = idempotency_body_fingerprint(&["ab", "c"]);
    let right = idempotency_body_fingerprint(&["a", "bc"]);
    assert_ne!(left, right, "field boundaries must be unambiguous");
}

#[test]
fn scoped_key_places_tenant_first() {
    let key = scoped_idempotency_key("ten_acme", "journal-posted", "jrn_1");
    assert_eq!(key, "idem-v2:ten_acme:journal-posted:jrn_1");
    let other = scoped_idempotency_key("ten_beta", "journal-posted", "jrn_1");
    assert_ne!(key, other);
}

#[test]
fn test_post_requires_balanced_open_period() {
    let journal = post_journal(journal_input()).expect("posted journal");

    assert_eq!(
        journal.total_debit_minor.value,
        journal.total_credit_minor.value
    );
    assert_eq!(journal.legal_entity_id.value.value, "le_kr_001");
    assert_eq!(journal.source_documents.value.len(), 1);

    let mut unbalanced = journal_input();
    unbalanced.lines[1].credit_minor = 99;
    assert_eq!(
        post_journal(unbalanced),
        Err(AccountingDomainError::UnbalancedJournal)
    );

    let mut self_balancing_line = journal_input();
    self_balancing_line.lines = vec![JournalLineInput {
        account_code: "EXP-WAGES".to_owned(),
        debit_minor: 100,
        credit_minor: 100,
    }];
    assert_eq!(
        post_journal(self_balancing_line),
        Err(AccountingDomainError::InvalidMoney)
    );

    let mut closed_period = journal_input();
    closed_period.period_state = PeriodState::Closed;
    assert_eq!(
        post_journal(closed_period),
        Err(AccountingDomainError::PeriodNotOpen)
    );

    let mut missing_source = journal_input();
    missing_source.source_documents.clear();
    assert_eq!(
        post_journal(missing_source),
        Err(AccountingDomainError::InvalidSourceDocumentRef)
    );
}
