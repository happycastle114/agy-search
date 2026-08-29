//! Exact-source Research recovery instructions.

use crate::types::Operation;

use super::PromptParts;

pub(super) const fn exact_research_retry_parts() -> PromptParts {
    (
        Operation::Research,
        "This is one isolated fail-closed exact-source Research recovery. Do not search or call \
         any tool. Use only CALLER_PREFETCHED_SOURCE_EVIDENCE for every literal \
         INPUT_JSON.source_restriction.urls member. Retain each exact URL unchanged. Copy only visible body prose or a \
         complete labeled table row; never reconstruct a row or fill a value from memory. If a \
         requested predicate is absent, report the evidence gap instead of guessing.",
        "Populate one audit candidate and one public finding for every independently requested \
         claim. Preserve every explicit count and keep each claim on its correct literal source. \
         Repeat the cited candidate's exact value verbatim in that finding's title or summary.",
        "Every candidate value must be an exact predicate-bearing phrase from its evidence excerpt. \
         Preserve all enumerated values in visible source order.",
        "Do not use local filesystem or artifact-inspection tools.",
    )
}
