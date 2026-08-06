//! Fail-closed validation for Antigravity-generated web-content artifacts.

mod attempt_state;
mod provenance;

#[cfg(test)]
use std::ffi::OsString;

use attempt_state::{AttemptState, InspectionContext, ToolStep};
use provenance::GeneratedContentRoot;

use super::stream::{Event, EventName, StepType, ToolName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ToolAttemptAssessment {
    Safe,
    UnlistedTool,
    Unsafe,
}

pub(super) fn assess_tool_attempts(events: &[Event]) -> ToolAttemptAssessment {
    let current = events
        .iter()
        .find(|event| event.kind == EventName::Init)
        .and_then(|event| event.conversation_id.clone());
    let mut attempts = AttemptState::default();
    let mut root = None;
    let mut saw_unlisted_tool = false;

    for event in events {
        let Some(step) = event
            .step_update
            .as_ref()
            .filter(|step| step.step_type == StepType::Tool)
        else {
            continue;
        };
        let Some(info) = step.tool_info.as_ref() else {
            return ToolAttemptAssessment::Unsafe;
        };
        if step.tool_name.is_some_and(|name| name != info.name) {
            return ToolAttemptAssessment::Unsafe;
        }
        let tool_step = ToolStep::new(step, info);
        match info.name {
            ToolName::SearchWeb => {}
            ToolName::ReadUrlContent => attempts.track_read(tool_step),
            ToolName::ViewFile | ToolName::GrepSearch => {
                let Some(current) = current.as_ref().filter(|value| value.is_path_safe()) else {
                    return ToolAttemptAssessment::Unsafe;
                };
                let generated_root =
                    root.get_or_insert_with(GeneratedContentRoot::from_environment);
                let Some(generated_root) = generated_root.as_ref() else {
                    return ToolAttemptAssessment::Unsafe;
                };
                let context = InspectionContext::new(current, generated_root);
                if !attempts.track_inspection(tool_step, context) {
                    return ToolAttemptAssessment::Unsafe;
                }
            }
            ToolName::Other => saw_unlisted_tool = true,
        }
    }
    if !attempts.has_balanced_attempts() {
        ToolAttemptAssessment::Unsafe
    } else if saw_unlisted_tool {
        ToolAttemptAssessment::UnlistedTool
    } else {
        ToolAttemptAssessment::Safe
    }
}

#[cfg(test)]
#[expect(
    dead_code,
    reason = "keeps the existing module-level path helper available to generated-content tests"
)]
pub(super) fn test_content_path(conversation_id: &str, producer: u64) -> Option<OsString> {
    GeneratedContentRoot::from_environment().map(|root| {
        root.test_content_path(conversation_id, producer)
            .into_os_string()
    })
}
