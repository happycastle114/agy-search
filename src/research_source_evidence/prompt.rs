use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::{error::AgyError, source_fetch::SafeSourceUrl};

const PROMPT_WINDOW_CHARACTERS: usize = 1_200;
const PROMPT_WINDOW_OVERLAP_CHARACTERS: usize = 300;
const EXTRACT_WINDOW_CHARACTERS: usize = 700;
const EXTRACT_WINDOW_OVERLAP_CHARACTERS: usize = 150;
const MAX_WINDOWS_PER_SOURCE: usize = 10;
const MAX_EXTRACT_WINDOWS: usize = 2;
const MAX_PROMPT_CONTEXT_BYTES: usize = 96 * 1_024;

#[derive(Clone, Copy)]
enum WindowOrdering {
    Relevance,
    Source,
}

#[derive(Serialize)]
struct PromptSource<'a> {
    url: &'a str,
    excerpts: Vec<String>,
}

struct RankedWindow {
    score: usize,
    sequence: usize,
    text: String,
}

pub(super) fn render_prompt_context(
    bodies: &HashMap<SafeSourceUrl, String>,
    query: &str,
) -> Result<String, AgyError> {
    let query_tokens = evidence_tokens(query);
    let mut sources = bodies.iter().collect::<Vec<_>>();
    sources.sort_unstable_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
    let fair_maximum = MAX_PROMPT_CONTEXT_BYTES
        .checked_div(bodies.len().max(1) * PROMPT_WINDOW_CHARACTERS * 2)
        .unwrap_or(1)
        .clamp(1, MAX_WINDOWS_PER_SOURCE);
    for maximum_windows in (1..=fair_maximum).rev() {
        let prompt_sources = sources
            .iter()
            .map(|(url, body)| PromptSource {
                url: url.as_str(),
                excerpts: select_windows(
                    body,
                    &query_tokens,
                    maximum_windows,
                    WindowOrdering::Source,
                    PROMPT_WINDOW_CHARACTERS,
                    PROMPT_WINDOW_OVERLAP_CHARACTERS,
                ),
            })
            .collect::<Vec<_>>();
        let rendered =
            serde_json::to_string(&prompt_sources).map_err(|_| AgyError::OutputInvalid)?;
        if rendered.len() <= MAX_PROMPT_CONTEXT_BYTES {
            return Ok(rendered);
        }
    }
    Err(AgyError::OutputInvalid)
}

pub(super) fn extract_content(body: &str, query: &str) -> Option<String> {
    let windows = select_windows(
        body,
        &evidence_tokens(query),
        MAX_EXTRACT_WINDOWS,
        WindowOrdering::Relevance,
        EXTRACT_WINDOW_CHARACTERS,
        EXTRACT_WINDOW_OVERLAP_CHARACTERS,
    );
    (!windows.is_empty()).then(|| windows.join("\n\n"))
}

fn select_windows(
    body: &str,
    query_tokens: &HashSet<String>,
    maximum_windows: usize,
    ordering: WindowOrdering,
    window_characters: usize,
    window_overlap_characters: usize,
) -> Vec<String> {
    let boundaries = body
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(body.len()))
        .collect::<Vec<_>>();
    if boundaries.len() <= window_characters + 1 {
        return vec![body.to_owned()];
    }

    let stride = window_characters - window_overlap_characters;
    let mut ranked = (0..boundaries.len().saturating_sub(1))
        .step_by(stride)
        .enumerate()
        .filter_map(|(sequence, start_character)| {
            let end_character =
                (start_character + window_characters).min(boundaries.len().saturating_sub(1));
            let start_byte = *boundaries.get(start_character)?;
            let end_byte = *boundaries.get(end_character)?;
            let text = body.get(start_byte..end_byte)?.trim().to_owned();
            let text_tokens = evidence_tokens(&text);
            let score = text_tokens.intersection(query_tokens).count();
            Some(RankedWindow {
                score,
                sequence,
                text,
            })
        })
        .filter(|window| !window.text.is_empty())
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    ranked.truncate(maximum_windows);
    if matches!(ordering, WindowOrdering::Source) {
        ranked.sort_unstable_by_key(|window| window.sequence);
    }
    ranked.into_iter().map(|window| window.text).collect()
}

fn evidence_tokens(value: &str) -> HashSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.chars().count() >= 3)
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_query_relevant_windows_without_rewriting_source_text() {
        let filler = "unrelated navigation ".repeat(100);
        let evidence = "Gemini 3.7 Flash is generally available (GA) and ready for production use.";
        let body = format!("{filler}{evidence}{filler}");

        let selected = select_windows(
            &body,
            &evidence_tokens("Gemini 3.7 generally available"),
            MAX_WINDOWS_PER_SOURCE,
            WindowOrdering::Source,
            PROMPT_WINDOW_CHARACTERS,
            PROMPT_WINDOW_OVERLAP_CHARACTERS,
        );

        assert!(selected.iter().any(|window| window.contains(evidence)));
        assert!(selected.len() <= MAX_WINDOWS_PER_SOURCE);
    }

    #[test]
    fn prompt_context_is_bounded_for_the_maximal_exact_source_set() {
        let body = "Gemini source evidence with exact relevant facts. ".repeat(12_000);
        let bodies = (0..20)
            .map(|index| {
                let url =
                    SafeSourceUrl::parse_redirect(&format!("https://example.com/source-{index}"))
                        .expect("fixture URL must be safe");
                (url, body.clone())
            })
            .collect::<HashMap<_, _>>();

        let rendered = render_prompt_context(&bodies, "Gemini relevant facts")
            .expect("bounded prompt context must render");

        assert!(rendered.len() <= MAX_PROMPT_CONTEXT_BYTES);
        assert_eq!(rendered.matches("https://example.com/source-").count(), 20);
    }
}
