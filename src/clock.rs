//! The system clock, read once at the edge and passed to the use cases as a value.

use anyhow::Context;
use chrono::Datelike;
use decider_adr::domain::date::Date;

/// Today's date in the local time zone.
///
/// The local date, not UTC, is what a person expects to see on the Proposed line.
pub(crate) fn today() -> anyhow::Result<Date> {
    let today = chrono::Local::now().date_naive();
    let year = u16::try_from(today.year()).context("the year is out of range")?;
    let month = u8::try_from(today.month()).context("the month is out of range")?;
    let day = u8::try_from(today.day()).context("the day is out of range")?;
    Date::new(year, month, day).context("the system clock gave a date that cannot be written")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_is_a_recent_date() {
        let today = today().unwrap();
        assert!(
            today > "2026-01-01".parse().unwrap(),
            "the clock gave {today}"
        );
    }
}
