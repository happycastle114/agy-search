use super::parse;

#[test]
fn parses_korean_full_date() {
    assert_eq!(
        parse("2026년 8월 6일").map(|date| date.to_string()),
        Ok("2026-08-06".to_owned())
    );
}

#[test]
fn parses_korean_table_date_with_bare_year() {
    for value in ["2026 8월 27일", "2026 08월 27일"] {
        assert_eq!(
            parse(value).map(|date| date.to_string()),
            Ok("2026-08-27".to_owned())
        );
    }
}

#[test]
fn parses_day_first_english_full_date() {
    assert_eq!(
        parse("28 August 2026").map(|date| date.to_string()),
        Ok("2026-08-28".to_owned())
    );
}

#[test]
fn parses_abbreviated_english_full_date() {
    assert_eq!(
        parse("Aug. 20, 2026").map(|date| date.to_string()),
        Ok("2026-08-20".to_owned())
    );
}

#[test]
fn parses_unambiguous_dotted_full_date() {
    for value in ["2026.8.27", "2026.08.27", "2026.8.27."] {
        assert_eq!(
            parse(value).map(|date| date.to_string()),
            Ok("2026-08-27".to_owned())
        );
    }
}

#[test]
fn parses_iso_datetime_with_explicit_offset() {
    assert_eq!(
        parse("2026-08-06T10:21:18+09:00").map(|date| date.to_string()),
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
    assert!(parse("2026년 8월").is_err());
}

#[test]
fn rejects_invalid_korean_month_and_day() {
    assert!(parse("2026년 13월 6일").is_err());
    assert!(parse("2026년 8월 32일").is_err());
}

#[test]
fn rejects_non_ascii_korean_date_digits() {
    assert!(parse("２０２６년 ８월 ６일").is_err());
}
