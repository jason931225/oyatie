#![forbid(unsafe_code)]
// A bounded linear walk over fixed-length slices; the lint is a hang detector, not a cost bound.
#![allow(long_running_const_eval)]

use std::collections::{BTreeMap, BTreeSet};

use port_engine_api::{
    PlanStep, PortError, Receipt, ReceiptAxis, RegionId, Renderer, RuleId, RulePack, SourceModel,
    TargetIr, TransformPlan, UnitId,
};

/// This crate's own source, for the engine-identity axis assembled by the facade.
pub const CRATE_SOURCES: &[(&str, &str)] = &[("lib.rs", include_str!("lib.rs"))];

const KERNEL_SOURCE: &str = include_str!("lib.rs");

const SEAM_TEST_SOURCE: &str = include_str!("../tests/seams.rs");

/// Corpus vocabulary the neutral engine may never contain. A canary set, not a decision
/// procedure; bytes, not text, so a needle is not itself part of the haystack.
pub const FORBIDDEN_CORPUS_TOKENS: [&[u8]; 5] = [
    &[107, 117, 98, 101],
    &[107, 56, 115],
    &[97, 112, 105, 109, 97, 99, 104, 105, 110, 101, 114, 121],
    &[101, 116, 99, 100],
    &[116, 97, 108, 111, 115],
];

/// Keywords whose grammar productions could place kernel code in a file this scan never reads.
/// Matched on IDENTIFIER boundaries: the productions accept any whitespace after the
/// keyword, so a needle carrying a fixed space is a hole.
pub const UNSCANNED_CODE_KEYWORDS: [&[u8]; 2] =
    [&[109, 111, 100], &[105, 110, 99, 108, 117, 100, 101]];

const fn lowercase_ascii(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + 32
    } else {
        byte
    }
}

/// Case-insensitive substring search usable in a `const` context. `token` MUST be lowercase.
#[must_use]
pub const fn contains_token(haystack: &[u8], token: &[u8]) -> bool {
    if token.is_empty() || token.len() > haystack.len() {
        return false;
    }
    let last_start = haystack.len() - token.len();
    let mut start = 0;
    while start <= last_start {
        let mut offset = 0;
        while offset < token.len() && lowercase_ascii(haystack[start + offset]) == token[offset] {
            offset += 1;
        }
        if offset == token.len() {
            return true;
        }
        start += 1;
    }
    false
}

const fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Boundary-anchored [`contains_token`], so the rule agrees with the grammar, not one spelling.
#[must_use]
pub const fn contains_word(haystack: &[u8], word: &[u8]) -> bool {
    if word.is_empty() || word.len() > haystack.len() {
        return false;
    }
    let last_start = haystack.len() - word.len();
    let mut start = 0;
    while start <= last_start {
        let mut offset = 0;
        while offset < word.len() && lowercase_ascii(haystack[start + offset]) == word[offset] {
            offset += 1;
        }
        let end = start + word.len();
        if offset == word.len()
            && (start == 0 || !is_identifier_byte(haystack[start - 1]))
            && (end == haystack.len() || !is_identifier_byte(haystack[end]))
        {
            return true;
        }
        start += 1;
    }
    false
}

const _: () = {
    let source = KERNEL_SOURCE.as_bytes();
    let seam_test = SEAM_TEST_SOURCE.as_bytes();
    let mut i = 0;
    while i < FORBIDDEN_CORPUS_TOKENS.len() {
        assert!(
            !contains_token(source, FORBIDDEN_CORPUS_TOKENS[i]),
            "the neutral kernel carries a corpus token (ADR-0637 D1): corpus vocabulary and \
             corpus-specific behaviour belong in corpus policy, never in the engine"
        );
        assert!(
            !contains_token(seam_test, FORBIDDEN_CORPUS_TOKENS[i]),
            "the seam test carries a corpus token (ADR-0637 D1): a corpus-specific fixture is a \
             corpus-specific branch with extra steps, and belongs in corpus policy"
        );
        i += 1;
    }
    let mut j = 0;
    while j < UNSCANNED_CODE_KEYWORDS.len() {
        assert!(
            !contains_word(source, UNSCANNED_CODE_KEYWORDS[j]),
            "kernel code would live in a file this neutrality scan cannot read: keep the kernel \
             one file, or derive the scanned set from the build rule's srcs before adding another"
        );
        j += 1;
    }
};

/// The classification of an emitted-byte change between two receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Delta {
    Unchanged,
    Explained {
        /// The regions whose bytes changed.
        regions: BTreeSet<RegionId>, // data_class: INTERNAL_ONLY
        /// The axes that moved.
        axes: BTreeSet<ReceiptAxis>, // data_class: INTERNAL_ONLY
    },
    /// Regions changed while every receipt axis held: RED, and a defect never repaired by
    /// editing the generated output.
    Unexplained {
        /// The regions whose bytes changed with no axis to account for them.
        regions: BTreeSet<RegionId>, // data_class: INTERNAL_ONLY
    },
    /// Regions changed and a receipt is unusable: an empty axis is absence of information.
    IncompleteReceipt {
        /// The regions whose bytes changed while the evidence was unusable.
        regions: BTreeSet<RegionId>, // data_class: INTERNAL_ONLY
    },
}

/// The verdict of a determinism check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict {
    Green,
    Red,
}

/// The outcome of [`verify`]: a verdict and the delta that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verification {
    /// Green or red.
    pub verdict: Verdict, // data_class: INTERNAL_ONLY
    /// Why.
    pub delta: Delta, // data_class: INTERNAL_ONLY
}

/// Build the deterministic [`TransformPlan`] for `model` under `pack`.
///
/// Pack order is ENFORCED: rules for a unit must be a strictly increasing subsequence.
pub fn plan(model: &dyn SourceModel, pack: &dyn RulePack) -> Result<TransformPlan, PortError> {
    let pair = pack.pair();
    if pair.source != model.language() {
        return Err(PortError::LanguageMismatch {
            expected: pair.source.clone(),
            actual: model.language().to_owned(),
        });
    }

    // Position, not just membership: a repeated declaration is refused where it is still visible.
    let mut declared: BTreeMap<RuleId, usize> = BTreeMap::new();
    for (position, rule) in pack.rules().into_iter().enumerate() {
        if declared.insert(rule.clone(), position).is_some() {
            return Err(PortError::DuplicateRule { rule });
        }
    }
    let mut seen: BTreeSet<UnitId> = BTreeSet::new();
    let mut steps: Vec<PlanStep> = Vec::new();

    for unit in model.units() {
        if !seen.insert(unit.clone()) {
            return Err(PortError::DuplicateUnit { unit });
        }
        let mut previous: Option<usize> = None;
        for rule in pack.rules_for(&unit) {
            let Some(&position) = declared.get(&rule) else {
                return Err(PortError::UndeclaredRule { unit, rule });
            };
            if previous.is_some_and(|last| position <= last) {
                return Err(PortError::RuleOrderViolation { unit, rule });
            }
            previous = Some(position);
            steps.push(PlanStep {
                unit: unit.clone(),
                rule,
            });
        }
    }

    Ok(TransformPlan {
        pair: pair.clone(),
        steps,
    })
}

/// Render `ir` with `renderer`, proving the emitted region set is exactly the declared one.
///
/// Repeated region identity is refused BEFORE the set collapses it and loses the duplicate.
pub fn emit(
    renderer: &dyn Renderer,
    ir: &dyn TargetIr,
) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError> {
    if renderer.target_language() != ir.target_language() {
        return Err(PortError::LanguageMismatch {
            expected: ir.target_language().to_owned(),
            actual: renderer.target_language().to_owned(),
        });
    }

    let mut declared: BTreeSet<RegionId> = BTreeSet::new();
    for region in ir.regions() {
        if !declared.insert(region.clone()) {
            return Err(PortError::DuplicateRegion { region });
        }
    }
    let rendered = renderer.render(ir)?;
    let emitted: BTreeSet<RegionId> = rendered.keys().cloned().collect();

    let missing: BTreeSet<RegionId> = declared.difference(&emitted).cloned().collect();
    let unexpected: BTreeSet<RegionId> = emitted.difference(&declared).cloned().collect();
    if !missing.is_empty() || !unexpected.is_empty() {
        return Err(PortError::RegionSetMismatch {
            missing,
            unexpected,
        });
    }

    Ok(rendered)
}

/// Classify an emitted-byte change against the two receipts that bracket it (ADR-0637 D2).
///
/// The changed set is DERIVED from the emitted bytes, never supplied; an empty axis buys
/// no explanation — see [`Delta::IncompleteReceipt`].
#[must_use]
pub fn verify(
    previous: &Receipt,
    previous_output: &BTreeMap<RegionId, Vec<u8>>,
    current: &Receipt,
    current_output: &BTreeMap<RegionId, Vec<u8>>,
) -> Verification {
    let mut changed_regions: BTreeSet<RegionId> = BTreeSet::new();
    for (region, bytes) in previous_output {
        if current_output.get(region) != Some(bytes) {
            changed_regions.insert(region.clone());
        }
    }
    for region in current_output.keys() {
        if !previous_output.contains_key(region) {
            changed_regions.insert(region.clone());
        }
    }

    if changed_regions.is_empty() {
        return Verification {
            verdict: Verdict::Green,
            delta: Delta::Unchanged,
        };
    }

    // After the unchanged return: an incomplete receipt must not redden an identical tree.
    if !previous.incomplete_axes().is_empty() || !current.incomplete_axes().is_empty() {
        return Verification {
            verdict: Verdict::Red,
            delta: Delta::IncompleteReceipt {
                regions: changed_regions,
            },
        };
    }

    let axes = previous.differing_axes(current);
    if axes.is_empty() {
        return Verification {
            verdict: Verdict::Red,
            delta: Delta::Unexplained {
                regions: changed_regions,
            },
        };
    }

    Verification {
        verdict: Verdict::Green,
        delta: Delta::Explained {
            regions: changed_regions,
            axes,
        },
    }
}
