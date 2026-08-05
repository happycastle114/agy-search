use super::super::element_name::same_name;
use super::{HtmlStructureError, Token, parse_token};

pub(crate) fn raw_text_end(
    html: &str,
    content_start: usize,
    name: &str,
) -> Result<usize, HtmlStructureError> {
    let mut cursor = content_start;
    while let Some(relative) = html.get(cursor..).and_then(|tail| tail.find("</")) {
        let close_at = cursor + relative;
        let name_start = close_at.checked_add(2).ok_or(HtmlStructureError)?;
        let candidate = html
            .get(name_start..name_start.saturating_add(name.len()))
            .ok_or(HtmlStructureError)?;
        let boundary = html
            .as_bytes()
            .get(name_start.saturating_add(name.len()))
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'>');
        if same_name(candidate, name) && boundary {
            return match parse_token(html, close_at, "")? {
                Token::Closing(closing) if same_name(name, closing.name) => Ok(closing.end),
                Token::Ignored { .. } | Token::Opening(_) | Token::Closing(_) => {
                    Err(HtmlStructureError)
                }
            };
        }
        cursor = name_start;
    }
    Err(HtmlStructureError)
}

pub(crate) fn next_markup(html: &str, cursor: usize) -> Option<usize> {
    html.get(cursor..)?.find('<').map(|offset| cursor + offset)
}
