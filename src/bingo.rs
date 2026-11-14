//! Shiny Bingo: a new 5×5 card every day at the game's daily reset (columns
//! S H I N Y, a free square in the middle) whose squares tick themselves
//! while you hunt, from the game's session log and the app's own hunting
//! time, missions and shinies. A full row, column or diagonal wins a little
//! reward, the first one each day also the daily reward (a booster pack or
//! battle pass XP, your pick), and all 25 is a blackout.
//!
//! The card comes from the day's number alone (the game server's calendar),
//! so everyone has the same card that day. Only progress is saved. Pure
//! logic: the app feeds in what happened and pays out what this says was won.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const SIZE: usize = 5;
pub const SQUARES: usize = SIZE * SIZE;
/// The free square in the middle.
pub const FREE: usize = 12;
/// The column letters.
pub const LETTERS: [&str; SIZE] = ["S", "H", "I", "N", "Y"];

/// What each line wins, and a blackout.
pub const LINE_CARD_XP: u64 = 50;
pub const LINE_TREATS: u32 = 1;
pub const BLACKOUT_PACKS: u32 = 1;

/// Battle pass XP for the daily reward's XP choice.
pub const DAILY_PASS_XP: u64 = 150;
/// Booster packs for the daily reward's pack choice.
pub const DAILY_PACKS: u32 = 1;

/// Past cards kept.
pub const HISTORY_MAX: usize = 30;
/// Latest ticks kept.
pub const RECENT_MAX: usize = 20;

/// The card on show at `now` (unix seconds): the day on the game server's
/// calendar (Central European time), so it changes at the daily reset.
pub fn card_day(now: i64) -> i64 {
    (now + crate::worldclock::SERVER.offset_at(now)).div_euclid(86_400)
}

// ───────────────────────────── squares ─────────────────────────────

/// What a square asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// Meet this Pokémon (National Dex number).
    Meet(u32),
    /// Meet this many Pokémon of a type (index into `TYPES`).
    Type(u8, u32),
    /// This many encounters on rare grass.
    RareGrass(u32),
    /// A double battle.
    Double,
    /// A Pokémon that evolved from a Sweet Scent roll on rare grass.
    Evolve,
    /// A Pokémon at this level or higher.
    LevelAtLeast(u32),
    /// A Pokémon under this level.
    LevelUnder(u32),
    /// A Pokémon whose name starts with this letter.
    Letter(char),
    /// This many encounters in one game session.
    Session(u32),
    /// This many encounters on today's card.
    Today(u32),
    /// This many different Pokémon today.
    Different(u32),
    /// Hunt this many minutes today (the app's hunting time).
    HuntMinutes(u32),
    /// An encounter after 9 pm (to 5 am).
    Late,
    /// An encounter between 5 and 8 am.
    Early,
    /// Find a shiny (the hard square).
    Shiny,
    /// Finish this many battle pass dailies in one day.
    Dailies(u32),
}

/// Types a square can ask for: (name, how many to meet).
pub const TYPES: [(&str, u32); 14] = [
    ("Normal", 10),
    ("Flying", 10),
    ("Water", 5),
    ("Psychic", 5),
    ("Ground", 5),
    ("Poison", 10),
    ("Bug", 3),
    ("Grass", 3),
    ("Rock", 3),
    ("Ghost", 3),
    ("Ice", 3),
    ("Fighting", 3),
    ("Electric", 1),
    ("Steel", 3),
];

/// Pokémon a square can ask you to meet: ones out in the wild in lots of
/// places (from the game's wild tables, without the world options).
pub const MEET: [(u32, &str); 32] = [
    (41, "Zubat"),
    (42, "Golbat"),
    (74, "Geodude"),
    (75, "Graveler"),
    (92, "Gastly"),
    (93, "Haunter"),
    (95, "Onix"),
    (66, "Machop"),
    (67, "Machoke"),
    (35, "Clefairy"),
    (43, "Oddish"),
    (54, "Psyduck"),
    (163, "Hoothoot"),
    (164, "Noctowl"),
    (194, "Wooper"),
    (195, "Quagsire"),
    (215, "Sneasel"),
    (265, "Wurmple"),
    (307, "Meditite"),
    (315, "Roselia"),
    (355, "Duskull"),
    (361, "Snorunt"),
    (396, "Starly"),
    (397, "Staravia"),
    (399, "Bidoof"),
    (400, "Bibarel"),
    (401, "Kricketot"),
    (403, "Shinx"),
    (406, "Budew"),
    (418, "Buizel"),
    (433, "Chingling"),
    (436, "Bronzor"),
];

/// Letters a square can ask for (plenty of Pokémon start with each).
const LETTER_POOL: [char; 9] = ['B', 'C', 'G', 'H', 'K', 'M', 'P', 'S', 'W'];

impl Goal {
    /// How much it takes.
    pub fn need(self) -> u32 {
        match self {
            Goal::Type(_, n) | Goal::RareGrass(n) | Goal::Session(n) | Goal::Today(n) | Goal::Different(n) | Goal::HuntMinutes(n) | Goal::Dailies(n) => n.max(1),
            _ => 1,
        }
    }

    /// The square's text.
    pub fn text(self) -> String {
        match self {
            Goal::Meet(d) => format!("Meet a {}", meet_name(d)),
            Goal::Type(t, 1) => {
                let name = TYPES[t as usize % TYPES.len()].0;
                let an = name.starts_with(['A', 'E', 'I', 'O', 'U']);
                format!("Meet {} {name}-type", if an { "an" } else { "a" })
            }
            Goal::Type(t, n) => format!("Meet {n} {}-types", TYPES[t as usize % TYPES.len()].0),
            Goal::RareGrass(1) => "An encounter on rare grass".into(),
            Goal::RareGrass(n) => format!("{n} encounters on rare grass"),
            Goal::Double => "A double battle".into(),
            Goal::Evolve => "A Sweet Scent evolution".into(),
            Goal::LevelAtLeast(l) => format!("Meet a Pokémon at level {l}+"),
            Goal::LevelUnder(l) => format!("Meet a Pokémon under level {l}"),
            Goal::Letter(c) => format!("A Pokémon starting with {c}"),
            Goal::Session(n) => format!("{} encounters in one session", thousands(n)),
            Goal::Today(n) => format!("{} encounters today", thousands(n)),
            Goal::Different(n) => format!("{n} different Pokémon today"),
            Goal::HuntMinutes(m) if m % 60 == 0 && m > 60 => format!("Hunt for {} hours today", m / 60),
            Goal::HuntMinutes(m) if m == 60 => "Hunt for an hour today".into(),
            Goal::HuntMinutes(m) => format!("Hunt for {m} minutes today"),
            Goal::Late => "Hunt after 9 pm".into(),
            Goal::Early => "Hunt before 8 am".into(),
            Goal::Shiny => "Find a shiny".into(),
            Goal::Dailies(n) => format!("Finish {n} battle pass dailies"),
        }
    }

    /// A few words on how it ticks (the square's tooltip).
    pub fn how(self) -> &'static str {
        match self {
            Goal::Meet(_) | Goal::Type(..) | Goal::LevelAtLeast(_) | Goal::LevelUnder(_) | Goal::Letter(_) => "Any wild Pokémon you meet in the game counts, read from the game's log.",
            Goal::RareGrass(_) => "Encounters on rare grass (Sweet Scent on a rare grass patch), from the game's log.",
            Goal::Double => "Two wild Pokémon in one battle, from the game's log.",
            Goal::Evolve => "When a Sweet Scent roll on rare grass turns a Pokémon into its evolution, from the game's log.",
            Goal::Session(_) => "Encounters in one game session (from opening the game to closing it).",
            Goal::Today(_) => "Every encounter since today's card came out.",
            Goal::Different(_) => "Different Pokémon met since today's card came out.",
            Goal::HuntMinutes(_) => "Today's hunting time, as the Shiny Hunter counts it.",
            Goal::Late | Goal::Early => "Meet a wild Pokémon at that time of day (the game log's clock).",
            Goal::Shiny => "Catch a shiny and mark it caught in the Shiny Hunter.",
            Goal::Dailies(_) => "Today's battle pass daily missions.",
        }
    }

    /// A short label for the square's badge.
    pub fn badge(self) -> String {
        match self {
            Goal::Meet(_) => String::new(),
            Goal::Type(t, _) => TYPES[t as usize % TYPES.len()].0[..3].to_uppercase(),
            Goal::RareGrass(1) => "RG".into(),
            Goal::RareGrass(n) => format!("RG{n}"),
            Goal::Double => "×2".into(),
            Goal::Evolve => "EVO".into(),
            Goal::LevelAtLeast(l) => format!("{l}+"),
            Goal::LevelUnder(l) => format!("<{l}"),
            Goal::Letter(c) => c.to_string(),
            Goal::Session(n) | Goal::Today(n) if n >= 1000 => format!("{}K", n / 1000),
            Goal::Session(n) | Goal::Today(n) => n.to_string(),
            Goal::Different(n) => n.to_string(),
            Goal::HuntMinutes(m) if m % 60 == 0 => format!("{}H", m / 60),
            Goal::HuntMinutes(m) => format!("{m}M"),
            Goal::Late => "9PM".into(),
            Goal::Early => "8AM".into(),
            Goal::Shiny => "★".into(),
            Goal::Dailies(n) => format!("{n}/{n}"),
        }
    }

    /// The square everyone finds hard (marked with a star).
    pub fn hard(self) -> bool {
        matches!(self, Goal::Shiny)
    }

    /// The type name, for a type square.
    pub fn type_name(self) -> Option<&'static str> {
        match self {
            Goal::Type(t, _) => Some(TYPES[t as usize % TYPES.len()].0),
            _ => None,
        }
    }
}

fn thousands(n: u32) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

pub fn meet_name(dex: u32) -> &'static str {
    MEET.iter().find(|m| m.0 == dex).map(|m| m.1).unwrap_or("Pokémon")
}

// ───────────────────────────── the card ─────────────────────────────

/// A small random number source, seeded by the day.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len())]
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// Day `day`'s card: 25 squares, the middle one free (None).
pub fn card(day: i64) -> [Option<Goal>; SQUARES] {
    let mut r = Rng(0x5EED_B1A6_DA11_0000 ^ (day as u64).wrapping_mul(0xA24B_AED4_963E_E407));
    let mut goals: Vec<Goal> = Vec::with_capacity(SQUARES - 1);
    // 7 Pokémon to meet, all different
    let mut meet: Vec<u32> = MEET.iter().map(|m| m.0).collect();
    r.shuffle(&mut meet);
    goals.extend(meet.iter().take(7).map(|d| Goal::Meet(*d)));
    // 4 types, all different
    let mut types: Vec<u8> = (0..TYPES.len() as u8).collect();
    r.shuffle(&mut types);
    goals.extend(types.iter().take(4).map(|t| Goal::Type(*t, TYPES[*t as usize].1)));
    // rare grass: one encounter, and a bigger count
    goals.push(Goal::RareGrass(1));
    goals.push(Goal::RareGrass(r.pick(&[10, 20, 30])));
    goals.push(Goal::Double);
    goals.push(Goal::Evolve);
    goals.push(if r.below(2) == 0 { Goal::LevelAtLeast(r.pick(&[30, 35])) } else { Goal::LevelUnder(r.pick(&[15, 20])) });
    goals.push(Goal::Letter(r.pick(&LETTER_POOL)));
    goals.push(Goal::Session(r.pick(&[200, 300])));
    goals.push(Goal::Today(r.pick(&[500, 800, 1000])));
    goals.push(Goal::Different(r.pick(&[6, 8])));
    goals.push(Goal::HuntMinutes(r.pick(&[30, 60, 90])));
    goals.push(if r.below(2) == 0 { Goal::Late } else { Goal::Early });
    goals.push(Goal::Shiny);
    goals.push(Goal::Dailies(3));
    debug_assert_eq!(goals.len(), SQUARES - 1);
    r.shuffle(&mut goals);
    let mut out = [None; SQUARES];
    let mut it = goals.into_iter();
    for (i, slot) in out.iter_mut().enumerate() {
        if i != FREE {
            *slot = it.next();
        }
    }
    out
}

/// Every line: 5 rows, 5 columns, then the two diagonals.
pub fn lines() -> [[usize; SIZE]; 12] {
    let mut out = [[0usize; SIZE]; 12];
    for i in 0..SIZE {
        for j in 0..SIZE {
            out[i][j] = i * SIZE + j;
            out[SIZE + i][j] = j * SIZE + i;
        }
    }
    for j in 0..SIZE {
        out[10][j] = j * SIZE + j;
        out[11][j] = j * SIZE + (SIZE - 1 - j);
    }
    out
}

/// A line's name ("Row 2", "Column H", "Diagonal").
pub fn line_name(i: usize) -> String {
    match i {
        0..=4 => format!("Row {}", i + 1),
        5..=9 => format!("Column {}", LETTERS[i - 5]),
        10 => "The S-to-Y diagonal".into(),
        _ => "The Y-to-S diagonal".into(),
    }
}

// ───────────────────────────── what happened ─────────────────────────────

/// One wild Pokémon met, with what the app knows about it.
#[derive(Debug, Clone, Default)]
pub struct Met {
    pub dex: u32,
    pub level: u32,
    pub rare_grass: bool,
    pub name: String,
    pub types: Vec<String>,
}

/// Something that can tick a square.
#[derive(Debug, Clone)]
pub enum Event {
    /// One battle's wild Pokémon (two in a double), the hour on the game's
    /// clock, and the game session (log file) it was in.
    Battle { mons: Vec<Met>, hour: u8, session: String },
    /// A Sweet Scent evolution on rare grass.
    Evolve,
    /// Today's hunting minutes, as the app counts them.
    HuntMinutes(u32),
    /// Battle pass dailies done today.
    Dailies(u32),
    /// A shiny caught.
    Shiny,
}

/// A past card, for the history.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Past {
    /// The card's day.
    pub day: i64,
    /// Squares done, one bit each.
    pub done: u32,
    pub lines: u32,
}

/// What a tick won (the app pays it out).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Won {
    /// Squares just ticked.
    pub squares: Vec<usize>,
    /// Lines just completed.
    pub lines: Vec<usize>,
    pub blackout: bool,
    /// The day's first line: the daily reward is waiting to be picked.
    pub daily: bool,
}

impl Won {
    pub fn is_empty(&self) -> bool {
        self.squares.is_empty() && self.lines.is_empty() && !self.blackout && !self.daily
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BingoFile {
    pub version: u32,
    /// The card this progress belongs to: its day (None until the first card).
    pub day: Option<i64>,
    /// Progress on each square.
    pub have: Vec<u32>,
    /// When each square was ticked (unix seconds; 0 = not yet).
    pub ticked: Vec<u64>,
    /// Lines paid out (indexes into `lines()`).
    pub paid: Vec<usize>,
    pub blackout: bool,
    /// Different Pokémon met on this card.
    pub day_species: Vec<u32>,
    /// Encounters on this card, in each game session (log file name).
    pub sessions: BTreeMap<String, u32>,
    /// How far each game log has been read (file name → bytes).
    pub logs: BTreeMap<String, u64>,
    /// The latest ticks, newest first: (when, what).
    pub recent: Vec<(u64, String)>,
    pub history: Vec<Past>,
    /// Blackouts ever.
    pub blackouts: u32,
    /// Lines ever.
    pub lines_ever: u32,
    /// Daily rewards won and not picked yet (they never run out).
    pub rewards_waiting: u32,
    /// The card day the last daily reward was won on (one a day).
    pub daily_won: Option<i64>,
    /// Daily rewards picked ever: (packs, battle pass XP).
    pub picked: (u32, u64),
}

impl BingoFile {
    pub fn has_data(&self) -> bool {
        self.day.is_some() || self.rewards_waiting > 0
    }

    /// Make sure the card on show is `day`'s (a new one starts empty, the
    /// old one goes into the history). True if it changed.
    pub fn roll(&mut self, day: i64) -> bool {
        if self.day == Some(day) && self.have.len() == SQUARES {
            return false;
        }
        if let Some(d) = self.day {
            if self.have.len() == SQUARES {
                self.history.insert(0, Past { day: d, done: self.done_mask(), lines: self.paid.len() as u32 });
                self.history.truncate(HISTORY_MAX);
            }
        }
        self.day = Some(day);
        self.have = vec![0; SQUARES];
        self.ticked = vec![0; SQUARES];
        self.paid.clear();
        self.blackout = false;
        self.day_species.clear();
        self.sessions.clear();
        self.recent.clear();
        true
    }

    /// Take one waiting daily reward (false if there's none).
    pub fn take_reward(&mut self, pack: bool) -> bool {
        if self.rewards_waiting == 0 {
            return false;
        }
        self.rewards_waiting -= 1;
        if pack {
            self.picked.0 += DAILY_PACKS;
        } else {
            self.picked.1 += DAILY_PASS_XP;
        }
        true
    }

    pub fn done(&self, card: &[Option<Goal>; SQUARES], i: usize) -> bool {
        match card[i] {
            None => true,
            Some(g) => self.have.get(i).copied().unwrap_or(0) >= g.need(),
        }
    }

    /// The free square doesn't count as one ticked.
    pub fn count(&self, card: &[Option<Goal>; SQUARES]) -> usize {
        (0..SQUARES).filter(|i| self.done(card, *i)).count()
    }

    fn done_mask(&self) -> u32 {
        let card = card(self.day.unwrap_or(0));
        (0..SQUARES).filter(|i| self.done(&card, *i)).fold(0, |m, i| m | 1 << i)
    }

    /// Lines done right now.
    pub fn lines_done(&self, card: &[Option<Goal>; SQUARES]) -> Vec<usize> {
        lines().iter().enumerate().filter(|(_, l)| l.iter().all(|i| self.done(card, *i))).map(|(i, _)| i).collect()
    }

    /// The lines nearest to done: (line, squares still to go), fewest first.
    pub fn closest(&self, card: &[Option<Goal>; SQUARES]) -> Vec<(usize, Vec<usize>)> {
        let mut v: Vec<(usize, Vec<usize>)> = lines()
            .iter()
            .enumerate()
            .map(|(i, l)| (i, l.iter().copied().filter(|s| !self.done(card, *s)).collect::<Vec<usize>>()))
            .filter(|(_, left)| !left.is_empty())
            .collect();
        v.sort_by_key(|(i, left)| (left.len(), *i));
        v
    }

    fn bump(&mut self, i: usize, to: u32, need: u32) {
        if let Some(h) = self.have.get_mut(i) {
            *h = (*h).max(to.min(need));
        }
    }

    /// Feed in something that happened at `now`. Returns what it won.
    pub fn apply(&mut self, ev: &Event, now: u64) -> Won {
        let Some(day) = self.day else { return Won::default() };
        let card = card(day);
        let before: Vec<bool> = (0..SQUARES).map(|i| self.done(&card, i)).collect();
        match ev {
            Event::Battle { mons, hour, session } => {
                let n = mons.len() as u32;
                if n == 0 {
                    return Won::default();
                }
                for m in mons {
                    if !self.day_species.contains(&m.dex) {
                        self.day_species.push(m.dex);
                    }
                }
                let s = self.sessions.entry(session.clone()).or_insert(0);
                *s += n;
                let session_n = *s;
                let today_n: u32 = self.sessions.values().sum();
                let rare = mons.iter().filter(|m| m.rare_grass).count() as u32;
                for i in 0..SQUARES {
                    let Some(g) = card[i] else { continue };
                    let have = self.have[i];
                    let need = g.need();
                    match g {
                        Goal::Meet(d) if mons.iter().any(|m| m.dex == d) => self.bump(i, 1, 1),
                        Goal::Type(..) => {
                            let t = g.type_name().unwrap_or("");
                            let k = mons.iter().filter(|m| m.types.iter().any(|x| x.eq_ignore_ascii_case(t))).count() as u32;
                            self.bump(i, have + k, need);
                        }
                        Goal::RareGrass(_) => self.bump(i, have + rare, need),
                        Goal::Double if n >= 2 => self.bump(i, 1, 1),
                        Goal::LevelAtLeast(l) if mons.iter().any(|m| m.level >= l) => self.bump(i, 1, 1),
                        Goal::LevelUnder(l) if mons.iter().any(|m| m.level > 0 && m.level < l) => self.bump(i, 1, 1),
                        Goal::Letter(c) if mons.iter().any(|m| m.name.chars().next().is_some_and(|f| f.to_ascii_uppercase() == c)) => self.bump(i, 1, 1),
                        Goal::Session(_) => self.bump(i, session_n, need),
                        Goal::Today(_) => self.bump(i, today_n, need),
                        Goal::Different(_) => self.bump(i, self.day_species.len() as u32, need),
                        Goal::Late if *hour >= 21 || *hour < 5 => self.bump(i, 1, 1),
                        Goal::Early if (5..8).contains(hour) => self.bump(i, 1, 1),
                        _ => {}
                    }
                }
            }
            Event::Evolve => {
                for i in 0..SQUARES {
                    if card[i] == Some(Goal::Evolve) {
                        self.bump(i, 1, 1);
                    }
                }
            }
            Event::HuntMinutes(m) => {
                for i in 0..SQUARES {
                    if let Some(g @ Goal::HuntMinutes(_)) = card[i] {
                        self.bump(i, *m, g.need());
                    }
                }
            }
            Event::Dailies(d) => {
                for i in 0..SQUARES {
                    if let Some(g @ Goal::Dailies(_)) = card[i] {
                        self.bump(i, *d, g.need());
                    }
                }
            }
            Event::Shiny => {
                for i in 0..SQUARES {
                    if card[i] == Some(Goal::Shiny) {
                        self.bump(i, 1, 1);
                    }
                }
            }
        }
        let mut won = Won::default();
        for i in 0..SQUARES {
            if !before[i] && self.done(&card, i) {
                won.squares.push(i);
                if let Some(t) = self.ticked.get_mut(i) {
                    *t = now;
                }
                if let Some(g) = card[i] {
                    self.recent.insert(0, (now, g.text()));
                }
            }
        }
        self.recent.truncate(RECENT_MAX);
        for l in self.lines_done(&card) {
            if !self.paid.contains(&l) {
                self.paid.push(l);
                self.lines_ever += 1;
                won.lines.push(l);
            }
        }
        if !won.lines.is_empty() && self.daily_won != Some(day) {
            self.daily_won = Some(day);
            self.rewards_waiting += 1;
            won.daily = true;
        }
        if !self.blackout && self.count(&card) == SQUARES {
            self.blackout = true;
            self.blackouts += 1;
            won.blackout = true;
        }
        won
    }
}

pub fn load_from(dir: &Path) -> (BingoFile, Option<String>) {
    crate::safefile::load(dir, "shiny-bingo", "Shiny Bingo")
}

pub fn save_to(dir: &Path, f: &BingoFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "shiny-bingo", f, first_this_run, BingoFile::has_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn met(dex: u32, name: &str, types: &[&str], level: u32, rare: bool) -> Met {
        Met { dex, level, rare_grass: rare, name: name.into(), types: types.iter().map(|t| t.to_string()).collect() }
    }

    fn battle(mons: Vec<Met>, hour: u8, session: &str) -> Event {
        Event::Battle { mons, hour, session: session.into() }
    }

    #[test]
    fn cards_are_full_fixed_and_differ_by_day() {
        for d in 20_700..20_800 {
            let c = card(d);
            assert!(c[FREE].is_none());
            assert_eq!(c.iter().filter(|g| g.is_some()).count(), SQUARES - 1);
            assert_eq!(c, card(d), "the same day is the same card");
            let mut seen: Vec<Goal> = Vec::new();
            for g in c.iter().flatten() {
                assert!(!seen.contains(g), "{g:?} twice on day {d}");
                seen.push(*g);
                assert!(!g.text().is_empty());
            }
        }
        assert_ne!(card(20_771), card(20_772));
        assert_eq!(lines().len(), 12);
        assert!(lines().iter().all(|l| l.contains(&FREE) == (l == &[10, 11, 12, 13, 14] || l == &[2, 7, 12, 17, 22] || l == &[0, 6, 12, 18, 24] || l == &[4, 8, 12, 16, 20])));
    }

    #[test]
    fn the_card_changes_at_the_daily_reset() {
        // 14 Nov 2026 22:30 UTC is 23:30 in Germany (UTC+1 in winter)
        let before = 1_794_695_400; // 23:30 CET
        let after = before + 3_600; // 00:30 CET, after the reset
        assert_eq!(card_day(after), card_day(before) + 1);
        assert_eq!(card_day(before), card_day(before - 20 * 3600));
    }

    #[test]
    fn texts_read_well() {
        assert_eq!(Goal::Type(2, 1).text(), "Meet a Water-type");
        assert_eq!(Goal::Type(12, 1).text(), "Meet an Electric-type");
        assert_eq!(Goal::Type(0, 10).text(), "Meet 10 Normal-types");
        assert_eq!(Goal::Session(300).text(), "300 encounters in one session");
        assert_eq!(Goal::Today(1000).text(), "1,000 encounters today");
        assert_eq!(Goal::HuntMinutes(60).text(), "Hunt for an hour today");
        assert_eq!(Goal::HuntMinutes(30).text(), "Hunt for 30 minutes today");
        assert_eq!(Goal::HuntMinutes(30).badge(), "30M");
        assert_eq!(Goal::Meet(399).text(), "Meet a Bidoof");
    }

    #[test]
    fn squares_tick_lines_pay_once_and_a_new_day_starts_clean() {
        let mut f = BingoFile::default();
        let d = 20_771;
        f.roll(d);
        let c = card(d);
        let mut lines_won = 0;
        let mut dailies = 0;
        let mut blackout = false;
        for i in 0..SQUARES {
            let Some(g) = c[i] else { continue };
            let ev = match g {
                Goal::Meet(x) => battle(vec![met(x, meet_name(x), &[], 20, false)], 12, "s1"),
                Goal::Type(..) => battle((0..g.need()).map(|_| met(1, "Bulbasaur", &[g.type_name().unwrap()], 20, false)).collect(), 12, "s1"),
                Goal::RareGrass(n) => battle((0..n).map(|_| met(1, "X", &[], 20, true)).collect(), 12, "s2"),
                Goal::Double => battle(vec![met(1, "X", &[], 20, false), met(1, "X", &[], 20, false)], 12, "s1"),
                Goal::Evolve => Event::Evolve,
                Goal::LevelAtLeast(l) => battle(vec![met(1, "X", &[], l, false)], 12, "s1"),
                Goal::LevelUnder(l) => battle(vec![met(1, "X", &[], l - 1, false)], 12, "s1"),
                Goal::Letter(ch) => battle(vec![met(1, &format!("{ch}ob"), &[], 20, false)], 12, "s1"),
                Goal::Session(n) | Goal::Today(n) => battle((0..n).map(|_| met(1, "X", &[], 20, false)).collect(), 12, "big"),
                Goal::Different(n) => battle((0..n).map(|k| met(1000 + k, "X", &[], 20, false)).collect(), 12, "s1"),
                Goal::HuntMinutes(m) => Event::HuntMinutes(m),
                Goal::Late => battle(vec![met(1, "X", &[], 20, false)], 22, "s1"),
                Goal::Early => battle(vec![met(1, "X", &[], 20, false)], 6, "s1"),
                Goal::Shiny => Event::Shiny,
                Goal::Dailies(n) => Event::Dailies(n),
            };
            let won = f.apply(&ev, 100 + i as u64);
            assert!(f.done(&c, i), "{g:?} didn't tick");
            lines_won += won.lines.len();
            dailies += won.daily as u32;
            blackout |= won.blackout;
        }
        assert_eq!(f.count(&c), SQUARES);
        assert_eq!(lines_won, 12);
        assert_eq!(f.paid.len(), 12);
        assert_eq!(dailies, 1, "one daily reward a day, however many lines");
        assert_eq!(f.rewards_waiting, 1);
        assert!(blackout && f.blackout && f.blackouts == 1);
        assert!(f.apply(&Event::Shiny, 999).is_empty(), "nothing more to win");
        // the reward waits until it's picked, even past the reset
        assert!(f.roll(d + 1));
        assert_eq!(f.rewards_waiting, 1);
        assert!(f.take_reward(false));
        assert!(!f.take_reward(true), "only one was waiting");
        assert_eq!(f.picked, (0, DAILY_PASS_XP));
        // a clean card, yesterday's in the history
        assert_eq!(f.count(&card(d + 1)), 1, "only the free square");
        assert_eq!(f.history[0].day, d);
        assert_eq!(f.history[0].lines, 12);
        assert!(!f.roll(d + 1));
    }

    #[test]
    fn counts_stay_capped_and_session_is_per_log() {
        let mut f = BingoFile::default();
        let (d, i, n) = (20_700..20_900)
            .find_map(|d| card(d).iter().enumerate().find_map(|(i, g)| if let Some(Goal::Session(n)) = g { Some((d, i, *n)) } else { None }))
            .unwrap();
        f.roll(d);
        let half = |s: &str| battle((0..n / 2).map(|_| met(1, "X", &[], 20, false)).collect(), 12, s);
        f.apply(&half("a"), 1);
        f.apply(&half("b"), 2);
        assert!(!f.done(&card(d), i), "two sessions don't add up");
        f.apply(&half("a"), 3);
        assert!(f.done(&card(d), i));
        assert_eq!(f.have[i], n);
    }
}
