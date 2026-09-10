//! Acceptance tests for invoice-lifecycle transitions: `get_invoice` point
//! lookup, idempotent same-state re-transition, sequential and boundary credit
//! notes, zero-amount rejection, and a credit note against a Paid invoice.

use billing_domain::{
    BillingAccount, BillingAccountCreate, BillingAccountState, BillingPeriod, CloudBillingError,
    CloudBillingLedger, CreditNoteCreate, InvoiceGenerate, InvoiceLineItemCreate, InvoiceState,
    Money, TaxInvoiceFormat,
};
use billing_metering::{MeterUnit, MeterUnitKind};
use data_boundary_kernel::DataClass;

fn units() -> Vec<MeterUnit> {
    vec![
        MeterUnit::new(MeterUnitKind::ResourceSecond, 3_600_000_000)
            .expect("unit fixture is valid"),
    ]
}

fn account_create() -> BillingAccountCreate {
    BillingAccountCreate {
        id: "ba_ten_alpha".to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-alpha".to_string(),
        regional_pack: "pack-electronic-tax".to_string(),
        payment_method: "pm_card_001".to_string(),
        credit_balance: Money::new("OYC", 10_000).expect("money fixture valid"),
        state: BillingAccountState::Active,
        data_class: DataClass::Financial,
        created_at_epoch_seconds: 1_700_000_000,
    }
}

fn invoice_line_item() -> InvoiceLineItemCreate {
    InvoiceLineItemCreate {
        id: "ili_compute_001".to_string(),
        resource_id: "oyatie:cloud:region-alpha:ten_alpha:instance:api-001".to_string(),
        description: "instance api-001 resource seconds".to_string(),
        units: units(),
        subtotal: Money::new("OYC", 100_000).expect("money fixture valid"),
        data_class: DataClass::Financial,
    }
}

fn invoice_generate() -> InvoiceGenerate {
    InvoiceGenerate {
        id: "inv_alpha_202605_001".to_string(),
        billing_account_id: "ba_ten_alpha".to_string(),
        tenant_id: "ten_alpha".to_string(),
        regional_pack: "pack-electronic-tax".to_string(),
        period: BillingPeriod::new(1_700_000_000, 1_700_086_400).expect("period fixture valid"),
        line_items: vec![invoice_line_item()],
        subtotal: Money::new("OYC", 100_000).expect("money fixture valid"),
        tax: Money::new("OYC", 10_000).expect("money fixture valid"),
        total: Money::new("OYC", 110_000).expect("money fixture valid"),
        tax_invoice_format: TaxInvoiceFormat::ElectronicTaxInvoice,
        tax_registration_id: "taxid/electronic/1234567890".to_string(),
        issued_at_epoch_seconds: 1_700_086_500,
        due_at_epoch_seconds: 1_700_604_900,
        data_class: DataClass::Financial,
    }
}

fn ledger_with_issued_invoice() -> (CloudBillingLedger, billing_domain::InvoiceId) {
    let account = BillingAccount::new(account_create()).expect("account fixture valid");
    let mut ledger = CloudBillingLedger::default();
    let invoice = ledger
        .generate_invoice(&account, invoice_generate())
        .expect("invoice fixture valid");
    let id = invoice.id.value.clone();
    (ledger, id)
}

fn credit_note(credit_minor_units: u64, line_item_suffix: &str) -> CreditNoteCreate {
    CreditNoteCreate {
        invoice_id: "inv_alpha_202605_001".to_string(),
        line_item_id: format!("ili_credit_{line_item_suffix}"),
        resource_id: "oyatie:cloud:region-alpha:ten_alpha:instance:api-001".to_string(),
        description: "compute overage correction".to_string(),
        units: units(),
        credit_minor_units,
        currency: "OYC".to_string(),
        data_class: DataClass::Financial,
    }
}

/// `get_invoice` returns `Some(&Invoice)` for an id known to the ledger.
#[test]
fn get_invoice_returns_some_for_known_id() {
    let (ledger, id) = ledger_with_issued_invoice();
    let inv = ledger
        .get_invoice(&id)
        .expect("known invoice id returns Some");
    assert_eq!(inv.id.value, id);
    assert_eq!(inv.state.value, InvoiceState::Issued);
}

/// `get_invoice` returns `None` for an id not in the ledger.
#[test]
fn get_invoice_returns_none_for_unknown_id() {
    let ledger = CloudBillingLedger::default();
    let unknown = billing_domain::InvoiceId::new("inv_unknown_999").expect("id fixture valid");
    assert!(
        ledger.get_invoice(&unknown).is_none(),
        "unknown invoice id returns None"
    );
}

/// `get_invoice` reflects the mutated state after a transition.
#[test]
fn get_invoice_reflects_state_after_transition() {
    let (mut ledger, id) = ledger_with_issued_invoice();
    ledger
        .transition_invoice(&id, InvoiceState::Paid)
        .expect("Issued -> Paid is legal");
    let inv = ledger
        .get_invoice(&id)
        .expect("invoice id still valid after transition");
    assert_eq!(
        inv.state.value,
        InvoiceState::Paid,
        "get_invoice must return the post-transition state"
    );
}

/// Re-transitioning an already-Paid invoice to Paid succeeds idempotently.
#[test]
fn transition_paid_to_paid_is_idempotent() {
    let (mut ledger, id) = ledger_with_issued_invoice();
    ledger
        .transition_invoice(&id, InvoiceState::Paid)
        .expect("Issued -> Paid first");
    let inv = ledger
        .transition_invoice(&id, InvoiceState::Paid)
        .expect("Paid -> Paid must be idempotent, not an error");
    assert_eq!(
        inv.state.value,
        InvoiceState::Paid,
        "state must remain Paid after idempotent re-transition"
    );
}

/// Re-transitioning an already-Void invoice to Void succeeds idempotently.
#[test]
fn transition_void_to_void_is_idempotent() {
    let (mut ledger, id) = ledger_with_issued_invoice();
    ledger
        .transition_invoice(&id, InvoiceState::Void)
        .expect("Issued -> Void first");
    let inv = ledger
        .transition_invoice(&id, InvoiceState::Void)
        .expect("Void -> Void must be idempotent, not an error");
    assert_eq!(
        inv.state.value,
        InvoiceState::Void,
        "state must remain Void after idempotent re-transition"
    );
}

/// Re-transitioning an Overdue invoice to Overdue succeeds idempotently.
#[test]
fn transition_overdue_to_overdue_is_idempotent() {
    let (mut ledger, id) = ledger_with_issued_invoice();
    ledger
        .transition_invoice(&id, InvoiceState::Overdue)
        .expect("Issued -> Overdue first");
    let inv = ledger
        .transition_invoice(&id, InvoiceState::Overdue)
        .expect("Overdue -> Overdue must be idempotent, not an error");
    assert_eq!(
        inv.state.value,
        InvoiceState::Overdue,
        "state must remain Overdue after idempotent re-transition"
    );
}

/// Two successive credit notes each reduce the subtotal; the second over-credit
/// guard operates on the already-reduced amount, not the original.
#[test]
fn sequential_credit_notes_reduce_subtotal_cumulatively() {
    let (mut ledger, _id) = ledger_with_issued_invoice();
    let inv = ledger
        .apply_credit_note("ten_alpha", credit_note(60_000, "001"))
        .expect("first credit note valid");
    assert_eq!(
        inv.subtotal.value.minor_units, 40_000,
        "after first credit of 60_000 subtotal is 40_000"
    );

    let inv = ledger
        .apply_credit_note("ten_alpha", credit_note(40_000, "002"))
        .expect("second credit note reducing to zero is valid");
    assert_eq!(
        inv.subtotal.value.minor_units, 0,
        "after second credit of 40_000 subtotal is 0"
    );

    let err = ledger
        .apply_credit_note("ten_alpha", credit_note(1, "003"))
        .expect_err("credit note against zero subtotal is rejected");
    assert_eq!(
        err,
        CloudBillingError::CreditNoteOverCredit,
        "any credit against a zero subtotal is CreditNoteOverCredit"
    );
}

/// A credit note whose `credit_minor_units` equals the full subtotal reduces it
/// to zero: the guard rejects only `>`, not `==`.
#[test]
fn credit_note_exact_full_subtotal_reduces_to_zero() {
    let (mut ledger, _id) = ledger_with_issued_invoice();
    let inv = ledger
        .apply_credit_note("ten_alpha", credit_note(100_000, "full"))
        .expect("crediting the full subtotal is valid");
    assert_eq!(
        inv.subtotal.value.minor_units, 0,
        "crediting 100_000 against a 100_000 subtotal produces 0"
    );
    assert_eq!(
        inv.line_items.value.len(),
        2,
        "credit note line item is appended"
    );
}

/// A credit note with `credit_minor_units = 0` is rejected with
/// `InvalidInvoiceLineItem` (zero-amount line items are never valid).
#[test]
fn credit_note_zero_amount_rejected() {
    let (mut ledger, _id) = ledger_with_issued_invoice();
    let err = ledger
        .apply_credit_note("ten_alpha", credit_note(0, "zero"))
        .expect_err("zero-amount credit note is never valid");
    assert_eq!(
        err,
        CloudBillingError::InvalidInvoiceLineItem,
        "zero credit_minor_units must yield InvalidInvoiceLineItem"
    );
}

/// A credit note against an invoice in Paid state is accepted.
/// Only Void blocks credit notes per the domain spec.
#[test]
fn credit_note_against_paid_invoice_succeeds() {
    let (mut ledger, id) = ledger_with_issued_invoice();
    ledger
        .transition_invoice(&id, InvoiceState::Paid)
        .expect("Issued -> Paid is legal");
    let inv = ledger
        .apply_credit_note("ten_alpha", credit_note(10_000, "refund"))
        .expect("post-payment credit note (refund) against Paid invoice is valid");
    assert_eq!(
        inv.subtotal.value.minor_units, 90_000,
        "refund credit reduces Paid invoice subtotal"
    );
}
