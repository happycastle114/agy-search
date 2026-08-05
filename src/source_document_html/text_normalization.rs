use super::{
    HtmlStructureError,
    element_name::{is_inert, is_raw_text, same_name},
    tokenizer::{Token, next_markup, parse_token, raw_text_end},
};

pub(crate) fn normalize_text(value: &str) -> String {
    let Ok(rendered) = rendered_text(value) else {
        return String::new();
    };
    let decoded = rendered
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn rendered_text(html: &str) -> Result<String, HtmlStructureError> {
    let mut text = String::new();
    let mut inert = Vec::new();
    let mut cursor = 0;
    let mut text_start = 0;
    while let Some(open_at) = next_markup(html, cursor) {
        if inert.is_empty() {
            text.push_str(html.get(text_start..open_at).ok_or(HtmlStructureError)?);
            text.push(' ');
        }
        let token = parse_token(html, open_at, "")?;
        cursor = token.end();
        text_start = cursor;
        match token {
            Token::Opening(opening) if is_raw_text(opening.name) => {
                if opening.self_closing {
                    return Err(HtmlStructureError);
                }
                cursor = raw_text_end(html, opening.end, opening.name)?;
                text_start = cursor;
            }
            Token::Opening(opening) if is_inert(opening.name) => {
                if opening.self_closing {
                    return Err(HtmlStructureError);
                }
                inert.push(opening.name);
            }
            Token::Closing(closing) if is_inert(closing.name) => {
                let expected = inert.pop().ok_or(HtmlStructureError)?;
                if !same_name(expected, closing.name) {
                    return Err(HtmlStructureError);
                }
            }
            Token::Ignored { .. } | Token::Opening(_) | Token::Closing(_) => {}
        }
    }
    if !inert.is_empty() {
        return Err(HtmlStructureError);
    }
    text.push_str(html.get(text_start..).ok_or(HtmlStructureError)?);
    Ok(text)
}
