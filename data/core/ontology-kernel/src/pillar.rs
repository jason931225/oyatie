#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! Ontology pillar: org/person isolation boundary.
//!
//! The Ontology substrate segments all typed objects into two mutually-exclusive
//! pillars — `org` and `person` — per Bominal-ADR-0132 org/person pillar
//! isolation.

/// The two pillars that partition every Ontology object.
///
/// Cedar policy enforces that org-admin principals cannot read `Person`-pillar
/// objects (Bominal-ADR-0132). The pillar is stored as the `wire_label` string
/// in Postgres and propagated in `ObjectMutated` Protobuf events.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum OntologyPillar {
    /// Organisation-scoped objects (companies, teams, workspaces).
    Org,
    /// Person-scoped objects (user profiles, employee records).
    Person,
}

impl OntologyPillar {
    /// The canonical wire label used in Postgres DDL, Protobuf, and Cedar.
    pub const fn wire_label(self) -> &'static str {
        match self {
            Self::Org => "org",
            Self::Person => "person",
        }
    }

    /// All pillar variants in declaration order.
    pub const fn all() -> [Self; 2] {
        [Self::Org, Self::Person]
    }

    /// Parse a wire label string, returning `None` for unknown labels.
    pub fn from_wire_label(label: &str) -> Option<Self> {
        match label.trim() {
            "org" => Some(Self::Org),
            "person" => Some(Self::Person),
            _ => None,
        }
    }
}

/// Error returned when an unknown pillar wire label is encountered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownPillarLabel(pub String);

impl std::fmt::Display for UnknownPillarLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown ontology pillar label {:?}; expected \"org\" or \"person\"",
            self.0
        )
    }
}

impl TryFrom<&str> for OntologyPillar {
    type Error = UnknownPillarLabel;

    fn try_from(label: &str) -> Result<Self, Self::Error> {
        Self::from_wire_label(label).ok_or_else(|| UnknownPillarLabel(label.to_string()))
    }
}

impl TryFrom<String> for OntologyPillar {
    type Error = UnknownPillarLabel;

    fn try_from(label: String) -> Result<Self, Self::Error> {
        Self::try_from(label.as_str())
    }
}

impl std::fmt::Display for OntologyPillar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.wire_label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_labels_are_canonical() {
        assert_eq!(OntologyPillar::Org.wire_label(), "org");
        assert_eq!(OntologyPillar::Person.wire_label(), "person");
    }

    #[test]
    fn all_returns_both_variants() {
        let all = OntologyPillar::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&OntologyPillar::Org));
        assert!(all.contains(&OntologyPillar::Person));
    }

    #[test]
    fn from_wire_label_round_trips() {
        for pillar in OntologyPillar::all() {
            assert_eq!(
                OntologyPillar::from_wire_label(pillar.wire_label()),
                Some(pillar)
            );
        }
    }

    #[test]
    fn from_wire_label_rejects_unknown() {
        assert_eq!(OntologyPillar::from_wire_label("unknown"), None);
        assert_eq!(OntologyPillar::from_wire_label(""), None);
        assert_eq!(OntologyPillar::from_wire_label("ORG"), None);
        assert_eq!(OntologyPillar::from_wire_label("Person"), None);
    }

    #[test]
    fn try_from_str_returns_ok_for_valid_labels() {
        assert_eq!(OntologyPillar::try_from("org"), Ok(OntologyPillar::Org));
        assert_eq!(
            OntologyPillar::try_from("person"),
            Ok(OntologyPillar::Person)
        );
    }

    #[test]
    fn try_from_str_returns_err_for_invalid_label() {
        let err = OntologyPillar::try_from("department").unwrap_err();
        assert_eq!(err.0, "department");
        assert!(err.to_string().contains("department"));
        assert!(err.to_string().contains("\"org\""));
        assert!(err.to_string().contains("\"person\""));
    }

    #[test]
    fn try_from_string_round_trips() {
        assert_eq!(
            OntologyPillar::try_from("org".to_string()),
            Ok(OntologyPillar::Org)
        );
        assert_eq!(
            OntologyPillar::try_from("person".to_string()),
            Ok(OntologyPillar::Person)
        );
    }

    #[test]
    fn display_matches_wire_label() {
        assert_eq!(OntologyPillar::Org.to_string(), "org");
        assert_eq!(OntologyPillar::Person.to_string(), "person");
    }

    #[test]
    fn pillar_isolation_org_and_person_are_distinct() {
        assert_ne!(OntologyPillar::Org, OntologyPillar::Person);
    }
}
