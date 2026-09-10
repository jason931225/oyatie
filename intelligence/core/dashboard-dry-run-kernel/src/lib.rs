//! Dry-run (what-if) kernel: an input describes a counterfactual world and
//! the output projects the decision it would produce.
//!
//! Nothing here mutates external state, and [`DryRunOnly`] takes `&self`, so
//! no implementation can acquire the means to.

use intelligence_account_domain::{AccountId, ProviderFamily, RouteExplanation};
use intelligence_dashboard_kernel::RoutingView;

/// Marker trait for dry-run analyses.
pub trait DryRunOnly {
    type Outcome;
    fn evaluate(&self) -> Self::Outcome;
}

/// What the route would be, had this provider and account been chosen. The
/// live route store is not consulted or written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfRouteChange {
    pub current: RouteExplanation,         // data_class: INTERNAL_ONLY
    pub proposed_provider: ProviderFamily, // data_class: INTERNAL_ONLY
    pub proposed_account_id: AccountId,    // data_class: INTERNAL_ONLY
    pub proposed_model: String,            // data_class: INTERNAL_ONLY
    pub rationale: String,                 // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfRouteChangeOutcome {
    pub baseline: RoutingView,     // data_class: INTERNAL_ONLY
    pub hypothetical: RoutingView, // data_class: INTERNAL_ONLY
    pub differs: bool,             // data_class: INTERNAL_ONLY
}

impl DryRunOnly for WhatIfRouteChange {
    type Outcome = WhatIfRouteChangeOutcome;

    fn evaluate(&self) -> Self::Outcome {
        let baseline = RoutingView::new(&self.current);
        let hypothetical_explanation = RouteExplanation {
            chosen_provider: self.proposed_provider,
            chosen_account_id: self.proposed_account_id.clone(),
            chosen_model: self.proposed_model.clone(),
            reason: self.rationale.clone(),
        };
        let hypothetical = RoutingView::new(&hypothetical_explanation);
        let differs = baseline != hypothetical;
        WhatIfRouteChangeOutcome {
            baseline,
            hypothetical,
            differs,
        }
    }
}

/// Whether `projected_spend_micros` would fit under `proposed_limit_micros`
/// as well as under `current_limit_micros`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfBudgetChange {
    pub account_id: AccountId,       // data_class: INTERNAL_ONLY
    pub current_limit_micros: u64,   // data_class: INTERNAL_ONLY
    pub proposed_limit_micros: u64,  // data_class: INTERNAL_ONLY
    pub projected_spend_micros: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfBudgetChangeOutcome {
    pub account_id: AccountId,          // data_class: INTERNAL_ONLY
    pub current_limit_micros: u64,      // data_class: INTERNAL_ONLY
    pub proposed_limit_micros: u64,     // data_class: INTERNAL_ONLY
    pub projected_spend_micros: u64,    // data_class: INTERNAL_ONLY
    pub fits_under_current: bool,       // data_class: INTERNAL_ONLY
    pub fits_under_proposed: bool,      // data_class: INTERNAL_ONLY
    pub headroom_micros_proposed: i128, // data_class: INTERNAL_ONLY
}

impl DryRunOnly for WhatIfBudgetChange {
    type Outcome = WhatIfBudgetChangeOutcome;

    fn evaluate(&self) -> Self::Outcome {
        let fits_under_current = self.projected_spend_micros <= self.current_limit_micros;
        let fits_under_proposed = self.projected_spend_micros <= self.proposed_limit_micros;
        let headroom = self.proposed_limit_micros as i128 - self.projected_spend_micros as i128;
        WhatIfBudgetChangeOutcome {
            account_id: self.account_id.clone(),
            current_limit_micros: self.current_limit_micros,
            proposed_limit_micros: self.proposed_limit_micros,
            projected_spend_micros: self.projected_spend_micros,
            fits_under_current,
            fits_under_proposed,
            headroom_micros_proposed: headroom,
        }
    }
}

/// Whether swapping the policy would change the routing decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfPolicyChange {
    pub current_policy: String,               // data_class: INTERNAL_ONLY
    pub proposed_policy: String,              // data_class: INTERNAL_ONLY
    pub current_route: RouteExplanation,      // data_class: INTERNAL_ONLY
    pub hypothetical_route: RouteExplanation, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhatIfPolicyChangeOutcome {
    pub current_policy: String,     // data_class: INTERNAL_ONLY
    pub proposed_policy: String,    // data_class: INTERNAL_ONLY
    pub baseline: RoutingView,      // data_class: INTERNAL_ONLY
    pub hypothetical: RoutingView,  // data_class: INTERNAL_ONLY
    pub policy_changes_route: bool, // data_class: INTERNAL_ONLY
}

impl DryRunOnly for WhatIfPolicyChange {
    type Outcome = WhatIfPolicyChangeOutcome;

    fn evaluate(&self) -> Self::Outcome {
        let baseline = RoutingView::new(&self.current_route);
        let hypothetical = RoutingView::new(&self.hypothetical_route);
        let policy_changes_route = baseline != hypothetical;
        WhatIfPolicyChangeOutcome {
            current_policy: self.current_policy.clone(),
            proposed_policy: self.proposed_policy.clone(),
            baseline,
            hypothetical,
            policy_changes_route,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intelligence_account_domain::{AccountId, ProviderFamily, RouteExplanation};

    fn aid(s: &str) -> AccountId {
        AccountId(s.to_owned())
    }

    fn baseline_route() -> RouteExplanation {
        RouteExplanation {
            chosen_provider: ProviderFamily::Claude,
            chosen_account_id: aid("a1"),
            chosen_model: "claude-sonnet-4-6".to_owned(),
            reason: "default".to_owned(),
        }
    }

    #[test]
    fn what_if_route_change_detects_diff() {
        let wir = WhatIfRouteChange {
            current: baseline_route(),
            proposed_provider: ProviderFamily::OpenAiOrCodex,
            proposed_account_id: aid("a2"),
            proposed_model: "gpt-5".to_owned(),
            rationale: "lower cost".to_owned(),
        };
        let out = wir.evaluate();
        assert!(out.differs);
        assert_eq!(
            out.hypothetical.chosen_provider(),
            ProviderFamily::OpenAiOrCodex
        );
        assert_eq!(out.baseline.chosen_provider(), ProviderFamily::Claude);
    }

    #[test]
    fn what_if_route_change_identity_no_diff() {
        let cur = baseline_route();
        let wir = WhatIfRouteChange {
            current: cur.clone(),
            proposed_provider: cur.chosen_provider,
            proposed_account_id: cur.chosen_account_id.clone(),
            proposed_model: cur.chosen_model.clone(),
            rationale: cur.reason.clone(),
        };
        let out = wir.evaluate();
        assert!(!out.differs);
    }

    #[test]
    fn what_if_budget_change_fits_under_proposed_but_not_current() {
        let wb = WhatIfBudgetChange {
            account_id: aid("a1"),
            current_limit_micros: 100,
            proposed_limit_micros: 1000,
            projected_spend_micros: 500,
        };
        let out = wb.evaluate();
        assert!(!out.fits_under_current);
        assert!(out.fits_under_proposed);
        assert_eq!(out.headroom_micros_proposed, 500);
    }

    #[test]
    fn what_if_budget_change_overruns_both() {
        let wb = WhatIfBudgetChange {
            account_id: aid("a1"),
            current_limit_micros: 100,
            proposed_limit_micros: 200,
            projected_spend_micros: 500,
        };
        let out = wb.evaluate();
        assert!(!out.fits_under_current);
        assert!(!out.fits_under_proposed);
        assert_eq!(out.headroom_micros_proposed, -300);
    }

    #[test]
    fn what_if_budget_change_fits_both() {
        let wb = WhatIfBudgetChange {
            account_id: aid("a1"),
            current_limit_micros: 1000,
            proposed_limit_micros: 2000,
            projected_spend_micros: 500,
        };
        let out = wb.evaluate();
        assert!(out.fits_under_current);
        assert!(out.fits_under_proposed);
        assert_eq!(out.headroom_micros_proposed, 1500);
    }

    #[test]
    fn what_if_policy_change_flips_route() {
        let cur = baseline_route();
        let alt = RouteExplanation {
            chosen_provider: ProviderFamily::Gemini,
            chosen_account_id: aid("a9"),
            chosen_model: "gemini-2.5-pro".to_owned(),
            reason: "policy-fallback".to_owned(),
        };
        let p = WhatIfPolicyChange {
            current_policy: "prefer-claude".to_owned(),
            proposed_policy: "prefer-gemini".to_owned(),
            current_route: cur,
            hypothetical_route: alt,
        };
        let out = p.evaluate();
        assert!(out.policy_changes_route);
    }

    #[test]
    fn what_if_policy_change_no_flip() {
        let cur = baseline_route();
        let p = WhatIfPolicyChange {
            current_policy: "p".to_owned(),
            proposed_policy: "q".to_owned(),
            current_route: cur.clone(),
            hypothetical_route: cur,
        };
        let out = p.evaluate();
        assert!(!out.policy_changes_route);
    }

    #[test]
    fn negative_dry_run_evaluator_is_immutable() {
        let wb = WhatIfBudgetChange {
            account_id: aid("a1"),
            current_limit_micros: 1,
            proposed_limit_micros: 2,
            projected_spend_micros: 0,
        };
        let a = wb.evaluate();
        let b = wb.evaluate();
        assert_eq!(a, b);

        let wir = WhatIfRouteChange {
            current: baseline_route(),
            proposed_provider: ProviderFamily::Aws,
            proposed_account_id: aid("a-aws"),
            proposed_model: "n/a".to_owned(),
            rationale: "test".to_owned(),
        };
        let a = wir.evaluate();
        let b = wir.evaluate();
        assert_eq!(a, b);
    }
}
