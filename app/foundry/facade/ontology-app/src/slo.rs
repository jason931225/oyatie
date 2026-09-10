//! Service level objectives as typed IR, and the pure renderer that turns
//! them into the checked-in OpenSLO payloads.
//!
//! The payloads under `app/foundry/observability/slos/` are OUTPUTS. They are
//! byte-compared against this renderer by the suite, so a hand-edited payload
//! fails rather than drifting from the objective the code believes it serves
//! — an SLO that disagrees with its own definition is worse than no SLO,
//! because it reports confidently.
//!
//! Rendering is string assembly with no serialization dependency, following
//! the platform's own precedent for hand-rendered exposition formats.

use std::collections::BTreeSet;

pub struct SloSpec {
    pub name: &'static str,              // data_class: INTERNAL_ONLY
    pub display_name: &'static str,      // data_class: INTERNAL_ONLY
    pub sli_class: &'static str,         // data_class: INTERNAL_ONLY
    pub description: &'static str,       // data_class: INTERNAL_ONLY
    pub good_query: &'static str,        // data_class: INTERNAL_ONLY
    pub total_query: &'static str,       // data_class: INTERNAL_ONLY
    pub target: &'static str,            // data_class: INTERNAL_ONLY
    pub objective_display: &'static str, // data_class: INTERNAL_ONLY
    /// Whether the ratio's sources are cumulative counters. False for a
    /// gauge-backed objective: declaring a gauge cumulative tells a consumer
    /// to `rate()` something that does not monotonically increase.
    pub counter: bool, // data_class: INTERNAL_ONLY
}

impl SloSpec {
    pub fn referenced_metrics(&self) -> BTreeSet<String> {
        const PREFIX: &str = "foundry_";
        let mut found = BTreeSet::new();
        for query in [self.good_query, self.total_query] {
            let mut consumed = 0;
            while let Some(at) = query[consumed..].find(PREFIX) {
                let start = consumed + at;
                let end = start + metric_name_len(&query[start..]);
                if is_whole_metric_name(query, start, end) {
                    found.insert(query[start..end].to_owned());
                }
                consumed = end;
            }
        }
        found
    }
}

fn is_metric_name_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn metric_name_len(tail: &str) -> usize {
    tail.find(|character: char| !is_metric_name_char(character))
        .unwrap_or(tail.len())
}

fn is_whole_metric_name(query: &str, start: usize, end: usize) -> bool {
    let continues_a_token = query[..start]
        .chars()
        .next_back()
        .is_some_and(|character| is_metric_name_char(character) || character == ':');
    let prefixes_a_recording_rule = query[end..].starts_with(':');
    !continues_a_token && !prefixes_a_recording_rule
}

const SERVICE: &str = "foundry-ontology";

/// Exporting the predicate rather than joining two gauges in the query is
/// deliberate: the join needs `ignoring(__name__)` label matching that
/// nothing here can execute against, and it would let the objective drift
/// from the readiness probe that answers the same question.
pub static SLOS: &[SloSpec] = &[
    SloSpec {
        name: "ontology-projection-freshness",
        display_name: "foundry-ontology — projection caught up to its log",
        sli_class: "freshness",
        description: "A scrape is good when every tenant the process could read had \
                      consumed its whole log. Reads from a lagging projection are \
                      answers about the past. This is a SCRAPE-level boolean, not a \
                      per-tenant ratio: the signal is process-wide and unlabelled \
                      because this surface is unauthenticated and must not become a \
                      tenancy oracle. A tenant whose log head could not be READ \
                      scores zero — an unreadable store is not a fresh one. A tenant \
                      that was merely BUSY does not, because a lock held by a request \
                      in flight is a service being used, and an objective that reds \
                      because the service is being used teaches operators to ignore \
                      it; lag persists, so a tenant genuinely behind is seen on the \
                      scrapes that are not contended. A scrape that observed NO \
                      tenant scores zero rather than vacuously good. What this \
                      cannot see is a partial wedge: if some tenants stay contended \
                      forever while the rest are fresh, the claim narrows to the \
                      tenants it could read and stays true. Alert on sustained \
                      non-zero foundry_projection_contended for that; it is not a \
                      question a per-scrape boolean can answer. /readyz answers the \
                      stricter question and orders its refusals so a measured fault \
                      outranks a busy tenant. Poison is excluded: a poisoned entry advances the \
                      fold, so counting it would make the objective unrecoverable. \
                      During a total outage no series is scraped at all, so budget \
                      consumption is the evaluator's no-data policy, not this \
                      objective's claim.",
        good_query: "sum(foundry_projection_fresh)",
        total_query: "count(foundry_projection_fresh)",
        target: "0.999",
        objective_display: "99.9%",
        counter: false,
    },
    SloSpec {
        name: "ontology-submit-availability",
        display_name: "foundry-ontology — Action submission availability",
        sli_class: "availability",
        description: "A submission is good when the writer accepted it into the log. A \
                      policy refusal counts against this objective deliberately: from the \
                      caller's side an Action it was entitled to submit and could not is \
                      an outage, and hiding refusals here would make the number flatter \
                      than the service.",
        good_query: "sum(rate(foundry_action_submit_served_total[5m]))",
        total_query: "sum(rate(foundry_action_submit_served_total[5m])) + \
                      sum(rate(foundry_action_submit_refused_total[5m]))",
        target: "0.99",
        objective_display: "99% of submissions accepted over 30d",
        counter: true,
    },
    SloSpec {
        name: "ontology-read-availability",
        display_name: "foundry-ontology — read surface availability",
        sli_class: "availability",
        description: "A read is good when the projection answered it. Refusals — absent \
                      credential, policy denial, unusable revision pin, a log the process \
                      could not read — are counted, so a surface that refuses everything \
                      cannot report itself available. /statusz counts here too: it is an \
                      authorized read-plane surface, so an operator polling it moves this \
                      ratio, and the number reflects polling frequency alongside service \
                      health.",
        good_query: "sum(rate(foundry_read_served_total[5m]))",
        total_query: "sum(rate(foundry_read_served_total[5m])) + \
                      sum(rate(foundry_read_refused_total[5m]))",
        target: "0.995",
        objective_display: "99.5% of reads answered over 30d",
        counter: true,
    },
];

pub fn render_openslo(spec: &SloSpec) -> String {
    format!(
        "# Generated from `app/foundry/facade/ontology-app/src/slo.rs`. Do not hand-edit:\n\
         # the suite compares this file byte-for-byte against the renderer.\n\
         apiVersion: openslo/v1\n\
         kind: SLO\n\
         metadata:\n  \
           name: {name}\n  \
           displayName: \"{display}\"\n  \
           labels:\n    \
             microservice: {service}\n    \
             sli_class: {sli_class}\n    \
             plane: app\n    \
             owner_team: foundry\n\
         spec:\n  \
           service: {service}\n  \
           description: |\n{description}\n  \
           indicator:\n    \
             metadata:\n      \
               name: {name}-indicator\n    \
             spec:\n      \
               ratioMetric:\n        \
                 counter: {counter}\n        \
                 good:\n          \
                   metricSource:\n            \
                     type: Prometheus\n            \
                     spec:\n              \
                       query: |\n                {good}\n        \
                 total:\n          \
                   metricSource:\n            \
                     type: Prometheus\n            \
                     spec:\n              \
                       query: |\n                {total}\n  \
           objectives:\n    \
             - target: {target}\n      \
               displayName: \"{objective_display}\"\n  \
           timeWindow:\n    \
             - duration: 30d\n      \
               isRolling: true\n  \
           budgetingMethod: Occurrences\n",
        name = spec.name,
        display = spec.display_name,
        service = SERVICE,
        sli_class = spec.sli_class,
        description = wrap_block(spec.description),
        counter = spec.counter,
        good = spec.good_query,
        total = spec.total_query,
        target = spec.target,
        objective_display = spec.objective_display,
    )
}

const YAML_BLOCK_INDENT: &str = "    ";
const DESCRIPTION_WRAP_COLUMN: usize = 72;

fn wrap_block(text: &str) -> String {
    text.split_whitespace()
        .fold(Vec::<String>::new(), |mut lines, word| {
            match lines.last_mut() {
                Some(line) if line.len() + 1 + word.len() <= DESCRIPTION_WRAP_COLUMN => {
                    line.push(' ');
                    line.push_str(word);
                }
                _ => lines.push(format!("{YAML_BLOCK_INDENT}{word}")),
            }
            lines
        })
        .join("\n")
}
