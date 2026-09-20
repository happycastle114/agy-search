//! Typed caller-owned source boundaries for Search and Research.

use std::{collections::HashSet, net::IpAddr, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use url::{Host, Url};

use crate::{error::AgyError, types::HttpUrl};

const MAX_RESTRICTIONS: usize = 20;

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct SourceDomain(String);

impl<'de> Deserialize<'de> for SourceDomain {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

impl SourceDomain {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    fn contains_host(&self, host: &str) -> bool {
        host == self.0
            || host
                .strip_suffix(&self.0)
                .is_some_and(|prefix| prefix.ends_with('.'))
    }
}

impl FromStr for SourceDomain {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.trim() != value
            || value.chars().any(char::is_whitespace)
            || value.contains(['/', '?', '#', '@', ':', '*'])
        {
            return Err("domain must be a bare DNS name");
        }
        let without_dot = value.strip_suffix('.').unwrap_or(value);
        if without_dot.is_empty() || without_dot.ends_with('.') {
            return Err("domain must contain at most one trailing dot");
        }
        if without_dot.parse::<IpAddr>().is_ok() {
            return Err("domain must not be an IP address");
        }
        let Host::Domain(canonical) =
            Host::parse(without_dot).map_err(|_| "domain must be a valid DNS name")?
        else {
            return Err("domain must not be an IP address");
        };
        let canonical = canonical.to_ascii_lowercase();
        if canonical.len() > 253
            || canonical.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err("domain must be a valid DNS name");
        }
        Ok(Self(canonical))
    }
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SourceRestriction {
    Unrestricted,
    Allowlist {
        domains: Vec<SourceDomain>,
        urls: Vec<HttpUrl>,
    },
}

impl SourceRestriction {
    pub(crate) fn parse(domains: Vec<SourceDomain>, urls: Vec<HttpUrl>) -> Result<Self, AgyError> {
        if domains.is_empty() && urls.is_empty() {
            return Ok(Self::Unrestricted);
        }
        if domains.len() + urls.len() > MAX_RESTRICTIONS
            || domains.iter().collect::<HashSet<_>>().len() != domains.len()
            || urls.iter().collect::<HashSet<_>>().len() != urls.len()
            || urls.iter().any(url_has_userinfo)
        {
            return Err(AgyError::InvalidCommand);
        }
        Ok(Self::Allowlist { domains, urls })
    }

    pub(crate) const fn is_unrestricted(&self) -> bool {
        matches!(self, Self::Unrestricted)
    }

    pub(crate) fn domains(&self) -> &[SourceDomain] {
        match self {
            Self::Unrestricted => &[],
            Self::Allowlist { domains, urls: _ } => domains,
        }
    }

    pub(crate) fn exact_urls(&self) -> &[HttpUrl] {
        match self {
            Self::Unrestricted => &[],
            Self::Allowlist { domains: _, urls } => urls,
        }
    }

    pub(crate) const fn is_exact_only(&self) -> bool {
        match self {
            Self::Unrestricted => false,
            Self::Allowlist { domains, urls } => domains.is_empty() && !urls.is_empty(),
        }
    }

    pub(crate) fn allows(&self, source: &HttpUrl) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Allowlist { domains, urls } => {
                urls.contains(source)
                    || Url::parse(source.as_str())
                        .ok()
                        .and_then(|url| url.host_str().map(str::to_owned))
                        .is_some_and(|host| {
                            domains.iter().any(|domain| domain.contains_host(&host))
                        })
            }
        }
    }

    pub(crate) fn allows_evidence_url(&self, source: &HttpUrl) -> bool {
        self.allows(source)
            && (!source.is_site_root()
                || matches!(
                    self,
                    Self::Allowlist { domains: _, urls } if urls.contains(source)
                ))
    }

    pub(crate) fn allows_search_query(&self, query: &str) -> bool {
        let domains = self.domains();
        if domains.is_empty() {
            return true;
        }
        let expected = domains
            .iter()
            .map(|domain| format!("site:{}", domain.as_str()))
            .collect::<Vec<_>>();
        let actual = query
            .split_ascii_whitespace()
            .filter(|token| token.starts_with("site:"))
            .collect::<Vec<_>>();
        let tokens = query.split_ascii_whitespace().collect::<Vec<_>>();
        actual.len() == expected.len()
            && actual
                .iter()
                .zip(&expected)
                .all(|(actual, expected)| *actual == expected)
            && tokens.windows(expected.len()).any(|window| {
                window
                    .iter()
                    .zip(&expected)
                    .all(|(actual, expected)| *actual == expected)
            })
    }
}

fn url_has_userinfo(source: &HttpUrl) -> bool {
    Url::parse(source.as_str())
        .is_ok_and(|url| !url.username().is_empty() || url.password().is_some())
}

#[cfg(test)]
#[path = "source_restriction/source_restriction_test.rs"]
mod source_restriction_test;
