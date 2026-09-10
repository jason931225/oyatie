//! The typed value plane: [`PropertyValue`] and its scalar carriers.

use std::collections::BTreeMap;

/// Why a scalar carrier or a value-type declaration was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueTypeError {
    /// A double was NaN or ±infinity; only finite doubles are admitted.
    NonFiniteDouble,
    /// A calendar date's components name no real day.
    InvalidDate,
    /// A struct schema declared no fields.
    EmptyStructSchema,
    /// A struct field name was blank.
    BlankStructFieldName,
    /// Two struct fields share one name.
    DuplicateStructField {
        /// The duplicated field name.
        name: String,
    },
    /// A declaration nests deeper than the admitted maximum.
    DepthExceeded,
}

/// A finite IEEE-754 double stored as a monotone `u64` key: negatives map
/// through bitwise NOT and non-negatives set the sign bit, so the DERIVED
/// `Eq`/`Ord`/`Hash` agree with numeric order by construction. `NaN` and
/// ±infinity are rejected at construction; `-0.0` folds to `0.0`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FiniteDouble(u64);

impl FiniteDouble {
    pub fn new(value: f64) -> Result<Self, ValueTypeError> {
        if !value.is_finite() {
            return Err(ValueTypeError::NonFiniteDouble);
        }
        let folded = if value == 0.0 { 0.0 } else { value };
        let bits = folded.to_bits();
        let key = if folded.is_sign_negative() {
            !bits
        } else {
            bits ^ (1u64 << 63)
        };
        Ok(Self(key))
    }

    /// The finite `f64` this carrier holds.
    pub fn get(self) -> f64 {
        if self.0 & (1u64 << 63) != 0 {
            f64::from_bits(self.0 ^ (1u64 << 63))
        } else {
            f64::from_bits(!self.0)
        }
    }

    pub const fn sort_key(self) -> u64 {
        self.0
    }
}

/// A validated proleptic-Gregorian calendar date.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CalendarDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CalendarDate {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, ValueTypeError> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(ValueTypeError::InvalidDate);
        }
        Ok(Self { year, month, day })
    }

    pub const fn year(self) -> i32 {
        self.year
    }

    pub const fn month(self) -> u8 {
        self.month
    }

    pub const fn day(self) -> u8 {
        self.day
    }
}

const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        _ => 28,
    }
}

/// SQLite-affinity storage class of a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageClass {
    Integer,
    Real,
    Text,
    Bytes,
}

/// A typed ontology property value. Every variant derives `Eq`/`Ord`, which
/// `Classified<PropertyValue>` requires; doubles go through
/// [`FiniteDouble`], so the derives are honest everywhere.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PropertyValue {
    String(String),
    Integer(i64),
    Double(FiniteDouble),
    Boolean(bool),
    Date(CalendarDate),
    Timestamp { epoch_millis: i64 },
    Array(Vec<PropertyValue>),
    Struct(BTreeMap<String, PropertyValue>),
}

impl PropertyValue {
    /// The `&str` inside a `String` value, `None` for every other variant —
    /// the legacy bridge's read path.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Static label of the variant, for fail-closed diagnostics that must
    /// never carry classified values.
    pub const fn type_label(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Integer(_) => "integer",
            Self::Double(_) => "double",
            Self::Boolean(_) => "boolean",
            Self::Date(_) => "date",
            Self::Timestamp { .. } => "timestamp",
            Self::Array(_) => "array",
            Self::Struct(_) => "struct",
        }
    }

    /// The 1:1 SQLite-affinity class of this value.
    pub const fn storage_class(&self) -> StorageClass {
        match self {
            Self::Integer(_) | Self::Boolean(_) | Self::Date(_) | Self::Timestamp { .. } => {
                StorageClass::Integer
            }
            Self::Double(_) => StorageClass::Real,
            Self::String(_) => StorageClass::Text,
            Self::Array(_) | Self::Struct(_) => StorageClass::Bytes,
        }
    }
}
