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

    pub(crate) fn is_site_root(&self) -> bool {
        Url::parse(self.as_str()).is_ok_and(|parsed| {
            let path = parsed.path().trim_matches('/');
            if path.is_empty() {
                return true;
            }
            let leaf = path.rsplit('/').next().unwrap_or(path);
            let stem = leaf.split('.').next().unwrap_or(leaf);
            SiteLandingName::parse(stem).is_some()
        })
    }

    pub(crate) fn scheme(&self) -> Option<HttpScheme> {
        Url::parse(self.as_str())
            .ok()
            .and_then(|parsed| HttpScheme::parse(parsed.scheme()))
    }

    pub(crate) fn source_kind(&self) -> SourceUrlKind {
        Url::parse(self.as_str()).map_or(SourceUrlKind::Direct, |parsed| {
            KnownSourceHost::parse(parsed.host_str()).source_kind(parsed.path())
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KnownSourceHost {
    VertexAiSearch,
    GoogleSearch,
    GoogleNews,
    GoogleShortener,
    NewsPortal,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SiteLandingName {
    Default,
    Home,
    Index,
    Main,
}

impl SiteLandingName {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "default" => Some(Self::Default),
            "home" => Some(Self::Home),
            "index" => Some(Self::Index),
            "main" => Some(Self::Main),
            _ => None,
        }
    }
}

impl KnownSourceHost {
    fn parse(host: Option<&str>) -> Self {
        const VERTEX_AI_SEARCH: &str = "vertexaisearch.cloud.google.com";
        const GOOGLE_SHORTENERS: [&str; 2] = ["g.co", "goo.gl"];
        const GOOGLE_TRANSPORT_LABELS: [&str; 5] = [
            "google",
            "googleadservices",
            "googleapis",
            "googleusercontent",
            "gstatic",
        ];
        const NEWS_PORTALS: [&str; 5] = [
            "v.daum.net",
            "news.daum.net",
            "n.news.naver.com",
            "news.naver.com",
            "news.nate.com",
        ];

        let Some(host) = host.map(|value| value.trim_end_matches('.')) else {
            return Self::Other;
        };
        if NEWS_PORTALS
            .iter()
            .any(|candidate| host.eq_ignore_ascii_case(candidate))
        {
            Self::NewsPortal
        } else if host.eq_ignore_ascii_case(VERTEX_AI_SEARCH) {
            Self::VertexAiSearch
        } else if GOOGLE_SHORTENERS
            .iter()
            .any(|candidate| host.eq_ignore_ascii_case(candidate))
        {
            Self::GoogleShortener
        } else {
            let mut labels = host.split('.');
            let first = labels.next();
            let second = labels.next();
            let is_google_transport = first.into_iter().chain(second).chain(labels).any(|label| {
                GOOGLE_TRANSPORT_LABELS
                    .iter()
                    .any(|candidate| label.eq_ignore_ascii_case(candidate))
            });
            if !is_google_transport {
                Self::Other
            } else if first.is_some_and(|label| label.eq_ignore_ascii_case("news"))
                && second.is_some_and(|label| label.eq_ignore_ascii_case("google"))
            {
                Self::GoogleNews
            } else {
                Self::GoogleSearch
            }
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
            Self::GoogleShortener => SourceUrlKind::GroundingRedirect,
            Self::NewsPortal => SourceUrlKind::NonSource,
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
            "https://www.google.co.kr/search?q=korean+market",
            "https://news.google.co.uk/articles/example",
            "https://googleusercontent.com/cached-source",
            "https://g.co/example",
        ] {
            let url = HttpUrl::parse(value).expect("Google transport URL must parse");
            assert_ne!(url.source_kind(), SourceUrlKind::Direct, "URL: {value}");
        }
    }

    #[test]
    fn known_news_portal_syndication_urls_are_never_direct_sources() {
        for value in [
            "https://v.daum.net/v/20260807120301584",
            "https://news.daum.net/example",
            "https://n.news.naver.com/article/001/0010000000",
            "https://news.naver.com/example",
            "https://news.nate.com/view/20260807n00001",
        ] {
            let url = HttpUrl::parse(value).expect("news portal URL must parse");
            assert_eq!(url.source_kind(), SourceUrlKind::NonSource, "URL: {value}");
        }
    }

    #[test]
    fn generic_site_landing_paths_are_not_search_evidence_pages() {
        for value in [
            "https://example.com/",
            "https://example.com/main/main.jsp",
            "https://example.com/report/index.html",
            "https://example.com/home.php?locale=ko",
        ] {
            let url = HttpUrl::parse(value).expect("site landing URL must parse");
            assert!(url.is_site_root(), "URL: {value}");
        }
        let article = HttpUrl::parse("https://example.com/news/article.html?id=7")
            .expect("article URL must parse");
        assert!(!article.is_site_root());
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
