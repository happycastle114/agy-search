//! Deterministic source-visible date parsing for tuple verification.

use crate::calendar_date::CalendarDate;

pub(crate) fn parse(value: &str) -> Result<CalendarDate, &'static str> {
    let normalized = value.trim();
    if let Ok(date) = CalendarDate::parse(normalized) {
        return Ok(date);
    }
    if normalized.get(10..11) == Some("T") {
        return parse_iso_datetime(normalized);
    }
    if normalized
        .chars()
        .any(|character| matches!(character, '년' | '월' | '일'))
    {
        return parse_korean(normalized);
    }
    let parts: Vec<_> = normalized.split_ascii_whitespace().collect();
    let [month, day, year] = parts.as_slice() else {
        return Err("source date must be ISO or an unambiguous English date");
    };
    let month = EnglishMonth::parse(month).ok_or("source month was not recognized")?;
    let day = day
        .strip_suffix(',')
        .and_then(|day| day.parse::<u8>().ok())
        .ok_or("source day was invalid")?;
    let year = year.parse::<u16>().map_err(|_| "source year was invalid")?;
    CalendarDate::parse(&format!("{year:04}-{:02}-{day:02}", month.number()))
}

fn parse_iso_datetime(value: &str) -> Result<CalendarDate, &'static str> {
    let date = value
        .get(..10)
        .ok_or("source datetime date was incomplete")?;
    let clock_and_zone = value
        .get(11..)
        .ok_or("source datetime clock was incomplete")?;
    let (clock, zone) = split_clock_and_zone(clock_and_zone)?;
    validate_clock(clock)?;
    validate_zone(zone)?;
    CalendarDate::parse(date)
}

fn split_clock_and_zone(value: &str) -> Result<(&str, &str), &'static str> {
    if let Some(clock) = value.strip_suffix('Z') {
        return Ok((clock, "Z"));
    }
    let offset = value
        .rfind(['+', '-'])
        .ok_or("source datetime requires an explicit zone")?;
    let clock = value
        .get(..offset)
        .ok_or("source datetime clock was invalid")?;
    let zone = value
        .get(offset..)
        .ok_or("source datetime zone was invalid")?;
    Ok((clock, zone))
}

fn validate_clock(value: &str) -> Result<(), &'static str> {
    let mut parts = value.split(':');
    parse_two_digits(parts.next(), 23).ok_or("source datetime hour was invalid")?;
    parse_two_digits(parts.next(), 59).ok_or("source datetime minute was invalid")?;
    let second = parts.next().ok_or("source datetime second was missing")?;
    if parts.next().is_some() {
        return Err("source datetime clock had extra fields");
    }
    let second = match second.split_once('.') {
        Some((whole, fraction))
            if !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()) =>
        {
            whole
        }
        Some(_) => return Err("source datetime fraction was invalid"),
        None => second,
    };
    parse_two_digits(Some(second), 59).ok_or("source datetime second was invalid")?;
    Ok(())
}

fn validate_zone(value: &str) -> Result<(), &'static str> {
    if value == "Z" {
        return Ok(());
    }
    let offset = value
        .strip_prefix(['+', '-'])
        .ok_or("source datetime zone sign was invalid")?;
    let mut parts = offset.split(':');
    parse_two_digits(parts.next(), 23).ok_or("source datetime zone hour was invalid")?;
    parse_two_digits(parts.next(), 59).ok_or("source datetime zone minute was invalid")?;
    if parts.next().is_some() {
        return Err("source datetime zone had extra fields");
    }
    Ok(())
}

fn parse_two_digits(value: Option<&str>, maximum: u8) -> Option<u8> {
    let value = value?;
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<u8>().ok().filter(|parsed| *parsed <= maximum)
}

fn parse_korean(value: &str) -> Result<CalendarDate, &'static str> {
    let parts: Vec<_> = value.split_ascii_whitespace().collect();
    let [year, month, day] = parts.as_slice() else {
        return Err("source date must be a complete Korean date");
    };
    let year = parse_korean_component(year, '년', 4..=4)
        .ok_or("source year must use four ASCII digits")?;
    let month =
        parse_korean_component(month, '월', 1..=2).ok_or("source month must use ASCII digits")?;
    let day = parse_korean_component(day, '일', 1..=2).ok_or("source day must use ASCII digits")?;
    CalendarDate::parse(&format!("{year:04}-{month:02}-{day:02}"))
}

fn parse_korean_component(
    value: &str,
    suffix: char,
    digit_count: std::ops::RangeInclusive<usize>,
) -> Option<u16> {
    let digits = value.strip_suffix(suffix)?;
    if !digit_count.contains(&digits.len()) || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u16>().ok()
}

#[derive(Clone, Copy, Debug)]
enum EnglishMonth {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl EnglishMonth {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "January" => Some(Self::January),
            "February" => Some(Self::February),
            "March" => Some(Self::March),
            "April" => Some(Self::April),
            "May" => Some(Self::May),
            "June" => Some(Self::June),
            "July" => Some(Self::July),
            "August" => Some(Self::August),
            "September" => Some(Self::September),
            "October" => Some(Self::October),
            "November" => Some(Self::November),
            "December" => Some(Self::December),
            _ => None,
        }
    }

    const fn number(self) -> u8 {
        match self {
            Self::January => 1,
            Self::February => 2,
            Self::March => 3,
            Self::April => 4,
            Self::May => 5,
            Self::June => 6,
            Self::July => 7,
            Self::August => 8,
            Self::September => 9,
            Self::October => 10,
            Self::November => 11,
            Self::December => 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_korean_full_date() {
        // Given: a complete Korean calendar date with ASCII digits.
        let value = "2026년 8월 6일";

        // When: the source date is parsed.
        let result = parse(value);

        // Then: it is normalized to the strict ISO calendar date.
        assert_eq!(
            result.map(|date| date.to_string()),
            Ok("2026-08-06".to_owned())
        );
    }

    #[test]
    fn parses_iso_datetime_with_explicit_offset() {
        // Given: an HTML time element's complete RFC 3339-style datetime value.
        let value = "2026-08-06T10:21:18+09:00";

        // When: the source-visible date is normalized.
        let result = parse(value);

        // Then: its explicit calendar day binds without inventing a date.
        assert_eq!(
            result.map(|date| date.to_string()),
            Ok("2026-08-06".to_owned())
        );
    }

    #[test]
    fn rejects_iso_datetime_without_a_valid_explicit_zone() {
        for value in [
            "2026-08-06T10:21:18",
            "2026-08-06T25:21:18+09:00",
            "2026-08-06T10:61:18+09:00",
            "2026-08-06T10:21:61+09:00",
            "2026-08-06T10:21:18+25:00",
            "2026-08-06T10:21:18+09:99",
            "2026-08-06T10:21:18+09:00junk",
        ] {
            assert!(parse(value).is_err(), "accepted invalid datetime: {value}");
        }
    }

    #[test]
    fn rejects_incomplete_korean_date() {
        // Given: a Korean date missing its day component.
        let value = "2026년 8월";

        // When: the source date is parsed.
        let result = parse(value);

        // Then: the incomplete date is rejected.
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_korean_month_and_day() {
        // Given: Korean dates with impossible month and day values.
        let invalid_month = "2026년 13월 6일";
        let invalid_day = "2026년 8월 32일";

        // When: each source date is parsed.
        let month_result = parse(invalid_month);
        let day_result = parse(invalid_day);

        // Then: neither impossible calendar date is accepted.
        assert!(month_result.is_err());
        assert!(day_result.is_err());
    }

    #[test]
    fn rejects_non_ascii_korean_date_digits() {
        // Given: a Korean date using full-width digits instead of ASCII digits.
        let value = "２０２６년 ８월 ６일";

        // When: the source date is parsed.
        let result = parse(value);

        // Then: non-ASCII digits are rejected at the source boundary.
        assert!(result.is_err());
    }
}
