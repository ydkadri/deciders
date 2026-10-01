//! Calendar dates for stage lines: `YYYY-MM-DD`, with no time or time zone.

use std::fmt;
use std::str::FromStr;

/// A calendar date, ordered chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

/// A value that is not a real calendar date written as `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid date {value:?}, expected a real date written as YYYY-MM-DD")]
pub struct DateError {
    value: String,
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Parse `part` only if it is exactly `width` ASCII digits (no sign, no spaces).
fn parse_digits<T: FromStr>(part: &str, width: usize) -> Option<T> {
    if part.len() == width && part.bytes().all(|byte| byte.is_ascii_digit()) {
        part.parse().ok()
    } else {
        None
    }
}

impl Date {
    /// Build a date from its parts.
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] if the year is outside 1 to 9999 (the range that
    /// reads back from `YYYY`) or the month and day do not exist in that year,
    /// such as 30 February.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DateError> {
        let valid = (1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day);
        if valid {
            Ok(Self { year, month, day })
        } else {
            Err(DateError {
                value: format!("{year:04}-{month:02}-{day:02}"),
            })
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

impl FromStr for Date {
    type Err = DateError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || DateError {
            value: text.to_owned(),
        };
        let mut parts = text.split('-');
        let (Some(year), Some(month), Some(day), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid());
        };
        let year = parse_digits(year, 4).ok_or_else(invalid)?;
        let month = parse_digits(month, 2).ok_or_else(invalid)?;
        let day = parse_digits(day, 2).ok_or_else(invalid)?;
        Self::new(year, month, day).map_err(|_| invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays_a_valid_date() {
        let date: Date = "2026-10-01".parse().unwrap();
        assert_eq!(
            date.to_string(),
            "2026-10-01",
            "round trip changed the text"
        );
    }

    #[test]
    fn accepts_a_leap_day_in_a_leap_year() {
        assert!("2024-02-29".parse::<Date>().is_ok(), "2024 is a leap year");
    }

    #[test]
    fn century_years_are_leap_only_when_divisible_by_400() {
        assert!("2000-02-29".parse::<Date>().is_ok(), "2000 is a leap year");
        assert!("1900-02-29".parse::<Date>().is_err(), "1900 is not");
    }

    #[test]
    fn rejects_a_leap_day_in_a_common_year() {
        assert!(
            "2023-02-29".parse::<Date>().is_err(),
            "2023 is not a leap year"
        );
    }

    #[test]
    fn rejects_dates_that_do_not_exist() {
        for text in [
            "2026-13-01",
            "2026-00-10",
            "2026-04-31",
            "2026-01-00",
            "0000-01-01",
        ] {
            assert!(text.parse::<Date>().is_err(), "{text} should be rejected");
        }
    }

    #[test]
    fn rejects_a_year_that_would_not_read_back() {
        assert!(
            Date::new(9999, 12, 31).is_ok(),
            "the last year that fits YYYY"
        );
        assert!(
            Date::new(10000, 1, 1).is_err(),
            "five digits do not fit YYYY"
        );
    }

    #[test]
    fn rejects_text_that_is_not_yyyy_mm_dd() {
        let cases = [
            "",
            "2026-7-1",
            "26-07-01",
            "2026/07/01",
            "+026-07-01",
            "2026-07-01-02",
            "2026-07",
            " 2026-07-01",
            "YYYY-MM-DD",
        ];
        for text in cases {
            assert!(text.parse::<Date>().is_err(), "{text:?} should be rejected");
        }
    }

    #[test]
    fn the_error_names_the_offending_text() {
        let error = "2026-02-30".parse::<Date>().unwrap_err();
        assert!(
            error.to_string().contains("2026-02-30"),
            "message was: {error}"
        );
    }

    #[test]
    fn orders_chronologically() {
        let earlier: Date = "2025-12-31".parse().unwrap();
        let later: Date = "2026-01-01".parse().unwrap();
        assert!(earlier < later, "year must dominate month and day");
        let june: Date = "2026-06-30".parse().unwrap();
        let july: Date = "2026-07-01".parse().unwrap();
        assert!(june < july, "month must dominate day");
    }
}
