//! Typed request documents sent to Antigravity.

use schemars::JsonSchema;
use serde::Serialize;

use crate::source_restriction::SourceRestriction;
use crate::temporal_contract::TemporalContract;
use crate::types::{
    DatePolicy, HttpUrl, NonEmptyText, Operation, RequiredSearchQuery, ResearchAttemptBudget,
    ResearchToolBudget, ScopePolicy, SourcePolicy, VerificationMode,
};
use crate::verification::ScopeLabel;

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchRequest {
    pub(crate) query: NonEmptyText,
    pub(crate) source_policy: SourcePolicy,
    pub(crate) scope_policy: ScopePolicy,
    pub(crate) date_policy: DatePolicy,
    pub(crate) verification: VerificationMode,
    pub(crate) temporal_contract: Option<TemporalContract>,
    #[serde(skip_serializing_if = "SourceRestriction::is_unrestricted")]
    pub(crate) source_restriction: SourceRestriction,
    pub(crate) max_results: u16,
    pub(crate) country: Option<NonEmptyText>,
    pub(crate) max_tokens_per_page: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemporalScopeRequest {
    pub(crate) query: NonEmptyText,
    pub(crate) source_policy: SourcePolicy,
    pub(crate) scope_policy: ScopePolicy,
    pub(crate) date_policy: DatePolicy,
    pub(crate) verification: VerificationMode,
    pub(crate) temporal_contract: Option<TemporalContract>,
    #[serde(skip_serializing_if = "SourceRestriction::is_unrestricted")]
    pub(crate) source_restriction: SourceRestriction,
    pub(crate) max_results: u16,
    pub(crate) country: Option<NonEmptyText>,
    pub(crate) max_tokens_per_page: Option<u32>,
    pub(crate) scope: ScopeLabel,
    pub(crate) required_search_query: RequiredSearchQuery,
}

impl TemporalScopeRequest {
    pub(crate) fn from_search(request: &SearchRequest, scope: ScopeLabel) -> Self {
        Self {
            query: request.query.clone(),
            source_policy: request.source_policy,
            scope_policy: request.scope_policy,
            date_policy: request.date_policy,
            verification: request.verification,
            temporal_contract: request.temporal_contract.clone(),
            max_results: request.max_results,
            source_restriction: request.source_restriction.clone(),
            country: request.country.clone(),
            max_tokens_per_page: request.max_tokens_per_page,
            required_search_query: RequiredSearchQuery::for_exact_scope(
                &request.query,
                scope.as_str(),
                request.source_restriction.domains(),
                request.country.as_ref(),
            ),
            scope,
        }
    }

    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExtractRequest {
    pub(crate) urls: Vec<HttpUrl>,
    pub(crate) query: Option<NonEmptyText>,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MapRequest {
    pub(crate) url: HttpUrl,
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    pub(crate) allow_external: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CrawlRequest {
    pub(crate) url: HttpUrl,
    pub(crate) limit: u16,
    pub(crate) instructions: Option<NonEmptyText>,
    pub(crate) allow_external: bool,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchRequest {
    pub(crate) query: NonEmptyText,
    pub(crate) source_policy: SourcePolicy,
    pub(crate) scope_policy: ScopePolicy,
    pub(crate) date_policy: DatePolicy,
    pub(crate) verification: VerificationMode,
    pub(crate) temporal_contract: Option<TemporalContract>,
    #[serde(skip_serializing_if = "SourceRestriction::is_unrestricted")]
    pub(crate) source_restriction: SourceRestriction,
    pub(crate) max_sources: u16,
    pub(crate) tool_call_budget: ResearchAttemptBudget,
}

#[derive(Clone, Debug)]
pub(crate) enum ContentRequest {
    Search(SearchRequest),
    Extract(ExtractRequest),
    Map(MapRequest),
    Crawl(CrawlRequest),
    Research(ResearchRequest),
}

impl ContentRequest {
    pub(crate) const fn operation(&self) -> Operation {
        match self {
            Self::Search(_) => Operation::Search,
            Self::Extract(_) => Operation::Extract,
            Self::Map(_) => Operation::Map,
            Self::Crawl(_) => Operation::Crawl,
            Self::Research(_) => Operation::Research,
        }
    }

    pub(crate) const fn verification(&self) -> VerificationMode {
        match self {
            Self::Search(request) => request.verification,
            Self::Research(request) => request.verification,
            Self::Extract(_) | Self::Map(_) | Self::Crawl(_) => VerificationMode::Standard,
        }
    }

    pub(crate) const fn temporal_contract(&self) -> Option<&TemporalContract> {
        match self {
            Self::Search(request) => request.temporal_contract.as_ref(),
            Self::Research(request) => request.temporal_contract.as_ref(),
            Self::Extract(_) | Self::Map(_) | Self::Crawl(_) => None,
        }
    }

    pub(crate) const fn source_restriction(&self) -> &SourceRestriction {
        match self {
            Self::Search(request) => &request.source_restriction,
            Self::Research(request) => &request.source_restriction,
            Self::Extract(_) | Self::Map(_) | Self::Crawl(_) => &SourceRestriction::Unrestricted,
        }
    }

    pub(crate) const fn search_result_limit(&self) -> Option<u16> {
        match self {
            Self::Search(request) => Some(request.max_results),
            Self::Extract(_) | Self::Map(_) | Self::Crawl(_) | Self::Research(_) => None,
        }
    }

    pub(crate) const fn tool_budget(&self) -> ResearchToolBudget {
        match self {
            Self::Search(request) => match request.verification {
                VerificationMode::Standard => ResearchToolBudget::StandardSearch,
                VerificationMode::TemporalComparison => ResearchToolBudget::TemporalSearch,
            },
            Self::Research(request) => ResearchToolBudget::Research(request.tool_call_budget),
            Self::Extract(_) | Self::Map(_) | Self::Crawl(_) => ResearchToolBudget::Single,
        }
    }

    pub(crate) fn tool_policy(&self) -> crate::types::ResearchToolPolicy {
        let budget = self.tool_budget();
        match self.source_restriction() {
            SourceRestriction::Unrestricted => crate::types::ResearchToolPolicy::Budget(budget),
            restriction @ SourceRestriction::Allowlist { .. } => {
                crate::types::ResearchToolPolicy::Restricted {
                    budget,
                    restriction: Box::new(restriction.clone()),
                }
            }
        }
    }

    pub(crate) fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Search(request) => serde_json::to_string(request),
            Self::Extract(request) => serde_json::to_string(request),
            Self::Map(request) => serde_json::to_string(request),
            Self::Crawl(request) => serde_json::to_string(request),
            Self::Research(request) => serde_json::to_string(request),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::types::ResearchAttemptBudget;

    #[test]
    fn research_request_serializes_explicit_attempt_budget() {
        // Given a synthesis request with four requested sources.
        let request = ContentRequest::Research(ResearchRequest {
            query: NonEmptyText::parse("compare releases").expect("test query must be valid"),
            source_policy: SourcePolicy::PrimaryFirst,
            scope_policy: ScopePolicy::CompleteRequestedScope,
            date_policy: DatePolicy::ExplicitSourceOnly,
            verification: VerificationMode::Standard,
            temporal_contract: None,
            source_restriction: SourceRestriction::Unrestricted,
            max_sources: 4,
            tool_call_budget: ResearchAttemptBudget::from_max_sources(4),
        });

        // When the caller request is serialized for Antigravity.
        let serialized: serde_json::Value =
            serde_json::from_str(&request.to_json().expect("research request must serialize"))
                .expect("serialized request must be JSON");

        // Then the exact derived attempt budget crosses the request boundary.
        assert_eq!(serialized.get("tool_call_budget"), Some(&json!(6)));
    }
}
