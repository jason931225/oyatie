#[test]
fn mandatory_item_cleared_without_evidence_returns_error() {
    let mut input = all_cleared_input();
    for item in &mut input.checklist {
        if item.kind == OnboardingChecklistItemKind::MandatoryTraining {
            item.is_cleared = true;
            item.evidence_ref = None; // cleared flag set but no AuditEvidenceRef
        }
    }

    let err = evaluate_onboarding_readiness(input)
        .expect_err("cleared mandatory item without evidence must return OnboardingItemNotCleared");

    assert_eq!(err, HrDomainError::OnboardingItemNotCleared);
}

#[test]
fn empty_checklist_returns_onboarding_items_required_error() {
    let input = OnboardingReadinessInput {
        employee_id: "emp_001".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        evaluated_at_epoch_seconds: 1_700_000_000,
        checklist: vec![],
    };

    let err =
        evaluate_onboarding_readiness(input).expect_err("empty checklist must return an error");

    assert_eq!(err, HrDomainError::OnboardingItemsRequired);
}

#[test]
fn duplicate_checklist_item_kinds_returns_error() {
    let mut input = all_cleared_input();
    // Duplicate RightToWorkI9
    input.checklist.push(OnboardingChecklistItem {
        kind: OnboardingChecklistItemKind::RightToWorkI9,
        is_mandatory: true,
        is_cleared: true,
        evidence_ref: Some(valid_evidence("rtw/duplicate")),
    });

    let err = evaluate_onboarding_readiness(input)
        .expect_err("duplicate item kinds must return an error");

    assert_eq!(err, HrDomainError::DuplicateOnboardingItem);
}

#[test]
fn invalid_employee_id_returns_invalid_employee_id_error() {
    let input = OnboardingReadinessInput {
        employee_id: "not-prefixed".to_owned(),
        ..all_cleared_input()
    };

    let err = evaluate_onboarding_readiness(input)
        .expect_err("invalid employee_id must return InvalidEmployeeId");

    assert_eq!(err, HrDomainError::InvalidEmployeeId);
}

#[test]
fn invalid_tenant_id_returns_invalid_tenant_id_error() {
    let input = OnboardingReadinessInput {
        tenant_id: "ten_".to_owned(), // prefix-only, empty suffix
        ..all_cleared_input()
    };

    let err = evaluate_onboarding_readiness(input)
        .expect_err("prefix-only tenant_id must return InvalidTenantId");

    assert_eq!(err, HrDomainError::InvalidTenantId);
}

#[test]
fn invalid_legal_entity_id_returns_invalid_legal_entity_id_error() {
    let input = OnboardingReadinessInput {
        legal_entity_id: "le_../escape".to_owned(),
        ..all_cleared_input()
    };

    let err = evaluate_onboarding_readiness(input)
        .expect_err("path-traversal legal_entity_id must return InvalidLegalEntityId");

    assert_eq!(err, HrDomainError::InvalidLegalEntityId);
}

#[test]
fn invalid_evidence_ref_on_cleared_mandatory_item_returns_error() {
    let mut input = all_cleared_input();
    for item in &mut input.checklist {
        if item.kind == OnboardingChecklistItemKind::RightToWorkI9 {
            item.evidence_ref = Some(AuditEvidenceRef {
                value: "audit/hr/onboarding/bearer-token".to_owned(),
            });
        }
    }

    let err = evaluate_onboarding_readiness(input)
        .expect_err("credential-shaped evidence ref must be rejected");

    assert_eq!(err, HrDomainError::InvalidAuditEvidenceRef);
}

#[test]
fn zero_evaluated_at_returns_invalid_evaluated_at_error() {
    let input = OnboardingReadinessInput {
        evaluated_at_epoch_seconds: 0,
        ..all_cleared_input()
    };

    let err = evaluate_onboarding_readiness(input)
        .expect_err("zero evaluated_at_epoch_seconds must return InvalidEvaluatedAt");

    assert_eq!(err, HrDomainError::InvalidEvaluatedAt);
}
