//! Trainer progress: XP, levels, titles and achievements. Pure logic: the
//! app gathers lifetime stats into `Lifetime`, this decides what's earned.

use std::collections::BTreeSet;

/// Everything achievements and XP are based on, across all runs.
#[derive(Debug, Clone, Default)]
pub struct Lifetime {
    pub encounters: u64,
    pub hunt_secs: u64,
    pub shinies: u64,
    /// Highest whole multiple of the odds any run has reached.
    pub best_run_odds_x: u32,
    /// Most encounters in one day (all runs together).
    pub best_day: u64,
    /// Most encounters in one session.
    pub best_session: u64,
    /// Longest run of consecutive days with encounters.
    pub longest_streak: u32,
    /// Different Pokémon hunted.
    pub targets: u32,
    /// Smallest "encounters ÷ odds" of any caught shiny (luckiest).
    pub luckiest_catch: Option<f64>,
    /// Largest "encounters ÷ odds" of any caught shiny (most persistent).
    pub longest_catch: Option<f64>,
    /// Best encounters-per-hour over a session of 30+ minutes.
    pub best_rate: u32,
    /// One-off events that happened (night_owl, show_off, …).
    pub events: BTreeSet<String>,
    /// Battles fought (all runs).
    pub battles: u64,
    /// Trainer level (filled in by the app before checking level goals).
    pub level: u32,
    /// Different species in the Shiny Dex.
    pub dex_caught: u32,
    /// Regions whose obtainable Pokémon are all in the Shiny Dex
    /// (Kanto, Johto, Hoenn, Sinnoh, Modern).
    pub regions_done: [bool; 5],
    /// XP earned from daily goals and weekly challenges.
    pub goal_xp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Bronze,
    Silver,
    Gold,
    Platinum,
}

impl Tier {
    pub fn xp(self) -> u64 {
        match self {
            Tier::Bronze => 100,
            Tier::Silver => 250,
            Tier::Gold => 500,
            Tier::Platinum => 1000,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Tier::Bronze => "Bronze",
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::Platinum => "Platinum",
        }
    }
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Tier::Bronze => [205, 127, 50],
            Tier::Silver => [196, 202, 214],
            Tier::Gold => [246, 200, 66],
            Tier::Platinum => [140, 226, 230],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Encounters,
    Time,
    Shinies,
    Luck,
    Dedication,
    Fun,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Encounters,
        Category::Time,
        Category::Shinies,
        Category::Luck,
        Category::Dedication,
        Category::Fun,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Encounters => "Encounters",
            Category::Time => "Time hunting",
            Category::Shinies => "Shinies",
            Category::Luck => "Luck",
            Category::Dedication => "Dedication",
            Category::Fun => "Just for fun",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Goal {
    Encounters(u64),
    HuntHours(u64),
    Shinies(u64),
    RunOdds(u32),
    Day(u64),
    Session(u64),
    Streak(u32),
    Targets(u32),
    /// Catch a shiny at or below this fraction of the odds.
    Lucky(f64),
    /// Catch a shiny at or above this multiple of the odds.
    Persistent(f64),
    Rate(u32),
    Battles(u64),
    Level(u32),
    Dex(u32),
    Region(u8),
    Event(&'static str),
}

pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub tier: Tier,
    pub cat: Category,
    pub goal: Goal,
    /// Hidden until unlocked (description shows "???").
    pub secret: bool,
}

const fn a(id: &'static str, name: &'static str, desc: &'static str, tier: Tier, cat: Category, goal: Goal) -> Achievement {
    Achievement { id, name, desc, tier, cat, goal, secret: false }
}
const fn s(id: &'static str, name: &'static str, desc: &'static str, tier: Tier, cat: Category, goal: Goal) -> Achievement {
    Achievement { id, name, desc, tier, cat, goal, secret: true }
}

use Category as C;
use Goal as G;
use Tier as T;

pub const ACHIEVEMENTS: &[Achievement] = &[
    // Encounters
    a("enc_100", "First Steps", "Encounter 100 Pokémon", T::Bronze, C::Encounters, G::Encounters(100)),
    a("enc_1k", "Getting Warmed Up", "Encounter 1,000 Pokémon", T::Bronze, C::Encounters, G::Encounters(1_000)),
    a("enc_5k", "Dedicated", "Encounter 5,000 Pokémon", T::Silver, C::Encounters, G::Encounters(5_000)),
    a("enc_10k", "Grinder", "Encounter 10,000 Pokémon", T::Silver, C::Encounters, G::Encounters(10_000)),
    a("enc_25k", "Unstoppable", "Encounter 25,000 Pokémon", T::Gold, C::Encounters, G::Encounters(25_000)),
    a("enc_50k", "Living Legend", "Encounter 50,000 Pokémon", T::Gold, C::Encounters, G::Encounters(50_000)),
    a("enc_100k", "Six Digits", "Encounter 100,000 Pokémon", T::Platinum, C::Encounters, G::Encounters(100_000)),
    a("enc_250k", "Quarter Million", "Encounter 250,000 Pokémon", T::Platinum, C::Encounters, G::Encounters(250_000)),
    a("battles_500", "Battle Ready", "Fight 500 wild battles", T::Bronze, C::Encounters, G::Battles(500)),
    a("battles_5k", "Battle Hardened", "Fight 5,000 wild battles", T::Silver, C::Encounters, G::Battles(5_000)),
    a("battles_20k", "Battle Master", "Fight 20,000 wild battles", T::Gold, C::Encounters, G::Battles(20_000)),
    // Time
    a("time_1", "Clocked In", "Hunt for 1 hour", T::Bronze, C::Time, G::HuntHours(1)),
    a("time_10", "Committed", "Hunt for 10 hours", T::Bronze, C::Time, G::HuntHours(10)),
    a("time_24", "Full Day", "Hunt for 24 hours in total", T::Silver, C::Time, G::HuntHours(24)),
    a("time_50", "Half Century", "Hunt for 50 hours", T::Silver, C::Time, G::HuntHours(50)),
    a("time_100", "Centurion", "Hunt for 100 hours", T::Gold, C::Time, G::HuntHours(100)),
    a("time_250", "Shiny Lifestyle", "Hunt for 250 hours", T::Gold, C::Time, G::HuntHours(250)),
    a("time_500", "Timeless", "Hunt for 500 hours", T::Platinum, C::Time, G::HuntHours(500)),
    // Shinies
    a("shiny_1", "Sparkle!", "Catch your first shiny", T::Silver, C::Shinies, G::Shinies(1)),
    a("shiny_3", "Hat Trick", "Catch 3 shinies", T::Silver, C::Shinies, G::Shinies(3)),
    a("shiny_5", "Shiny Collector", "Catch 5 shinies", T::Gold, C::Shinies, G::Shinies(5)),
    a("shiny_10", "Shiny Showcase", "Catch 10 shinies", T::Gold, C::Shinies, G::Shinies(10)),
    a("shiny_25", "Dazzling", "Catch 25 shinies", T::Platinum, C::Shinies, G::Shinies(25)),
    a("shiny_50", "Shiny Hoard", "Catch 50 shinies", T::Platinum, C::Shinies, G::Shinies(50)),
    a("dex_10", "Dex Starter", "Have 10 different shinies in your Shiny Dex", T::Silver, C::Shinies, G::Dex(10)),
    a("dex_50", "Dex Devotee", "Have 50 different shinies in your Shiny Dex", T::Gold, C::Shinies, G::Dex(50)),
    a("dex_100", "Living Shiny Dex", "Have 100 different shinies in your Shiny Dex", T::Platinum, C::Shinies, G::Dex(100)),
    a("region_kanto", "Kanto Complete", "Every obtainable Kanto Pokémon in your Shiny Dex", T::Platinum, C::Shinies, G::Region(0)),
    a("region_johto", "Johto Complete", "Every obtainable Johto Pokémon in your Shiny Dex", T::Platinum, C::Shinies, G::Region(1)),
    a("region_hoenn", "Hoenn Complete", "Every obtainable Hoenn Pokémon in your Shiny Dex", T::Platinum, C::Shinies, G::Region(2)),
    a("region_sinnoh", "Sinnoh Complete", "Every obtainable Sinnoh Pokémon in your Shiny Dex", T::Platinum, C::Shinies, G::Region(3)),
    a("region_modern", "Modern Complete", "Every obtainable modern Pokémon in your Shiny Dex", T::Platinum, C::Shinies, G::Region(4)),
    a("level_10", "Rising Star", "Reach trainer level 10", T::Bronze, C::Shinies, G::Level(10)),
    a("level_25", "Seasoned", "Reach trainer level 25", T::Silver, C::Shinies, G::Level(25)),
    a("level_50", "Elite Hunter", "Reach trainer level 50", T::Gold, C::Shinies, G::Level(50)),
    a("level_100", "Champion", "Reach trainer level 100", T::Platinum, C::Shinies, G::Level(100)),
    // Luck
    a("luck_half", "Ahead of Schedule", "Catch a shiny in under half the odds", T::Bronze, C::Luck, G::Lucky(0.5)),
    a("luck_quarter", "Lucky Break", "Catch a shiny in under a quarter of the odds", T::Silver, C::Luck, G::Lucky(0.25)),
    a("luck_tenth", "Ridiculously Lucky", "Catch a shiny in under a tenth of the odds", T::Gold, C::Luck, G::Lucky(0.1)),
    a("luck_100", "Did That Just Happen?", "Catch a shiny within 1% of the odds", T::Platinum, C::Luck, G::Lucky(0.01)),
    a("odds_1", "Hit the Odds", "Reach the odds (1×) on a hunt", T::Bronze, C::Luck, G::RunOdds(1)),
    a("odds_2", "Double Trouble", "Reach 2× the odds on a hunt", T::Silver, C::Luck, G::RunOdds(2)),
    a("odds_3", "Triple Threat", "Reach 3× the odds on a hunt", T::Gold, C::Luck, G::RunOdds(3)),
    a("persist_3", "Never Give Up", "Catch a shiny after 3× the odds", T::Gold, C::Luck, G::Persistent(3.0)),
    a("persist_5", "Unshakeable", "Catch a shiny after 5× the odds", T::Platinum, C::Luck, G::Persistent(5.0)),
    // Dedication
    a("day_1k", "Big Day", "1,000 encounters in one day", T::Bronze, C::Dedication, G::Day(1_000)),
    a("day_2500", "Huge Day", "2,500 encounters in one day", T::Silver, C::Dedication, G::Day(2_500)),
    a("day_5k", "Monster Day", "5,000 encounters in one day", T::Gold, C::Dedication, G::Day(5_000)),
    a("day_10k", "Legendary Day", "10,000 encounters in one day", T::Platinum, C::Dedication, G::Day(10_000)),
    a("session_500", "Marathon", "500 encounters in one session", T::Bronze, C::Dedication, G::Session(500)),
    a("session_2k", "Ultramarathon", "2,000 encounters in one session", T::Silver, C::Dedication, G::Session(2_000)),
    a("session_5k", "Iron Will", "5,000 encounters in one session", T::Gold, C::Dedication, G::Session(5_000)),
    a("streak_3", "Habit Forming", "Hunt 3 days in a row", T::Bronze, C::Dedication, G::Streak(3)),
    a("streak_7", "Weekly Ritual", "Hunt 7 days in a row", T::Silver, C::Dedication, G::Streak(7)),
    a("streak_14", "Fortnight Focus", "Hunt 14 days in a row", T::Gold, C::Dedication, G::Streak(14)),
    a("streak_30", "Devoted", "Hunt 30 days in a row", T::Platinum, C::Dedication, G::Streak(30)),
    a("rate_300", "Quick Hands", "300 encounters an hour over a 30+ minute session", T::Bronze, C::Dedication, G::Rate(300)),
    a("rate_400", "Speed Hunter", "400 encounters an hour over a 30+ minute session", T::Silver, C::Dedication, G::Rate(400)),
    a("rate_500", "Lightning Fast", "500 encounters an hour over a 30+ minute session", T::Gold, C::Dedication, G::Rate(500)),
    a("targets_3", "Explorer", "Hunt 3 different Pokémon", T::Bronze, C::Dedication, G::Targets(3)),
    a("targets_10", "Variety Show", "Hunt 10 different Pokémon", T::Silver, C::Dedication, G::Targets(10)),
    a("targets_25", "Collector's Eye", "Hunt 25 different Pokémon", T::Gold, C::Dedication, G::Targets(25)),
    // Fun
    s("night_owl", "Night Owl", "Hunt between midnight and 5am", T::Bronze, C::Fun, G::Event("night_owl")),
    s("early_bird", "Early Bird", "Hunt between 5am and 7am", T::Bronze, C::Fun, G::Event("early_bird")),
    a("show_off", "Show Off", "Share or save a hunt image", T::Bronze, C::Fun, G::Event("show_off")),
    a("stylish", "Stylish", "Give the app your own colours", T::Bronze, C::Fun, G::Event("stylish")),
    s("comeback", "The Comeback", "Pick a saved hunt back up", T::Bronze, C::Fun, G::Event("comeback")),
    a("named", "Name Tag", "Give yourself a trainer name", T::Bronze, C::Fun, G::Event("named")),
    a("gif_maker", "Animator", "Make an animated share GIF", T::Bronze, C::Fun, G::Event("gif_maker")),
    s("celebrated", "Party Time", "Hit a milestone celebration", T::Bronze, C::Fun, G::Event("celebrated")),
    s("trophy_room", "Trophy Room", "Open a caught shiny in your Shiny Dex", T::Bronze, C::Fun, G::Event("trophy_room")),
    s("caught_it", "Gotcha!", "Press Caught! on a hunt", T::Silver, C::Fun, G::Event("caught_it")),
];

/// The Pokémon pictured on each achievement (all different; Gen 1-5 so the
/// sprites animate).
pub fn mascot(id: &str) -> &'static str {
    match id {
        "enc_100" => "pichu",
        "enc_1k" => "charmander",
        "enc_5k" => "machop",
        "enc_10k" => "excadrill",
        "enc_25k" => "tauros",
        "enc_50k" => "dragonite",
        "enc_100k" => "porygon-z",
        "time_1" => "hoothoot",
        "time_10" => "timburr",
        "time_24" => "solrock",
        "time_50" => "lunatone",
        "time_100" => "celebi",
        "time_250" => "slaking",
        "time_500" => "dialga",
        "shiny_1" => "gyarados",
        "shiny_3" => "dugtrio",
        "shiny_5" => "sableye",
        "shiny_10" => "milotic",
        "shiny_25" => "jirachi",
        "luck_half" => "jolteon",
        "luck_quarter" => "chansey",
        "luck_tenth" => "blissey",
        "luck_100" => "psyduck",
        "odds_1" => "spinda",
        "odds_2" => "doduo",
        "odds_3" => "dodrio",
        "persist_3" => "magikarp",
        "persist_5" => "shuckle",
        "day_1k" => "wailord",
        "day_2500" => "snorlax",
        "day_5k" => "tyranitar",
        "session_500" => "rapidash",
        "session_2k" => "arcanine",
        "streak_3" => "bidoof",
        "streak_7" => "bronzong",
        "streak_30" => "lucario",
        "rate_400" => "ninjask",
        "targets_3" => "sentret",
        "targets_10" => "eevee",
        "night_owl" => "noctowl",
        "early_bird" => "pidgey",
        "show_off" => "mr-mime",
        "stylish" => "smeargle",
        "comeback" => "ho-oh",
        "enc_250k" => "arceus",
        "battles_500" => "tyrogue",
        "battles_5k" => "hitmonlee",
        "battles_20k" => "hitmontop",
        "shiny_50" => "meowth",
        "level_10" => "staryu",
        "level_25" => "starmie",
        "level_50" => "metagross",
        "level_100" => "rayquaza",
        "day_10k" => "groudon",
        "session_5k" => "steelix",
        "streak_14" => "xatu",
        "rate_300" => "electrode",
        "rate_500" => "zapdos",
        "targets_25" => "unown",
        "named" => "chatot",
        "gif_maker" => "ditto",
        "celebrated" => "togepi",
        "trophy_room" => "kecleon",
        "caught_it" => "pikachu",
        "dex_10" => "rotom",
        "dex_50" => "porygon2",
        "dex_100" => "mew",
        "region_kanto" => "bulbasaur",
        "region_johto" => "chikorita",
        "region_hoenn" => "treecko",
        "region_sinnoh" => "turtwig",
        "region_modern" => "snivy",
        _ => "",
    }
}

pub fn find(id: &str) -> Option<&'static Achievement> {
    ACHIEVEMENTS.iter().find(|a| a.id == id)
}

/// (current, target) towards an achievement, for progress bars.
pub fn progress(a: &Achievement, l: &Lifetime) -> (f64, f64) {
    match a.goal {
        G::Encounters(n) => (l.encounters as f64, n as f64),
        G::HuntHours(h) => (l.hunt_secs as f64 / 3600.0, h as f64),
        G::Shinies(n) => (l.shinies as f64, n as f64),
        G::RunOdds(k) => (l.best_run_odds_x as f64, k as f64),
        G::Day(n) => (l.best_day as f64, n as f64),
        G::Session(n) => (l.best_session as f64, n as f64),
        G::Streak(n) => (l.longest_streak as f64, n as f64),
        G::Targets(n) => (l.targets as f64, n as f64),
        G::Rate(r) => (l.best_rate as f64, r as f64),
        G::Battles(n) => (l.battles as f64, n as f64),
        G::Level(n) => (l.level as f64, n as f64),
        G::Dex(n) => (l.dex_caught as f64, n as f64),
        G::Region(r) => (if l.regions_done.get(r as usize).copied().unwrap_or(false) { 1.0 } else { 0.0 }, 1.0),
        G::Lucky(f) => (if l.luckiest_catch.is_some_and(|m| m <= f) { 1.0 } else { 0.0 }, 1.0),
        G::Persistent(f) => (if l.longest_catch.is_some_and(|m| m >= f) { 1.0 } else { 0.0 }, 1.0),
        G::Event(e) => (if l.events.contains(e) { 1.0 } else { 0.0 }, 1.0),
    }
}

pub fn earned(a: &Achievement, l: &Lifetime) -> bool {
    let (c, t) = progress(a, l);
    c >= t
}

/// Which ladder an achievement belongs to (tiered ones share one, and are
/// listed in order). One-offs are their own series.
pub fn series_of(id: &str) -> &str {
    const LADDERS: [&str; 14] = [
        "enc", "battles", "time", "shiny", "level", "luck", "odds", "persist", "day", "session", "streak", "rate", "targets", "dex",
    ];
    match id.split_once('_') {
        Some((pre, _)) if LADDERS.contains(&pre) => pre,
        _ => id,
    }
}

/// All achievements in a series, in order.
pub fn series(name: &str) -> Vec<&'static Achievement> {
    ACHIEVEMENTS.iter().filter(|a| series_of(a.id) == name).collect()
}

/// Whether the goal is a yes/no thing (no progress bar).
pub fn is_binary(a: &Achievement) -> bool {
    matches!(a.goal, G::Lucky(_) | G::Persistent(_) | G::Event(_) | G::Region(_))
}

// ───────────────────────────── XP and levels ─────────────────────────────

pub const MAX_LEVEL: u32 = 100;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Xp {
    pub encounters: u64,
    pub time: u64,
    pub shinies: u64,
    pub achievements: u64,
    pub goals: u64,
}

impl Xp {
    pub fn total(&self) -> u64 {
        self.encounters + self.time + self.shinies + self.achievements + self.goals
    }
}

/// XP: 1 per encounter, 1 per minute hunted, 1,000 per shiny, plus achievements.
pub fn xp(l: &Lifetime, unlocked: &[&str]) -> Xp {
    Xp {
        encounters: l.encounters,
        time: l.hunt_secs / 60,
        shinies: l.shinies * 1000,
        achievements: unlocked.iter().filter_map(|id| find(id)).map(|a| a.tier.xp()).sum(),
        goals: l.goal_xp,
    }
}

/// Total XP needed to reach `level` (level 1 = 0).
pub fn xp_for(level: u32) -> u64 {
    let l = level.max(1) as u64;
    50 * (l - 1) * l
}

pub fn level_for(xp: u64) -> u32 {
    let mut l = 1;
    while l < MAX_LEVEL && xp >= xp_for(l + 1) {
        l += 1;
    }
    l
}

/// (xp into this level, xp needed for the next level).
pub fn level_progress(xp: u64) -> (u64, u64) {
    let l = level_for(xp);
    if l >= MAX_LEVEL {
        return (1, 1);
    }
    (xp - xp_for(l), xp_for(l + 1) - xp_for(l))
}

pub fn title(level: u32) -> &'static str {
    match level {
        0..=4 => "Rookie Trainer",
        5..=9 => "Shiny Seeker",
        10..=14 => "Sparkle Scout",
        15..=19 => "Shiny Spotter",
        20..=29 => "Shiny Hunter",
        30..=39 => "Veteran Hunter",
        40..=49 => "Shiny Expert",
        50..=64 => "Shiny Master",
        65..=79 => "Shiny Ace",
        80..=99 => "Shiny Legend",
        _ => "Shiny Champion",
    }
}

/// Badge metal for a level: (name, main colour, light colour).
pub fn badge_metal(level: u32) -> (&'static str, [u8; 3], [u8; 3]) {
    match level {
        0..=9 => ("Bronze", [176, 104, 44], [236, 168, 104]),
        10..=19 => ("Silver", [150, 158, 176], [232, 236, 244]),
        20..=34 => ("Gold", [214, 160, 28], [255, 230, 120]),
        35..=49 => ("Platinum", [96, 186, 196], [214, 250, 252]),
        50..=74 => ("Diamond", [70, 140, 240], [190, 230, 255]),
        _ => ("Master", [150, 80, 230], [255, 170, 240]),
    }
}

/// Longest run of consecutive day numbers in a sorted set.
pub fn longest_streak(days: &BTreeSet<i64>) -> u32 {
    let (mut best, mut cur, mut prev) = (0u32, 0u32, None::<i64>);
    for &d in days {
        cur = if prev == Some(d - 1) { cur + 1 } else { 1 };
        best = best.max(cur);
        prev = Some(d);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unique() {
        let mut seen = BTreeSet::new();
        for a in ACHIEVEMENTS {
            assert!(seen.insert(a.id), "duplicate {}", a.id);
            if let G::Event(e) = a.goal {
                assert_eq!(e, a.id);
            }
        }
        assert!(ACHIEVEMENTS.len() >= 60);
    }

    #[test]
    fn mascots_unique() {
        let mut seen = BTreeSet::new();
        for a in ACHIEVEMENTS {
            let m = mascot(a.id);
            assert!(!m.is_empty(), "no mascot for {}", a.id);
            assert!(seen.insert(m), "{} used twice", m);
        }
    }

    #[test]
    fn levels() {
        assert_eq!(xp_for(1), 0);
        assert_eq!(xp_for(2), 100);
        assert_eq!(xp_for(10), 4500);
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(99), 1);
        assert_eq!(level_for(100), 2);
        assert_eq!(level_for(4500), 10);
        assert_eq!(level_for(10_000_000), MAX_LEVEL);
        assert_eq!(level_progress(150), (50, 200));
        assert_eq!(title(1), "Rookie Trainer");
        assert_eq!(title(100), "Shiny Champion");
    }

    #[test]
    fn earning() {
        let mut l = Lifetime {
            encounters: 2_574,
            hunt_secs: 3 * 3600 + 5,
            best_run_odds_x: 1,
            ..Default::default()
        };
        let got: Vec<&str> = ACHIEVEMENTS.iter().filter(|a| earned(a, &l)).map(|a| a.id).collect();
        assert_eq!(got, vec!["enc_100", "enc_1k", "time_1", "odds_1"]);
        l.luckiest_catch = Some(0.2);
        l.events.insert("night_owl".into());
        assert!(earned(find("luck_quarter").unwrap(), &l));
        assert!(!earned(find("luck_tenth").unwrap(), &l));
        assert!(earned(find("night_owl").unwrap(), &l));
        let x = xp(&l, &["enc_100", "enc_1k", "odds_1"]);
        assert_eq!(x.encounters, 2574);
        assert_eq!(x.time, 180);
        assert_eq!(x.achievements, 300);
        let (c, t) = progress(find("enc_5k").unwrap(), &l);
        assert_eq!((c as u64, t as u64), (2574, 5000));
    }

    #[test]
    fn ladders_in_order() {
        assert_eq!(series_of("enc_5k"), "enc");
        assert_eq!(series_of("night_owl"), "night_owl");
        for name in ["enc", "time", "shiny", "day", "rate", "battles", "level"] {
            let s = series(name);
            assert!(s.len() >= 3, "{name}");
            // each step is at least as hard as the one before
            let l = Lifetime::default();
            let targets: Vec<f64> = s.iter().map(|a| progress(a, &l).1).collect();
            assert!(targets.windows(2).all(|w| w[0] <= w[1]), "{name}: {targets:?}");
            // the same category throughout
            assert!(s.iter().all(|a| a.cat == s[0].cat));
        }
    }

    #[test]
    fn streaks() {
        let d: BTreeSet<i64> = [1, 2, 3, 5, 6, 10].into_iter().collect();
        assert_eq!(longest_streak(&d), 3);
        assert_eq!(longest_streak(&BTreeSet::new()), 0);
    }
}
