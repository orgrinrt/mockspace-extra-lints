//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Days, the one unit the derived values are counted in.

/// The days from 1970-01-01 to a civil date (Howard Hinnant's algorithm).
const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a count of days since 1970-01-01, the inverse of `days_from_civil`.
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn number(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The days since 1970-01-01 that a date `YYYY-MM-DD`, or a stamp `YYYY-MM-DDTHH:MM:SSZ` falls on.
/// Any other text, and a month or a day out of range, give nothing.
pub fn day_of(text: &str) -> Option<i64> {
    let date = match text.len() {
        10 => text,
        20 if text.as_bytes()[10] == b'T' && text.ends_with('Z') => &text[.. 10],
        _ => return None,
    };
    let b = date.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (
        number(&date[.. 4])?,
        number(&date[5 .. 7])?,
        number(&date[8 ..])?,
    );
    if !(1 ..= 12).contains(&m) || !(1 ..= 31).contains(&d) {
        return None;
    }
    let day = days_from_civil(y, m, d);
    (civil_from_days(day) == (y, m, d)).then_some(day)
}

/// The date `YYYY-MM-DD` of a count of days since 1970-01-01.
pub fn date_of(day: i64) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{y:04}-{m:02}-{d:02}")
}
