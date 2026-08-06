use std::{fmt, str::FromStr, time::Duration};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Clone, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct NonEmptyText(#[schemars(length(min = 1))] String);

impl NonEmptyText {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err("value must be non-empty");
        }
        Ok(Self(normalized.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
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
    pub(crate) fn duration(self) -> Duration {
        Duration::from_secs_f64(self.0)
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
