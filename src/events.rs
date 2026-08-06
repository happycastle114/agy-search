//! Strict parsing of Antigravity `stream-json` evidence and terminal output.

use std::marker::PhantomData;

use crate::{
    error::AgyError,
    response::Document as ResponseDocument,
    source_restriction::SourceRestriction,
    types::{HttpUrl, Operation, ResearchToolPolicy},
};

mod generated_content_policy;
mod research_tool_policy;
mod sequence;
mod source_policy;
mod stream;

#[derive(Debug)]
pub(crate) struct PendingGrounding;

#[derive(Debug)]
pub(crate) struct GroundingResolved;

#[derive(Debug)]
pub(crate) enum GroundingRequirement {
    None,
    Restricted {
        transports: Vec<HttpUrl>,
        restriction: Box<SourceRestriction>,
    },
}

#[derive(Debug)]
pub(crate) struct ParsedRun<State> {
    pub(crate) response: ResponseDocument,
    pub(crate) grounding: GroundingRequirement,
    state: PhantomData<State>,
}

#[derive(Debug)]
pub(crate) enum StructuredRunError {
    Invalid(AgyError),
    RecoverableUnlistedTool(Box<ResponseDocument>),
}

impl StructuredRunError {
    pub(crate) fn into_public_error(self) -> AgyError {
        match self {
            Self::Invalid(error) => error,
            Self::RecoverableUnlistedTool(_) => AgyError::OutputInvalid,
        }
    }
}

impl From<AgyError> for StructuredRunError {
    fn from(error: AgyError) -> Self {
        Self::Invalid(error)
    }
}

impl ParsedRun<GroundingResolved> {
    pub(crate) fn into_response(self) -> ResponseDocument {
        self.response
    }
}

impl ParsedRun<PendingGrounding> {
    pub(crate) fn mark_resolved(self) -> ParsedRun<GroundingResolved> {
        ParsedRun {
            response: self.response,
            grounding: self.grounding,
            state: PhantomData,
        }
    }
}

pub(crate) fn parse_structured_run(
    output: &[u8],
    operation: Operation,
    policy: &ResearchToolPolicy,
) -> Result<ParsedRun<PendingGrounding>, StructuredRunError> {
    let text = std::str::from_utf8(output).map_err(|_| AgyError::OutputInvalid)?;
    let events = stream::parse_events(text)?;
    sequence::validate(&events)?;
    let value = sequence::terminal_output(&events)?;
    let response = ResponseDocument::parse(operation, value)?;
    match research_tool_policy::assess_evidence_policy(operation, policy, &events) {
        research_tool_policy::EvidencePolicyAssessment::Satisfied => {}
        research_tool_policy::EvidencePolicyAssessment::RecoverableUnlistedTool => {
            return Err(StructuredRunError::RecoverableUnlistedTool(Box::new(
                response,
            )));
        }
        research_tool_policy::EvidencePolicyAssessment::Rejected => {
            return Err(AgyError::OutputInvalid.into());
        }
    }
    Ok(ParsedRun {
        response,
        grounding: source_policy::grounding_requirement(&events, policy),
        state: PhantomData,
    })
}

#[cfg(test)]
mod event_test;
#[cfg(test)]
mod generated_content_test;
