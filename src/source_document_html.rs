//! Conservative structural HTML extraction for source panels and release rows.

#[path = "source_document_html/attribute_selection.rs"]
mod attribute_selection;
#[path = "source_document_html/element_name.rs"]
mod element_name;
#[path = "source_document_html/text_normalization.rs"]
mod text_normalization;
#[path = "source_document_html/tokenizer.rs"]
mod tokenizer;

use thiserror::Error;

pub(crate) use attribute_selection::{AttributedElement, elements_with_attribute};
pub(crate) use text_normalization::normalize_text;

#[derive(Debug, Error)]
#[error("source HTML structure was invalid")]
pub(crate) struct HtmlStructureError;
