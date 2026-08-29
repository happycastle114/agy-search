use super::{MAXIMUM_PROJECTED_CONTEXT_CHARACTERS, MINIMUM_EXCERPT_CHARACTERS};

pub(super) fn find_ascii_case_insensitive(body: &str, value: &str) -> Option<(usize, usize)> {
    let start = body
        .to_ascii_lowercase()
        .find(&value.to_ascii_lowercase())?;
    Some((start, start + value.len()))
}

pub(super) fn evidence_context(body: &str, value_start: usize, value_end: usize) -> String {
    let sentence_start = previous_sentence_end(&body[..value_start]).unwrap_or(0);
    let sentence_end =
        next_sentence_end(&body[value_end..]).map_or(body.len(), |index| value_end + index);
    let sentence = body[sentence_start..sentence_end].trim();
    if sentence.chars().count() <= MAXIMUM_PROJECTED_CONTEXT_CHARACTERS
        && sentence.chars().count() >= MINIMUM_EXCERPT_CHARACTERS
    {
        return sentence.to_owned();
    }

    let mut start = value_start.saturating_sub(MAXIMUM_PROJECTED_CONTEXT_CHARACTERS / 2);
    while !body.is_char_boundary(start) {
        start += 1;
    }
    let mut end = (value_end + MAXIMUM_PROJECTED_CONTEXT_CHARACTERS / 2).min(body.len());
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    body[start..end].trim().to_owned()
}

pub(super) fn forward_evidence_context(body: &str, start: usize, required_end: usize) -> String {
    let available = body.get(start..).unwrap_or_default();
    let bounded_end = available
        .char_indices()
        .nth(MAXIMUM_PROJECTED_CONTEXT_CHARACTERS)
        .map_or(body.len(), |(offset, _)| start + offset);
    let mut end = required_end.max(bounded_end.min(body.len()));
    while !body.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    if end < body.len()
        && let Some((offset, _)) = body[start..end]
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
    {
        end = start + offset;
    }
    body[start..end].trim().to_owned()
}

fn previous_sentence_end(text: &str) -> Option<usize> {
    text.char_indices().rev().find_map(|(index, character)| {
        let after = index + character.len_utf8();
        is_sentence_end(text, after, character).then_some(after)
    })
}

fn next_sentence_end(text: &str) -> Option<usize> {
    text.char_indices().find_map(|(index, character)| {
        let after = index + character.len_utf8();
        is_sentence_end(text, after, character).then_some(after)
    })
}

fn is_sentence_end(text: &str, after: usize, character: char) -> bool {
    ['.', '?', '!'].contains(&character)
        && text[after..].chars().next().is_none_or(char::is_whitespace)
}

pub(super) fn canonical_evidence_text(value: &str) -> String {
    let normalized = crate::source_document_html::normalize_text(value);
    let mut canonical = String::with_capacity(normalized.len());
    for character in normalized.chars() {
        if matches!(character, ':' | ';' | ',' | '.' | ')' | '!' | '?') && canonical.ends_with(' ')
        {
            canonical.pop();
        }
        canonical.push(character);
    }
    canonical
}
