use crate::cidr::cidr_contains_cidr;
use crate::error::CloudNetworkError;
use crate::identifier::SecurityGroupId;
use crate::route::RouteDestination;
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RuleDirection {
    Ingress,
    Egress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum IpProtocol {
    Tcp,
    Udp,
    Icmp,
    Any,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityRule {
    pub direction: RuleDirection,       // data_class: PUBLIC
    pub protocol: IpProtocol,           // data_class: PUBLIC
    pub port_range: Option<(u16, u16)>, // data_class: PUBLIC
    pub cidr: RouteDestination,         // data_class: PUBLIC
    pub description: String,            // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityGroupCreate {
    pub id: String,               // data_class: INTERNAL_ONLY
    pub rules: Vec<SecurityRule>, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityGroup {
    pub id: SecurityGroupId,      // data_class: INTERNAL_ONLY
    pub rules: Vec<SecurityRule>, // data_class: PUBLIC
}

/// The direction + L4 attributes of a network flow to be evaluated against a
/// [`SecurityGroup`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowMatch {
    pub direction: RuleDirection,    // data_class: PUBLIC
    pub protocol: IpProtocol,        // data_class: PUBLIC
    pub port: Option<u16>,           // data_class: PUBLIC
    pub peer_cidr: RouteDestination, // data_class: PUBLIC
}

/// Result of evaluating a [`FlowMatch`] against a [`SecurityGroup`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    /// The flow is allowed; `matched_rule` is the first matching rule.
    Allow { matched_rule: SecurityRule },
    /// The flow is denied; `matched_rule` is `None` when no rule matched.
    Deny { matched_rule: Option<SecurityRule> },
}

impl SecurityGroup {
    pub fn new(input: SecurityGroupCreate) -> Result<Self, CloudNetworkError> {
        let id = SecurityGroupId::new(input.id)?;
        for rule in &input.rules {
            validate_security_rule(rule)?;
        }
        Ok(Self {
            id,
            rules: input.rules,
        })
    }

    /// Evaluate a [`FlowMatch`] against this group's rules (first-match wins).
    ///
    /// Returns `Ok(Decision::Allow { matched_rule })` when the first matching
    /// rule is found, or `Ok(Decision::Deny { matched_rule: None })` when no
    /// rule matches.
    pub fn evaluate(&self, flow: &FlowMatch) -> Result<Decision, CloudNetworkError> {
        for rule in &self.rules {
            if !rule_matches(rule, flow)? {
                continue;
            }
            return Ok(Decision::Allow {
                matched_rule: rule.clone(),
            });
        }
        Ok(Decision::Deny { matched_rule: None })
    }

    /// Return all (shadowing, shadowed) rule pairs where the first rule fully
    /// subsumes the second (same direction, protocol compatible, CIDR contains,
    /// port range contains).
    pub fn detect_shadowed_rules(
        &self,
    ) -> Result<Vec<(SecurityRule, SecurityRule)>, CloudNetworkError> {
        let mut pairs = Vec::new();
        let rules = &self.rules;
        for i in 0..rules.len() {
            for j in (i + 1)..rules.len() {
                let a = &rules[i];
                let b = &rules[j];
                if rule_subsumes(a, b)? {
                    pairs.push((a.clone(), b.clone()));
                }
            }
        }
        Ok(pairs)
    }
}

fn validate_security_rule(rule: &SecurityRule) -> Result<(), CloudNetworkError> {
    if let Some((start, end)) = rule.port_range
        && (start == 0 || end == 0 || start > end || matches!(rule.protocol, IpProtocol::Icmp))
    {
        return Err(CloudNetworkError::InvalidSecurityRule);
    }
    if rule.description.trim().is_empty() || rule.description.len() > 128 {
        return Err(CloudNetworkError::InvalidSecurityRule);
    }
    Ok(())
}

/// Returns `true` when `rule` matches every dimension of `flow`.
fn rule_matches(rule: &SecurityRule, flow: &FlowMatch) -> Result<bool, CloudNetworkError> {
    if rule.direction != flow.direction {
        return Ok(false);
    }
    if rule.protocol != IpProtocol::Any && rule.protocol != flow.protocol {
        return Ok(false);
    }
    match (rule.port_range, flow.port) {
        (Some((lo, hi)), Some(p)) => {
            if p < lo || p > hi {
                return Ok(false);
            }
        }
        (Some(_), None) => return Ok(false),
        (None, _) => {}
    }
    if !cidr_contains_cidr(&rule.cidr, &flow.peer_cidr)? {
        return Ok(false);
    }
    Ok(true)
}

/// Returns `true` when rule `a` fully subsumes rule `b`:
/// same direction, compatible protocol (`a` is `Any` or equal), CIDR of `a`
/// contains CIDR of `b`, and port range of `a` contains port range of `b`.
fn rule_subsumes(a: &SecurityRule, b: &SecurityRule) -> Result<bool, CloudNetworkError> {
    if a.direction != b.direction {
        return Ok(false);
    }
    if a.protocol != IpProtocol::Any && a.protocol != b.protocol {
        return Ok(false);
    }
    if !cidr_contains_cidr(&a.cidr, &b.cidr)? {
        return Ok(false);
    }
    // Port range subsumption: a's range must contain b's range.
    match (a.port_range, b.port_range) {
        (None, _) => {}
        (Some(_), None) => return Ok(false),
        (Some((a_lo, a_hi)), Some((b_lo, b_hi))) => {
            if a_lo > b_lo || a_hi < b_hi {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
