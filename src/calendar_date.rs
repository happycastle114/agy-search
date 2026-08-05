//! Strict, lexically ordered ISO calendar dates for temporal verification.

use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Clone, Debug, Eq, Hash, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct CalendarDate(String);

impl CalendarDate {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let bytes = value.as_bytes();
        if bytes.len() != 10
            || bytes.get(4).copied() != Some(b'-')
            || bytes.get(7).copied() != Some(b'-')
        {
            return Err("date must use YYYY-MM-DD");
        }
        let year = component(value, 0..4)?;
        let month = component(value, 5..7)?;
        let day = component(value, 8..10)?;
        if year == 0
            || !(1..=12).contains(&month)
            || !(1..=days_in_month(year, month)).contains(&day)
        {
            return Err("date must be a valid calendar day");
        }
        Ok(Self(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CalendarDate {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl<'de> Deserialize<'de> for CalendarDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

fn component(value: &str, range: std::ops::Range<usize>) -> Result<u16, &'static str> {
    value
        .get(range)
        .filter(|part| part.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|part| part.parse::<u16>().ok())
        .ok_or("date must use ASCII decimal digits")
}

const fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_date_checks_real_days() {
        assert!(CalendarDate::parse("2024-02-29").is_ok());
        assert!(CalendarDate::parse("2026-02-29").is_err());
        assert!(CalendarDate::parse("2026-13-01").is_err());
        assert!(CalendarDate::parse("August 3, 2026").is_err());
    }
}
