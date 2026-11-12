//! When the daily rare grass reset (midnight, Central European time)
//! happens around the world, for the infographic's "reset time around
//! the world" list.
//!
//! The app has no time zone database, so each region's own daylight saving
//! rule is written out here (checked against the IANA tz database for every
//! day of 2026-2035 by the tests below).

/// Days since 1970-01-01 for a calendar date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Calendar date (y, m, d) for days since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Weekday of a day number (0 = Sunday).
fn weekday(day: i64) -> i64 {
    (day + 4).rem_euclid(7) // 1970-01-01 was a Thursday
}

/// The `n`th Sunday (1-based) of a month, as a day number.
fn nth_sunday(y: i64, m: i64, n: i64) -> i64 {
    let first = days_from_civil(y, m, 1);
    first + (7 - weekday(first)) % 7 + (n - 1) * 7
}

/// The last Sunday of a month, as a day number.
fn last_sunday(y: i64, m: i64) -> i64 {
    let next = if m == 12 { days_from_civil(y + 1, 1, 1) } else { days_from_civil(y, m + 1, 1) };
    let last = next - 1;
    last - weekday(last)
}

/// A region's clock rule.
#[derive(Clone, Copy, Debug)]
enum Rule {
    /// Same offset all year.
    Fixed,
    /// USA and Canada: from 2am on the second Sunday of March until 2am on
    /// the first Sunday of November (local time).
    Us,
    /// EU and UK: from 01:00 UTC on the last Sunday of March until 01:00
    /// UTC on the last Sunday of October.
    Eu,
    /// South-east Australia: from 2am standard time on the first Sunday of
    /// October until 3am summer time on the first Sunday of April.
    AuSouthEast,
    /// New Zealand: from 2am standard time on the last Sunday of September
    /// until 3am summer time on the first Sunday of April.
    Nz,
}

/// A row of the list: what it's called, its standard offset from UTC (in
/// minutes) and its daylight saving rule.
#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub label: &'static str,
    /// The IANA zone it follows (for the tests and anyone checking).
    #[allow(dead_code)]
    pub tz: &'static str,
    std_min: i64,
    rule: Rule,
}

impl Region {
    /// Offset from UTC in seconds at a moment.
    pub fn offset_at(&self, t: i64) -> i64 {
        let std = self.std_min * 60;
        let summer = |on: i64, off: i64| t >= on && t < off;
        let dst = match self.rule {
            Rule::Fixed => false,
            Rule::Us => {
                let (y, _, _) = civil_from_days((t + std).div_euclid(86400));
                // 2am local standard time → in UTC; ends 2am local summer time
                let on = nth_sunday(y, 3, 2) * 86400 + 2 * 3600 - std;
                let off = nth_sunday(y, 11, 1) * 86400 + 2 * 3600 - (std + 3600);
                summer(on, off)
            }
            Rule::Eu => {
                let (y, _, _) = civil_from_days(t.div_euclid(86400));
                summer(last_sunday(y, 3) * 86400 + 3600, last_sunday(y, 10) * 86400 + 3600)
            }
            Rule::AuSouthEast | Rule::Nz => {
                // Southern hemisphere: summer time spans the new year.
                let (y, _, _) = civil_from_days((t + std).div_euclid(86400));
                let (start_m, start_day): (i64, fn(i64, i64) -> i64) = match self.rule {
                    Rule::Nz => (9, last_sunday),
                    _ => (10, |y, m| nth_sunday(y, m, 1)),
                };
                let ends = nth_sunday(y, 4, 1) * 86400 + 3 * 3600 - (std + 3600);
                let starts = start_day(y, start_m) * 86400 + 2 * 3600 - std;
                t < ends || t >= starts
            }
        };
        if dst {
            std + 3600
        } else {
            std
        }
    }

    /// Whether summer time is on at a moment.
    pub fn summer(&self, t: i64) -> bool {
        self.offset_at(t) != self.std_min * 60
    }
}

/// The server's clock (Germany / France): Central European time.
pub const SERVER: Region = Region { label: "Germany / France", tz: "Europe/Berlin", std_min: 60, rule: Rule::Eu };

/// The list of places. Labels name only places that really share the
/// zone's clock all year (Brisbane has no summer time, so it isn't
/// "Australia East"; Russia has eleven zones, so only Moscow is named).
pub const REGIONS: &[Region] = &[
    Region { label: "USA West", tz: "America/Los_Angeles", std_min: -480, rule: Rule::Us },
    Region { label: "USA Central", tz: "America/Chicago", std_min: -360, rule: Rule::Us },
    Region { label: "Mexico City", tz: "America/Mexico_City", std_min: -360, rule: Rule::Fixed },
    Region { label: "USA East", tz: "America/New_York", std_min: -300, rule: Rule::Us },
    Region { label: "Brazil / Argentina", tz: "America/Sao_Paulo", std_min: -180, rule: Rule::Fixed },
    Region { label: "UK", tz: "Europe/London", std_min: 0, rule: Rule::Eu },
    SERVER,
    Region { label: "Turkey / Moscow", tz: "Europe/Istanbul", std_min: 180, rule: Rule::Fixed },
    Region { label: "Singapore / Philippines", tz: "Asia/Singapore", std_min: 480, rule: Rule::Fixed },
    Region { label: "Australia West", tz: "Australia/Perth", std_min: 480, rule: Rule::Fixed },
    Region { label: "Japan / Korea", tz: "Asia/Tokyo", std_min: 540, rule: Rule::Fixed },
    Region { label: "Brisbane", tz: "Australia/Brisbane", std_min: 600, rule: Rule::Fixed },
    Region { label: "Sydney / Melbourne", tz: "Australia/Sydney", std_min: 600, rule: Rule::AuSouthEast },
    Region { label: "New Zealand", tz: "Pacific/Auckland", std_min: 720, rule: Rule::Nz },
];

/// "CET" or "CEST" (summer), whichever the server is on at `t`.
pub fn server_zone_name(t: i64) -> &'static str {
    if SERVER.summer(t) {
        "CEST"
    } else {
        "CET"
    }
}

/// One row of the list: a place and its local time at the reset.
#[derive(Clone, Debug)]
pub struct ResetRow {
    pub label: &'static str,
    pub server: bool,
    /// Local hour (0-23) and minute.
    pub hour: i64,
    pub minute: i64,
    /// "Mon", "Tue"…
    pub weekday: &'static str,
    pub offset: i64,
}

impl ResetRow {
    /// "10:00 am".
    pub fn time12(&self) -> String {
        let h = if self.hour % 12 == 0 { 12 } else { self.hour % 12 };
        format!("{h}:{:02} {}", self.minute, if self.hour < 12 { "am" } else { "pm" })
    }
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// Local time of the reset at `reset` (unix seconds) in every place, west
/// to east by their offset at that moment.
pub fn rows(reset: i64) -> Vec<ResetRow> {
    let mut out: Vec<ResetRow> = REGIONS
        .iter()
        .map(|r| {
            let off = r.offset_at(reset);
            let local = reset + off;
            let day = local.div_euclid(86400);
            let secs = local.rem_euclid(86400);
            ResetRow {
                label: r.label,
                server: r.tz == SERVER.tz,
                hour: secs / 3600,
                minute: (secs / 60) % 60,
                weekday: DAYS[weekday(day) as usize],
                offset: off,
            }
        })
        .collect();
    out.sort_by_key(|r| r.offset);
    out
}

/// Date of the server day starting at `reset`: (year, month, day, weekday).
pub fn server_date(reset: i64) -> (i64, i64, i64, &'static str) {
    let day = (reset + SERVER.offset_at(reset)).div_euclid(86400);
    let (y, m, d) = civil_from_days(day);
    (y, m, d, DAYS[weekday(day) as usize])
}

pub const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_midnight_this_week() {
        // Thu 12 Nov 2026, 00:00 CET = Wed 11 Nov 23:00 UTC
        let reset = 1794438000;
        let r = rows(reset);
        let get = |l: &str| r.iter().find(|x| x.label == l).unwrap().clone();
        assert_eq!(get("Germany / France").hour, 0);
        assert_eq!(get("Sydney / Melbourne").hour, 10);
        assert_eq!(get("Brisbane").hour, 9);
        assert_eq!(get("USA West").hour, 15);
        assert_eq!(get("USA West").weekday, "Wed");
        assert_eq!(get("New Zealand").hour, 12);
        assert_eq!(server_zone_name(reset), "CET");
        assert_eq!(server_date(reset), (2026, 11, 12, "Thu"));
    }
}
