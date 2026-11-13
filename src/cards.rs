//! Card Collector: the original Base Set and Base Set 2 cards, booster packs,
//! XP and missions. Hunting earns XP (every encounter the session log
//! reports, rare grass, new species, shinies, missions); a full bar is a
//! booster pack to open. Everything you own is saved in `cards.json` (written
//! safely, with rolling backups, like the other planner files).
//!
//! The card list ships with the app (`assets/cards.txt`, from the
//! PokemonTCG/pokemon-tcg-data project, MIT). The card pictures don't: they
//! download the first time they're shown and are kept in the cache.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

/// XP for one booster pack.
pub const XP_PER_PACK: u64 = 250;
/// XP per encounter the game logs, and the extras.
pub const XP_ENCOUNTER: u64 = 1;
pub const XP_RARE_GRASS: u64 = 1;
pub const XP_NEW_SPECIES: u64 = 10;
pub const XP_SHINY: u64 = 500;
/// Cards in a booster: 1 rare, 3 uncommons, 5 commons, 2 Energy.
#[cfg_attr(not(test), allow(dead_code))]
pub const PACK_SIZE: usize = 11;

/// A set: its id (as the card data names it), name, year and colour.
pub struct Set {
    pub id: &'static str,
    pub name: &'static str,
    pub year: u16,
    /// A Pokémon on its booster (dex number), drawn from the app's sprites
    /// until the pack art downloads.
    pub mascot: u16,
    pub col: [u8; 3],
    /// The booster pack art (each pack gets one of these at random).
    pub packs: &'static [&'static str],
}

pub const SETS: [Set; 2] = [
    Set { id: "base1", name: "Base Set", year: 1999, mascot: 6, col: [232, 92, 52], packs: &["base-set-pack-0", "base-set-pack-1", "base-set-pack-2"] },
    Set { id: "base4", name: "Base Set 2", year: 2000, mascot: 9, col: [62, 128, 232], packs: &["base-set-2-pack-0", "base-set-2-pack-1", "base-set-2-pack-2", "base-set-2-pack-3"] },
];

/// Where a pack art picture downloads from.
pub fn pack_art_url(name: &str) -> String {
    format!("https://pokesymbols.com/images/tcg/sets/booster-pack-art/{name}.jpg")
}

/// The order the cards sit in the pack, front to back: the commons and
/// uncommons, the rare, then the 2 Energy at the back. Moving the back two
/// to the front (the usual way to open one) leaves the rare for last.
pub fn packed_order(reveal: &[String]) -> Vec<String> {
    if reveal.len() < 3 {
        return reveal.to_vec();
    }
    let mut v = reveal[2..].to_vec();
    v.extend_from_slice(&reveal[..2]);
    v
}

pub fn set(id: &str) -> Option<&'static Set> {
    SETS.iter().find(|s| s.id == id)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Rarity {
    /// Basic Energy (no rarity symbol on the card).
    Energy,
    Common,
    Uncommon,
    Rare,
    Holo,
}

impl Rarity {
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Energy => "Energy",
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Holo => "Holo Rare",
        }
    }
    /// The symbol printed on the card: ● ◆ ★.
    pub fn symbol(self) -> &'static str {
        match self {
            Rarity::Energy => "",
            Rarity::Common => "●",
            Rarity::Uncommon => "◆",
            Rarity::Rare | Rarity::Holo => "★",
        }
    }
    fn parse(s: &str) -> Rarity {
        match s {
            "Common" => Rarity::Common,
            "Uncommon" => Rarity::Uncommon,
            "Rare" => Rarity::Rare,
            "Rare Holo" => Rarity::Holo,
            _ => Rarity::Energy,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Pokemon,
    Trainer,
    Energy,
}

#[derive(Clone, Debug)]
pub struct Card {
    /// "base1-4"
    pub id: String,
    pub set: &'static str,
    pub num: u16,
    pub name: String,
    pub rarity: Rarity,
    pub kind: Kind,
    /// "Fire", "Water"… (Pokémon only).
    pub ty: String,
    pub hp: u16,
    /// National Dex number (Pokémon only).
    pub dex: u16,
    /// "Basic", "Stage 1", "Stage 2"; "Special" for Double Colorless.
    pub stage: String,
}

impl Card {
    /// The card picture: small (for lists) or large.
    pub fn image_url(&self, large: bool) -> String {
        format!("https://images.pokemontcg.io/{}/{}{}.png", self.set, self.num, if large { "_hires" } else { "" })
    }
    /// What it is, for filters: the Pokémon's type, or Trainer / Energy.
    pub fn group(&self) -> &str {
        match self.kind {
            Kind::Pokemon => &self.ty,
            Kind::Trainer => "Trainer",
            Kind::Energy => "Energy",
        }
    }
    pub fn set_name(&self) -> &'static str {
        set(self.set).map_or("", |s| s.name)
    }
}

/// Every card, in set then number order.
pub fn all() -> &'static [Card] {
    static CARDS: OnceLock<Vec<Card>> = OnceLock::new();
    CARDS.get_or_init(|| parse(include_str!("../assets/cards.txt")))
}

pub fn by_id(id: &str) -> Option<&'static Card> {
    all().iter().find(|c| c.id == id)
}

/// The cards of one set.
pub fn of_set(set: &str) -> Vec<&'static Card> {
    all().iter().filter(|c| c.set == set).collect()
}

fn parse(text: &str) -> Vec<Card> {
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = line.split('|').collect();
        if f.len() < 9 {
            continue;
        }
        let Some(s) = SETS.iter().find(|s| s.id == f[0]) else { continue };
        let kind = match f[4] {
            "Trainer" => Kind::Trainer,
            "Energy" => Kind::Energy,
            _ => Kind::Pokemon,
        };
        let num: u16 = f[1].parse().unwrap_or(0);
        out.push(Card {
            id: format!("{}-{num}", s.id),
            set: s.id,
            num,
            name: f[2].to_string(),
            rarity: Rarity::parse(f[3]),
            kind,
            ty: f[5].to_string(),
            hp: f[6].parse().unwrap_or(0),
            dex: f[7].parse().unwrap_or(0),
            stage: f[8].to_string(),
        });
    }
    out
}

// ───────────────────────────── packs ─────────────────────────────

/// A small random number source (packs don't need more).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }
    pub fn seeded_now(extra: u64) -> Rng {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng::new(t.wrapping_mul(6364136223846793005).wrapping_add(extra))
    }
    pub fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// `k` different picks from `from`.
fn pick<'a>(rng: &mut Rng, from: &[&'a Card], k: usize) -> Vec<&'a Card> {
    let mut pool: Vec<&Card> = from.to_vec();
    let mut out = Vec::new();
    for _ in 0..k.min(pool.len()) {
        out.push(pool.swap_remove(rng.below(pool.len())));
    }
    out
}

/// Open a booster of `set`: 2 Energy, 5 commons, 3 uncommons, then the rare
/// (a holo one time in three), in the order they're revealed.
pub fn open_pack(set: &str, rng: &mut Rng) -> Vec<String> {
    let cards = of_set(set);
    let by = |r: Rarity| -> Vec<&Card> { cards.iter().copied().filter(|c| c.rarity == r).collect() };
    let mut out: Vec<&Card> = Vec::new();
    let energy = by(Rarity::Energy);
    for _ in 0..2 {
        if !energy.is_empty() {
            out.push(energy[rng.below(energy.len())]);
        }
    }
    out.extend(pick(rng, &by(Rarity::Common), 5));
    out.extend(pick(rng, &by(Rarity::Uncommon), 3));
    let holo = rng.below(3) == 0;
    let rares = by(if holo { Rarity::Holo } else { Rarity::Rare });
    out.extend(pick(rng, &rares, 1));
    out.into_iter().map(|c| c.id.clone()).collect()
}

// ───────────────────────────── missions ─────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MKind {
    /// Meet this many wild Pokémon.
    Encounters,
    /// Meet this many on rare grass.
    RareGrass,
    /// Meet this many different species.
    Species,
    /// Hunt for this many minutes.
    Minutes,
    /// Open this many packs.
    OpenPacks,
    /// Find a shiny.
    Shiny,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mission {
    pub kind: MKind,
    pub goal: u64,
    pub xp: u64,
    /// Done, and its XP given.
    #[serde(default)]
    pub done: bool,
}

impl Mission {
    pub fn text(&self) -> String {
        let g = self.goal;
        match self.kind {
            MKind::Encounters => format!("Meet {g} wild Pokémon"),
            MKind::RareGrass => format!("Meet {g} Pokémon on rare grass"),
            MKind::Species => format!("Meet {g} different species"),
            MKind::Minutes => format!("Hunt for {g} minutes"),
            MKind::OpenPacks => format!("Open {g} pack{}", if g == 1 { "" } else { "s" }),
            MKind::Shiny => if g == 1 { "Find a shiny".to_string() } else { format!("Find {g} shinies") },
        }
    }
    pub fn icon(&self) -> &'static str {
        match self.kind {
            MKind::Encounters => "⚔",
            MKind::RareGrass => "🌿",
            MKind::Species => "📖",
            MKind::Minutes => "🕘",
            MKind::OpenPacks => "🎁",
            MKind::Shiny => "✨",
        }
    }
}

/// What you've done in a day or a week (for missions).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    /// Day number (local) for a day, Monday's day number for a week.
    pub when: i64,
    pub encounters: u64,
    pub rare: u64,
    /// National Dex numbers met.
    pub species: Vec<u32>,
    /// Hunting time in seconds (for a day: from the hunt's own clock).
    pub secs: u64,
    pub packs: u64,
    pub shinies: u64,
    pub missions: Vec<Mission>,
}

impl Tally {
    pub fn progress(&self, m: &Mission) -> u64 {
        match m.kind {
            MKind::Encounters => self.encounters,
            MKind::RareGrass => self.rare,
            MKind::Species => self.species.len() as u64,
            MKind::Minutes => self.secs / 60,
            MKind::OpenPacks => self.packs,
            MKind::Shiny => self.shinies,
        }
        .min(m.goal)
    }
}

/// Today's three missions (the same all day for everyone).
pub fn daily_missions(day: i64) -> Vec<Mission> {
    let m = |kind, goal, xp| Mission { kind, goal, xp, done: false };
    let pool = [
        [m(MKind::Encounters, 300, 100), m(MKind::Species, 8, 80), m(MKind::OpenPacks, 1, 40)],
        [m(MKind::Encounters, 600, 150), m(MKind::Minutes, 30, 80), m(MKind::RareGrass, 25, 80)],
        [m(MKind::Encounters, 1000, 200), m(MKind::Species, 12, 100), m(MKind::OpenPacks, 2, 60)],
        [m(MKind::Minutes, 60, 120), m(MKind::RareGrass, 50, 100), m(MKind::Encounters, 400, 120)],
        [m(MKind::Encounters, 800, 180), m(MKind::Species, 6, 60), m(MKind::Minutes, 45, 100)],
    ];
    pool[day.rem_euclid(pool.len() as i64) as usize].to_vec()
}

/// This week's two missions.
pub fn weekly_missions(week: i64) -> Vec<Mission> {
    let m = |kind, goal, xp| Mission { kind, goal, xp, done: false };
    let pool = [
        [m(MKind::Encounters, 5000, 600), m(MKind::Shiny, 1, 750)],
        [m(MKind::Species, 30, 500), m(MKind::OpenPacks, 10, 400)],
        [m(MKind::Minutes, 300, 600), m(MKind::RareGrass, 300, 500)],
    ];
    pool[(week / 7).rem_euclid(pool.len() as i64) as usize].to_vec()
}

/// Monday's day number for the week a day is in.
pub fn week_of(day: i64) -> i64 {
    // 1 Jan 1970 was a Thursday (weekday 3 counting Monday as 0)
    day - (day + 3).rem_euclid(7)
}

// ───────────────────────────── the save file ─────────────────────────────

/// One opened pack, for "Latest pulls".
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pull {
    pub when: u64,
    pub set: String,
    pub cards: Vec<String>,
}

/// Where the XP bar overlay sits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarCfg {
    /// Show it while the overlay runs.
    pub show: bool,
    /// Its spot when it's on its own.
    pub pos: Option<[f32; 2]>,
    /// Snapped under the shiny overlay (they move together).
    pub snapped: bool,
    /// Its own overlay: shown even when the shiny overlay isn't running.
    /// (No longer used: the bar is always pinned under the overlay.)
    pub alone: bool,
    /// Pinned under the overlay's right end (else its left).
    pub right: bool,
}

impl Default for BarCfg {
    fn default() -> Self {
        BarCfg { show: true, pos: None, snapped: true, alone: false, right: false }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CardFile {
    pub version: u32,
    /// XP towards the next pack, and ever.
    pub xp: u64,
    pub xp_total: u64,
    /// Packs waiting to be opened.
    pub packs: u32,
    pub opened: u32,
    /// How many of each card (by id).
    pub owned: BTreeMap<String, u32>,
    /// When each card was first pulled.
    pub first: BTreeMap<String, u64>,
    /// The last packs opened, newest first.
    pub pulls: Vec<Pull>,
    pub day: Tally,
    pub week: Tally,
    pub bar: BarCfg,
    /// The card-art choice has been offered (on first open).
    pub art_asked: bool,
    /// The opening playmat's colour ("" = the set's own).
    pub mat: String,
    /// How far each game session log has been read for XP (file name →
    /// bytes). Empty until the first read, which starts from the logs' ends.
    pub logs: BTreeMap<String, u64>,
    /// This player's Trainer ID (also in the trade record).
    pub trainer: u64,
    /// How many trades in the trade record this save has taken in.
    pub trades_seen: u32,
    /// Cards that came from trades (by id), so they can be told apart.
    pub traded_in: BTreeMap<String, u32>,
    /// The latest XP earned, newest first: (when, XP, what for).
    pub recent: Vec<(u64, u64, String)>,
}

/// Something that earned XP (shown on the bar for a moment).
#[derive(Clone, Debug, PartialEq)]
pub struct Gain {
    pub xp: u64,
    pub why: String,
    /// Packs this gain filled.
    pub packs: u32,
}

impl CardFile {
    pub fn has_data(&self) -> bool {
        self.xp_total > 0 || !self.owned.is_empty() || self.packs > 0
    }

    /// Different cards owned (in a set, or all).
    pub fn unique(&self, set: Option<&str>) -> usize {
        self.owned.iter().filter(|(id, n)| **n > 0 && set.map_or(true, |s| id.starts_with(&format!("{s}-")))).count()
    }

    pub fn count(&self, id: &str) -> u32 {
        self.owned.get(id).copied().unwrap_or(0)
    }

    /// Add XP; a full bar becomes a pack.
    pub fn add_xp(&mut self, xp: u64, why: &str) -> Gain {
        if xp > 0 {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            self.recent.insert(0, (now, xp, why.to_string()));
            self.recent.truncate(25);
        }
        self.xp += xp;
        self.xp_total += xp;
        let mut packs = 0;
        while self.xp >= XP_PER_PACK {
            self.xp -= XP_PER_PACK;
            self.packs += 1;
            packs += 1;
        }
        Gain { xp, why: why.to_string(), packs }
    }

    /// Start a new day or week if it's turned over.
    pub fn roll_over(&mut self, today: i64) {
        if self.day.when != today || self.day.missions.is_empty() {
            self.day = Tally { when: today, missions: daily_missions(today), ..Default::default() };
        }
        let wk = week_of(today);
        if self.week.when != wk || self.week.missions.is_empty() {
            self.week = Tally { when: wk, missions: weekly_missions(wk), ..Default::default() };
        }
    }

    /// Wild Pokémon met (one battle): XP for each, rare grass and new
    /// species today; then any missions finished.
    pub fn met(&mut self, today: i64, dex: &[u32], rare_grass: bool) -> Vec<Gain> {
        self.roll_over(today);
        let n = dex.len().max(1) as u64;
        // what each part earned, so the reason shows the sum:
        // "2 met +2 · rare grass +2 · new today +10"
        let mut parts = vec![format!("{n} met +{}", n * XP_ENCOUNTER)];
        let mut xp = n * XP_ENCOUNTER;
        if rare_grass {
            xp += n * XP_RARE_GRASS;
            parts.push(format!("rare grass +{}", n * XP_RARE_GRASS));
        }
        let mut gains = Vec::new();
        for t in [&mut self.day, &mut self.week] {
            t.encounters += n;
            if rare_grass {
                t.rare += n;
            }
        }
        let mut new_today = 0;
        for d in dex {
            if *d > 0 && !self.day.species.contains(d) {
                self.day.species.push(*d);
                new_today += 1;
            }
            if *d > 0 && !self.week.species.contains(d) {
                self.week.species.push(*d);
            }
        }
        if new_today > 0 {
            xp += new_today * XP_NEW_SPECIES;
            parts.push(format!("new today +{}", new_today * XP_NEW_SPECIES));
        }
        let why = parts.join(" · ");
        gains.push(self.add_xp(xp, &why));
        gains.extend(self.check_missions());
        gains
    }

    /// A shiny found.
    pub fn shiny(&mut self, today: i64) -> Vec<Gain> {
        self.roll_over(today);
        self.day.shinies += 1;
        self.week.shinies += 1;
        let mut g = vec![self.add_xp(XP_SHINY, "Shiny found!")];
        g.extend(self.check_missions());
        g
    }

    /// Today's hunting time (from the hunt's clock).
    pub fn hunt_time(&mut self, today: i64, secs_today: u64) -> Vec<Gain> {
        self.roll_over(today);
        if secs_today <= self.day.secs {
            return Vec::new();
        }
        let add = secs_today - self.day.secs;
        self.day.secs = secs_today;
        self.week.secs += add;
        self.check_missions()
    }

    /// Give the XP for missions just finished.
    pub fn check_missions(&mut self) -> Vec<Gain> {
        let mut gains = Vec::new();
        for weekly in [false, true] {
            let t = if weekly { &self.week } else { &self.day };
            let done: Vec<usize> = t.missions.iter().enumerate().filter(|(_, m)| !m.done && t.progress(m) >= m.goal).map(|(i, _)| i).collect();
            for i in done {
                let t = if weekly { &mut self.week } else { &mut self.day };
                t.missions[i].done = true;
                let (xp, text) = (t.missions[i].xp, t.missions[i].text());
                gains.push(self.add_xp(xp, &format!("{} mission: {text}", if weekly { "Weekly" } else { "Daily" })));
            }
        }
        gains
    }

    /// Open one pack of `set`. Returns the cards (in reveal order) and
    /// which of them are new to the collection.
    pub fn open(&mut self, set: &str, rng: &mut Rng, today: i64, now: u64) -> Option<(Vec<String>, Vec<bool>, Vec<Gain>)> {
        if self.packs == 0 || crate::cards::set(set).is_none() {
            return None;
        }
        self.roll_over(today);
        self.packs -= 1;
        self.opened += 1;
        let cards = open_pack(set, rng);
        let mut new = Vec::new();
        for id in &cards {
            let n = self.owned.entry(id.clone()).or_insert(0);
            new.push(*n == 0);
            *n += 1;
            self.first.entry(id.clone()).or_insert(now);
        }
        self.pulls.insert(0, Pull { when: now, set: set.to_string(), cards: cards.clone() });
        self.pulls.truncate(20);
        self.day.packs += 1;
        self.week.packs += 1;
        let gains = self.check_missions();
        Some((cards, new, gains))
    }

    /// Duplicates you could trade in, and the XP they'd give.
    pub fn spare(&self) -> (u32, u64) {
        let mut n = 0;
        let mut xp = 0;
        for (id, c) in &self.owned {
            if *c > 1 {
                let extra = c - 1;
                n += extra;
                xp += extra as u64 * by_id(id).map_or(0, |c| trade_xp(c.rarity));
            }
        }
        (n, xp)
    }

    /// Trade every spare duplicate (keeping one of each) for XP.
    pub fn trade_spares(&mut self) -> Option<Gain> {
        let (n, xp) = self.spare();
        if n == 0 {
            return None;
        }
        for c in self.owned.values_mut() {
            if *c > 1 {
                *c = 1;
            }
        }
        Some(self.add_xp(xp, &format!("Traded {n} duplicates")))
    }
}

/// XP for trading in one duplicate.
pub fn trade_xp(r: Rarity) -> u64 {
    match r {
        Rarity::Energy => 1,
        Rarity::Common => 3,
        Rarity::Uncommon => 6,
        Rarity::Rare => 15,
        Rarity::Holo => 40,
    }
}

pub fn load_from(dir: &Path) -> (CardFile, Option<String>) {
    crate::safefile::load(dir, "cards", "Card Collector")
}

pub fn save_to(dir: &Path, f: &CardFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "cards", f, first_this_run, CardFile::has_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sets_are_complete() {
        assert_eq!(of_set("base1").len(), 102);
        assert_eq!(of_set("base4").len(), 130);
        assert_eq!(by_id("base1-4").map(|c| c.name.as_str()), Some("Charizard"));
        assert_eq!(by_id("base1-4").map(|c| c.rarity), Some(Rarity::Holo));
        assert!(all().iter().all(|c| c.num > 0 && !c.name.is_empty()));
    }

    #[test]
    fn packs_have_the_right_mix() {
        let mut rng = Rng::new(7);
        let mut holos = 0;
        for _ in 0..600 {
            let p = open_pack("base1", &mut rng);
            assert_eq!(p.len(), PACK_SIZE);
            let r: Vec<Rarity> = p.iter().map(|id| by_id(id).unwrap().rarity).collect();
            assert_eq!(r.iter().filter(|x| **x == Rarity::Energy).count(), 2);
            assert_eq!(r.iter().filter(|x| **x == Rarity::Common).count(), 5);
            assert_eq!(r.iter().filter(|x| **x == Rarity::Uncommon).count(), 3);
            assert!(matches!(r[10], Rarity::Rare | Rarity::Holo));
            holos += (r[10] == Rarity::Holo) as u32;
            // no doubles among the commons and uncommons
            let mut ids = p[2..10].to_vec();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), 8);
        }
        // about one in three
        assert!((150..250).contains(&holos), "{holos}");
    }

    #[test]
    fn back_two_to_the_front_leaves_the_rare_last() {
        let mut rng = Rng::new(3);
        let reveal = open_pack("base1", &mut rng);
        let packed = packed_order(&reveal);
        // the rare sits third from the back, the Energy behind it
        assert_eq!(packed[8], reveal[10]);
        assert!(packed[9..].iter().all(|id| by_id(id).unwrap().rarity == Rarity::Energy));
        // move the back two to the front: the reveal order, rare last
        let mut moved = packed[9..].to_vec();
        moved.extend_from_slice(&packed[..9]);
        assert_eq!(moved, reveal);
    }

    #[test]
    fn xp_fills_packs_and_missions_pay_once() {
        let mut f = CardFile::default();
        let day = 20_000;
        f.met(day, &[1], false);
        assert_eq!(f.xp, XP_ENCOUNTER + XP_NEW_SPECIES);
        // same species again: no new-species bonus
        f.met(day, &[1], true);
        assert_eq!(f.xp, XP_ENCOUNTER + XP_NEW_SPECIES + XP_ENCOUNTER + XP_RARE_GRASS);
        let before = f.packs;
        f.add_xp(XP_PER_PACK * 2, "test");
        assert_eq!(f.packs, before + 2);
        // finish an encounter mission: its XP comes once
        let goal = f.day.missions.iter().find(|m| m.kind == MKind::Encounters).map(|m| m.goal);
        if let Some(goal) = goal {
            let total = f.xp_total;
            for _ in 0..goal {
                f.met(day, &[], false);
            }
            let after = f.xp_total;
            f.met(day, &[], false);
            assert!(f.day.missions.iter().any(|m| m.kind == MKind::Encounters && m.done));
            assert_eq!(f.xp_total - after, XP_ENCOUNTER, "no second payout");
            assert!(after > total + goal);
        }
    }

    #[test]
    fn days_and_weeks_turn_over() {
        let mut f = CardFile::default();
        f.met(20_000, &[5], false);
        assert_eq!(f.day.encounters, 1);
        f.met(20_001, &[5], false);
        assert_eq!(f.day.encounters, 1, "a new day starts fresh");
        // a Monday starts its own week
        let mon = week_of(20_000);
        assert_eq!(week_of(mon + 6), mon);
        assert_eq!(week_of(mon + 7), mon + 7);
    }

    #[test]
    fn opening_adds_cards_and_trading_keeps_one() {
        let mut f = CardFile { packs: 3, ..Default::default() };
        let mut rng = Rng::new(1);
        for _ in 0..3 {
            let (cards, new, _) = f.open("base4", &mut rng, 20_000, 1).unwrap();
            assert_eq!(cards.len(), PACK_SIZE);
            assert_eq!(new.len(), PACK_SIZE);
        }
        assert!(f.open("base4", &mut rng, 20_000, 1).is_none(), "no packs left");
        assert_eq!(f.owned.values().sum::<u32>(), 33);
        let (spare, xp) = f.spare();
        if spare > 0 {
            let g = f.trade_spares().unwrap();
            assert_eq!(g.xp, xp);
            assert!(f.owned.values().all(|n| *n == 1));
        }
    }
}
