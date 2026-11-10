//! Per-run hunting statistics: active hunting time, per-day and per-session
//! counts, encounter rate and milestones. Pure logic (no UI, no OS calls
//! except the local time-zone offset), so it can be tested on its own.

#[cfg(not(offline_test))]
use serde::{Deserialize, Serialize};

/// More than this many seconds between encounters counts as a break: the gap
/// isn't added to hunting time.
pub const IDLE_GAP: u64 = 300;
/// More than this many seconds without an encounter starts a new session.
pub const SESSION_GAP: u64 = 30 * 60;
const MAX_SESSIONS: usize = 400;
const MAX_DAYS: usize = 3000;
const MAX_MILESTONES: usize = 200;

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct DayStat {
    /// Local day number (days since 1970-01-01 in local time).
    pub day: i64,
    pub encounters: u64,
    pub secs: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct Session {
    pub start: u64,
    pub end: u64,
    pub encounters: u64,
    pub secs: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct Milestone {
    pub unix: u64,
    pub count: u64,
    pub label: String,
}

/// A hunt journal entry.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct Note {
    pub unix: u64,
    /// Encounters when the note was written.
    pub count: u64,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(not(offline_test), derive(Serialize, Deserialize))]
#[cfg_attr(not(offline_test), serde(default))]
pub struct RunStats {
    /// When tracking began for this run (0 = not yet).
    pub tracking_since: u64,
    /// Encounters the run already had before tracking began.
    pub base_count: u64,
    /// Active hunting time (breaks over IDLE_GAP not counted).
    pub hunt_secs: u64,
    /// Time of the last counted encounter.
    pub last_unix: u64,
    pub days: Vec<DayStat>,
    pub sessions: Vec<Session>,
    /// Highest thousand already celebrated (e.g. 2000).
    pub best_thousand: u64,
    /// Highest whole multiple of the odds already celebrated.
    pub best_odds_x: u32,
    pub milestones: Vec<Milestone>,
    /// Hunt journal.
    pub notes: Vec<Note>,
    /// Encounters per species (national dex number) this run.
    pub species: std::collections::BTreeMap<u32, u64>,
    /// Encounter count when the target was last seen (0 = not yet / unknown).
    pub target_last: u64,
    /// Encounters per local hour (`local_hour`), last month or so.
    pub hours: std::collections::BTreeMap<i64, u64>,
    /// When hourly counts began (older hours are estimated from sessions).
    pub hours_since: u64,
    /// Longest stretch of encounters without seeing the target (tracked
    /// from the version that added it; 0 = unknown).
    pub target_gap_max: u64,
}

/// Hours of hourly counts kept.
const KEEP_HOURS: i64 = 24 * 35;

/// Local hour number for a unix time (local day * 24 + hour of day).
pub fn local_hour(unix: u64) -> i64 {
    (unix as i64 + utc_offset_secs()).div_euclid(3600)
}

/// "3pm" / "12am" for a local hour number.
pub fn fmt_hour(h: i64) -> String {
    match h.rem_euclid(24) {
        0 => "12am".to_string(),
        x @ 1..=11 => format!("{x}am"),
        12 => "12pm".to_string(),
        x => format!("{}pm", x - 12),
    }
}

/// A milestone that was just reached.
#[derive(Debug, Clone, PartialEq)]
pub enum Reached {
    Thousand(u64),
    Odds(u32),
}

impl Reached {
    pub fn label(&self) -> String {
        match self {
            Reached::Thousand(n) => format!("{} encounters", thousands(*n)),
            Reached::Odds(1) => "Reached the odds (1×)".to_string(),
            Reached::Odds(k) => format!("{k}× the odds"),
        }
    }
}

impl RunStats {
    /// Start tracking (once). Existing encounters become the base, and
    /// milestones already passed are marked so they don't fire on upgrade.
    pub fn ensure_started(&mut self, now: u64, count: u64, odds_one_in: Option<f64>) {
        if self.tracking_since != 0 {
            return;
        }
        self.tracking_since = now;
        self.base_count = count;
        self.best_thousand = count / 1000 * 1000;
        self.best_odds_x = odds_one_in.map_or(0, |o| odds_multiple(count, o));
    }

    /// Record `n` encounters at `now` (unix secs) on local day `day`.
    pub fn record(&mut self, now: u64, day: i64, n: u64) {
        self.record_gap(now, day, n, IDLE_GAP);
    }

    /// Like `record`, but a wait longer than `idle_gap` seconds since the
    /// last encounter isn't counted as hunting time (the timer paused).
    pub fn record_gap(&mut self, now: u64, day: i64, n: u64, idle_gap: u64) {
        if n == 0 {
            return;
        }
        let delta = if self.last_unix > 0 { now.saturating_sub(self.last_unix) } else { u64::MAX };
        let add = if delta <= idle_gap { delta } else { 0 };
        if self.sessions.is_empty() || delta > SESSION_GAP {
            self.sessions.push(Session { start: now, end: now, encounters: 0, secs: 0 });
            if self.sessions.len() > MAX_SESSIONS {
                self.sessions.remove(0);
            }
        }
        if let Some(s) = self.sessions.last_mut() {
            s.end = now;
            s.encounters += n;
            s.secs += add;
        }
        self.hunt_secs += add;
        if self.hours_since == 0 {
            self.hours_since = now;
        }
        let h = local_hour(now);
        *self.hours.entry(h).or_insert(0) += n;
        if self.hours.len() > KEEP_HOURS as usize {
            self.hours = self.hours.split_off(&(h - KEEP_HOURS));
        }
        let d = self.day_mut(day);
        d.encounters += n;
        d.secs += add;
        self.last_unix = now;
    }

    /// Undo `n` manual encounters (the -1 button): taken off today and the
    /// current session.
    pub fn unrecord(&mut self, day: i64, n: u64) {
        if let Some(d) = self.days.iter_mut().find(|d| d.day == day) {
            d.encounters = d.encounters.saturating_sub(n);
        }
        if let Some(s) = self.sessions.last_mut() {
            s.encounters = s.encounters.saturating_sub(n);
        }
        if let Some((_, c)) = self.hours.iter_mut().next_back() {
            *c = c.saturating_sub(n);
        }
    }

    /// Encounters in each local hour from `from` to `to` (inclusive). Hours
    /// from before hourly counts began are estimated by spreading each
    /// session's encounters evenly over its time.
    pub fn hourly(&self, from: i64, to: i64) -> Vec<u64> {
        if to < from {
            return Vec::new();
        }
        let len = (to - from + 1) as usize;
        let mut est = vec![0f64; len];
        for s in &self.sessions {
            if self.hours_since != 0 && s.end >= self.hours_since {
                continue; // counted exactly
            }
            let off = utc_offset_secs();
            let (a, b) = ((s.start as i64 + off) as f64, (s.end as i64 + off) as f64);
            if b <= a {
                let h = (a / 3600.0).floor() as i64;
                if (from..=to).contains(&h) {
                    est[(h - from) as usize] += s.encounters as f64;
                }
                continue;
            }
            let per_sec = s.encounters as f64 / (b - a);
            let mut h = (a / 3600.0).floor() as i64;
            while (h as f64) * 3600.0 < b {
                let lo = a.max(h as f64 * 3600.0);
                let hi = b.min((h + 1) as f64 * 3600.0);
                if hi > lo && (from..=to).contains(&h) {
                    est[(h - from) as usize] += per_sec * (hi - lo);
                }
                h += 1;
            }
        }
        (0..len)
            .map(|i| {
                let h = from + i as i64;
                self.hours.get(&h).copied().unwrap_or(0) + est[i].round() as u64
            })
            .collect()
    }

    fn day_mut(&mut self, day: i64) -> &mut DayStat {
        let pos = match self.days.binary_search_by_key(&day, |d| d.day) {
            Ok(i) => i,
            Err(i) => {
                self.days.insert(i, DayStat { day, encounters: 0, secs: 0 });
                if self.days.len() > MAX_DAYS {
                    self.days.remove(0);
                    return self.day_mut(day);
                }
                i
            }
        };
        &mut self.days[pos]
    }

    /// Check for new milestones after the count changed. Returns those just
    /// reached (each fires once per run).
    pub fn check_milestones(&mut self, now: u64, count: u64, odds_one_in: Option<f64>) -> Vec<Reached> {
        let mut out = Vec::new();
        let th = count / 1000 * 1000;
        if th >= 1000 && th > self.best_thousand {
            self.best_thousand = th;
            out.push(Reached::Thousand(th));
        }
        if let Some(o) = odds_one_in {
            let k = odds_multiple(count, o);
            if k >= 1 && k > self.best_odds_x {
                self.best_odds_x = k;
                out.push(Reached::Odds(k));
            }
        }
        for r in &out {
            self.milestones.push(Milestone { unix: now, count, label: r.label() });
        }
        if self.milestones.len() > MAX_MILESTONES {
            let extra = self.milestones.len() - MAX_MILESTONES;
            self.milestones.drain(..extra);
        }
        out
    }

    /// Encounters counted since tracking began.
    pub fn tracked_encounters(&self) -> u64 {
        self.days.iter().map(|d| d.encounters).sum()
    }

    /// Encounters per hour of active hunting (needs a few minutes of data).
    pub fn per_hour(&self) -> Option<f64> {
        if self.hunt_secs < 120 {
            return None;
        }
        // The first encounter of each session has no time attached; leave
        // those out so the rate isn't inflated.
        let firsts = self.sessions.len() as u64;
        let n = self.tracked_encounters().saturating_sub(firsts);
        (n > 0).then(|| n as f64 * 3600.0 / self.hunt_secs as f64)
    }

    pub fn today(&self, day: i64) -> (u64, u64) {
        self.days
            .iter()
            .find(|d| d.day == day)
            .map_or((0, 0), |d| (d.encounters, d.secs))
    }

    /// The session in progress (an encounter within SESSION_GAP), if any.
    pub fn current_session(&self, now: u64) -> Option<&Session> {
        let s = self.sessions.last()?;
        (now.saturating_sub(s.end) <= SESSION_GAP).then_some(s)
    }

    pub fn best_day(&self) -> Option<&DayStat> {
        self.days.iter().max_by_key(|d| d.encounters)
    }

    pub fn days_hunted(&self) -> usize {
        self.days.iter().filter(|d| d.encounters > 0).count()
    }

    /// Seconds of hunting needed to reach the next whole multiple of the
    /// odds at the current rate, and that multiple.
    pub fn eta_to_odds(&self, count: u64, odds_one_in: f64) -> Option<(u64, u32)> {
        let rate = self.per_hour()?;
        let k = odds_multiple(count, odds_one_in) + 1;
        let target = (odds_one_in * k as f64).ceil();
        let remaining = (target - count as f64).max(0.0);
        Some(((remaining / rate * 3600.0).round() as u64, k))
    }
}

// ───────────────────────────── insights (stats screen) ─────────────────────────────

/// Share of hunters (0..1) who'd have found the shiny within `n` encounters.
pub fn found_by(n: u64, odds_one_in: f64) -> f64 {
    if odds_one_in <= 1.0 {
        return 1.0;
    }
    1.0 - (1.0 - 1.0 / odds_one_in).powf(n as f64)
}

/// Encounters within which a share `q` (0..1) of hunters find the shiny.
/// Shiny odds have no memory, so this is also "from here" for anyone still
/// hunting.
pub fn encounters_for(q: f64, odds_one_in: f64) -> u64 {
    if odds_one_in <= 1.0 {
        return 1;
    }
    let q = q.clamp(0.0, 0.999_999);
    ((1.0 - q).ln() / (1.0 - 1.0 / odds_one_in).ln()).ceil().max(0.0) as u64
}

/// Day of the week for a local day number, 0 = Monday (1970-01-01 was a Thursday).
pub fn weekday(day: i64) -> usize {
    (day + 3).rem_euclid(7) as usize
}

pub const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Encounters and active hunting seconds that fell in one hour of the day.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HourSlot {
    pub encounters: f64,
    pub secs: f64,
}

impl HourSlot {
    /// Encounters per hour while hunting in this slot (None if too little data).
    pub fn rate(&self) -> Option<f64> {
        (self.secs >= 900.0 && self.encounters > 0.0).then(|| self.encounters * 3600.0 / self.secs)
    }
}

impl RunStats {
    /// Average seconds per encounter while hunting.
    pub fn secs_per_encounter(&self) -> Option<f64> {
        self.per_hour().filter(|r| *r > 0.0).map(|r| 3600.0 / r)
    }

    /// Encounters and hunting time by hour of the day (local, 0 = midnight).
    /// Each session is spread over the clock hours it covered.
    pub fn by_hour_of_day(&self) -> [HourSlot; 24] {
        let mut out = [HourSlot::default(); 24];
        let off = utc_offset_secs();
        for s in &self.sessions {
            let (a, b) = (s.start as i64 + off, s.end as i64 + off);
            if b <= a {
                let h = a.div_euclid(3600).rem_euclid(24) as usize;
                out[h].encounters += s.encounters as f64;
                out[h].secs += s.secs as f64;
                continue;
            }
            let span = (b - a) as f64;
            let mut h = a.div_euclid(3600);
            let last = b.div_euclid(3600).min(h + 24 * 14); // ignore silly spans
            while h <= last {
                let lo = a.max(h * 3600);
                let hi = b.min((h + 1) * 3600);
                if hi > lo {
                    let f = (hi - lo) as f64 / span;
                    let slot = &mut out[h.rem_euclid(24) as usize];
                    slot.encounters += s.encounters as f64 * f;
                    slot.secs += s.secs as f64 * f;
                }
                h += 1;
            }
        }
        out
    }

    /// (encounters, seconds) for each day of the week, Monday first.
    pub fn by_weekday(&self) -> [(u64, u64); 7] {
        let mut out = [(0u64, 0u64); 7];
        for d in &self.days {
            let w = weekday(d.day);
            out[w].0 += d.encounters;
            out[w].1 += d.secs;
        }
        out
    }

    /// ((encounters, secs) over the 7 days ending `today`, same for the 7 before).
    pub fn week_compare(&self, today: i64) -> ((u64, u64), (u64, u64)) {
        let mut this = (0, 0);
        let mut last = (0, 0);
        for d in &self.days {
            if d.day > today - 7 && d.day <= today {
                this.0 += d.encounters;
                this.1 += d.secs;
            } else if d.day > today - 14 && d.day <= today - 7 {
                last.0 += d.encounters;
                last.1 += d.secs;
            }
        }
        (this, last)
    }

    /// The longest session by hunting time.
    pub fn longest_session(&self) -> Option<&Session> {
        self.sessions.iter().filter(|s| s.secs > 0).max_by_key(|s| s.secs)
    }

    /// The fastest session of at least 10 minutes, with its encounters per hour.
    pub fn fastest_session(&self) -> Option<(&Session, f64)> {
        self.sessions
            .iter()
            .filter(|s| s.secs >= 600 && s.encounters > 1)
            .map(|s| (s, (s.encounters - 1) as f64 * 3600.0 / s.secs as f64))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Average hunting time per session.
    pub fn avg_session_secs(&self) -> Option<u64> {
        let real: Vec<&Session> = self.sessions.iter().filter(|s| s.secs > 0).collect();
        (!real.is_empty()).then(|| real.iter().map(|s| s.secs).sum::<u64>() / real.len() as u64)
    }

    /// Note a target sighting at `count`, keeping the longest gap between sightings.
    pub fn note_target_seen(&mut self, count: u64) {
        let gap = count.saturating_sub(self.target_last);
        if gap > self.target_gap_max {
            self.target_gap_max = gap;
        }
        self.target_last = count;
    }
}

/// Whole multiples of the odds reached by `count` encounters.
pub fn odds_multiple(count: u64, odds_one_in: f64) -> u32 {
    if odds_one_in <= 0.0 {
        return 0;
    }
    (count as f64 / odds_one_in + 1e-9).floor().min(1e6) as u32
}

// ───────────────────────────── time helpers ─────────────────────────────

/// Seconds east of UTC for local time right now.
pub fn utc_offset_secs() -> i64 {
    imp::utc_offset_secs()
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Local day number for a unix time.
pub fn local_day(unix: u64) -> i64 {
    (unix as i64 + utc_offset_secs()).div_euclid(86_400)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// (year, month, day) for a day number.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "10 Nov 2026"
pub fn fmt_day(day: i64) -> String {
    let (y, m, d) = civil_from_days(day);
    format!("{d} {} {y}", MONTHS[(m - 1) as usize])
}

/// "10 Nov"
pub fn fmt_day_short(day: i64) -> String {
    let (_, m, d) = civil_from_days(day);
    format!("{d} {}", MONTHS[(m - 1) as usize])
}

/// "10 Nov 2026" for a unix time, in local time.
pub fn fmt_date(unix: u64) -> String {
    fmt_day(local_day(unix))
}

/// "3:05 pm" local time.
pub fn fmt_clock(unix: u64) -> String {
    let secs = (unix as i64 + utc_offset_secs()).rem_euclid(86_400);
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    let (h12, ap) = match h {
        0 => (12, "am"),
        1..=11 => (h, "am"),
        12 => (12, "pm"),
        _ => (h - 12, "pm"),
    };
    format!("{h12}:{m:02} {ap}")
}

/// "6h 12m", "45m", "2d 3h"
pub fn fmt_duration(secs: u64) -> String {
    let m = secs / 60;
    let (d, h, m) = (m / 1440, (m / 60) % 24, m % 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m")
    }
}

pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(windows)]
mod imp {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        dow: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        ms: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(t: *mut SystemTime);
        fn GetSystemTime(t: *mut SystemTime);
    }
    fn secs(t: &SystemTime) -> i64 {
        super::days_from_civil(t.year as i64, t.month as u32, t.day as u32) * 86_400
            + t.hour as i64 * 3600
            + t.minute as i64 * 60
            + t.second as i64
    }
    pub fn utc_offset_secs() -> i64 {
        let mut l = SystemTime::default();
        let mut u = SystemTime::default();
        unsafe {
            GetSystemTime(&mut u);
            GetLocalTime(&mut l);
        }
        // Round to 15 minutes (the two calls can straddle a second).
        let off = secs(&l) - secs(&u);
        (off as f64 / 900.0).round() as i64 * 900
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn utc_offset_secs() -> i64 {
        std::env::var("TZ_OFFSET_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luck_maths() {
        // 1 in 4096: about 63% have it by the odds, half by ~2839.
        let p = found_by(4096, 4096.0);
        assert!((p - 0.6322).abs() < 0.001, "{p}");
        assert_eq!(encounters_for(0.5, 4096.0), 2839);
        assert!(found_by(encounters_for(0.9, 4096.0), 4096.0) >= 0.9);
        assert_eq!(found_by(0, 4096.0), 0.0);
        assert_eq!(found_by(5, 1.0), 1.0);
    }

    #[test]
    fn weekdays_and_weeks() {
        assert_eq!(weekday(0), 3); // 1970-01-01 Thursday
        assert_eq!(weekday(days_from_civil(2026, 11, 9)), 0); // a Monday
        let mut st = RunStats::default();
        st.days = vec![
            DayStat { day: 100, encounters: 10, secs: 60 },
            DayStat { day: 95, encounters: 5, secs: 30 },
            DayStat { day: 90, encounters: 7, secs: 20 },
            DayStat { day: 80, encounters: 99, secs: 99 },
        ];
        assert_eq!(st.week_compare(100), ((15, 90), (7, 20)));
        let w = st.by_weekday();
        assert_eq!(w.iter().map(|x| x.0).sum::<u64>(), 121);
    }

    #[test]
    fn hour_of_day_spreads_sessions() {
        std::env::set_var("TZ_OFFSET_SECS", "0");
        let mut st = RunStats::default();
        // 21:30 → 22:30, 100 encounters, 3600 s
        let start = 10 * 86_400 + 21 * 3600 + 1800;
        st.sessions.push(Session { start, end: start + 3600, encounters: 100, secs: 3600 });
        let h = st.by_hour_of_day();
        assert!((h[21].encounters - 50.0).abs() < 1e-6);
        assert!((h[22].encounters - 50.0).abs() < 1e-6);
        assert_eq!(h[21].rate(), Some(100.0));
        assert_eq!(h[3].encounters, 0.0);
    }

    #[test]
    fn sessions_and_target_gaps() {
        let mut st = RunStats::default();
        st.sessions.push(Session { start: 0, end: 600, encounters: 50, secs: 600 });
        st.sessions.push(Session { start: 9000, end: 12600, encounters: 200, secs: 3600 });
        st.sessions.push(Session { start: 20000, end: 20100, encounters: 30, secs: 100 });
        assert_eq!(st.longest_session().unwrap().secs, 3600);
        let (f, r) = st.fastest_session().unwrap();
        assert_eq!(f.secs, 600);
        assert!((r - 294.0).abs() < 1e-6);
        assert_eq!(st.avg_session_secs(), Some(1433));
        st.note_target_seen(40);
        st.note_target_seen(55);
        st.note_target_seen(300);
        assert_eq!(st.target_gap_max, 245);
        assert_eq!(st.target_last, 300);
    }

    #[test]
    fn hourly_counts_and_estimates() {
        let mut st = RunStats::default();
        // an old session from before hourly counts: 120 over 2 hours
        st.sessions.push(Session { start: 36_000, end: 36_000 + 7_200, encounters: 120, secs: 7_200 });
        // now hourly counting starts
        let t = 100_000;
        st.record(t, local_day(t), 5);
        st.record(t + 60, local_day(t), 2);
        let h0 = local_hour(36_000);
        let est = st.hourly(h0, h0 + 2);
        assert_eq!(est, vec![60, 60, 0]);
        let h = local_hour(t);
        assert_eq!(st.hourly(h, h), vec![7]);
        st.unrecord(local_day(t), 3);
        assert_eq!(st.hourly(h, h), vec![4]);
        assert_eq!(fmt_hour(0), "12am");
        assert_eq!(fmt_hour(15), "3pm");
    }

    #[test]
    fn civil_round_trip() {
        for z in [-1000i64, 0, 1, 20_000, 20_767, 30_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(days_from_civil(2026, 11, 10), 20_767);
        assert_eq!(fmt_day(20_767), "10 Nov 2026");
    }

    #[test]
    fn time_and_sessions() {
        let mut s = RunStats::default();
        s.ensure_started(1000, 50, Some(2048.0));
        assert_eq!(s.base_count, 50);
        s.record(1000, 1, 2); // first: no time
        s.record(1030, 1, 2); // +30s
        s.record(1060, 1, 2); // +30s
        s.record(1060 + 600, 1, 2); // break: no time, same session
        s.record(1660 + 20, 2, 2); // +20s, next day
        assert_eq!(s.hunt_secs, 80);
        assert_eq!(s.sessions.len(), 1);
        assert_eq!(s.tracked_encounters(), 10);
        assert_eq!(s.today(1), (8, 60));
        assert_eq!(s.today(2), (2, 20));
        s.record(1680 + 4000, 2, 2); // >30 min: new session
        assert_eq!(s.sessions.len(), 2);
        assert_eq!(s.current_session(5680).unwrap().encounters, 2);
        assert!(s.current_session(5680 + 3600).is_none());
        s.unrecord(2, 1);
        assert_eq!(s.today(2), (3, 20));
        assert_eq!(s.days_hunted(), 2);
    }

    #[test]
    fn rate_and_eta() {
        let mut s = RunStats::default();
        // 1 encounter every 18s for an hour = 200/h
        for i in 0..=200u64 {
            s.record(10_000 + i * 18, 5, 1);
        }
        let r = s.per_hour().unwrap();
        assert!((r - 200.0).abs() < 0.5, "{r}");
        let (secs, k) = s.eta_to_odds(1048, 2048.0).unwrap();
        assert_eq!(k, 1);
        assert!((secs as f64 - 5.0 * 3600.0).abs() < 60.0, "{secs}");
        let (_, k) = s.eta_to_odds(2100, 2048.0).unwrap();
        assert_eq!(k, 2);
    }

    #[test]
    fn milestones_fire_once() {
        let mut s = RunStats::default();
        s.ensure_started(1, 2322, Some(2048.0)); // existing run: no catch-up fireworks
        assert!(s.check_milestones(2, 2322, Some(2048.0)).is_empty());
        assert_eq!(s.check_milestones(3, 3000, Some(2048.0)), vec![Reached::Thousand(3000)]);
        assert!(s.check_milestones(4, 2999, Some(2048.0)).is_empty());
        assert!(s.check_milestones(5, 3000, Some(2048.0)).is_empty());
        assert_eq!(
            s.check_milestones(6, 4096, Some(2048.0)),
            vec![Reached::Thousand(4000), Reached::Odds(2)]
        );
        let mut f = RunStats::default();
        f.ensure_started(1, 0, Some(2048.0));
        assert_eq!(f.check_milestones(2, 1000, Some(2048.0)), vec![Reached::Thousand(1000)]);
        assert_eq!(f.check_milestones(3, 2048, Some(2048.0)), vec![Reached::Thousand(2000), Reached::Odds(1)]);
        assert_eq!(f.milestones.len(), 3);
        assert_eq!(Reached::Odds(1).label(), "Reached the odds (1×)");
    }

    #[test]
    fn formatting() {
        assert_eq!(fmt_duration(59), "0m");
        assert_eq!(fmt_duration(45 * 60), "45m");
        assert_eq!(fmt_duration(6 * 3600 + 12 * 60), "6h 12m");
        assert_eq!(fmt_duration(51 * 3600), "2d 3h");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(fmt_clock(0), "12:00 am");
        assert_eq!(fmt_clock(13 * 3600 + 5 * 60), "1:05 pm");
    }
}
