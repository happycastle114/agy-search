//! Closed URL classifications for publisher and landing-page handling.

use super::SourceUrlKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum KnownSourceHost {
    VertexAiSearch,
    GoogleSearch,
    GoogleNews,
    GoogleAsset,
    GoogleShortener,
    NewsPortal,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SiteLandingName {
    Archive,
    Archives,
    Contents,
    Default,
    Headlines,
    Home,
    Index,
    Listing,
    Latest,
    Main,
    News,
    NewsRoom,
    Newsroom,
    Releases,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StructuredCollectionName {
    List,
}

impl StructuredCollectionName {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "list" => Some(Self::List),
            _ => None,
        }
    }
}

impl SiteLandingName {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "archive" => Some(Self::Archive),
            "archives" => Some(Self::Archives),
            "contents" => Some(Self::Contents),
            "default" => Some(Self::Default),
            "headlines" => Some(Self::Headlines),
            "home" => Some(Self::Home),
            "index" => Some(Self::Index),
            "listing" => Some(Self::Listing),
            "latest" => Some(Self::Latest),
            "main" => Some(Self::Main),
            "news" => Some(Self::News),
            "news-room" => Some(Self::NewsRoom),
            "newsroom" => Some(Self::Newsroom),
            "releases" => Some(Self::Releases),
            _ => None,
        }
    }
}

impl KnownSourceHost {
    pub(super) fn parse(host: Option<&str>) -> Self {
        const VERTEX_AI_SEARCH: &str = "vertexaisearch.cloud.google.com";
        const GOOGLE_SHORTENERS: [&str; 2] = ["g.co", "goo.gl"];
        const GOOGLE_ASSET_HOSTS: [&str; 4] = [
            "googleadservices.com",
            "googleapis.com",
            "googleusercontent.com",
            "gstatic.com",
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
        } else if GOOGLE_ASSET_HOSTS.iter().any(|candidate| {
            host.eq_ignore_ascii_case(candidate)
                || host
                    .strip_suffix(candidate)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        }) {
            Self::GoogleAsset
        } else {
            Self::parse_google_host(host)
        }
    }

    fn parse_google_host(host: &str) -> Self {
        let mut labels = host.split('.');
        let first = labels.next();
        let second = labels.next();
        if first.is_some_and(|label| label.eq_ignore_ascii_case("news"))
            && second.is_some_and(|label| label.eq_ignore_ascii_case("google"))
        {
            Self::GoogleNews
        } else if first.is_some_and(|label| {
            label.eq_ignore_ascii_case("google") || label.eq_ignore_ascii_case("www")
        }) && (first.is_some_and(|label| label.eq_ignore_ascii_case("google"))
            || second.is_some_and(|label| label.eq_ignore_ascii_case("google")))
        {
            Self::GoogleSearch
        } else {
            Self::Other
        }
    }

    pub(super) fn source_kind(self, path: &str) -> SourceUrlKind {
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
            Self::NewsPortal | Self::Other => SourceUrlKind::Direct,
            Self::VertexAiSearch | Self::GoogleSearch | Self::GoogleNews | Self::GoogleAsset => {
                SourceUrlKind::NonSource
            }
        }
    }
}
