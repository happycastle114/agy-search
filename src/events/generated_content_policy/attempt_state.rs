//! Sequential read and inspection lifecycle tracking for generated content.

use std::{collections::HashMap, path::PathBuf};

use crate::types::HttpUrl;

use super::{
    super::stream::{ConversationId, StepIndex, StepState, StepUpdate, ToolInfo, ToolName},
    provenance::GeneratedContentRoot,
};

#[derive(Debug, Eq, PartialEq)]
struct ReadAttempt {
    conversation_id: ConversationId,
    url: HttpUrl,
}

#[derive(Debug, Eq, PartialEq)]
struct InspectionAttempt {
    conversation_id: ConversationId,
    producer: StepIndex,
    tool: ToolName,
    path: PathBuf,
    query: Option<String>,
}

#[derive(Default)]
pub(super) struct AttemptState {
    active_reads: HashMap<StepIndex, ReadAttempt>,
    completed_reads: Vec<(ConversationId, StepIndex)>,
    active_inspections: HashMap<StepIndex, InspectionAttempt>,
}

#[derive(Clone, Copy)]
pub(super) struct ToolStep<'a> {
    step: &'a StepUpdate,
    info: &'a ToolInfo,
}

impl<'a> ToolStep<'a> {
    pub(super) const fn new(step: &'a StepUpdate, info: &'a ToolInfo) -> Self {
        Self { step, info }
    }
}

#[derive(Clone, Copy)]
pub(super) struct InspectionContext<'a> {
    current: &'a ConversationId,
    root: &'a GeneratedContentRoot,
}

impl<'a> InspectionContext<'a> {
    pub(super) const fn new(current: &'a ConversationId, root: &'a GeneratedContentRoot) -> Self {
        Self { current, root }
    }
}

impl AttemptState {
    pub(super) fn track_read(&mut self, tool_step: ToolStep<'_>) {
        let Some((index, attempt)) = read_attempt(tool_step) else {
            return;
        };
        match tool_step.step.state {
            Some(StepState::Active) => {
                self.active_reads.insert(index, attempt);
            }
            Some(StepState::Done) if tool_step.info.error.is_none() => {
                if self.active_reads.remove(&index).as_ref() == Some(&attempt) {
                    self.completed_reads.push((attempt.conversation_id, index));
                }
            }
            Some(StepState::Done | StepState::Error | StepState::Other) | None => {}
        }
    }

    pub(super) fn track_inspection(
        &mut self,
        tool_step: ToolStep<'_>,
        context: InspectionContext<'_>,
    ) -> bool {
        let Some((index, attempt)) = self.inspection_attempt(tool_step, context) else {
            return false;
        };
        match tool_step.step.state {
            Some(StepState::Active) => self.active_inspections.insert(index, attempt).is_none(),
            Some(StepState::Done) if tool_step.info.error.is_none() => {
                self.active_inspections.remove(&index).as_ref() == Some(&attempt)
            }
            Some(StepState::Done | StepState::Error | StepState::Other) | None => false,
        }
    }

    pub(super) fn has_balanced_attempts(&self) -> bool {
        self.active_reads.is_empty() && self.active_inspections.is_empty()
    }

    fn inspection_attempt(
        &self,
        tool_step: ToolStep<'_>,
        context: InspectionContext<'_>,
    ) -> Option<(StepIndex, InspectionAttempt)> {
        let index = tool_step.step.step_index?;
        let conversation_id = tool_step.step.conversation_id.clone()?;
        if &conversation_id != context.current {
            return None;
        }
        let parameters = tool_step.info.parameters.as_ref()?;
        let path = match tool_step.info.name {
            ToolName::ViewFile => parameters.absolute_path.clone()?,
            ToolName::GrepSearch => parameters.search_path.clone()?,
            ToolName::SearchWeb | ToolName::ReadUrlContent | ToolName::Other => return None,
        };
        let producer = self.completed_reads.iter().find_map(|(owner, producer)| {
            (owner == context.current
                && *producer < index
                && context.root.permits_inspection(
                    tool_step.info.name,
                    context.current,
                    *producer,
                    &path,
                ))
            .then_some(*producer)
        })?;
        let query = match tool_step.info.name {
            ToolName::GrepSearch => parameters.query.clone(),
            ToolName::ViewFile => None,
            ToolName::SearchWeb | ToolName::ReadUrlContent | ToolName::Other => return None,
        };
        Some((
            index,
            InspectionAttempt {
                conversation_id,
                producer,
                tool: tool_step.info.name,
                path,
                query,
            },
        ))
    }
}

fn read_attempt(tool_step: ToolStep<'_>) -> Option<(StepIndex, ReadAttempt)> {
    let index = tool_step.step.step_index?;
    let conversation_id = tool_step.step.conversation_id.clone()?;
    let url = tool_step
        .info
        .parameters
        .as_ref()
        .and_then(|parameters| parameters.url.as_deref())
        .and_then(|value| HttpUrl::parse(value).ok())?;
    Some((
        index,
        ReadAttempt {
            conversation_id,
            url,
        },
    ))
}
