//! Strict parsing of Antigravity `stream-json` evidence and terminal output.

use serde::Deserialize;
use serde_json::Value;

use crate::{error::AgyError, response::Document as ResponseDocument, types::Operation};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum EventName {
    Init,
    StepUpdate,
    Result,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum StepType {
    Tool,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum StepState {
    #[serde(rename = "DONE")]
    Done,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ResearchTool {
    SearchWeb,
    ReadUrlContent,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ToolInfo {
    name: ResearchTool,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct StepUpdate {
    step_type: StepType,
    #[serde(default)]
    state: Option<StepState>,
    #[serde(default)]
    tool_info: Option<ToolInfo>,
}

#[derive(Debug, Deserialize)]
struct ResultEvent {
    #[serde(default)]
    structured_output: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct Event {
    #[serde(rename = "event")]
    kind: EventName,
    #[serde(default)]
    step_update: Option<StepUpdate>,
    #[serde(default)]
    result: Option<ResultEvent>,
}

pub(crate) fn parse_structured_run(
    output: &[u8],
    operation: Operation,
) -> Result<ResponseDocument, AgyError> {
    let text = std::str::from_utf8(output).map_err(|_| AgyError::OutputInvalid)?;
    let events = parse_events(text)?;
    let tools = completed_research_tools(&events);
    if !has_required_evidence(operation, &tools) {
        return Err(AgyError::OutputInvalid);
    }
    let value = terminal_output(&events)?;
    ResponseDocument::parse(operation, value)
}

fn parse_events(output: &str) -> Result<Vec<Event>, AgyError> {
    let events: Result<Vec<_>, _> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect();
    let events = events.map_err(|_| AgyError::OutputInvalid)?;
    if events.is_empty() {
        return Err(AgyError::OutputInvalid);
    }
    if !valid_sequence(&events) {
        return Err(AgyError::OutputInvalid);
    }
    Ok(events)
}

fn valid_sequence(events: &[Event]) -> bool {
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
    starts_with_init && ends_with_result && init_count == 1 && result_count == 1 && !has_unknown
}

fn completed_research_tools(events: &[Event]) -> Vec<ResearchTool> {
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
        if tool != ResearchTool::Other && !tools.contains(&tool) {
            tools.push(tool);
        }
    }
    tools
}

fn has_required_evidence(operation: Operation, tools: &[ResearchTool]) -> bool {
    match operation {
        Operation::Search | Operation::Research => tools.contains(&ResearchTool::SearchWeb),
        Operation::Extract | Operation::Crawl => tools.contains(&ResearchTool::ReadUrlContent),
        Operation::Map => {
            tools.contains(&ResearchTool::SearchWeb)
                || tools.contains(&ResearchTool::ReadUrlContent)
        }
    }
}

fn terminal_output(events: &[Event]) -> Result<Value, AgyError> {
    events
        .iter()
        .rev()
        .find(|event| event.kind == EventName::Result)
        .and_then(|event| event.result.as_ref())
        .and_then(|result| result.structured_output.clone())
        .ok_or(AgyError::OutputInvalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_mcp_tool_does_not_prove_web_research() {
        let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"call_mcp_tool"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

        assert!(parse_structured_run(stream, Operation::Search).is_err());
    }

    #[test]
    fn terminal_result_must_be_the_final_event() {
        let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}
{"event":"init"}"#;

        assert!(parse_structured_run(stream, Operation::Search).is_err());
    }

    #[test]
    fn completed_tool_with_error_does_not_prove_research() {
        let stream = br#"{"event":"init"}
{"event":"step_update","step_update":{"state":"DONE","step_type":"tool","tool_info":{"name":"search_web","error":"private failure"}}}
{"event":"result","result":{"structured_output":{"object":"search","results":[{"title":"Source","url":"https://example.com/","snippet":"Evidence"}]}}}"#;

        assert!(parse_structured_run(stream, Operation::Search).is_err());
    }
}
