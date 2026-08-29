use crate::types::CalendarDate;

pub(crate) fn nearest_date_binding(
    body: &str,
    date: &CalendarDate,
    value_start: usize,
    value_end: usize,
) -> Option<(usize, usize)> {
    let body_lower = body.to_ascii_lowercase();
    date_variants(date)
        .into_iter()
        .flat_map(|variant| {
            let variant_lower = variant.to_ascii_lowercase();
            body_lower
                .match_indices(&variant_lower)
                .map(move |(start, _)| (start, start + variant.len()))
                .collect::<Vec<_>>()
        })
        .min_by_key(|(start, end)| {
            if *end < value_start {
                value_start - *end
            } else {
                start.saturating_sub(value_end)
            }
        })
}

fn date_variants(date: &CalendarDate) -> Vec<String> {
    let year = date.as_str().get(..4).unwrap_or_default();
    let month_number = date
        .as_str()
        .get(5..7)
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or_default();
    let day = date
        .as_str()
        .get(8..10)
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or_default();
    let month = match month_number {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    };
    let month_abbreviation = match month_number {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    };
    [
        date.as_str().to_owned(),
        format!("{month} {day}, {year}"),
        format!("{day} {month} {year}"),
        format!("{month_abbreviation}. {day}, {year}"),
        format!("{month_abbreviation} {day}, {year}"),
        format!("{year}년 {month_number}월 {day}일"),
        format!("{year} {month_number}월 {day}일"),
        format!("{year} {month_number:02}월 {day:02}일"),
        format!("{year}.{month_number}.{day}"),
        format!("{year}.{month_number:02}.{day:02}"),
        format!("{year}/{month_number}/{day}"),
        format!("{year}/{month_number:02}/{day:02}"),
    ]
    .into_iter()
    .filter(|variant| !variant.trim().is_empty())
    .collect()
}
