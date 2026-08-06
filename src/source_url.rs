//! Validated HTTP(S) source URLs and transport classification.

use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use url::Url;

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct HttpUrl(
    #[schemars(regex(pattern = r"^https?://[^\s]+$"))]
    String,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceUrlKind {
    Direct,
    GroundingRedirect,
    NonSource,
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
        Url::parse(self.as_str()).map_or(SourceUrlKind::Direct, |parsed| {
            KnownGoogleHost::parse(parsed.host_str()).source_kind(parsed.path())
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KnownGoogleHost {
    VertexAiSearch,
    GoogleSearch,
    GoogleNews,
    Other,
}

impl KnownGoogleHost {
    fn parse(host: Option<&str>) -> Self {
        const VERTEX_AI_SEARCH: &str = "vertexaisearch.cloud.google.com";
        const GOOGLE_SEARCH: [&str; 2] = ["google.com", "www.google.com"];
        const GOOGLE_NEWS: &str = "news.google.com";

        let Some(host) = host.map(|value| value.trim_end_matches('.')) else {
            return Self::Other;
        };
        if host.eq_ignore_ascii_case(VERTEX_AI_SEARCH) {
            Self::VertexAiSearch
        } else if GOOGLE_SEARCH
            .iter()
            .any(|candidate| host.eq_ignore_ascii_case(candidate))
        {
            Self::GoogleSearch
        } else if host.eq_ignore_ascii_case(GOOGLE_NEWS) {
            Self::GoogleNews
        } else {
            Self::Other
        }
    }

    fn source_kind(self, path: &str) -> SourceUrlKind {
        const GROUNDING_PATH: &str = "/grounding-api-redirect/";
        const GOOGLE_REDIRECT_PATH: &str = "/url";
        const GOOGLE_NEWS_REDIRECT_PATHS: [&str; 3] = ["/articles/", "/read/", "/rss/articles/"];

        match self {
            Self::VertexAiSearch if path.starts_with(GROUNDING_PATH) => {
                SourceUrlKind::GroundingRedirect
            }
            Self::GoogleSearch if path == GOOGLE_REDIRECT_PATH => SourceUrlKind::GroundingRedirect,
            Self::GoogleNews
                if GOOGLE_NEWS_REDIRECT_PATHS
                    .iter()
                    .any(|prefix| path.starts_with(prefix)) =>
            {
                SourceUrlKind::GroundingRedirect
            }
            Self::VertexAiSearch | Self::GoogleSearch | Self::GoogleNews => {
                SourceUrlKind::NonSource
            }
            Self::Other => SourceUrlKind::Direct,
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

    #[test]
    fn known_google_transport_and_wrapper_urls_are_never_direct() {
        for value in [
            "https://vertexaisearch.cloud.google.com./grounding-api-redirect/token",
            "https://vertexaisearch.cloud.google.com/another-transport-path",
            "https://www.google.com/url?q=https%3A%2F%2Fexample.com",
            "https://google.com/url?url=https%3A%2F%2Fexample.com",
            "https://news.google.com/articles/example",
        ] {
            let url = HttpUrl::parse(value).expect("Google transport URL must parse");
            assert_ne!(url.source_kind(), SourceUrlKind::Direct, "URL: {value}");
        }
    }

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
