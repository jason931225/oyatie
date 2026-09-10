//! Surveying source the engine has never seen.
//!
//! The embedded fixture corpus everything else here ports is the right thing to prove determinism
//! against and the wrong thing to measure MATURITY against, because a corpus written alongside the
//! engine only ever contains what the engine already handles. This takes a snapshot from a path
//! instead, and reports a ranked work list rather than a score.

use std::path::Path;

use port_engine_rulepack::LoadedRulePack;
use port_engine_snapshot::admit_reproducible_pair;
use port_engine_transform::{SurveyReport, survey};

use super::PipelineError;

/// Survey an extracted snapshot against the embedded rule pack.
///
/// # Errors
/// [`PipelineError`] when the snapshot cannot be read or admitted. A snapshot the engine cannot
/// TRANSLATE is not an error — that is the measurement.
pub fn survey_snapshot(path: &Path) -> Result<SurveyReport, PipelineError> {
    let bytes = std::fs::read(path).map_err(|err| {
        PipelineError::Emit(port_engine_api::PortError::Render {
            detail: format!("read snapshot {}: {err}", path.display()),
        })
    })?;
    // Admitted against ITSELF, the same way every embedded fixture is: this is a single artifact
    // from one extraction, so the reproducibility pair has one member and the digest check is over
    // the encoder rather than over two runs.
    let admitted = admit_reproducible_pair(&bytes, &bytes).map_err(PipelineError::Admit)?;
    let pack = LoadedRulePack::load_embedded_go_rust().map_err(PipelineError::Rulepack)?;
    Ok(survey(admitted.as_model(), &pack))
}

/// Render a survey as a report a person reads to decide what to build next.
#[must_use]
pub fn render_survey(report: &SurveyReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "declarations={} translated={} refused={} deferred={} uncaptured={} coverage={:.1}%\n",
        report.total(),
        report.translated.len(),
        report.refused.len(),
        report.deferred.len(),
        report.uncaptured.len(),
        report.coverage()
    ));
    if report.total() == report.translated.len() {
        return out;
    }
    out.push_str("\n# what is missing, ranked by how many declarations it blocks\n");
    for (reason, count) in report.ranked_reasons() {
        out.push_str(&format!("{count:>5}  {reason}\n"));
        // One example site per cause. A ranked count says how much a rule would unblock; the site
        // is what somebody opens to decide what the rule should SAY.
        if let Some(example) = report.example_of(&reason) {
            out.push_str(&format!("       e.g. {} {}\n", example.kind, example.name));
        }
    }
    out
}
