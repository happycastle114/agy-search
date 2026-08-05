//! Validated HTTP(S) source URLs and transport classification.

use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use url::Url;

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct HttpUrl(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceUrlKind {
    Direct,
    GroundingRedirect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HttpScheme {
    Http,
    Https,
}

impl HttpScheme {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            _ => None,
        }
    }
}

impl HttpUrl {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let normalized = value.trim();
        let mut parsed = Url::parse(normalized).map_err(|_| "URL must use HTTP(S)")?;
        HttpScheme::parse(parsed.scheme()).ok_or("URL must use HTTP(S)")?;
        let explicit_authority = normalized
            .find("://")
            .is_some_and(|index| index == parsed.scheme().len());
        if !explicit_authority || parsed.host_str().is_none() {
            return Err("URL must use HTTP(S)");
        }
        parsed.set_fragment(None);
        Ok(Self(parsed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn same_origin(&self, other: &Self) -> bool {
        let left = Url::parse(self.as_str());
        let right = Url::parse(other.as_str());
        matches!((left, right), (Ok(left), Ok(right)) if left.origin() == right.origin())
    }

    pub(crate) fn scheme(&self) -> Option<HttpScheme> {
        Url::parse(self.as_str())
            .ok()
            .and_then(|parsed| HttpScheme::parse(parsed.scheme()))
    }

    pub(crate) fn source_kind(&self) -> SourceUrlKind {
        const GROUNDING_HOST: &str = "vertexaisearch.cloud.google.com";
        const GROUNDING_PATH_PREFIX: &str = "/grounding-api-redirect/";

        match Url::parse(self.as_str()) {
            Ok(parsed)
                if parsed.host_str() == Some(GROUNDING_HOST)
                    && parsed.path().starts_with(GROUNDING_PATH_PREFIX) =>
            {
                SourceUrlKind::GroundingRedirect
            }
            Ok(_) | Err(_) => SourceUrlKind::Direct,
        }
    }
}

impl FromStr for HttpUrl {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for HttpUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsed_http_urls_with_one_host_share_an_origin(
            host in "[a-z]{1,12}",
            left_path in "[a-z0-9/]{0,20}",
            right_path in "[a-z0-9/]{0,20}",
        ) {
            let left = HttpUrl::parse(&format!("https://{host}.example/{left_path}"));
            let right = HttpUrl::parse(&format!("https://{host}.example/{right_path}"));
            prop_assert!(matches!((left, right), (Ok(left), Ok(right)) if left.same_origin(&right)));
        }
    }
}
