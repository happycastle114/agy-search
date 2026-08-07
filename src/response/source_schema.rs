//! JSON Schema narrowing for caller-owned Search and Research sources.

use serde_json::{Value, json};

use crate::{error::AgyError, source_restriction::SourceRestriction};

const GROUNDING_TRANSPORT_PATTERN: &str =
    r"^https://vertexaisearch\.cloud\.google\.com/grounding-api-redirect/[^\s]+$";

pub(super) fn require_grounding_transport_urls(schema: &mut Value) -> Result<(), AgyError> {
    let definition = schema
        .pointer_mut("/$defs/HttpUrl")
        .and_then(Value::as_object_mut)
        .ok_or(AgyError::InvalidCommand)?;
    definition.insert(
        "pattern".to_owned(),
        Value::String(GROUNDING_TRANSPORT_PATTERN.to_owned()),
    );
    definition.insert(
        "description".to_owned(),
        Value::String(
            "Exact Google grounding transport URL copied from the completed search_web result; the caller resolves it to the terminal publisher URL."
                .to_owned(),
        ),
    );
    Ok(())
}

pub(super) fn narrow_source_urls(
    schema: &mut Value,
    restriction: &SourceRestriction,
) -> Result<(), AgyError> {
    let SourceRestriction::Allowlist { domains, urls } = restriction else {
        return Ok(());
    };
    let mut alternatives = Vec::new();
    if !urls.is_empty() {
        alternatives.push(json!({
            "enum": urls.iter().map(crate::types::HttpUrl::as_str).collect::<Vec<_>>()
        }));
    }
    alternatives.extend(domains.iter().map(|domain| {
        let escaped = domain.as_str().replace('.', r"\.");
        json!({
            "pattern": format!(r"^https?://(?:[A-Za-z0-9-]+\.)*{escaped}(?::[0-9]+)?(?:[/?#]|$)")
        })
    }));
    let mut narrowed = 0;
    for pointer in [
        "/$defs/WebSource/properties/url",
        "/$defs/ScopeEvidence/properties/url",
        "/$defs/ResearchFinding/properties/citations/items",
    ] {
        let Some(definition) = schema.pointer_mut(pointer).and_then(Value::as_object_mut) else {
            continue;
        };
        definition.insert("anyOf".to_owned(), Value::Array(alternatives.clone()));
        definition.remove("format");
        definition.remove("pattern");
        narrowed += 1;
    }
    if narrowed >= 2 {
        Ok(())
    } else {
        Err(AgyError::InvalidCommand)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;
    use crate::{source_restriction::SourceDomain, types::HttpUrl};

    #[test]
    fn allowlist_schema_contains_canonical_domain_and_exact_url_alternatives() {
        let restriction = SourceRestriction::parse(
            vec![SourceDomain::from_str("RUST-LANG.ORG.").expect("valid test domain")],
            vec![HttpUrl::parse("https://doc.rust-lang.org/book/").expect("valid test URL")],
        )
        .expect("valid test restriction");
        let mut schema = json!({"$defs":{
            "WebSource":{"properties":{"url":{"type":"string"}}},
            "ScopeEvidence":{"properties":{"url":{"type":"string"}}}
        }});

        narrow_source_urls(&mut schema, &restriction).expect("schema must narrow");

        let alternatives = schema
            .pointer("/$defs/WebSource/properties/url/anyOf")
            .and_then(Value::as_array)
            .expect("alternatives must exist");
        assert_eq!(
            alternatives
                .first()
                .and_then(|value| value.pointer("/enum/0")),
            Some(&json!("https://doc.rust-lang.org/book/"))
        );
        assert!(
            alternatives
                .iter()
                .filter_map(|value| value.get("pattern").and_then(Value::as_str))
                .any(|pattern| pattern.contains("rust-lang\\.org"))
        );
    }
}
