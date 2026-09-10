use crate::identity::{LanguagePair, RuleId, UnitId};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PlanStep {
    pub unit: UnitId, // data_class: INTERNAL_ONLY
    pub rule: RuleId, // data_class: INTERNAL_ONLY
}

/// The deterministic, ordered transform to execute. Data only: holding it does not run it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformPlan {
    pub pair: LanguagePair,   // data_class: INTERNAL_ONLY
    pub steps: Vec<PlanStep>, // data_class: INTERNAL_ONLY
}
