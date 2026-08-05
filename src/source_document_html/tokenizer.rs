use super::HtmlStructureError;

#[path = "tokenizer/raw_text.rs"]
mod raw_text;

pub(super) use raw_text::{next_markup, raw_text_end};

#[derive(Debug)]
pub(super) enum Token<'a> {
    Ignored { end: usize },
    Opening(OpeningToken<'a>),
    Closing(ClosingToken<'a>),
}

impl Token<'_> {
    pub(super) const fn end(&self) -> usize {
        match self {
            Self::Ignored { end } => *end,
            Self::Opening(opening) => opening.end,
            Self::Closing(closing) => closing.end,
        }
    }
}

#[derive(Debug)]
pub(super) struct OpeningToken<'a> {
    pub(super) name: &'a str,
    pub(super) value: AttributeState,
    pub(super) self_closing: bool,
    pub(super) end: usize,
}

#[derive(Debug)]
pub(super) struct ClosingToken<'a> {
    pub(super) name: &'a str,
    pub(super) start: usize,
    end: usize,
}

#[derive(Debug)]
pub(super) enum AttributeState {
    Missing,
    Present(Option<String>),
}

pub(super) fn parse_token<'a>(
    html: &'a str,
    start: usize,
    wanted: &str,
) -> Result<Token<'a>, HtmlStructureError> {
    let tail = html.get(start..).ok_or(HtmlStructureError)?;
    if !tail.starts_with('<') {
        return Err(HtmlStructureError);
    }
    if tail.starts_with("<!--") {
        let end = tail
            .find("-->")
            .map(|offset| start + offset + 3)
            .ok_or(HtmlStructureError)?;
        return Ok(Token::Ignored { end });
    }
    let end = tag_end(html, start)?;
    let content = html.get(start + 1..end - 1).ok_or(HtmlStructureError)?;
    if content.starts_with(['!', '?']) {
        return Ok(Token::Ignored { end });
    }
    if let Some(closing) = content.strip_prefix('/') {
        let name = closing_name(closing)?;
        return Ok(Token::Closing(ClosingToken { name, start, end }));
    }
    let (name, value, self_closing) = opening_parts(content, wanted)?;
    Ok(Token::Opening(OpeningToken {
        name,
        value,
        self_closing,
        end,
    }))
}

fn tag_end(html: &str, start: usize) -> Result<usize, HtmlStructureError> {
    let bytes = html.as_bytes();
    let mut cursor = start.checked_add(1).ok_or(HtmlStructureError)?;
    let mut quote = None;
    while let Some(byte) = bytes.get(cursor).copied() {
        match quote {
            Some(expected) if expected == byte => quote = None,
            Some(_) => {}
            None => match byte {
                b'\'' | b'\"' => quote = Some(byte),
                b'>' => return cursor.checked_add(1).ok_or(HtmlStructureError),
                b'<' => return Err(HtmlStructureError),
                _ => {}
            },
        }
        cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
    }
    Err(HtmlStructureError)
}

fn closing_name(content: &str) -> Result<&str, HtmlStructureError> {
    let name_end = name_end(content)?;
    if !content
        .get(name_end..)
        .ok_or(HtmlStructureError)?
        .trim()
        .is_empty()
    {
        return Err(HtmlStructureError);
    }
    content.get(..name_end).ok_or(HtmlStructureError)
}

fn opening_parts<'a>(
    content: &'a str,
    wanted: &str,
) -> Result<(&'a str, AttributeState, bool), HtmlStructureError> {
    let name_end = name_end(content)?;
    let name = content.get(..name_end).ok_or(HtmlStructureError)?;
    let mut cursor = name_end;
    let mut wanted_value = AttributeState::Missing;
    let bytes = content.as_bytes();
    loop {
        skip_space(bytes, &mut cursor);
        if cursor == bytes.len() {
            return Ok((name, wanted_value, false));
        }
        if bytes.get(cursor) == Some(&b'/') {
            cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
            skip_space(bytes, &mut cursor);
            return (cursor == bytes.len())
                .then_some((name, wanted_value, true))
                .ok_or(HtmlStructureError);
        }
        let attribute_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| is_attribute_byte(*byte))
        {
            cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
        }
        if cursor == attribute_start {
            return Err(HtmlStructureError);
        }
        let attribute = content
            .get(attribute_start..cursor)
            .ok_or(HtmlStructureError)?;
        skip_space(bytes, &mut cursor);
        let value = if bytes.get(cursor) == Some(&b'=') {
            cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
            skip_space(bytes, &mut cursor);
            Some(attribute_value(content, &mut cursor)?)
        } else {
            None
        };
        if attribute == wanted {
            if value.as_deref().is_some_and(str::is_empty) {
                return Err(HtmlStructureError);
            }
            if matches!(wanted_value, AttributeState::Present(_)) {
                return Err(HtmlStructureError);
            }
            wanted_value = AttributeState::Present(value);
        }
    }
}

fn attribute_value(content: &str, cursor: &mut usize) -> Result<String, HtmlStructureError> {
    let bytes = content.as_bytes();
    let first = bytes.get(*cursor).copied().ok_or(HtmlStructureError)?;
    let start = match first {
        b'\'' | b'\"' => {
            *cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
            let start = *cursor;
            while bytes.get(*cursor).is_some_and(|byte| *byte != first) {
                *cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
            }
            if bytes.get(*cursor) != Some(&first) {
                return Err(HtmlStructureError);
            }
            let end = *cursor;
            *cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
            return Ok(content
                .get(start..end)
                .ok_or(HtmlStructureError)?
                .to_owned());
        }
        _ => *cursor,
    };
    while bytes
        .get(*cursor)
        .is_some_and(|byte| is_attribute_byte(*byte))
    {
        *cursor = cursor.checked_add(1).ok_or(HtmlStructureError)?;
    }
    Ok(content
        .get(start..*cursor)
        .ok_or(HtmlStructureError)?
        .to_owned())
}

fn name_end(value: &str) -> Result<usize, HtmlStructureError> {
    let bytes = value.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphabetic) {
        return Err(HtmlStructureError);
    }
    let mut end = 1;
    while bytes.get(end).is_some_and(|byte| is_name_byte(*byte)) {
        end = end.checked_add(1).ok_or(HtmlStructureError)?;
    }
    Ok(end)
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor = cursor.saturating_add(1);
    }
}

const fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

const fn is_attribute_byte(byte: u8) -> bool {
    !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'=' | b'\'' | b'\"' | b'<' | b'>')
}
