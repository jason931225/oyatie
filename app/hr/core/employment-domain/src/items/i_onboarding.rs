/// The kinds of pre-hire onboarding checklist items.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OnboardingChecklistItemKind {
    RightToWorkI9,
    BackgroundCheck,
    EquipmentProvisioning,
    AccessGrant,
    MandatoryTraining,
}

/// A single item on the onboarding checklist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardingChecklistItem {
    pub kind: OnboardingChecklistItemKind,
    pub is_mandatory: bool,
    pub is_cleared: bool,
    pub evidence_ref: Option<AuditEvidenceRef>,
}

/// Input to the onboarding readiness evaluator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardingReadinessInput {
    pub employee_id: String,
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub checklist: Vec<OnboardingChecklistItem>,
    pub evaluated_at_epoch_seconds: u64,
}

/// The outcome of the onboarding readiness evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OnboardingDecision {
    Ready,
    NotReady,
}

/// Decision output from `evaluate_onboarding_readiness`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardingReadinessDecision {
    pub employee_id: Classified<EmployeeId>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub decision: Classified<OnboardingDecision>,
    pub outstanding_items: Classified<Vec<OnboardingChecklistItemKind>>,
    pub schema_version: Classified<u32>,
}

/// Pure evaluator: validates identifiers, rejects empty/duplicate checklists,
/// and determines whether all mandatory items are cleared with evidence.
///
/// Returns `Ok(OnboardingReadinessDecision)` on a valid, non-duplicate input.
/// Returns `Err(HrDomainError)` for invalid identifiers, empty checklist,
/// duplicate item kinds, zero `evaluated_at_epoch_seconds`, or a mandatory
/// item that is marked cleared but supplies no evidence ref.
pub fn evaluate_onboarding_readiness(
    input: OnboardingReadinessInput,
) -> Result<OnboardingReadinessDecision, HrDomainError> {
    validate_identifier(
        &input.employee_id,
        EMPLOYEE_ID_PREFIX,
        HrDomainError::InvalidEmployeeId,
    )?;
    validate_identifier(
        &input.tenant_id,
        TENANT_ID_PREFIX,
        HrDomainError::InvalidTenantId,
    )?;
    validate_identifier(
        &input.legal_entity_id,
        LEGAL_ENTITY_ID_PREFIX,
        HrDomainError::InvalidLegalEntityId,
    )?;
    if input.evaluated_at_epoch_seconds == 0 {
        return Err(HrDomainError::InvalidEvaluatedAt);
    }
    if input.checklist.is_empty() {
        return Err(HrDomainError::OnboardingItemsRequired);
    }

    // Reject duplicate item kinds.
    let mut seen = std::collections::HashSet::new();
    for item in &input.checklist {
        if !seen.insert(item.kind) {
            return Err(HrDomainError::DuplicateOnboardingItem);
        }
    }

    // Validate all evidence refs supplied on items (mandatory or optional).
    // Also enforce: a mandatory item marked is_cleared=true MUST supply an
    // evidence ref — cleared without evidence is an error, not a NOT_READY.
    for item in &input.checklist {
        if let Some(ref ev) = item.evidence_ref {
            validate_evidence_ref(&ev.value)?;
        }
        if item.is_mandatory && item.is_cleared && item.evidence_ref.is_none() {
            return Err(HrDomainError::OnboardingItemNotCleared);
        }
    }

    // Build a lookup of checklist items by kind for absent-kind detection.
    let item_map: std::collections::HashMap<OnboardingChecklistItemKind, &OnboardingChecklistItem> =
        input.checklist.iter().map(|i| (i.kind, i)).collect();

    // Canonical mandatory kinds per spec (right-to-work/I-9, background-check,
    // mandatory training are always required when at least one mandatory item
    // is present in the checklist).
    const CANONICAL_MANDATORY: [OnboardingChecklistItemKind; 3] = [
        OnboardingChecklistItemKind::RightToWorkI9,
        OnboardingChecklistItemKind::BackgroundCheck,
        OnboardingChecklistItemKind::MandatoryTraining,
    ];

    let has_any_mandatory = input.checklist.iter().any(|i| i.is_mandatory);

    let mut outstanding_set = std::collections::HashSet::new();

    // Items present and marked mandatory but not cleared+evidenced.
    for item in &input.checklist {
        if item.is_mandatory && !(item.is_cleared && item.evidence_ref.is_some()) {
            outstanding_set.insert(item.kind);
        }
    }

    // When the checklist has at least one mandatory item, canonical mandatory
    // kinds that are entirely absent from the checklist are also blockers.
    if has_any_mandatory {
        for kind in &CANONICAL_MANDATORY {
            if !item_map.contains_key(kind) {
                outstanding_set.insert(*kind);
            }
        }
    }

    let mut outstanding: Vec<OnboardingChecklistItemKind> = outstanding_set.into_iter().collect();
    outstanding.sort();

    let decision = if outstanding.is_empty() {
        OnboardingDecision::Ready
    } else {
        OnboardingDecision::NotReady
    };

    Ok(OnboardingReadinessDecision {
        employee_id: internal(EmployeeId {
            value: input.employee_id,
        }),
        tenant_id: internal(TenantId {
            value: input.tenant_id,
        }),
        legal_entity_id: internal(LegalEntityId {
            value: input.legal_entity_id,
        }),
        decision: internal(decision),
        outstanding_items: internal(outstanding),
        schema_version: public(ONBOARDING_READINESS_SCHEMA_VERSION),
    })
}
