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
    if normalized.contains('.')
        && normalized
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return parse_dotted(normalized);
    }
    let parts: Vec<_> = normalized.split_ascii_whitespace().collect();
    let [first, second, year] = parts.as_slice() else {
        return Err("source date must be ISO or an unambiguous English date");
    };
    let (month, day) = match EnglishMonth::parse(first) {
        Some(month) => (
            month,
            second
                .strip_suffix(',')
                .and_then(|day| day.parse::<u8>().ok())
                .ok_or("source day was invalid")?,
        ),
        None => (
            EnglishMonth::parse(second).ok_or("source month was not recognized")?,
            first.parse::<u8>().map_err(|_| "source day was invalid")?,
        ),
    };
    let year = year.parse::<u16>().map_err(|_| "source year was invalid")?;
    CalendarDate::parse(&format!("{year:04}-{:02}-{day:02}", month.number()))
}

fn parse_dotted(value: &str) -> Result<CalendarDate, &'static str> {
    let parts = value.trim_end_matches('.').split('.').collect::<Vec<_>>();
    let [year, month, day] = parts.as_slice() else {
        return Err("source dotted date must have three components");
    };
    if year.len() != 4
        || ![year, month, day]
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err("source dotted date must use ASCII decimal components");
    }
    let year = year.parse::<u16>().map_err(|_| "source year was invalid")?;
    let month = month
        .parse::<u8>()
        .map_err(|_| "source month was invalid")?;
    let day = day.parse::<u8>().map_err(|_| "source day was invalid")?;
    CalendarDate::parse(&format!("{year:04}-{month:02}-{day:02}"))
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
    let year = year.strip_suffix('년').unwrap_or(year);
    if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("source year must use four ASCII digits");
    }
    let year = year
        .parse::<u16>()
        .map_err(|_| "source year must use four ASCII digits")?;
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
            "January" | "Jan" | "Jan." => Some(Self::January),
            "February" | "Feb" | "Feb." => Some(Self::February),
            "March" | "Mar" | "Mar." => Some(Self::March),
            "April" | "Apr" | "Apr." => Some(Self::April),
            "May" => Some(Self::May),
            "June" | "Jun" | "Jun." => Some(Self::June),
            "July" | "Jul" | "Jul." => Some(Self::July),
            "August" | "Aug" | "Aug." => Some(Self::August),
            "September" | "Sep" | "Sep." | "Sept" | "Sept." => Some(Self::September),
            "October" | "Oct" | "Oct." => Some(Self::October),
            "November" | "Nov" | "Nov." => Some(Self::November),
            "December" | "Dec" | "Dec." => Some(Self::December),
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
#[path = "source_date/source_date_test.rs"]
mod tests;
