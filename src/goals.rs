//! Daily goals and weekly challenges. Pure logic: the app passes in what
//! happened this week; this says what's done.

use std::collections::{BTreeMap, BTreeSet};

/// XP for meeting the daily goal / finishing a weekly challenge.
pub const DAILY_XP: u64 = 100;
pub const WEEKLY_XP: u64 = 250;

/// Monday-based week number for a local day number (day 0 = Thu 1 Jan 1970).
pub fn week_of(day: i64) -> i64 {
    (day + 3).div_euclid(7)
}

/// First and last day of a week.
pub fn week_days(week: i64) -> (i64, i64) {
    let start = week * 7 - 3;
    (start, start + 6)
}

/// What happened this week, gathered across every run.
#[derive(Debug, Clone, Default)]
pub struct WeekStats {
    /// day -> (encounters, seconds hunted)
    pub days: BTreeMap<i64, (u64, u64)>,
    /// (encounters, seconds) for each session that started this week
    pub sessions: Vec<(u64, u64)>,
    /// Pokémon hunted (with encounters) this week
    pub targets: BTreeSet<String>,
}

impl WeekStats {
    pub fn encounters(&self) -> u64 {
        self.days.values().map(|d| d.0).sum()
    }
    pub fn secs(&self) -> u64 {
        self.days.values().map(|d| d.1).sum()
    }
    pub fn active_days(&self) -> u32 {
        self.days.values().filter(|d| d.0 > 0).count() as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Encounters(u64),
    Days(u32),
    SessionMinutes(u64),
    SessionEncounters(u64),
    Targets(u32),
    Hours(u64),
    BigDay(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct Challenge {
    pub key: &'static str,
    pub title: &'static str,
    pub kind: Kind,
}

const POOL: [Challenge; 10] = [
    Challenge { key: "enc3k", title: "Encounter 3,000 Pokémon", kind: Kind::Encounters(3_000) },
    Challenge { key: "enc6k", title: "Encounter 6,000 Pokémon", kind: Kind::Encounters(6_000) },
    Challenge { key: "days4", title: "Hunt on 4 different days", kind: Kind::Days(4) },
    Challenge { key: "days6", title: "Hunt on 6 different days", kind: Kind::Days(6) },
    Challenge { key: "sess60", title: "Have a 1-hour hunting session", kind: Kind::SessionMinutes(60) },
    Challenge { key: "sess500", title: "500 encounters in one session", kind: Kind::SessionEncounters(500) },
    Challenge { key: "targets2", title: "Hunt 2 different Pokémon", kind: Kind::Targets(2) },
    Challenge { key: "hours5", title: "Hunt for 5 hours", kind: Kind::Hours(5) },
    Challenge { key: "hours10", title: "Hunt for 10 hours", kind: Kind::Hours(10) },
    Challenge { key: "bigday", title: "1,000 encounters in a single day", kind: Kind::BigDay(1_000) },
];

/// The three challenges for a week (the same for everyone, picked from the
/// pool by the week number; never two of the same kind).
pub fn challenges(week: i64) -> Vec<Challenge> {
    let mut x = (week as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    let mut out: Vec<Challenge> = Vec::new();
    let mut guard = 0;
    while out.len() < 3 && guard < 200 {
        guard += 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let c = POOL[(x % POOL.len() as u64) as usize];
        let same_kind = out.iter().any(|o| std::mem::discriminant(&o.kind) == std::mem::discriminant(&c.kind));
        if !same_kind {
            out.push(c);
        }
    }
    out
}

/// (current, target) for a challenge.
pub fn progress(c: &Challenge, w: &WeekStats) -> (f64, f64) {
    match c.kind {
        Kind::Encounters(n) => (w.encounters() as f64, n as f64),
        Kind::Days(n) => (w.active_days() as f64, n as f64),
        Kind::SessionMinutes(m) => (w.sessions.iter().map(|s| s.1 / 60).max().unwrap_or(0) as f64, m as f64),
        Kind::SessionEncounters(n) => (w.sessions.iter().map(|s| s.0).max().unwrap_or(0) as f64, n as f64),
        Kind::Targets(n) => (w.targets.len() as f64, n as f64),
        Kind::Hours(h) => (w.secs() as f64 / 3600.0, h as f64),
        Kind::BigDay(n) => (w.days.values().map(|d| d.0).max().unwrap_or(0) as f64, n as f64),
    }
}

pub fn done(c: &Challenge, w: &WeekStats) -> bool {
    let (a, b) = progress(c, w);
    a >= b
}

/// Days in a row (ending today, or yesterday if today isn't done yet) on
/// which the daily goal was met.
pub fn streak(done_days: &BTreeSet<i64>, today: i64) -> u32 {
    let mut d = if done_days.contains(&today) { today } else { today - 1 };
    let mut n = 0;
    while done_days.contains(&d) {
        n += 1;
        d -= 1;
    }
    n
}

pub fn day_key(day: i64) -> String {
    format!("day:{day}")
}

pub fn week_key(week: i64, c: &Challenge) -> String {
    format!("week:{week}:{}", c.key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weeks() {
        // 10 Nov 2026 (day 20767) is a Tuesday: its week starts Monday 9 Nov.
        let w = week_of(20_767);
        let (a, b) = week_days(w);
        assert_eq!(a, 20_766);
        assert_eq!(b, 20_772);
        assert_eq!(week_of(20_766), w);
        assert_eq!(week_of(20_765), w - 1);
        assert_eq!(week_of(20_772), w);
    }

    #[test]
    fn three_different_challenges() {
        for w in 2900..3000 {
            let c = challenges(w);
            assert_eq!(c.len(), 3);
            for i in 0..3 {
                for j in i + 1..3 {
                    assert_ne!(std::mem::discriminant(&c[i].kind), std::mem::discriminant(&c[j].kind));
                }
            }
        }
        // stable for a given week
        assert_eq!(challenges(2967)[0].key, challenges(2967)[0].key);
    }

    #[test]
    fn progress_and_streak() {
        let mut w = WeekStats::default();
        w.days.insert(1, (800, 3600));
        w.days.insert(2, (1200, 7200));
        w.sessions = vec![(300, 1800), (600, 4000)];
        w.targets.insert("riolu".into());
        let enc = Challenge { key: "x", title: "", kind: Kind::Encounters(2000) };
        assert!(done(&enc, &w));
        let sess = Challenge { key: "x", title: "", kind: Kind::SessionMinutes(60) };
        assert!(done(&sess, &w));
        let big = Challenge { key: "x", title: "", kind: Kind::BigDay(1000) };
        assert!(done(&big, &w));
        let t2 = Challenge { key: "x", title: "", kind: Kind::Targets(2) };
        assert!(!done(&t2, &w));
        let d: BTreeSet<i64> = [5, 6, 7, 9].into_iter().collect();
        assert_eq!(streak(&d, 9), 1);
        assert_eq!(streak(&d, 8), 3);
        assert_eq!(streak(&d, 10), 1);
        assert_eq!(streak(&d, 12), 0);
    }
}
