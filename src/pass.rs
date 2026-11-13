//! The free battle pass: a 12-week season with daily and weekly missions
//! and a 30-tier reward track. Pure logic: the app passes in what happened;
//! this says what's open, what's done and what's earned.
//!
//! Every mission done is pass XP. Doing every mission of the season is
//! exactly enough for tier 30.

use crate::goals::{Kind, WeekStats};

pub const WEEKS: i64 = 12;
pub const DAYS: i64 = WEEKS * 7;
pub const TIERS: u32 = 30;

pub const DAILY_PER_DAY: usize = 3;
pub const WEEKLY_PER_WEEK: usize = 5;
/// Pass XP for a mission.
pub const DAILY_PASS_XP: u64 = 100;
pub const WEEKLY_PASS_XP: u64 = 300;
/// Trainer XP for a mission (the daily goal keeps its old 100).
pub const DAILY_XP: u64 = 50;
pub const GOAL_XP: u64 = 100;
pub const WEEKLY_XP: u64 = 250;

/// All the pass XP there is in a season.
pub const SEASON_XP: u64 = DAYS as u64 * DAILY_PER_DAY as u64 * DAILY_PASS_XP + WEEKS as u64 * WEEKLY_PER_WEEK as u64 * WEEKLY_PASS_XP;
/// Pass XP per tier.
pub const TIER_XP: u64 = SEASON_XP / TIERS as u64;

// ───────────────────────────── seasons ─────────────────────────────

/// A season: 12 weeks, the same dates for everyone, with its own 30
/// rewards. A new season is one more entry in `SEASONS`; the old ones stay,
/// so what players earned in them stays theirs (and anything they reached
/// can still be claimed), shown on the old season's own Locker page.
#[derive(Debug)]
pub struct Season {
    pub num: u32,
    pub name: &'static str,
    /// Local day number of its first day.
    pub start_day: i64,
    pub rewards: &'static [Reward],
}

/// Every season, oldest first.
pub static SEASONS: [Season; 1] = [
    // Two Worlds: Friday 13 Nov 2026 to Thursday 4 Feb 2027.
    Season { num: 1, name: "Two Worlds", start_day: 20_770, rewards: &REWARDS },
];

/// The season the pass shows: the newest that has started (it stays the
/// one shown after it ends, until the next one starts), or the first.
pub fn current(today: i64) -> &'static Season {
    SEASONS.iter().rev().find(|s| s.start_day <= today).unwrap_or(&SEASONS[0])
}

/// The seasons that have started, newest first (at least the first).
pub fn started(today: i64) -> Vec<&'static Season> {
    let mut v: Vec<&'static Season> = SEASONS.iter().filter(|s| s.start_day <= today).collect();
    if v.is_empty() {
        v.push(&SEASONS[0]);
    }
    v.reverse();
    v
}

/// The look of a battle pass title, by its name (None for other titles).
pub fn title_theme(name: &str) -> Option<Theme> {
    all_rewards().find(|r| r.kind == Kind2::Title && r.name == name).map(|r| r.theme)
}

/// Every reward of every season.
pub fn all_rewards() -> impl Iterator<Item = &'static Reward> {
    SEASONS.iter().flat_map(|s| s.rewards.iter())
}

impl Season {
    pub fn last_day(&self) -> i64 {
        self.start_day + DAYS - 1
    }

    /// Whether `day` is in the season.
    pub fn active(&self, day: i64) -> bool {
        (self.start_day..=self.last_day()).contains(&day)
    }

    /// Whether it's over.
    pub fn ended(&self, today: i64) -> bool {
        today > self.last_day()
    }

    /// The season's week (0..12) of a day, if it's in the season.
    pub fn week_of(&self, day: i64) -> Option<i64> {
        self.active(day).then(|| (day - self.start_day) / 7)
    }

    /// First and last day of a season week.
    pub fn week_days(&self, week: i64) -> (i64, i64) {
        let a = self.start_day + week * 7;
        (a, a + 6)
    }

    /// Days left in the season (0 once it's over).
    pub fn days_left(&self, today: i64) -> i64 {
        (self.last_day() - today + 1).clamp(0, DAYS)
    }

    /// A day's three missions: the daily goal, then two more (different
    /// kinds; the same for everyone).
    pub fn daily(&self, day: i64) -> Vec<Mission> {
        daily_for(day, self.salt())
    }

    /// A week's five missions (different kinds).
    pub fn weekly(&self, week: i64) -> Vec<Mission> {
        weekly_for(week, self.salt())
    }

    /// Season 1 keeps the plain seeds, so its missions never change.
    fn salt(&self) -> u64 {
        (self.num as u64).saturating_sub(1) << 40
    }

    /// The key a done mission is kept under. The daily goal keeps its old
    /// "day:" key and weekly missions start with "week:", so the goal
    /// achievements and streaks carry on counting them.
    pub fn daily_key(&self, day: i64, m: &Mission) -> String {
        match m.ask {
            Ask::DailyGoal => crate::goals::day_key(day),
            _ => format!("pass:{}:d{day}:{}", self.num, m.key),
        }
    }

    pub fn weekly_key(&self, week: i64, m: &Mission) -> String {
        format!("week:s{}w{week}:{}", self.num, m.key)
    }

    /// Pass XP a done-mission key is worth in this season (0 for anything else).
    pub fn xp_of(&self, key: &str) -> u64 {
        if let Some(rest) = key.strip_prefix(&format!("week:s{}w", self.num)) {
            return if rest.contains(':') { WEEKLY_PASS_XP } else { 0 };
        }
        if key.starts_with(&format!("pass:{}:d", self.num)) {
            return DAILY_PASS_XP;
        }
        if let Some(d) = key.strip_prefix("day:").and_then(|d| d.parse::<i64>().ok()) {
            return if self.active(d) { DAILY_PASS_XP } else { 0 };
        }
        0
    }
}

// ───────────────────────────── missions ─────────────────────────────

/// What a mission asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ask {
    /// The daily goal: encounters today (the amount comes from settings).
    DailyGoal,
    /// Hunt for this many minutes in the day.
    Minutes(u64),
    /// One session of this many minutes.
    SessionMinutes(u64),
    /// One session with this many encounters.
    SessionEncounters(u64),
    /// This many separate sessions.
    Sessions(u32),
    /// This many different Pokémon.
    Targets(u32),
    /// A weekly challenge (counted from when it opened).
    Week(Kind),
}

#[derive(Debug, Clone, Copy)]
pub struct Mission {
    pub key: &'static str,
    pub title: &'static str,
    pub ask: Ask,
}

const fn m(key: &'static str, title: &'static str, ask: Ask) -> Mission {
    Mission { key, title, ask }
}

const DAILY_POOL: [Mission; 9] = [
    m("min30", "Hunt for 30 minutes", Ask::Minutes(30)),
    m("min45", "Hunt for 45 minutes", Ask::Minutes(45)),
    m("min60", "Hunt for an hour", Ask::Minutes(60)),
    m("sess20", "Have a 20-minute session", Ask::SessionMinutes(20)),
    m("sess150", "150 encounters in one session", Ask::SessionEncounters(150)),
    m("sess300", "300 encounters in one session", Ask::SessionEncounters(300)),
    m("sessions2", "Hunt in 2 separate sessions", Ask::Sessions(2)),
    m("targets2", "Hunt 2 different Pokémon", Ask::Targets(2)),
    m("sess40", "Have a 40-minute session", Ask::SessionMinutes(40)),
];

const WEEKLY_POOL: [Mission; 14] = [
    m("enc3k", "Encounter 3,000 Pokémon", Ask::Week(Kind::Encounters(3_000))),
    m("enc5k", "Encounter 5,000 Pokémon", Ask::Week(Kind::Encounters(5_000))),
    m("days4", "Hunt on 4 different days", Ask::Week(Kind::Days(4))),
    m("days6", "Hunt on 6 different days", Ask::Week(Kind::Days(6))),
    m("sess60", "Have a 1-hour session", Ask::Week(Kind::SessionMinutes(60))),
    m("sess90", "Have a 90-minute session", Ask::Week(Kind::SessionMinutes(90))),
    m("sess500", "500 encounters in one session", Ask::Week(Kind::SessionEncounters(500))),
    m("sess800", "800 encounters in one session", Ask::Week(Kind::SessionEncounters(800))),
    m("targets3", "Hunt 3 different Pokémon", Ask::Week(Kind::Targets(3))),
    m("targets4", "Hunt 4 different Pokémon", Ask::Week(Kind::Targets(4))),
    m("hours5", "Hunt for 5 hours", Ask::Week(Kind::Hours(5))),
    m("hours8", "Hunt for 8 hours", Ask::Week(Kind::Hours(8))),
    m("bigday1k", "1,000 encounters in a single day", Ask::Week(Kind::BigDay(1_000))),
    m("bigday1500", "1,500 encounters in a single day", Ask::Week(Kind::BigDay(1_500))),
];

fn rng(seed: u64) -> impl FnMut() -> u64 {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}

fn kind_of(a: &Ask) -> u8 {
    match a {
        Ask::DailyGoal => 0,
        Ask::Minutes(_) => 1,
        Ask::SessionMinutes(_) => 2,
        Ask::SessionEncounters(_) => 3,
        Ask::Sessions(_) => 4,
        Ask::Targets(_) => 5,
        Ask::Week(k) => match k {
            Kind::Encounters(_) => 10,
            Kind::Days(_) => 11,
            Kind::SessionMinutes(_) => 12,
            Kind::SessionEncounters(_) => 13,
            Kind::Targets(_) => 14,
            Kind::Hours(_) => 15,
            Kind::BigDay(_) => 16,
        },
    }
}

fn daily_for(day: i64, salt: u64) -> Vec<Mission> {
    let mut out = vec![m("goal", "Reach your daily goal", Ask::DailyGoal)];
    let mut next = rng(day as u64 ^ 0x5EA5_0001 ^ salt);
    let mut guard = 0;
    while out.len() < DAILY_PER_DAY && guard < 200 {
        guard += 1;
        let c = DAILY_POOL[(next() % DAILY_POOL.len() as u64) as usize];
        if !out.iter().any(|o| kind_of(&o.ask) == kind_of(&c.ask)) {
            out.push(c);
        }
    }
    out
}

fn weekly_for(week: i64, salt: u64) -> Vec<Mission> {
    let mut out: Vec<Mission> = Vec::new();
    let mut next = rng(week as u64 ^ 0x5EA5_1000 ^ salt);
    let mut guard = 0;
    while out.len() < WEEKLY_PER_WEEK && guard < 400 {
        guard += 1;
        let c = WEEKLY_POOL[(next() % WEEKLY_POOL.len() as u64) as usize];
        if !out.iter().any(|o| kind_of(&o.ask) == kind_of(&c.ask)) {
            out.push(c);
        }
    }
    out
}

/// What happened on one day, for daily missions.
#[derive(Debug, Clone, Default)]
pub struct DayStats {
    pub encounters: u64,
    pub secs: u64,
    /// (encounters, seconds) of each session that started that day
    pub sessions: Vec<(u64, u64)>,
    pub targets: usize,
}

/// (current, target) of a daily mission. `goal` is the daily goal amount.
pub fn daily_progress(m: &Mission, d: &DayStats, goal: u64) -> (f64, f64) {
    match m.ask {
        Ask::DailyGoal => (d.encounters as f64, goal.max(1) as f64),
        Ask::Minutes(n) => ((d.secs / 60) as f64, n as f64),
        Ask::SessionMinutes(n) => (d.sessions.iter().map(|s| s.1 / 60).max().unwrap_or(0) as f64, n as f64),
        Ask::SessionEncounters(n) => (d.sessions.iter().map(|s| s.0).max().unwrap_or(0) as f64, n as f64),
        Ask::Sessions(n) => (d.sessions.iter().filter(|s| s.0 > 0).count() as f64, n as f64),
        Ask::Targets(n) => (d.targets as f64, n as f64),
        Ask::Week(_) => (0.0, 1.0),
    }
}

/// (current, target) of a weekly mission, from everything since it opened.
pub fn weekly_progress(m: &Mission, w: &WeekStats) -> (f64, f64) {
    match m.ask {
        Ask::Week(kind) => crate::goals::progress(&crate::goals::Challenge { key: m.key, title: m.title, kind }, w),
        _ => (0.0, 1.0),
    }
}

/// Tier reached with this much pass XP (0..=30).
pub fn tier(xp: u64) -> u32 {
    ((xp / TIER_XP) as u32).min(TIERS)
}

// ───────────────────────────── rewards ─────────────────────────────

/// The season's two looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Giratina's upside-down world: violet, crimson and gold.
    Distortion,
    /// Northern lights over Snowpoint City: icy teal, soft violet and white.
    Aurora,
    /// Both at once.
    Both,
}

impl Theme {
    /// The colour titles from this look are shown in.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Theme::Distortion => [222, 44, 78],
            Theme::Aurora => [46, 210, 194],
            Theme::Both => [160, 130, 255],
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Theme::Distortion => "Distortion World",
            Theme::Aurora => "Snowpoint Aurora",
            Theme::Both => "Two Worlds",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind2 {
    Plate,
    Frame,
    Title,
    /// A hunt share image style (animated or not).
    Style(crate::card::CardStyle),
}

#[derive(Debug, Clone, Copy)]
pub struct Reward {
    pub tier: u32,
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind2,
    pub theme: Theme,
}

const fn rw(tier: u32, id: &'static str, name: &'static str, kind: Kind2, theme: Theme) -> Reward {
    Reward { tier, id, name, kind, theme }
}

use crate::card::CardStyle as S;
use Kind2::{Frame, Plate, Style, Title};
use Theme::{Aurora as A, Both as B, Distortion as D};

/// Season 1's free track: one reward a tier.
pub static REWARDS: [Reward; 30] = [
    rw(1, "plate_rift", "Rift Plate", Plate, D),
    rw(2, "frame_aurora_ring", "Aurora Ring", Frame, A),
    rw(3, "title_rift_walker", "Rift Walker", Title, D),
    rw(4, "plate_snowpoint", "Snowpoint Plate", Plate, A),
    rw(5, "frame_shadow_ring", "Shadow Ring", Frame, D),
    rw(6, "title_aurora_chaser", "Aurora Chaser", Title, A),
    rw(7, "plate_reverse", "Reverse World", Plate, D),
    rw(8, "frame_frost_crystal", "Frost Crystal", Frame, A),
    rw(9, "title_distortion_diver", "Distortion Diver", Title, D),
    rw(10, "style_distortion", "Distortion World", Style(S::Distortion), D),
    rw(11, "frame_griseous", "Griseous Gold", Frame, D),
    rw(12, "plate_northern_lights", "Northern Lights", Plate, A),
    rw(13, "title_snowpoint_sentinel", "Snowpoint Sentinel", Title, A),
    rw(14, "plate_tendril", "Shadow Tendrils", Plate, D),
    rw(15, "frame_diamond_dust", "Diamond Dust", Frame, A),
    rw(16, "title_renegades_herald", "Renegade's Herald", Title, D),
    rw(17, "plate_polar_night", "Polar Night", Plate, A),
    rw(18, "frame_rift_eye", "Rift Eye", Frame, D),
    rw(19, "title_frostbound", "Frostbound", Title, A),
    rw(20, "style_aurora", "Snowpoint Aurora", Style(S::Aurora), A),
    rw(21, "plate_griseous_crown", "Griseous Crown", Plate, D),
    rw(22, "frame_borealis", "Borealis Halo", Frame, A),
    rw(23, "title_worldbreaker", "Worldbreaker", Title, D),
    rw(24, "plate_crystal_veil", "Crystal Veil", Plate, A),
    rw(25, "frame_eclipse", "Eclipse Ring", Frame, D),
    rw(26, "title_borealis_keeper", "Borealis Keeper", Title, A),
    rw(27, "plate_twin_worlds", "Twin Worlds", Plate, B),
    rw(28, "frame_origin_halo", "Origin Halo", Frame, B),
    rw(29, "style_distortion_live", "Distortion World · Animated", Style(S::DistortionLive), D),
    rw(30, "style_aurora_live", "Snowpoint Aurora · Animated", Style(S::AuroraLive), A),
];

/// The share image styles that come from the pass (locked until claimed).
pub fn style_reward(s: crate::card::CardStyle) -> Option<&'static Reward> {
    all_rewards().find(|r| r.kind == Kind2::Style(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_never_share_a_reward_id() {
        // owned rewards are kept by id, so a new season reusing an id would
        // hand players it for free (or take the old one's look away)
        let mut seen = std::collections::HashSet::new();
        for r in all_rewards() {
            assert!(seen.insert(r.id), "reward id {} is used twice", r.id);
        }
        for w in SEASONS.windows(2) {
            assert!(w[0].num < w[1].num && w[0].last_day() < w[1].start_day, "seasons go in order and don't overlap");
        }
    }

    #[test]
    fn season_maths() {
        assert_eq!(SEASON_XP, 43_200);
        assert_eq!(TIER_XP * TIERS as u64, SEASON_XP);
        assert_eq!(tier(SEASON_XP), 30);
        assert_eq!(tier(SEASON_XP - 1), 29);
        let s1 = &SEASONS[0];
        let sd = s1.start_day;
        assert_eq!(s1.week_of(sd), Some(0));
        assert_eq!(s1.week_of(sd + 83), Some(11));
        assert_eq!(s1.week_of(sd + 84), None);
        assert_eq!(s1.week_of(sd - 1), None);
        assert!(std::ptr::eq(current(sd + 200), s1), "an ended season stays the one shown until the next");
        let ids: Vec<&str> = all_rewards().map(|r| r.id).collect();
        for (i, a) in ids.iter().enumerate() {
            assert!(!ids[i + 1..].contains(a), "{a} twice");
        }
        // every reward tier once, in order
        for (i, r) in REWARDS.iter().enumerate() {
            assert_eq!(r.tier, i as u32 + 1);
        }
    }

    #[test]
    fn missions_differ_and_are_stable() {
        let s1 = &SEASONS[0];
        for d in s1.start_day..=s1.last_day() {
            let ms = s1.daily(d);
            assert_eq!(ms.len(), DAILY_PER_DAY);
            assert!(matches!(ms[0].ask, Ask::DailyGoal));
            for i in 0..ms.len() {
                for j in i + 1..ms.len() {
                    assert_ne!(kind_of(&ms[i].ask), kind_of(&ms[j].ask));
                }
            }
            assert_eq!(s1.daily(d)[1].key, ms[1].key);
        }
        for w in 0..WEEKS {
            let ms = s1.weekly(w);
            assert_eq!(ms.len(), WEEKLY_PER_WEEK);
        }
    }

    #[test]
    fn keys_and_xp() {
        let s1 = &SEASONS[0];
        let sd = s1.start_day;
        let d = s1.daily(sd);
        assert_eq!(s1.xp_of(&s1.daily_key(sd, &d[0])), DAILY_PASS_XP);
        assert_eq!(s1.xp_of(&s1.daily_key(sd, &d[1])), DAILY_PASS_XP);
        assert_eq!(s1.xp_of(&crate::goals::day_key(sd - 1)), 0);
        let w = s1.weekly(0);
        assert_eq!(s1.xp_of(&s1.weekly_key(0, &w[0])), WEEKLY_PASS_XP);
        assert_eq!(s1.xp_of("week:2967:enc3k"), 0);
        // the keys season 1 has always used
        assert_eq!(s1.weekly_key(0, &w[0]), format!("week:s1w0:{}", w[0].key));
        assert_eq!(s1.daily_key(sd, &d[1]), format!("pass:1:d{sd}:{}", d[1].key));
    }
}
