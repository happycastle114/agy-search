//! Stream sequence and terminal-evidence validation.

use serde_json::Value;

use crate::{error::AgyError, types::Operation};

use super::stream::{Event, EventName, StepState, StepType, ToolName};

pub(super) fn validate(events: &[Event]) -> Result<(), AgyError> {
    let starts_with_init = events
        .first()
        .is_some_and(|event| event.kind == EventName::Init);
    let ends_with_result = events
        .last()
        .is_some_and(|event| event.kind == EventName::Result);
    let init_count = events
        .iter()
        .filter(|event| event.kind == EventName::Init)
        .count();
    let result_count = events
        .iter()
        .filter(|event| event.kind == EventName::Result)
        .count();
    let has_unknown = events.iter().any(|event| event.kind == EventName::Other);
    if starts_with_init && ends_with_result && init_count == 1 && result_count == 1 && !has_unknown
    {
        return Ok(());
    }
    Err(AgyError::OutputInvalid)
}

pub(super) fn terminal_output(events: &[Event]) -> Result<Value, AgyError> {
    events
        .iter()
        .rev()
        .find(|event| event.kind == EventName::Result)
        .and_then(|event| event.result.as_ref())
        .and_then(|result| result.structured_output.clone())
        .ok_or(AgyError::OutputInvalid)
}

pub(super) fn completed_research_tools(events: &[Event]) -> Vec<ToolName> {
    let mut tools = Vec::new();
    for event in events {
        if event.kind != EventName::StepUpdate {
            continue;
        }
        let Some(step) = &event.step_update else {
            continue;
        };
        if step.step_type != StepType::Tool || step.state != Some(StepState::Done) {
            continue;
        }
        let Some(tool) = step
            .tool_info
            .as_ref()
            .filter(|info| info.error.is_none())
            .map(|info| info.name)
        else {
            continue;
        };
        if tool.is_web_evidence() && !tools.contains(&tool) {
            tools.push(tool);
        }
    }
    tools
}

pub(super) fn has_required_evidence(operation: Operation, tools: &[ToolName]) -> bool {
    match operation {
        Operation::Search | Operation::Research => tools.contains(&ToolName::SearchWeb),
        Operation::Extract | Operation::Crawl => tools.contains(&ToolName::ReadUrlContent),
        Operation::Map => {
            tools.contains(&ToolName::SearchWeb) || tools.contains(&ToolName::ReadUrlContent)
        }
    }
}
