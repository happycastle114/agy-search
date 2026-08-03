//! Validated scalar types and closed downstream variants.

use std::{fmt, str::FromStr, time::Duration};

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use url::Url;

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct NonEmptyText(String);

impl NonEmptyText {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err("value must be non-empty");
        }
        Ok(Self(normalized.to_owned()))
    }
}

impl FromStr for NonEmptyText {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for NonEmptyText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct HttpUrl(String);

impl HttpUrl {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let normalized = value.trim();
        let mut parsed = Url::parse(normalized).map_err(|_| "URL must use HTTP(S)")?;
        match parsed.scheme() {
            "http" | "https" => {}
            _ => return Err("URL must use HTTP(S)"),
        }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelSlug(String);

impl ModelSlug {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ModelSlug {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_whitespace) {
            return Err("model must be one non-empty slug");
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum Effort {
    Low,
    Medium,
    High,
}

impl fmt::Display for Effort {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RunMode {
    Plan,
}

impl fmt::Display for RunMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Plan => "plan",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    StreamJson,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StreamJson => "stream-json",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TimeoutSeconds(f64);

impl TimeoutSeconds {
    pub(crate) fn discovery_duration(self) -> Duration {
        let seconds = if self.0 < 30.0 { self.0 } else { 30.0 };
        Duration::from_secs_f64(seconds)
    }

    pub(crate) fn duration(self) -> Duration {
        Duration::from_secs_f64(self.0)
    }

    pub(crate) fn print_value(self) -> String {
        format!("{}s", self.0)
    }
}

impl Default for TimeoutSeconds {
    fn default() -> Self {
        Self(120.0)
    }
}

impl FromStr for TimeoutSeconds {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let seconds = value
            .parse::<f64>()
            .map_err(|_| "timeout must be a finite number between 1 and 1800 seconds")?;
        if !seconds.is_finite() || !(1.0..=1800.0).contains(&seconds) {
            return Err("timeout must be a finite number between 1 and 1800 seconds");
        }
        Ok(Self(seconds))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Search,
    Extract,
    Map,
    Crawl,
    Research,
}

impl fmt::Display for Operation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Search => "search",
            Self::Extract => "extract",
            Self::Map => "map",
            Self::Crawl => "crawl",
            Self::Research => "research",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn model_slug_rejects_embedded_whitespace(
            prefix in "[a-z0-9-]{1,12}",
            suffix in "[a-z0-9-]{1,12}",
            whitespace in prop_oneof![Just(" "), Just("\t"), Just("\n")],
        ) {
            let candidate = format!("{prefix}{whitespace}{suffix}");
            prop_assert!(ModelSlug::from_str(&candidate).is_err());
        }

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
