use std::collections::BTreeMap;
use std::fmt;

use port_engine_api::{Digest, PortError, RECEIPT_AXES, Receipt, ReceiptAxis, RegionId, RulePack};
use port_engine_hash::digest_str;
use port_engine_kernel::{Delta, Verdict, Verification, verify};
use port_engine_rulepack::{LoadedRulePack, RulepackError};
use port_engine_rust_ir::{EmptyRenderer, RustFn, RustIr, RustItem, RustRenderer, Visibility};
use port_engine_snapshot::AdmitError;

use crate::driver;

/// The closed scenario vocabulary, named once so a scenario cannot be built under a name
/// [`scenario_matches`] does not answer for.
const UNCHANGED: &str = "unchanged";
const EXPLAINED: &str = "explained";
const UNEXPLAINED: &str = "unexplained";
const INCOMPLETE: &str = "incomplete";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioResult {
    pub name: &'static str,
    pub verification: Verification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SixAxisReport {
    /// Fleet pin identity used on every complete receipt.
    pub pin: String,
    pub scenarios: Vec<ScenarioResult>,
}

impl SixAxisReport {
    #[must_use]
    pub fn all_expected(&self) -> bool {
        self.scenarios.iter().all(scenario_matches)
    }
}

fn scenario_matches(s: &ScenarioResult) -> bool {
    match s.name {
        UNCHANGED => {
            s.verification.verdict == Verdict::Green
                && matches!(s.verification.delta, Delta::Unchanged)
        }
        EXPLAINED => {
            s.verification.verdict == Verdict::Green
                && matches!(
                    &s.verification.delta,
                    Delta::Explained { axes, .. }
                        if axes.contains(&ReceiptAxis::Formatter)
                            && axes.contains(&ReceiptAxis::Snapshot)
                )
        }
        UNEXPLAINED => {
            s.verification.verdict == Verdict::Red
                && matches!(s.verification.delta, Delta::Unexplained { .. })
        }
        INCOMPLETE => {
            s.verification.verdict == Verdict::Red
                && matches!(s.verification.delta, Delta::IncompleteReceipt { .. })
        }
        _ => false,
    }
}

/// Build a synthetic six-axis receipt for the local verification scenarios.
fn scenario_receipt(
    pin: &str,
    snapshot_label: &str,
    formatter: &str,
    engine: &Digest,
    rulepack: &Digest,
    toolchain: &Digest,
) -> Receipt {
    Receipt {
        pin: pin.to_owned(),
        snapshot_digest: digest_str(snapshot_label),
        engine_digest: engine.clone(),
        rulepack_digest: rulepack.clone(),
        toolchain_digest: toolchain.clone(),
        formatter_digest: digest_str(formatter),
    }
}

/// How many axes the closed set holds. `port-engine-api` asserts the six at compile time.
#[must_use]
pub fn receipt_axis_count() -> usize {
    RECEIPT_AXES.len()
}

/// # Errors
/// [`PortError`] from the empty renderer / kernel emit seam.
pub fn emit_empty_tree(formatter: &str) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
    let ir = RustIr::new(&["stub"]);
    let renderer = EmptyRenderer::new(formatter);
    port_engine_kernel::emit(&renderer, &ir)
}

/// Emit region bytes through the typed IR and the real formatter.
///
/// Takes an ITEM rather than a source string. The scenario this drives compares two emits whose
/// only difference is the formatter axis, so what it needs is the same tree rendered twice — and a
/// string argument would have made "the same tree" a claim about two parses rather than a fact.
///
/// # Errors
/// [`PortError`] from item assembly or [`RustRenderer::render_rust_ir`].
pub fn emit_typed_tree(item: RustItem) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
    let mut ir = RustIr::new(&["stub"]);
    ir.set_items("stub", vec![item])?;
    RustRenderer::new().render_rust_ir(&ir)
}

#[must_use]
pub fn stub_item() -> RustItem {
    RustItem::Function(RustFn {
        docs: Vec::new(),
        vis: Visibility::Public,
        name: "stub".into(),
        receiver: None,
        params: Vec::new(),
        ret: None,
        body: Some(Vec::new()),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum E2eError {
    Rulepack(RulepackError),
    Admit(AdmitError),
    Port(PortError),
    Unexpected { name: &'static str, actual: String },
}

impl fmt::Display for E2eError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rulepack(err) => write!(f, "receipt e2e rulepack load failed: {err}"),
            Self::Admit(err) => write!(f, "receipt e2e snapshot admit failed: {err}"),
            Self::Port(err) => write!(f, "receipt e2e emit failed: {err}"),
            Self::Unexpected { name, actual } => {
                write!(f, "receipt e2e scenario `{name}` unexpected: {actual}")
            }
        }
    }
}

impl std::error::Error for E2eError {}

/// # Errors
/// [`E2eError`] on rulepack/admit/emit failure or unexpected verdict.
pub fn run_six_axis_e2e() -> Result<SixAxisReport, E2eError> {
    if receipt_axis_count() != 6 {
        return Err(E2eError::Unexpected {
            name: "axis-count",
            actual: format!("expected 6 axes, got {}", receipt_axis_count()),
        });
    }

    let admitted = driver::smoke_admit_snapshot().map_err(E2eError::Admit)?;
    let pin = admitted.pin().to_owned();
    let pack = LoadedRulePack::load_embedded().map_err(E2eError::Rulepack)?;
    let engine = driver::smoke_engine_digest();
    let toolchain = driver::smoke_toolchain_digest();
    let rulepack = pack.digest();

    let empty_a = emit_empty_tree("fmt-empty-a").map_err(E2eError::Port)?;
    let empty_b = emit_empty_tree("fmt-empty-a").map_err(E2eError::Port)?;
    let rendered = emit_typed_tree(stub_item()).map_err(E2eError::Port)?;

    let receipt_empty = scenario_receipt(
        &pin,
        "snapshot-empty-stub-v0",
        "fmt-empty-a",
        &engine,
        &rulepack,
        &toolchain,
    );
    let receipt_syn = Receipt {
        pin: pin.clone(),
        snapshot_digest: admitted.artifact_digest().clone(),
        engine_digest: engine.clone(),
        rulepack_digest: rulepack.clone(),
        toolchain_digest: toolchain.clone(),
        formatter_digest: digest_str("fmt-syn-b"),
    };
    let receipt_incomplete = Receipt {
        pin: String::new(),
        snapshot_digest: admitted.artifact_digest().clone(),
        engine_digest: engine.clone(),
        rulepack_digest: rulepack.clone(),
        toolchain_digest: toolchain.clone(),
        formatter_digest: digest_str("fmt-syn-b"),
    };

    let scenarios = vec![
        ScenarioResult {
            name: UNCHANGED,
            verification: verify(&receipt_empty, &empty_a, &receipt_empty, &empty_b),
        },
        ScenarioResult {
            name: EXPLAINED,
            verification: verify(&receipt_empty, &empty_a, &receipt_syn, &rendered),
        },
        ScenarioResult {
            name: UNEXPLAINED,
            verification: verify(&receipt_empty, &empty_a, &receipt_empty, &rendered),
        },
        ScenarioResult {
            name: INCOMPLETE,
            verification: verify(&receipt_empty, &empty_a, &receipt_incomplete, &rendered),
        },
    ];

    let report = SixAxisReport { pin, scenarios };
    for s in &report.scenarios {
        if !scenario_matches(s) {
            return Err(E2eError::Unexpected {
                name: s.name,
                actual: format!("{:?}", s.verification),
            });
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_axis_e2e_matches_expected_verdicts() {
        let report = run_six_axis_e2e().expect("the receipt e2e must hold");
        assert!(report.all_expected());
        assert_eq!(report.scenarios.len(), 4);
        assert!(!report.pin.is_empty());
        assert_eq!(receipt_axis_count(), 6);
    }

    #[test]
    fn scenario_receipt_has_no_incomplete_axes() {
        let engine = digest_str("engine");
        let rulepack = digest_str("rulepack");
        let toolchain = digest_str("toolchain");
        let r = scenario_receipt("pin", "snap", "fmt", &engine, &rulepack, &toolchain);
        assert!(r.incomplete_axes().is_empty());
        assert!(r.engine_digest.0.starts_with("sha256:"));
        assert!(r.rulepack_digest.0.starts_with("sha256:"));
        assert!(r.toolchain_digest.0.starts_with("sha256:"));
    }

    /// Every scenario the harness builds is one [`scenario_matches`] answers for.
    ///
    /// The `_ => false` arm turns an unanswered name into a silent failure rather than a loud one,
    /// so the named constants are the only names allowed through.
    #[test]
    fn every_scenario_name_is_answered_for() {
        let report = run_six_axis_e2e().expect("the receipt e2e must hold");
        let names: Vec<&str> = report.scenarios.iter().map(|s| s.name).collect();
        assert_eq!(names, vec![UNCHANGED, EXPLAINED, UNEXPLAINED, INCOMPLETE]);
    }
}
