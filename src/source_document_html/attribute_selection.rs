use super::{
    HtmlStructureError,
    element_name::{is_inert, is_raw_text, is_void, same_name},
    tokenizer::{AttributeState, OpeningToken, Token, next_markup, parse_token, raw_text_end},
};

#[derive(Debug)]
pub(crate) struct AttributedElement {
    pub(crate) value: Option<String>,
    pub(crate) content: String,
}

pub(crate) fn elements_with_attribute(
    html: &str,
    wanted: &str,
) -> Result<Vec<AttributedElement>, HtmlStructureError> {
    let mut selection = Selection::default();
    let mut cursor = 0;
    while let Some(open_at) = next_markup(html, cursor) {
        let token = parse_token(html, open_at, wanted)?;
        cursor = token.end();
        match token {
            Token::Ignored { .. } => {}
            Token::Opening(opening) if is_raw_text(opening.name) => {
                if opening.self_closing {
                    return Err(HtmlStructureError);
                }
                cursor = raw_text_end(html, opening.end, opening.name)?;
            }
            Token::Opening(opening) => selection.open(opening)?,
            Token::Closing(closing) => selection.close(html, closing.name, closing.start)?,
        }
    }
    selection.finish()
}

#[derive(Default)]
struct Selection<'a> {
    captures: Vec<Capture<'a>>,
    elements: Vec<(usize, AttributedElement)>,
    inert: Vec<&'a str>,
    order: usize,
}

impl<'a> Selection<'a> {
    fn open(&mut self, opening: OpeningToken<'a>) -> Result<(), HtmlStructureError> {
        self.increment_depths(opening.name)?;
        let hidden = !self.inert.is_empty() || is_inert(opening.name);
        if is_inert(opening.name) {
            if opening.self_closing {
                return Err(HtmlStructureError);
            }
            self.inert.push(opening.name);
        }
        if let AttributeState::Present(value) = opening.value {
            if hidden || opening.self_closing || is_void(opening.name) {
                return Err(HtmlStructureError);
            }
            self.captures.push(Capture {
                name: opening.name,
                content_start: opening.end,
                depth: 1,
                value,
                order: self.order,
            });
            self.order = self.order.checked_add(1).ok_or(HtmlStructureError)?;
        }
        Ok(())
    }

    fn close(
        &mut self,
        html: &str,
        name: &str,
        content_end: usize,
    ) -> Result<(), HtmlStructureError> {
        let mut index = 0;
        while let Some(capture) = self.captures.get_mut(index) {
            if !same_name(capture.name, name) {
                index += 1;
                continue;
            }
            capture.depth = capture.depth.checked_sub(1).ok_or(HtmlStructureError)?;
            if capture.depth != 0 {
                index += 1;
                continue;
            }
            let capture = self.captures.remove(index);
            self.elements.push((
                capture.order,
                AttributedElement {
                    value: capture.value,
                    content: html
                        .get(capture.content_start..content_end)
                        .ok_or(HtmlStructureError)?
                        .to_owned(),
                },
            ));
        }
        if is_inert(name) {
            let expected = self.inert.pop().ok_or(HtmlStructureError)?;
            if !same_name(expected, name) {
                return Err(HtmlStructureError);
            }
        }
        Ok(())
    }

    fn increment_depths(&mut self, name: &str) -> Result<(), HtmlStructureError> {
        for capture in &mut self.captures {
            if same_name(capture.name, name) {
                capture.depth = capture.depth.checked_add(1).ok_or(HtmlStructureError)?;
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<AttributedElement>, HtmlStructureError> {
        if !self.captures.is_empty() || !self.inert.is_empty() {
            return Err(HtmlStructureError);
        }
        self.elements.sort_by_key(|(order, _)| *order);
        Ok(self
            .elements
            .into_iter()
            .map(|(_, element)| element)
            .collect())
    }
}

#[derive(Debug)]
struct Capture<'a> {
    name: &'a str,
    content_start: usize,
    depth: usize,
    value: Option<String>,
    order: usize,
}
