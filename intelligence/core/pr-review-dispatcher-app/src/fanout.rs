//! Reviewer-panel topology.

/// One critique lens in the reviewer panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FacetId {
    // Critique lenses.
    F1Linus,
    F2Hyperscaler,
    F3Adversarial,
    F4Ergonomic,
    F5Quality,
    F6Alternatives,
    F7Security,
    F8Performance,
    F9Compliance,
    F10Reversibility,
    F11Observability,
    F13Migration,

    // Meta lenses.
    M1ChallengeAssumption,
    M2ZoomedOutFit,

    // Own-policy adherence lenses.
    A1NamingAdherence,
    A2DocumentationAdherence,
    A3StructureAdherence,
    A4ArchitectureAdherence,
    A5DependencyAdherence,
    A6SchemaAdherence,
    A7AlgorithmAdherence,
}

impl FacetId {
    /// Identifier used in the reviewer id and the evidence filename built by
    /// [`FacetDispatch::new`].
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::F1Linus => "F1_linus",
            Self::F2Hyperscaler => "F2_hyperscaler",
            Self::F3Adversarial => "F3_adversarial",
            Self::F4Ergonomic => "F4_ergonomic",
            Self::F5Quality => "F5_quality",
            Self::F6Alternatives => "F6_alternatives",
            Self::F7Security => "F7_security",
            Self::F8Performance => "F8_performance",
            Self::F9Compliance => "F9_compliance",
            Self::F10Reversibility => "F10_reversibility",
            Self::F11Observability => "F11_observability",
            Self::F13Migration => "F13_migration",
            Self::M1ChallengeAssumption => "M1_challenge_assumption",
            Self::M2ZoomedOutFit => "M2_zoomed_out_fit",
            Self::A1NamingAdherence => "A1_naming_adherence",
            Self::A2DocumentationAdherence => "A2_documentation_adherence",
            Self::A3StructureAdherence => "A3_structure_adherence",
            Self::A4ArchitectureAdherence => "A4_architecture_adherence",
            Self::A5DependencyAdherence => "A5_dependency_adherence",
            Self::A6SchemaAdherence => "A6_schema_adherence",
            Self::A7AlgorithmAdherence => "A7_algorithm_adherence",
        }
    }

    /// The baseline subset of [`Self::full_panel_v23`].
    #[must_use]
    pub const fn baseline_always_required() -> [Self; 9] {
        [
            Self::F1Linus,
            Self::F2Hyperscaler,
            Self::F3Adversarial,
            Self::F4Ergonomic,
            Self::F5Quality,
            Self::F6Alternatives,
            Self::F7Security,
            Self::F8Performance,
            Self::F9Compliance,
        ]
    }

    /// Every facet, for the change classes that trigger the whole panel.
    #[must_use]
    pub const fn full_panel_v23() -> [Self; 21] {
        [
            Self::F1Linus,
            Self::F2Hyperscaler,
            Self::F3Adversarial,
            Self::F4Ergonomic,
            Self::F5Quality,
            Self::F6Alternatives,
            Self::F7Security,
            Self::F8Performance,
            Self::F9Compliance,
            Self::F10Reversibility,
            Self::F11Observability,
            Self::F13Migration,
            Self::M1ChallengeAssumption,
            Self::M2ZoomedOutFit,
            Self::A1NamingAdherence,
            Self::A2DocumentationAdherence,
            Self::A3StructureAdherence,
            Self::A4ArchitectureAdherence,
            Self::A5DependencyAdherence,
            Self::A6SchemaAdherence,
            Self::A7AlgorithmAdherence,
        ]
    }
}

/// A request to fan out one facet to one subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetDispatch {
    pub change_id: String,
    pub facet: FacetId,
    pub reviewer_id: String,
    /// Where the subagent must write its finding.
    pub evidence_path: String,
}

impl FacetDispatch {
    /// `tool` is the subagent runtime tag, e.g. `claude-critic`.
    #[must_use]
    pub fn new(tool: &str, facet: FacetId, change_id: &str) -> Self {
        let reviewer_id = format!("{tool}-{slug}-{change_id}", slug = facet.slug());
        let evidence_path = format!(
            "evidence/pipeline-maturity-glue/ip-004-pr-review/{change_id}/{slug}.json",
            slug = facet.slug()
        );
        Self {
            change_id: change_id.to_string(),
            facet,
            reviewer_id,
            evidence_path,
        }
    }
}

/// Build the fan-out plan for one PR. Deterministic in both order and
/// reviewer id, so an audit-chain replay reproduces it exactly.
#[must_use]
pub fn fan_out_facets(change_id: &str, tool: &str) -> Vec<FacetDispatch> {
    FacetId::full_panel_v23()
        .into_iter()
        .map(|facet| FacetDispatch::new(tool, facet, change_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_panel_v23_has_21_distinct_facets() {
        let panel = FacetId::full_panel_v23();
        let mut slugs: Vec<&'static str> = panel.iter().map(|f| f.slug()).collect();
        slugs.sort_unstable();
        let original_len = slugs.len();
        slugs.dedup();
        assert_eq!(slugs.len(), 21);
        assert_eq!(original_len, 21);
    }

    #[test]
    fn baseline_panel_is_always_required_f1_through_f9() {
        let baseline = FacetId::baseline_always_required();
        assert_eq!(baseline.len(), 9);
        assert_eq!(baseline[0], FacetId::F1Linus);
        assert_eq!(baseline[8], FacetId::F9Compliance);
    }

    #[test]
    fn fan_out_produces_distinct_reviewer_ids_per_facet() {
        let plan = fan_out_facets("M01-P17-IP-004-pr42", "claude-critic");
        let mut reviewer_ids: Vec<String> = plan.iter().map(|d| d.reviewer_id.clone()).collect();
        reviewer_ids.sort();
        let original = reviewer_ids.len();
        reviewer_ids.dedup();
        assert_eq!(reviewer_ids.len(), 21);
        assert_eq!(original, 21);
    }

    #[test]
    fn fan_out_evidence_path_matches_acceptance_contract() {
        let plan = fan_out_facets("pr42", "claude-critic");
        let f1 = plan
            .iter()
            .find(|d| d.facet == FacetId::F1Linus)
            .expect("baseline panel contains F1");
        assert_eq!(
            f1.evidence_path,
            "evidence/pipeline-maturity-glue/ip-004-pr-review/pr42/F1_linus.json"
        );
    }

    #[test]
    fn reviewer_id_format_is_tool_dash_facet_dash_change_id() {
        let dispatch = FacetDispatch::new("claude-critic", FacetId::M1ChallengeAssumption, "pr7");
        assert_eq!(
            dispatch.reviewer_id,
            "claude-critic-M1_challenge_assumption-pr7"
        );
    }
}
