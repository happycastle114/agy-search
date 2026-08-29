//! Research-tool attempt policy validation.

use crate::types::{Operation, ResearchToolBudget, ResearchToolPolicy};

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
    RecoverableFailedWebTool,
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
    match failed_web_tool_attempts(events) {
        Some(true) if operation == Operation::Search => {
            return EvidencePolicyAssessment::RecoverableFailedWebTool;
        }
        Some(true) | None => return EvidencePolicyAssessment::Rejected,
        Some(false) => {}
    }
    let Some(attempt_count) = completed_research_attempt_count(events) else {
        return EvidencePolicyAssessment::Rejected;
    };
    let tools = completed_research_tools(events);
    let evidence_is_sufficient = match policy {
        ResearchToolPolicy::Budget(budget) => {
            budget_is_satisfied(operation, *budget, &tools, attempt_count, policy.maximum())
        }
        ResearchToolPolicy::Restricted {
            budget,
            restriction,
        } => {
            budget_is_satisfied(operation, *budget, &tools, attempt_count, policy.maximum())
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

fn budget_is_satisfied(
    operation: Operation,
    budget: ResearchToolBudget,
    tools: &[ToolName],
    attempt_count: usize,
    maximum: usize,
) -> bool {
    match budget {
        ResearchToolBudget::PrefetchedEvidence => tools.is_empty() && attempt_count == 0,
        ResearchToolBudget::SiteDiscovery
        | ResearchToolBudget::StandardSearch
        | ResearchToolBudget::TemporalSearch
        | ResearchToolBudget::Research(_) => {
            tools == [ToolName::SearchWeb]
                && has_required_evidence(operation, tools)
                && attempt_count <= maximum
        }
    }
}

fn failed_web_tool_attempts(events: &[Event]) -> Option<bool> {
    let current = events
        .iter()
        .find(|event| event.kind == EventName::Init)?
        .conversation_id
        .as_ref()?;
    let mut active = Vec::new();
    let mut saw_failure = false;
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
            }
            Some(StepState::Done) if info.error.is_none() => {
                if let Some(position) = active.iter().position(|attempt| *attempt == identity) {
                    active.remove(position);
                }
            }
            Some(StepState::Error) if info.error.is_some() => {
                let position = active.iter().position(|attempt| *attempt == identity)?;
                active.remove(position);
                saw_failure = true;
            }
            Some(StepState::Active | StepState::Done | StepState::Error | StepState::Other)
            | None => return None,
        }
    }
    if saw_failure && !active.is_empty() {
        None
    } else {
        Some(saw_failure)
    }
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
