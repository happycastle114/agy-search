//! Exact-query validation for one temporal-scope Search attempt sequence.

use crate::types::{RequiredSearchQuery, ScopedQueryKind};

use super::{
    super::stream::{Event, EventName, StepState, StepType, ToolInfo, ToolName},
    completed_research_attempt_count,
};

pub(in crate::events) fn attempts_are_valid(
    events: &[Event],
    required_query: &RequiredSearchQuery,
) -> bool {
    let Some(attempt_count) = completed_research_attempt_count(events) else {
        return false;
    };
    let tool_steps = events
        .iter()
        .filter_map(|event| {
            (event.kind == EventName::StepUpdate)
                .then_some(event.step_update.as_ref())
                .flatten()
                .filter(|step| step.step_type == StepType::Tool)
        })
        .collect::<Vec<_>>();
    if tool_steps.is_empty()
        || tool_steps.iter().any(|step| {
            !matches!(
                step.tool_info.as_ref(),
                Some(ToolInfo {
                    name: ToolName::SearchWeb,
                    ..
                })
            )
        })
    {
        return false;
    }
    let active = tool_steps
        .iter()
        .filter(|step| step.state == Some(StepState::Active))
        .collect::<Vec<_>>();
    let Some(first_active) = active.first() else {
        return false;
    };
    if attempt_count > 2
        || active.len() > 2
        || !query_matches_exact(first_active, required_query)
        || active
            .iter()
            .skip(1)
            .any(|step| !query_has_required_prefix(step, required_query))
    {
        return false;
    }
    let successful = tool_steps
        .iter()
        .filter_map(|step| {
            (step.state == Some(StepState::Done))
                .then_some(step.tool_info.as_ref())
                .flatten()
                .filter(|info| info.error.is_none())
        })
        .collect::<Vec<_>>();
    !successful.is_empty()
        && successful.len() <= active.len()
        && successful
            .iter()
            .all(|info| query_matches_info(info, required_query))
}

fn query_matches_exact(
    step: &super::super::stream::StepUpdate,
    required_query: &RequiredSearchQuery,
) -> bool {
    step.tool_info.as_ref().is_some_and(|info| {
        info.parameters
            .as_ref()
            .and_then(|parameters| parameters.query.as_deref())
            .is_some_and(|query| required_query.classify(query) == ScopedQueryKind::Initial)
    })
}

fn query_has_required_prefix(
    step: &super::super::stream::StepUpdate,
    required_query: &RequiredSearchQuery,
) -> bool {
    step.tool_info
        .as_ref()
        .is_some_and(|info| query_matches_info(info, required_query))
}

fn query_matches_info(info: &ToolInfo, required_query: &RequiredSearchQuery) -> bool {
    info.parameters
        .as_ref()
        .and_then(|parameters| parameters.query.as_deref())
        .is_some_and(|query| required_query.classify(query) != ScopedQueryKind::Invalid)
}
