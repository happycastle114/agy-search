//! HOME-rooted provenance for Antigravity-generated content artifacts.

use std::path::PathBuf;

use super::super::stream::{ConversationId, StepIndex, ToolName};

#[derive(Debug)]
pub(super) struct GeneratedContentRoot(PathBuf);

impl GeneratedContentRoot {
    pub(super) fn from_environment() -> Option<Self> {
        let home = std::env::var_os(home_environment_key())?;
        let home = PathBuf::from(home);
        home.is_absolute()
            .then(|| Self(home.join(".gemini/antigravity-cli/brain")))
    }

    pub(super) fn content_path(
        &self,
        conversation_id: &ConversationId,
        producer: StepIndex,
    ) -> PathBuf {
        self.step_path(conversation_id, producer).join("content.md")
    }

    pub(super) fn permits_inspection(
        &self,
        tool: ToolName,
        conversation_id: &ConversationId,
        producer: StepIndex,
        path: &std::path::Path,
    ) -> bool {
        match tool {
            ToolName::ViewFile => self.content_path(conversation_id, producer) == path,
            ToolName::GrepSearch => {
                self.content_path(conversation_id, producer) == path
                    || self.step_path(conversation_id, producer) == path
            }
            ToolName::SearchWeb | ToolName::ReadUrlContent | ToolName::Other => false,
        }
    }

    #[cfg(test)]
    #[expect(
        dead_code,
        reason = "supports the existing facade-level generated-content path test helper"
    )]
    pub(super) fn test_content_path(&self, conversation_id: &str, producer: u64) -> PathBuf {
        self.0
            .join(conversation_id)
            .join(".system_generated/steps")
            .join(producer.to_string())
            .join("content.md")
    }

    fn step_path(&self, conversation_id: &ConversationId, producer: StepIndex) -> PathBuf {
        self.0
            .join(conversation_id.as_str())
            .join(".system_generated/steps")
            .join(producer.0.to_string())
    }
}

#[cfg(windows)]
const fn home_environment_key() -> &'static str {
    "USERPROFILE"
}

#[cfg(not(windows))]
const fn home_environment_key() -> &'static str {
    "HOME"
}
