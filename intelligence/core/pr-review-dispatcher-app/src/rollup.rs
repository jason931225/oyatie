//! Rollup-verdict aggregation for the reviewer panel.

use std::collections::BTreeMap;

use crate::fanout::FacetId;

/// Per-facet recommendation from one subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetFinding {
    pub facet: FacetId,
    pub reviewer_id: String,
    pub recommendation: FacetRecommendation,
}

/// What one facet's subagent recommends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetRecommendation {
    Approve,
    ChangesRequested,
    Reject,
}

/// PR-level verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    ChangesRequested,
    Reject,
}

impl Verdict {
    /// Written verbatim into the rollup JSON, so a rename changes a
    /// consumed wire value.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::ChangesRequested => "CHANGES_REQUESTED",
            Self::Reject => "REJECT",
        }
    }

    /// Written verbatim into the admission log, so a rename changes a
    /// consumed wire value.
    #[must_use]
    pub const fn admission_event(self) -> &'static str {
        match self {
            Self::Approve => "pr-review-approved",
            Self::ChangesRequested | Self::Reject => "pr-review-fix-requested",
        }
    }
}

/// Roll a set of per-facet findings up to one PR-level verdict.
///
/// Hazard: an empty slice yields `Approve`, because no facet objected. That
/// is not the same as a reviewed approval — call
/// [`audit_panel_completeness`] first and refuse an incomplete panel.
#[must_use]
pub fn rollup_verdict(findings: &[FacetFinding]) -> Verdict {
    let mut has_change_request = false;
    for finding in findings {
        match finding.recommendation {
            FacetRecommendation::Reject => return Verdict::Reject,
            FacetRecommendation::ChangesRequested => has_change_request = true,
            FacetRecommendation::Approve => {}
        }
    }
    if has_change_request {
        Verdict::ChangesRequested
    } else {
        Verdict::Approve
    }
}

/// Result of comparing the realized panel against a required set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelCompletenessReport {
    pub required: Vec<FacetId>,
    pub present: Vec<FacetId>,
    pub missing: Vec<FacetId>,
    pub duplicate_reviewer_ids: Vec<String>,
}

impl PanelCompletenessReport {
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty() && self.duplicate_reviewer_ids.is_empty()
    }
}

/// Audit the realized panel against the required facet set.
///
/// A `reviewer_id` shared across facets means one agent wore every lens, so
/// the lenses were never independent; the panel is reported incomplete.
#[must_use]
pub fn audit_panel_completeness(
    required: &[FacetId],
    findings: &[FacetFinding],
) -> PanelCompletenessReport {
    let present_set: std::collections::BTreeSet<FacetId> =
        findings.iter().map(|f| f.facet).collect();

    let missing: Vec<FacetId> = required
        .iter()
        .copied()
        .filter(|facet| !present_set.contains(facet))
        .collect();

    PanelCompletenessReport {
        required: required.to_vec(),
        present: present_set.into_iter().collect(),
        missing,
        duplicate_reviewer_ids: reviewer_ids_spanning_multiple_facets(findings),
    }
}

fn reviewer_ids_spanning_multiple_facets(findings: &[FacetFinding]) -> Vec<String> {
    let mut facets_by_reviewer: BTreeMap<String, std::collections::BTreeSet<FacetId>> =
        BTreeMap::new();
    for finding in findings {
        facets_by_reviewer
            .entry(finding.reviewer_id.clone())
            .or_default()
            .insert(finding.facet);
    }
    facets_by_reviewer
        .into_iter()
        .filter_map(|(id, facets)| (facets.len() > 1).then_some(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approve(facet: FacetId) -> FacetFinding {
        FacetFinding {
            facet,
            reviewer_id: format!("claude-{}-changeX", facet.slug()),
            recommendation: FacetRecommendation::Approve,
        }
    }

    fn changes_requested(facet: FacetId) -> FacetFinding {
        FacetFinding {
            facet,
            reviewer_id: format!("claude-{}-changeX", facet.slug()),
            recommendation: FacetRecommendation::ChangesRequested,
        }
    }

    fn reject(facet: FacetId) -> FacetFinding {
        FacetFinding {
            facet,
            reviewer_id: format!("claude-{}-changeX", facet.slug()),
            recommendation: FacetRecommendation::Reject,
        }
    }

    #[test]
    fn empty_findings_default_to_approve() {
        assert_eq!(rollup_verdict(&[]), Verdict::Approve);
    }

    #[test]
    fn all_approve_rolls_up_to_approve() {
        let findings = vec![approve(FacetId::F1Linus), approve(FacetId::F2Hyperscaler)];
        assert_eq!(rollup_verdict(&findings), Verdict::Approve);
    }

    #[test]
    fn any_changes_requested_rolls_up_to_changes_requested() {
        let findings = vec![
            approve(FacetId::F1Linus),
            changes_requested(FacetId::F7Security),
            approve(FacetId::F2Hyperscaler),
        ];
        assert_eq!(rollup_verdict(&findings), Verdict::ChangesRequested);
    }

    #[test]
    fn any_reject_dominates_changes_requested() {
        let findings = vec![
            approve(FacetId::F1Linus),
            changes_requested(FacetId::F7Security),
            reject(FacetId::F9Compliance),
            approve(FacetId::F2Hyperscaler),
        ];
        assert_eq!(rollup_verdict(&findings), Verdict::Reject);
    }

    #[test]
    fn verdict_labels_match_check_run_contract() {
        assert_eq!(Verdict::Approve.label(), "APPROVE");
        assert_eq!(Verdict::ChangesRequested.label(), "CHANGES_REQUESTED");
        assert_eq!(Verdict::Reject.label(), "REJECT");
    }

    #[test]
    fn approve_emits_admission_event_others_emit_fix_requested() {
        assert_eq!(Verdict::Approve.admission_event(), "pr-review-approved");
        assert_eq!(
            Verdict::ChangesRequested.admission_event(),
            "pr-review-fix-requested"
        );
        assert_eq!(Verdict::Reject.admission_event(), "pr-review-fix-requested");
    }

    #[test]
    fn completeness_report_detects_missing_facet() {
        let required = vec![FacetId::F1Linus, FacetId::F7Security];
        let findings = vec![approve(FacetId::F1Linus)];
        let report = audit_panel_completeness(&required, &findings);
        assert_eq!(report.missing, vec![FacetId::F7Security]);
        assert!(!report.is_complete());
    }

    #[test]
    fn completeness_report_detects_reviewer_id_duplicated_across_facets() {
        let findings = vec![
            FacetFinding {
                facet: FacetId::F1Linus,
                reviewer_id: "claude-omnibus-changeX".into(),
                recommendation: FacetRecommendation::Approve,
            },
            FacetFinding {
                facet: FacetId::F2Hyperscaler,
                reviewer_id: "claude-omnibus-changeX".into(),
                recommendation: FacetRecommendation::Approve,
            },
        ];
        let required = vec![FacetId::F1Linus, FacetId::F2Hyperscaler];
        let report = audit_panel_completeness(&required, &findings);
        assert!(report.missing.is_empty());
        assert_eq!(
            report.duplicate_reviewer_ids,
            vec!["claude-omnibus-changeX"]
        );
        assert!(!report.is_complete());
    }

    #[test]
    fn complete_panel_no_duplicates_reports_complete() {
        let required = vec![FacetId::F1Linus, FacetId::F2Hyperscaler];
        let findings = vec![approve(FacetId::F1Linus), approve(FacetId::F2Hyperscaler)];
        let report = audit_panel_completeness(&required, &findings);
        assert!(report.is_complete());
    }
}
