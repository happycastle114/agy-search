//! Parsed Antigravity `stream-json` event data.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use crate::error::AgyError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum EventName {
    Init,
    StepUpdate,
    Result,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StepType {
    Tool,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub(super) enum StepState {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "DONE")]
    Done,
    #[serde(rename = "ERROR")]
    Error,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ToolName {
    SearchWeb,
    ReadUrlContent,
    ViewFile,
    GrepSearch,
    #[serde(other)]
    Other,
}

impl ToolName {
    pub(super) const fn is_web_evidence(self) -> bool {
        matches!(self, Self::SearchWeb | Self::ReadUrlContent)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(transparent)]
pub(super) struct ConversationId(String);

impl ConversationId {
    pub(super) fn is_path_safe(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 128
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd)]
#[serde(transparent)]
pub(super) struct StepIndex(pub(super) u64);

#[derive(Debug, Deserialize)]
pub(super) struct ToolInfo {
    pub(super) name: ToolName,
    #[serde(default)]
    pub(super) error: Option<Value>,
    #[serde(default)]
    pub(super) parameters: Option<ToolParameters>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub(super) struct ToolParameters {
    #[serde(default, alias = "Query")]
    pub(super) query: Option<String>,
    #[serde(default, rename = "Url", alias = "url")]
    pub(super) url: Option<String>,
    #[serde(default, rename = "AbsolutePath", alias = "absolute_path")]
    pub(super) absolute_path: Option<PathBuf>,
    #[serde(default, rename = "SearchPath", alias = "search_path")]
    pub(super) search_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StepUpdate {
    #[serde(default)]
    pub(super) conversation_id: Option<ConversationId>,
    #[serde(default)]
    pub(super) step_index: Option<StepIndex>,
    pub(super) step_type: StepType,
    #[serde(default)]
    pub(super) state: Option<StepState>,
    #[serde(default)]
    pub(super) tool_info: Option<ToolInfo>,
    #[serde(default)]
    pub(super) tool_name: Option<ToolName>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ResultEvent {
    #[serde(default)]
    pub(super) structured_output: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Event {
    #[serde(rename = "event")]
    pub(super) kind: EventName,
    #[serde(default)]
    pub(super) conversation_id: Option<ConversationId>,
    #[serde(default)]
    pub(super) step_update: Option<StepUpdate>,
    #[serde(default)]
    pub(super) result: Option<ResultEvent>,
}

pub(super) fn parse_events(output: &str) -> Result<Vec<Event>, AgyError> {
    let events: Result<Vec<_>, _> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect();
    let events = events.map_err(|_| AgyError::OutputInvalid)?;
    if events.is_empty() {
        return Err(AgyError::OutputInvalid);
    }
    Ok(events)
}
