//! Heading-delimited fallback sections for small non-panel sources.

use crate::source_document_html::{HtmlStructureError, normalize_text};

pub(crate) fn document_title(html: &str) -> Result<Option<String>, HtmlStructureError> {
    let lowercase = html.to_ascii_lowercase();
    let Some(open_at) = lowercase.find("<title") else {
        return Ok(None);
    };
    let open_end = lowercase
        .get(open_at..)
        .and_then(|tail| tail.find('>'))
        .map(|offset| open_at + offset)
        .ok_or(HtmlStructureError)?;
    let content_start = open_end + 1;
    let content_end = lowercase
        .get(content_start..)
        .and_then(|tail| tail.find("</title"))
        .map(|offset| content_start + offset)
        .ok_or(HtmlStructureError)?;
    let title = normalize_text(
        html.get(content_start..content_end)
            .ok_or(HtmlStructureError)?,
    );
    Ok((!title.is_empty()).then_some(title))
}

pub(crate) fn heading_sections(html: &str) -> Result<Vec<(String, String)>, HtmlStructureError> {
    let mut headings = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = html.get(cursor..).and_then(|tail| tail.find('<')) {
        let open_at = cursor + relative;
        let open_end = html
            .get(open_at..)
            .and_then(|tail| tail.find('>'))
            .map(|offset| open_at + offset)
            .ok_or(HtmlStructureError)?;
        let opening = html.get(open_at + 1..open_end).ok_or(HtmlStructureError)?;
        let tag = opening
            .split_ascii_whitespace()
            .next()
            .map(|tag| tag.trim_end_matches('/'))
            .unwrap_or_default();
        cursor = open_end + 1;
        if !is_heading(tag) {
            continue;
        }
        let (content_end, element_end) = matching_close(html, tag, open_end + 1)?;
        let label = html
            .get(open_end + 1..content_end)
            .ok_or(HtmlStructureError)?
            .to_owned();
        headings.push((open_at, element_end, label));
        cursor = element_end;
    }
    let mut sections = Vec::with_capacity(headings.len());
    for (index, (_, body_start, label)) in headings.iter().enumerate() {
        let body_end = headings
            .get(index + 1)
            .map_or(html.len(), |(open_at, _, _)| *open_at);
        let body = html.get(*body_start..body_end).ok_or(HtmlStructureError)?;
        sections.push((normalize_text(label), normalize_text(body)));
    }
    Ok(sections)
}

fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn matching_close(
    html: &str,
    tag: &str,
    content_start: usize,
) -> Result<(usize, usize), HtmlStructureError> {
    let close = format!("</{tag}");
    let close_at = html
        .get(content_start..)
        .and_then(|tail| tail.find(&close))
        .map(|offset| content_start + offset)
        .ok_or(HtmlStructureError)?;
    let close_end = html
        .get(close_at..)
        .and_then(|tail| tail.find('>'))
        .map(|offset| close_at + offset + 1)
        .ok_or(HtmlStructureError)?;
    Ok((close_at, close_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_a_case_insensitive_document_title() {
        let html = "<html><head><TITLE> Release 1.0 &amp; notes | Example </TITLE></head></html>";

        assert_eq!(
            document_title(html)
                .expect("title HTML must parse")
                .as_deref(),
            Some("Release 1.0 & notes | Example")
        );
    }
}
