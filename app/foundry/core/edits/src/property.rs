//! The wire property: name, tier tag, data-class tag, and typed value.

use crate::value::WireValue;

/// Why a wire property was refused at construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WirePropertyError {
    EmptyPropertyName,
    NotTrimmedPropertyName,
}

/// The property tiers, u8-tagged for the wire. The numbering is byte-law
/// from birth; `edit_vocabulary.rs` freezes it.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WireTier {
    Scalar,
    Vector,
    Timeseries,
    Geo,
    Ciphertext,
    Struct,
}

impl WireTier {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Scalar => 0,
            Self::Vector => 1,
            Self::Timeseries => 2,
            Self::Geo => 3,
            Self::Ciphertext => 4,
            Self::Struct => 5,
        }
    }

    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Scalar),
            1 => Some(Self::Vector),
            2 => Some(Self::Timeseries),
            3 => Some(Self::Geo),
            4 => Some(Self::Ciphertext),
            5 => Some(Self::Struct),
            _ => None,
        }
    }
}

/// The privacy-program data classes, u8-tagged for the wire. The
/// platform's own label mapping is many-to-one, so the boundary
/// conversion owns picking the canonical in-memory class per label; this
/// numbering must never assume bijectivity with any in-memory enum.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WireDataClass {
    Public,
    InternalOnly,
    PiiIdentifying,
    PiiQuasiIdentifier,
    Phi,
    Pci,
    Financial,
    FinancialRegulatedCredit,
    BehavioralTenantProduct,
    BehavioralAds,
    DeclaredPreference,
    SearchQuery,
    SensitivePipaArt23,
}

impl WireDataClass {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Public => 0,
            Self::InternalOnly => 1,
            Self::PiiIdentifying => 2,
            Self::PiiQuasiIdentifier => 3,
            Self::Phi => 4,
            Self::Pci => 5,
            Self::Financial => 6,
            Self::FinancialRegulatedCredit => 7,
            Self::BehavioralTenantProduct => 8,
            Self::BehavioralAds => 9,
            Self::DeclaredPreference => 10,
            Self::SearchQuery => 11,
            Self::SensitivePipaArt23 => 12,
        }
    }

    pub const fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Public),
            1 => Some(Self::InternalOnly),
            2 => Some(Self::PiiIdentifying),
            3 => Some(Self::PiiQuasiIdentifier),
            4 => Some(Self::Phi),
            5 => Some(Self::Pci),
            6 => Some(Self::Financial),
            7 => Some(Self::FinancialRegulatedCredit),
            8 => Some(Self::BehavioralTenantProduct),
            9 => Some(Self::BehavioralAds),
            10 => Some(Self::DeclaredPreference),
            11 => Some(Self::SearchQuery),
            12 => Some(Self::SensitivePipaArt23),
            _ => None,
        }
    }

    /// The platform's stable vocabulary for the class.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::InternalOnly => "INTERNAL_ONLY",
            Self::PiiIdentifying => "PII_IDENTIFYING",
            Self::PiiQuasiIdentifier => "PII_QUASI_IDENTIFIER",
            Self::Phi => "PHI",
            Self::Pci => "PCI",
            Self::Financial => "FINANCIAL",
            Self::FinancialRegulatedCredit => "FINANCIAL_REGULATED_CREDIT",
            Self::BehavioralTenantProduct => "BEHAVIORAL_TENANT_PRODUCT",
            Self::BehavioralAds => "BEHAVIORAL_ADS",
            Self::DeclaredPreference => "DECLARED_PREFERENCE",
            Self::SearchQuery => "SEARCH_QUERY",
            Self::SensitivePipaArt23 => "SENSITIVE_PIPA_ART23",
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireProperty {
    pub name: String,              // data_class: INTERNAL_ONLY
    pub tier: WireTier,            // data_class: INTERNAL_ONLY
    pub data_class: WireDataClass, // data_class: INTERNAL_ONLY
    pub value: WireValue,          // data_class: PII_IDENTIFYING
}

impl WireProperty {
    pub fn new(
        name: impl Into<String>,
        tier: WireTier,
        data_class: WireDataClass,
        value: WireValue,
    ) -> Result<Self, WirePropertyError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(WirePropertyError::EmptyPropertyName);
        }
        if name.trim() != name {
            return Err(WirePropertyError::NotTrimmedPropertyName);
        }
        Ok(Self {
            name,
            tier,
            data_class,
            value,
        })
    }
}
