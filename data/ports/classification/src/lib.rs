//! Agreed cross-owner surface for data-classification values.
//!
//! This compatibility port gives other capabilities a provider-owned import
//! path without forking Data's established value types.

#![forbid(unsafe_code)]

pub use data_boundary_kernel::{
    Classified, DataClass, DataClassification, NonPrivacyDataClass, OperationalDataClass,
    PRIVACY_PROGRAM_DATA_CLASS_LABELS, PrivacyDataClass, SubjectDataMarker,
    data_classes_from_privacy_data_classes, most_restrictive_privacy_data_class,
    parse_data_class_label, parse_data_class_pascal_label, parse_operational_data_class_label,
    parse_subject_data_marker_label, privacy_data_classes_from,
};
