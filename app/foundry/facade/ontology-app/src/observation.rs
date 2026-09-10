use crate::composition::AppState;

/// One pass over the served tenants: both totals and the number of tenants
/// that could not be read, taken TOGETHER.
///
/// Together is the whole point. An earlier revision computed each gauge in
/// its own pass, so a tenant unreadable during one pass and readable during
/// the next produced `lag 0` beside `unknown 0` — a pair that reads as
/// healthy and describes no state the process was ever in. Three passes also
/// meant three `MAX(ordinal)` queries per tenant per unauthenticated scrape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Observation {
    /// Tenants whose status was actually read on this pass. A total that
    /// nobody contributed to is not a small total; it is no measurement.
    pub observed: u64, // data_class: INTERNAL_ONLY
    pub lag: u64,        // data_class: INTERNAL_ONLY
    pub poisoned: u64,   // data_class: INTERNAL_ONLY
    pub contended: u64,  // data_class: INTERNAL_ONLY
    pub unreadable: u64, // data_class: INTERNAL_ONLY
}

impl Observation {
    /// Caught up AND completely observed — the readiness predicate itself,
    /// evaluated from the same KIND of observation the exposition renders:
    /// one pass per answer, so no single answer can contradict itself. The
    /// probe and a scrape are separate requests taking separate passes, and
    /// they may of course differ — that is time passing, not disagreement.
    ///
    /// A tenant nobody could read is not evidence of freshness, so it is not
    /// readiness either. Poison does NOT enter it: a poisoned entry advances
    /// the fold and touches nothing else, so counting it as un-ready would
    /// red the instrument exactly when the system is making progress.
    pub fn is_caught_up(&self) -> bool {
        self.lag == 0 && self.contended == 0 && self.unreadable == 0
    }

    /// Contention fails OPEN here where `is_caught_up` fails closed: a held
    /// lock is a request in flight, and lag persists, so a busy tenant is
    /// seen on the next uncontended pass.
    ///
    /// `observed > 0` stops that from going vacuous — a wedged store holds
    /// every mutex, and "every tenant we could read is caught up" is true of
    /// the empty set.
    pub fn is_fresh(&self) -> bool {
        self.observed > 0 && self.lag == 0 && self.unreadable == 0
    }
}

pub fn observe(state: &AppState) -> Observation {
    let mut seen = Observation {
        observed: 0,
        lag: 0,
        poisoned: 0,
        contended: 0,
        unreadable: 0,
    };
    for (tenant_id, tenant) in &state.tenants {
        let Ok(tenant) = tenant.try_lock() else {
            seen.contended += 1;
            continue;
        };
        match tenant.sync_status(tenant_id) {
            Ok(status) => {
                seen.observed += 1;
                seen.lag += status.lag;
                seen.poisoned += status.poisoned_count;
            }
            Err(_) => seen.unreadable += 1,
        }
    }
    seen
}
