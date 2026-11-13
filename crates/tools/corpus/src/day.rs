//! Days since 1970-01-01, which is what the index dates a document by, and how a report writes
//! one. Kept once here because every tool that prints a hit prints its date.

/// Today as days since 1970-01-01.
#[must_use]
pub fn today() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    i64::try_from(secs / 86_400).unwrap_or(0)
}

/// A day since 1970-01-01 as `YYYY-MM-DD`.
#[must_use]
pub fn ymd(day: i64) -> String {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_written_as_a_date() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(11_016), "2000-02-29");
        assert_eq!(ymd(20_000), "2024-10-04");
        assert_eq!(ymd(-1), "1969-12-31");
    }

    #[test]
    fn today_is_after_the_year_two_thousand_and_twenty_five() {
        assert!(today() > 20_000);
    }
}
