mod date;
mod value;

pub(super) use date::nearest_date_binding;
use value::find_bound_value;

use super::{
    MINIMUM_EXCERPT_CHARACTERS, MINIMUM_EXCERPT_WORDS, MINIMUM_SEARCH_EXCERPT_CHARACTERS,
    MINIMUM_SEARCH_EXCERPT_WORDS, MINIMUM_VALUE_CHARACTERS,
    text::{canonical_evidence_text, evidence_context, find_ascii_case_insensitive},
};

pub(super) struct BoundEvidence {
    pub(super) context: String,
    pub(super) value: String,
    pub(super) value_end: usize,
    pub(super) value_start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceConnector {
    And,
    Or,
}

impl EvidenceConnector {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct EvidenceToken {
    normalized: String,
    start: usize,
    end: usize,
}

pub(super) fn bind_evidence(body: &str, excerpt: &str, value: &str) -> Option<BoundEvidence> {
    bind_evidence_with_minimum(
        body,
        excerpt,
        value,
        MINIMUM_EXCERPT_CHARACTERS,
        MINIMUM_EXCERPT_WORDS,
    )
}

fn bind_search_evidence(body: &str, excerpt: &str, value: &str) -> Option<BoundEvidence> {
    bind_evidence_with_minimum(
        body,
        excerpt,
        value,
        MINIMUM_SEARCH_EXCERPT_CHARACTERS,
        MINIMUM_SEARCH_EXCERPT_WORDS,
    )
}

pub(super) fn bind_search_candidate_evidence(
    body: &str,
    excerpt: &str,
    value: &str,
    date: Option<&crate::types::CalendarDate>,
) -> Option<BoundEvidence> {
    if let Some(bound) = bind_search_evidence(body, excerpt, value) {
        return Some(bound);
    }
    let excerpt = canonical_evidence_text(excerpt);
    let value = canonical_evidence_text(value);
    if !evidence_shape_is_well_formed(
        &excerpt,
        &value,
        MINIMUM_SEARCH_EXCERPT_CHARACTERS,
        MINIMUM_SEARCH_EXCERPT_WORDS,
    ) {
        return None;
    }
    if compact_percentage_token(&value)
        .is_some_and(|percentage| find_ascii_case_insensitive(&excerpt, percentage).is_some())
        && let Some((start, end)) = find_compact_percentage(body, &value, date)
    {
        return Some(BoundEvidence {
            context: evidence_context(body, start, end),
            value: body[start..end].to_owned(),
            value_end: end,
            value_start: start,
        });
    }
    find_ascii_case_insensitive(&excerpt, &value)?;
    let value_date = crate::source_date::parse(&value).ok()?;
    if date.is_some_and(|date| date.as_str() != value_date.as_str()) {
        return None;
    }
    let (start, end) = nearest_date_binding(body, &value_date, 0, 0)?;
    Some(BoundEvidence {
        context: evidence_context(body, start, end),
        value: body[start..end].to_owned(),
        value_end: end,
        value_start: start,
    })
}

pub(super) fn find_compact_percentage(
    body: &str,
    value: &str,
    date: Option<&crate::types::CalendarDate>,
) -> Option<(usize, usize)> {
    let percentage = compact_percentage_token(value)?;
    if let Some(exact) = find_ascii_case_insensitive(body, percentage) {
        return Some(exact);
    }
    let numeric = percentage.strip_suffix('%')?;
    let date = date?;
    body.match_indices(numeric)
        .filter_map(|(start, matched)| {
            let end = start + matched.len();
            let (date_start, date_end) = nearest_date_binding(body, date, start, end)?;
            let distance = if date_end < start {
                start - date_end
            } else {
                date_start.saturating_sub(end)
            };
            Some((distance, start, end))
        })
        .min()
        .map(|(_, start, end)| (start, end))
}

fn compact_percentage_token(value: &str) -> Option<&str> {
    value
        .split_whitespace()
        .map(|token| {
            token.trim_matches(|character: char| {
                matches!(
                    character,
                    '(' | ')' | '[' | ']' | '{' | '}' | ':' | ';' | ','
                )
            })
        })
        .find(|token| {
            (3..=16).contains(&token.chars().count())
                && token.contains('%')
                && token.bytes().any(|byte| byte.is_ascii_digit())
        })
}

fn bind_evidence_with_minimum(
    body: &str,
    excerpt: &str,
    value: &str,
    minimum_characters: usize,
    minimum_words: usize,
) -> Option<BoundEvidence> {
    let excerpt = canonical_evidence_text(excerpt);
    let value = canonical_evidence_text(value);
    if !evidence_input_is_well_formed(&excerpt, &value, minimum_characters, minimum_words) {
        return None;
    }
    let (start, end) = find_bound_value(body, &value)?;
    Some(BoundEvidence {
        context: evidence_context(body, start, end),
        value: body[start..end].to_owned(),
        value_end: end,
        value_start: start,
    })
}

fn evidence_input_is_well_formed(
    excerpt: &str,
    value: &str,
    minimum_characters: usize,
    minimum_words: usize,
) -> bool {
    evidence_shape_is_well_formed(excerpt, value, minimum_characters, minimum_words)
        && excerpt
            .to_ascii_lowercase()
            .contains(&value.to_ascii_lowercase())
}

fn evidence_shape_is_well_formed(
    excerpt: &str,
    value: &str,
    minimum_characters: usize,
    minimum_words: usize,
) -> bool {
    excerpt.chars().count() >= minimum_characters
        && excerpt.split_whitespace().count() >= minimum_words
        && value.chars().count() >= MINIMUM_VALUE_CHARACTERS
}
