use super::{EvidenceConnector, EvidenceToken};
use crate::research_source_evidence::{
    MAXIMUM_NORMALIZED_VALUE_SPAN_CHARACTERS, MINIMUM_FALLBACK_VALUE_CHARACTERS,
    MINIMUM_FALLBACK_VALUE_WORDS, text::find_ascii_case_insensitive,
};

pub(super) fn find_bound_value(body: &str, value: &str) -> Option<(usize, usize)> {
    if let Some(range) = find_ascii_case_insensitive(body, value) {
        return Some(range);
    }
    let words = value.split_whitespace().collect::<Vec<_>>();
    for width in (MINIMUM_FALLBACK_VALUE_WORDS..=words.len()).rev() {
        for window in words.windows(width) {
            let phrase = window.join(" ");
            for candidate in [phrase.as_str(), trim_outer_punctuation(&phrase)] {
                if candidate.chars().count() >= MINIMUM_FALLBACK_VALUE_CHARACTERS
                    && let Some(range) = find_ascii_case_insensitive(body, candidate)
                {
                    return Some(range);
                }
            }
        }
    }
    find_normalized_token_window(body, value)
}

fn find_normalized_token_window(body: &str, value: &str) -> Option<(usize, usize)> {
    let body_tokens = evidence_tokens(body);
    let value_tokens = evidence_tokens(value)
        .into_iter()
        .map(|token| token.normalized)
        .collect::<Vec<_>>();
    for width in (MINIMUM_FALLBACK_VALUE_WORDS..=value_tokens.len()).rev() {
        for value_window in value_tokens.windows(width) {
            for body_window in body_tokens.windows(width) {
                if value_window
                    .iter()
                    .zip(body_window)
                    .all(|(expected, actual)| expected == &actual.normalized)
                {
                    let start = body_window.first()?.start;
                    let end = body_window.last()?.end;
                    let span_characters = body[start..end].chars().count();
                    if (MINIMUM_FALLBACK_VALUE_CHARACTERS
                        ..=MAXIMUM_NORMALIZED_VALUE_SPAN_CHARACTERS)
                        .contains(&span_characters)
                    {
                        return Some((start, end));
                    }
                }
            }
        }
    }
    None
}

fn evidence_tokens(value: &str) -> Vec<EvidenceToken> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    for (index, character) in value.char_indices() {
        if character.is_alphanumeric() {
            token_start.get_or_insert(index);
            continue;
        }
        if let Some(start) = token_start.take() {
            push_evidence_token(&mut tokens, value, start, index);
        }
    }
    if let Some(start) = token_start {
        push_evidence_token(&mut tokens, value, start, value.len());
    }
    tokens
}

fn push_evidence_token(tokens: &mut Vec<EvidenceToken>, value: &str, start: usize, end: usize) {
    let normalized = value[start..end].to_lowercase();
    if EvidenceConnector::parse(&normalized).is_none() {
        tokens.push(EvidenceToken {
            normalized,
            start,
            end,
        });
    }
}

fn trim_outer_punctuation(value: &str) -> &str {
    value.trim_matches(|character: char| {
        matches!(
            character,
            '(' | ')' | '[' | ']' | '{' | '}' | ':' | ';' | ',' | '.'
        )
    })
}
