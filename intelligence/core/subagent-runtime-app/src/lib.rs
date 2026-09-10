//! Binary surface for the per-facet subagent runtime.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod anthropic;

pub use anthropic::AnthropicSubagentPort;
pub use intelligence_subagent_runtime_usecase::{
    FacetFindingJson, FacetPromptTemplate, FacetRecommendation, FacetSlug, MockSubagentPort,
    SubagentError, SubagentPort, SubagentRequest, SubagentResponse, fanout_panel_v23,
};
