//! Deterministic source-visible date parsing for tuple verification.

use crate::calendar_date::CalendarDate;

pub(crate) fn parse(value: &str) -> Result<CalendarDate, &'static str> {
    let normalized = value.trim();
    if let Ok(date) = CalendarDate::parse(normalized) {
        return Ok(date);
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
