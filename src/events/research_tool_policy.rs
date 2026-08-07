//! Research-tool attempt policy validation.

use crate::types::{Operation, ResearchToolPolicy};

use super::{
    generated_content_policy::{self, ToolAttemptAssessment},
    sequence::{completed_research_tools, has_required_evidence},
    source_policy,
    stream::{Event, EventName, StepIndex, StepState, StepType, ToolName, ToolParameters},
};

mod scoped_search;

pub(super) use scoped_search::attempts_are_valid as scoped_search_attempts_are_valid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EvidencePolicyAssessment {
    Satisfied,
    RecoverableUnlistedTool,
    Rejected,
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct AttemptIdentity<'a> {
    step_index: Option<StepIndex>,
    tool: ToolName,
    parameters: Option<&'a ToolParameters>,
}

pub(super) fn assess_evidence_policy(
    operation: Operation,
    policy: &ResearchToolPolicy,
    events: &[Event],
) -> EvidencePolicyAssessment {
    let tool_assessment = generated_content_policy::assess_tool_attempts(events);
    if tool_assessment == ToolAttemptAssessment::Unsafe {
        return EvidencePolicyAssessment::Rejected;
    }
    let Some(attempt_count) = completed_research_attempt_count(events) else {
        return EvidencePolicyAssessment::Rejected;
    };
    let tools = completed_research_tools(events);
    let evidence_is_sufficient = match policy {
        ResearchToolPolicy::Budget(_) => {
            has_required_evidence(operation, &tools) && attempt_count <= policy.maximum()
        }
        ResearchToolPolicy::Restricted {
            budget: _,
            restriction,
        } => {
            restricted_evidence_is_sufficient(operation, restriction, &tools, events)
                && attempt_count <= policy.maximum()
                && source_policy::attempts_satisfy_restriction(events, restriction)
        }
        ResearchToolPolicy::ScopedTemporalSearch(required_query) => {
            operation == Operation::Search
                && scoped_search_attempts_are_valid(events, required_query)
        }
        ResearchToolPolicy::RestrictedScopedTemporalSearch {
            required_query,
            restriction,
        } => {
            operation == Operation::Search
                && scoped_search_attempts_are_valid(events, required_query)
                && source_policy::attempts_satisfy_restriction(events, restriction)
        }
    };
    if !evidence_is_sufficient {
        EvidencePolicyAssessment::Rejected
    } else if tool_assessment == ToolAttemptAssessment::UnlistedTool {
        EvidencePolicyAssessment::RecoverableUnlistedTool
    } else {
        EvidencePolicyAssessment::Satisfied
    }
}

fn restricted_evidence_is_sufficient(
    operation: Operation,
    restriction: &crate::source_restriction::SourceRestriction,
    tools: &[ToolName],
    events: &[Event],
) -> bool {
    let direct_read_can_prove = matches!(operation, Operation::Search | Operation::Research)
        && restriction.has_exact_urls();
    has_required_evidence(operation, tools)
        || (direct_read_can_prove && has_completed_paired_direct_read(events))
}

/// Finds one successful direct-read pair after the caller has validated every attempt.
///
/// `assess_evidence_policy` runs the tool-attempt assessment and
/// `completed_research_attempt_count` before this helper, so later failed,
/// unbalanced, or foreign attempts cannot be hidden by this first valid pair.
fn has_completed_paired_direct_read(events: &[Event]) -> bool {
    let Some(current) = events
        .iter()
        .find(|event| event.kind == EventName::Init)
        .and_then(|event| event.conversation_id.as_ref())
    else {
        return false;
    };
    let mut active = Vec::new();

    for event in events {
        let Some(step) = event
            .step_update
            .as_ref()
            .filter(|step| step.step_type == StepType::Tool)
        else {
            continue;
        };
        let Some(info) = step.tool_info.as_ref() else {
            return false;
        };
        if info.name != ToolName::ReadUrlContent {
            continue;
        }
        if step.conversation_id.as_ref() != Some(current) {
            return false;
        }
        let identity = AttemptIdentity {
            step_index: step.step_index,
            tool: info.name,
            parameters: info.parameters.as_ref(),
        };
        match step.state {
            Some(StepState::Active) if info.error.is_none() => active.push(identity),
            Some(StepState::Done) if info.error.is_none() => {
                let Some(position) = active.iter().position(|attempt| *attempt == identity) else {
                    return false;
                };
                active.remove(position);
                return true;
            }
            Some(StepState::Active | StepState::Done | StepState::Error | StepState::Other)
            | None => return false,
        }
    }
    false
}

fn completed_research_attempt_count(events: &[Event]) -> Option<usize> {
    let current = events
        .iter()
        .find(|event| event.kind == EventName::Init)?
        .conversation_id
        .as_ref()?;
    let mut active = Vec::new();
    let mut attempt_count = 0_usize;

    for event in events {
        let Some(step) = event
            .step_update
            .as_ref()
            .filter(|step| step.step_type == StepType::Tool)
        else {
            continue;
        };
        let info = step.tool_info.as_ref()?;
        if !info.name.is_web_evidence() {
            continue;
        }
        if step.conversation_id.as_ref() != Some(current) {
            return None;
        }
        let identity = AttemptIdentity {
            step_index: step.step_index,
            tool: info.name,
            parameters: info.parameters.as_ref(),
        };
        match step.state {
            Some(StepState::Active) if info.error.is_none() => {
                if active.contains(&identity) {
                    return None;
                }
                active.push(identity);
                attempt_count = attempt_count.checked_add(1)?;
            }
            Some(StepState::Done) if info.error.is_none() => {
                if let Some(position) = active.iter().position(|attempt| *attempt == identity) {
                    active.remove(position);
                } else {
                    attempt_count = attempt_count.checked_add(1)?;
                }
            }
            Some(StepState::Active | StepState::Done | StepState::Error | StepState::Other)
            | None => return None,
        }
    }

    active.is_empty().then_some(attempt_count)
}
