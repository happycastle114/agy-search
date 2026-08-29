//! Deterministic same-URL exact-value proof and source-context projection.

mod binding;
mod fetch;
mod prompt;
mod research;
mod search;
mod text;
mod title;

pub(crate) use research::SourceEvidenceSnapshot;
pub(crate) use search::SearchSourceEvidence;

const MINIMUM_EXCERPT_CHARACTERS: usize = 40;
const MINIMUM_EXCERPT_WORDS: usize = 5;
const MINIMUM_SEARCH_EXCERPT_CHARACTERS: usize = 20;
const MINIMUM_SEARCH_EXCERPT_WORDS: usize = 3;
const MINIMUM_VALUE_CHARACTERS: usize = 4;
const MINIMUM_FALLBACK_VALUE_WORDS: usize = 2;
const MINIMUM_FALLBACK_VALUE_CHARACTERS: usize = 12;
const MINIMUM_PAGE_HEADING_CHARACTERS: usize = 8;
const MAXIMUM_PROJECTED_CONTEXT_CHARACTERS: usize = 480;
const MAXIMUM_NORMALIZED_VALUE_SPAN_CHARACTERS: usize = 240;

#[cfg(test)]
use crate::{error::AgyError, source_fetch::SafeSourceUrl, types::NonEmptyText};
#[cfg(test)]
use binding::{
    bind_evidence, bind_search_candidate_evidence, find_compact_percentage, nearest_date_binding,
};
#[cfg(test)]
use search::SearchEvidencePage;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use text::canonical_evidence_text;
#[cfg(test)]
use tokio::time::Instant;

#[cfg(test)]
mod tests;
