//! Restricted direct-read validation and provisional grounding requirements.

use std::collections::HashSet;

use crate::{
    source_restriction::SourceRestriction,
    types::{HttpUrl, ResearchToolPolicy, SourceUrlKind},
};

use super::{
    GroundingRequirement,
    stream::{Event, EventName, StepType, ToolName},
};

pub(super) fn attempts_satisfy_restriction(
    events: &[Event],
    restriction: &SourceRestriction,
) -> bool {
    events
        .iter()
        .filter(|event| event.kind == EventName::StepUpdate)
        .filter_map(|event| event.step_update.as_ref())
        .filter(|step| step.step_type == StepType::Tool)
        .all(|step| {
            let Some(info) = &step.tool_info else {
                return false;
            };
            match info.name {
                ToolName::SearchWeb if restriction.domains().is_empty() => true,
                ToolName::SearchWeb => info
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.query.as_deref())
                    .is_some_and(|query| restriction.allows_search_query(query)),
                ToolName::ReadUrlContent => info
                    .parameters
                    .as_ref()
                    .and_then(|parameters| parameters.url.as_deref())
                    .and_then(|url| HttpUrl::parse(url).ok())
                    .is_some_and(|url| match url.source_kind() {
                        SourceUrlKind::Direct => restriction.allows(&url),
                        SourceUrlKind::GroundingRedirect => true,
                    }),
                ToolName::ViewFile | ToolName::GrepSearch => true,
                ToolName::Other => false,
            }
        })
}

pub(super) fn grounding_requirement(
    events: &[Event],
    policy: &ResearchToolPolicy,
) -> GroundingRequirement {
    let restriction = match policy {
        ResearchToolPolicy::Budget(_) | ResearchToolPolicy::ScopedTemporalSearch(_) => {
            return GroundingRequirement::None;
        }
        ResearchToolPolicy::Restricted {
            budget: _,
            restriction,
        }
        | ResearchToolPolicy::RestrictedScopedTemporalSearch {
            required_query: _,
            restriction,
        } => restriction.clone(),
    };
    let mut seen = HashSet::new();
    let transports = events
        .iter()
        .filter(|event| event.kind == EventName::StepUpdate)
        .filter_map(|event| event.step_update.as_ref())
        .filter(|step| step.step_type == StepType::Tool)
        .filter_map(|step| step.tool_info.as_ref())
        .filter(|info| info.name == ToolName::ReadUrlContent)
        .filter_map(|info| info.parameters.as_ref()?.url.as_deref())
        .filter_map(|url| HttpUrl::parse(url).ok())
        .filter(|url| url.source_kind() == SourceUrlKind::GroundingRedirect)
        .filter(|url| seen.insert(url.clone()))
        .collect();
    GroundingRequirement::Restricted {
        transports,
        restriction,
    }
}
