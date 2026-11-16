//! Card Collector: the Wizards-era cards (Base Set to Skyridge), booster packs,
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
pub const XP_PER_PACK: u64 = 400;
/// XP per encounter the game logs, and the extras.
pub const XP_ENCOUNTER: u64 = 1;
pub const XP_RARE_GRASS: u64 = 1;
pub const XP_NEW_SPECIES: u64 = 10;
pub const XP_SHINY: u64 = 500;
/// Cards in a classic booster: 1 rare, 3 uncommons, 5 commons, 2 Energy.
/// (Each set's own size is `Set::size`: the e-Card sets have 9.)
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
    /// Basic Energy in each pack (taken out of the common slots): the early
    /// expansions had none.
    pub energy: u8,
    /// Common slots (Energy included), uncommons, and reverse holo slots
    /// (any card but a basic Energy, from Legendary Collection on).
    pub commons: u8,
    pub uncommons: u8,
    pub reverse: u8,
    /// When it comes out in the app (unix seconds; 0 = it's out).
    pub release: u64,
}

impl Set {
    /// Out by `now` (unix seconds).
    pub fn out_at(&self, now: u64) -> bool {
        self.release <= now
    }
    pub fn out(&self) -> bool {
        self.out_at(now())
    }
    /// Cards in one of its boosters.
    pub fn size(&self) -> usize {
        (self.commons + self.uncommons + self.reverse) as usize + 1
    }
    /// An e-Card set (Expedition, Aquapolis, Skyridge): a different card frame.
    pub fn ecard(&self) -> bool {
        self.id.starts_with("ecard")
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Every set, in the order they come out in the app (new ones go on the end:
/// trade codes use the place in this list). One new set every Monday at the
/// game's daily reset, 00:00 CET (23:00 UTC on the Sunday; 10 am in Sydney),
/// in the order the real ones came out. Southern Islands is left out: it was
/// only sold in collections, never in boosters.
pub const SETS: [Set; 15] = [
    Set { id: "base1", name: "Base Set", year: 1999, mascot: 6, col: [232, 92, 52], packs: &["base-set-pack-0", "base-set-pack-1", "base-set-pack-2"], energy: 2, commons: 7, uncommons: 3, reverse: 0, release: 0 },
    Set { id: "base4", name: "Base Set 2", year: 2000, mascot: 9, col: [62, 128, 232], packs: &["base-set-2-pack-0", "base-set-2-pack-1", "base-set-2-pack-2", "base-set-2-pack-3"], energy: 2, commons: 7, uncommons: 3, reverse: 0, release: 0 },
    // Mon 16 Nov 2026
    Set { id: "base2", name: "Jungle", year: 1999, mascot: 123, col: [76, 160, 72], packs: &["jungle-pack-0", "jungle-pack-1", "jungle-pack-2"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_794_783_600 },
    // Mon 23 Nov 2026
    Set { id: "base3", name: "Fossil", year: 1999, mascot: 145, col: [206, 170, 82], packs: &["fossil-pack-0", "fossil-pack-1", "fossil-pack-2"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_795_388_400 },
    // Mon 30 Nov 2026
    Set { id: "base5", name: "Team Rocket", year: 2000, mascot: 52, col: [120, 62, 142], packs: &["team-rocket-pack-0", "team-rocket-pack-1"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_795_993_200 },
    // Mon 7 Dec 2026
    Set { id: "gym1", name: "Gym Heroes", year: 2000, mascot: 95, col: [48, 156, 168], packs: &["gym-heroes-pack-0", "gym-heroes-pack-1", "gym-heroes-pack-2", "gym-heroes-pack-3"], energy: 2, commons: 7, uncommons: 3, reverse: 0, release: 1_796_598_000 },
    // Mon 14 Dec 2026
    Set { id: "gym2", name: "Gym Challenge", year: 2000, mascot: 150, col: [146, 84, 196], packs: &["gym-challenge-pack-0", "gym-challenge-pack-1", "gym-challenge-pack-2", "gym-challenge-pack-3"], energy: 2, commons: 7, uncommons: 3, reverse: 0, release: 1_797_202_800 },
    // Mon 21 Dec 2026: 11 cards, 1 of them a basic Energy
    Set { id: "neo1", name: "Neo Genesis", year: 2000, mascot: 249, col: [214, 168, 52], packs: &["neo-genesis-pack-0", "neo-genesis-pack-1", "neo-genesis-pack-2", "neo-genesis-pack-3"], energy: 1, commons: 7, uncommons: 3, reverse: 0, release: 1_797_807_600 },
    // Mon 28 Dec 2026
    Set { id: "neo2", name: "Neo Discovery", year: 2001, mascot: 201, col: [98, 120, 168], packs: &["neo-discovery-pack-0", "neo-discovery-pack-1", "neo-discovery-pack-2", "neo-discovery-pack-3"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_798_412_400 },
    // Mon 4 Jan 2027
    Set { id: "neo3", name: "Neo Revelation", year: 2001, mascot: 250, col: [222, 102, 54], packs: &["neo-revelation-pack-0", "neo-revelation-pack-1", "neo-revelation-pack-2", "neo-revelation-pack-3"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_799_017_200 },
    // Mon 11 Jan 2027
    Set { id: "neo4", name: "Neo Destiny", year: 2002, mascot: 197, col: [76, 64, 112], packs: &["neo-destiny-pack-0", "neo-destiny-pack-1", "neo-destiny-pack-2", "neo-destiny-pack-3"], energy: 0, commons: 7, uncommons: 3, reverse: 0, release: 1_799_622_000 },
    // Mon 18 Jan 2027: 11 cards, 1 of them a reverse holo
    Set { id: "base6", name: "Legendary Collection", year: 2002, mascot: 151, col: [196, 112, 168], packs: &["legendary-collection-pack-0", "legendary-collection-pack-1", "legendary-collection-pack-2", "legendary-collection-pack-3"], energy: 0, commons: 6, uncommons: 3, reverse: 1, release: 1_800_226_800 },
    // Mon 25 Jan 2027: e-Card boosters are 9 cards (5 commons, 2 uncommons,
    // a reverse holo and the rare); basic Energy only came in theme decks
    Set { id: "ecard1", name: "Expedition", year: 2002, mascot: 3, col: [94, 150, 88], packs: &["expedition-pack-0", "expedition-pack-1", "expedition-pack-2", "expedition-pack-3"], energy: 0, commons: 5, uncommons: 2, reverse: 1, release: 1_800_831_600 },
    // Mon 1 Feb 2027
    Set { id: "ecard2", name: "Aquapolis", year: 2003, mascot: 230, col: [52, 128, 196], packs: &["aquapolis-pack-0", "aquapolis-pack-1", "aquapolis-pack-2", "aquapolis-pack-3"], energy: 0, commons: 5, uncommons: 2, reverse: 1, release: 1_801_436_400 },
    // Mon 8 Feb 2027
    Set { id: "ecard3", name: "Skyridge", year: 2003, mascot: 251, col: [112, 168, 96], packs: &["skyridge-pack-0", "skyridge-pack-1", "skyridge-pack-2", "skyridge-pack-3"], energy: 0, commons: 5, uncommons: 2, reverse: 1, release: 1_802_041_200 },
];

/// The sets out now, in order.
pub fn sets_out() -> Vec<&'static Set> {
    let now = now();
    SETS.iter().filter(|s| s.out_at(now)).collect()
}

/// The sets still to come, soonest first.
pub fn sets_coming() -> Vec<&'static Set> {
    let now = now();
    SETS.iter().filter(|s| !s.out_at(now)).collect()
}

/// Every card of the sets out now.
pub fn all_out() -> Vec<&'static Card> {
    let now = now();
    all().iter().filter(|c| set(c.set).is_some_and(|s| s.out_at(now))).collect()
}

/// Where a pack art picture downloads from.
pub fn pack_art_url(name: &str) -> String {
    format!("https://pokesymbols.com/images/tcg/sets/booster-pack-art/{name}.jpg")
}

/// The order the cards sit in the pack, front to back: the commons and
/// uncommons, the rare, then the 2 Energy at the back. Moving the back two
/// to the front (the usual way to open one) leaves the rare for last.
/// A set without Energy (Jungle, Fossil, Team Rocket) has the rare at the
/// back already.
pub fn packed_order(reveal: &[String]) -> Vec<String> {
    let energy = reveal.iter().take(2).filter(|id| by_id(id).is_some_and(|c| c.rarity == Rarity::Energy)).count();
    if reveal.len() < 3 || energy < 2 {
        return reveal.to_vec();
    }
    let mut v = reveal[2..].to_vec();
    v.extend_from_slice(&reveal[..2]);
    v
}

pub fn set(id: &str) -> Option<&'static Set> {
    SETS.iter().find(|s| s.id == id)
}

fn set_of(id: &str) -> Option<&'static Set> {
    set(id)
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
    /// Its number as printed: "4", or "H5" for the Aquapolis and Skyridge
    /// holos (kept as 1001 and up, so they sort after the rest).
    pub fn number(&self) -> String {
        if self.num > 1000 {
            format!("H{}", self.num - 1000)
        } else {
            self.num.to_string()
        }
    }
    /// The card picture: small (for lists) or large.
    pub fn image_url(&self, large: bool) -> String {
        format!("https://images.pokemontcg.io/{}/{}{}.png", self.set, self.number(), if large { "_hires" } else { "" })
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

/// How many times a card was printed on its sheet, against the others of
/// its rarity (8 = the usual). From the English rarity guide on Elite
/// Fourum (pack-opening data and photos of the uncut sheets):
/// * Base Set: Charizard, Blastoise and Venusaur were printed once fewer on
///   the holo sheet (7 against 8); Machamp was never in booster packs at all
///   (it came in the starter deck), so here it's the rarest pull there is.
/// * Base Set and Base Set 2 Basic Energy: Fire and Fighting rarest, Grass
///   and Water in between, Lightning and Psychic most common.
/// * Jungle, Fossil, Team Rocket (Dark Raichu included) and the Gym sets: no
///   differences beyond chance.
/// * The Shining Pokémon (Neo Revelation, Neo Destiny) and the Crystal
///   Pokémon (Aquapolis, Skyridge) sit in the rare slot but turn up about four
///   times less often than a holo.
pub fn weight(c: &Card) -> u32 {
    match (c.set, c.num) {
        ("base1", 8) => 1,
        ("neo3", 65..=66) | ("neo4", 106..=113) | ("ecard2", 148..=150) | ("ecard3", 145..=150) => 2,
        ("base1", 2 | 4 | 15) => 7,
        ("base1" | "base4", _) if c.rarity == Rarity::Energy => match c.name.as_str() {
            "Fire Energy" | "Fighting Energy" => 6,
            "Grass Energy" | "Water Energy" => 8,
            _ => 10,
        },
        _ => 8,
    }
}

/// `k` different picks from `from`, each as likely as its `weight`.
fn pick<'a>(rng: &mut Rng, from: &[&'a Card], k: usize) -> Vec<&'a Card> {
    let mut pool: Vec<&Card> = from.to_vec();
    let mut out = Vec::new();
    for _ in 0..k.min(pool.len()) {
        let total: u32 = pool.iter().map(|c| weight(c)).sum();
        let mut r = rng.below(total.max(1) as usize) as u32;
        let i = pool.iter().position(|c| {
            let w = weight(c);
            if r < w {
                true
            } else {
                r -= w;
                false
            }
        });
        out.push(pool.swap_remove(i.unwrap_or(0)));
    }
    out
}

/// About how many packs of its set it takes to pull this card once.
pub fn packs_per(c: &Card) -> Option<f64> {
    let s = set(c.set)?;
    let cards = of_set(c.set);
    let pool: Vec<&Card> = cards.iter().copied().filter(|x| x.rarity == c.rarity).collect();
    let total: u32 = pool.iter().map(|x| weight(x)).sum();
    if total == 0 {
        return None;
    }
    let share = weight(c) as f64 / total as f64;
    let has_energy = cards.iter().any(|x| x.rarity == Rarity::Energy);
    let n_energy = if has_energy { s.energy as f64 } else { 0.0 };
    let chance = match c.rarity {
        Rarity::Holo => share / 3.0,
        Rarity::Rare => share * 2.0 / 3.0,
        Rarity::Uncommon => 1.0 - (1.0 - share).powf(s.uncommons as f64),
        Rarity::Common => 1.0 - (1.0 - share).powf(s.commons as f64 - n_energy),
        Rarity::Energy => 1.0 - (1.0 - share).powf(n_energy),
    };
    (chance > 0.0).then(|| 1.0 / chance)
}

/// Open a booster of `set`: its Energy, commons and uncommons, any reverse
/// holo, then the rare (a holo one time in three), in the order they're
/// revealed. A classic one: 2 Energy, 5 commons, 3 uncommons, the rare.
pub fn open_pack(set: &str, rng: &mut Rng) -> Vec<String> {
    let cards = of_set(set);
    let by = |r: Rarity| -> Vec<&Card> { cards.iter().copied().filter(|c| c.rarity == r).collect() };
    let s = set_of(set);
    let (commons, uncommons, reverse) = s.map_or((7, 3, 0), |s| (s.commons as usize, s.uncommons as usize, s.reverse as usize));
    let mut out: Vec<&Card> = Vec::new();
    let energy = by(Rarity::Energy);
    let n_energy = if energy.is_empty() { 0 } else { s.map_or(2, |s| s.energy as usize) };
    for _ in 0..n_energy {
        out.extend(pick(rng, &energy, 1));
    }
    out.extend(pick(rng, &by(Rarity::Common), commons.saturating_sub(n_energy)));
    out.extend(pick(rng, &by(Rarity::Uncommon), uncommons));
    // the reverse holo slot: any card but a basic Energy, commons most often
    for _ in 0..reverse {
        let r = match rng.below(10) {
            0 => {
                if rng.below(3) == 0 {
                    Rarity::Holo
                } else {
                    Rarity::Rare
                }
            }
            1..=3 => Rarity::Uncommon,
            _ => Rarity::Common,
        };
        let pool = by(r);
        out.extend(pick(rng, if pool.is_empty() { &cards } else { &pool }, 1));
    }
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
        // each day's three add up to a pack and a half (600 XP)
        [m(MKind::Encounters, 300, 270), m(MKind::Species, 8, 210), m(MKind::OpenPacks, 1, 120)],
        [m(MKind::Encounters, 600, 300), m(MKind::Minutes, 30, 150), m(MKind::RareGrass, 25, 150)],
        [m(MKind::Encounters, 1000, 330), m(MKind::Species, 12, 165), m(MKind::OpenPacks, 2, 105)],
        [m(MKind::Minutes, 60, 210), m(MKind::RareGrass, 50, 180), m(MKind::Encounters, 400, 210)],
        [m(MKind::Encounters, 800, 315), m(MKind::Species, 6, 105), m(MKind::Minutes, 45, 180)],
    ];
    pool[day.rem_euclid(pool.len() as i64) as usize].to_vec()
}

/// This week's two missions.
pub fn weekly_missions(week: i64) -> Vec<Mission> {
    let m = |kind, goal, xp| Mission { kind, goal, xp, done: false };
    let pool = [
        [m(MKind::Encounters, 5000, 960), m(MKind::Shiny, 1, 1200)],
        [m(MKind::Species, 30, 800), m(MKind::OpenPacks, 10, 640)],
        [m(MKind::Minutes, 300, 960), m(MKind::RareGrass, 300, 800)],
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

/// The XP bar on the overlay. It's always pinned under the shiny overlay
/// (it never floats or remembers a spot of its own), and it's off until it's
/// ticked on the Start overlay screen. (Saves from before keep their old
/// "show" / "pos" / "snapped" / "alone" fields, which are ignored now, so
/// everyone starts with it off.)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarCfg {
    /// Ticked on the Start overlay screen.
    pub on: bool,
    /// Pinned under the overlay's right end (else its left).
    pub right: bool,
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
    /// Battle pass pack rewards already put in (by reward id), so a pack
    /// reward can only ever be added once.
    pub pass_packs: Vec<String>,
    /// Sets still to come: tell me when it's out, and download its art
    /// then (by set id).
    pub notify: Vec<String>,
    pub auto_get: Vec<String>,
    /// Sets whose coming out has been seen to (told, downloaded).
    pub seen_out: Vec<String>,
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

    /// Put in the packs of battle pass reward `id` (once ever). Returns
    /// whether they went in now.
    pub fn pass_gift(&mut self, id: &str, packs: u32) -> bool {
        if self.pass_packs.iter().any(|p| p == id) {
            return false;
        }
        self.pass_packs.push(id.to_string());
        self.packs += packs;
        true
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
        if self.packs == 0 || !crate::cards::set(set).is_some_and(|s| s.out_at(now)) {
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
    fn daily_missions_make_a_pack_and_rarer_cards_are_rarer() {
        for d in 0..5 {
            let xp: u64 = daily_missions(d).iter().map(|m| m.xp).sum();
            assert_eq!(xp, XP_PER_PACK * 3 / 2, "day {d}: a pack and a half");
        }
        let card = |id: &str| by_id(id).unwrap();
        let zard = packs_per(card("base1-4")).unwrap();
        let chansey = packs_per(card("base1-3")).unwrap();
        let machamp = packs_per(card("base1-8")).unwrap();
        assert!(zard > chansey && machamp > zard * 5.0, "{zard} {chansey} {machamp}");
        // Jungle's holos are all alike
        assert_eq!(packs_per(card("base2-1")).map(|x| x.round()), packs_per(card("base2-16")).map(|x| x.round()));
        // and it shows in packs
        let mut rng = Rng::new(5);
        let (mut z, mut c, mut m) = (0, 0, 0);
        for _ in 0..60_000 {
            for id in open_pack("base1", &mut rng) {
                match id.as_str() {
                    "base1-4" => z += 1,
                    "base1-3" => c += 1,
                    "base1-8" => m += 1,
                    _ => {}
                }
            }
        }
        assert!(c > z && z > m * 4, "chansey {c} charizard {z} machamp {m}");
    }

    #[test]
    fn the_new_sets_and_their_packs() {
        assert_eq!(of_set("base2").len(), 64);
        assert_eq!(of_set("base3").len(), 62);
        assert_eq!(of_set("base5").len(), 83);
        assert_eq!(of_set("gym1").len(), 132);
        assert_eq!(of_set("gym2").len(), 132);
        // one a week, Mondays, in order
        for w in SETS[2..].windows(2) {
            assert_eq!(w[1].release - w[0].release, 7 * 86_400);
        }
        for set in ["base2", "base3", "base5", "gym1", "gym2"] {
            let s = super::set(set).unwrap();
            assert!(!s.packs.is_empty());
            let mut rng = Rng::new(9);
            for _ in 0..30 {
                let p = open_pack(set, &mut rng);
                assert_eq!(p.len(), PACK_SIZE, "{set}");
                let energy = p.iter().filter(|id| by_id(id).unwrap().rarity == Rarity::Energy).count();
                assert_eq!(energy, s.energy as usize, "{set}");
                let rare = by_id(p.last().unwrap()).unwrap().rarity;
                assert!(rare >= Rarity::Rare, "{set} ends on a rare");
                // no Energy: the rare is at the back as it is
                if s.energy == 0 {
                    assert_eq!(packed_order(&p), p);
                }
            }
        }
        // not out yet: can't be opened
        let mut f = CardFile { packs: 1, ..Default::default() };
        let mut rng = Rng::new(1);
        assert!(f.open("gym2", &mut rng, 0, SETS[6].release - 1).is_none());
        assert!(f.open("gym2", &mut rng, 0, SETS[6].release).is_some());
    }

    #[test]
    fn the_neo_legendary_and_e_card_sets() {
        for (set, n) in [("neo1", 111), ("neo2", 75), ("neo3", 66), ("neo4", 113), ("base6", 110), ("ecard1", 159), ("ecard2", 182), ("ecard3", 182)] {
            assert_eq!(of_set(set).len(), n, "{set}");
            let s = super::set(set).unwrap();
            assert_eq!(s.packs.len(), 4, "{set} pack art");
            let mut rng = Rng::new(11);
            for _ in 0..40 {
                let p = open_pack(set, &mut rng);
                assert_eq!(p.len(), s.size(), "{set}");
                let rare = by_id(p.last().unwrap()).unwrap().rarity;
                assert!(rare >= Rarity::Rare, "{set} ends on a rare");
            }
        }
        assert_eq!(super::set("neo1").unwrap().size(), 11);
        assert_eq!(super::set("base6").unwrap().size(), 11);
        assert_eq!(super::set("ecard1").unwrap().size(), 9);
        // the H holos keep their printed number
        let h = by_id("ecard2-1001").unwrap();
        assert_eq!(h.number(), "H1");
        assert!(h.image_url(true).ends_with("/ecard2/H1_hires.png"));
        assert_eq!(h.rarity, Rarity::Holo);
        // Shining and Crystal cards are rarer than the other holos
        let gyarados = packs_per(by_id("neo3-65").unwrap()).unwrap();
        let ho_oh = packs_per(by_id("neo3-7").unwrap()).unwrap();
        assert!(gyarados > ho_oh * 3.0, "{gyarados} {ho_oh}");
        // every set comes out at 00:00 CET on a Monday (23:00 UTC Sunday)
        for s in &SETS[2..] {
            assert_eq!(s.release % 86_400, 23 * 3600, "{}", s.id);
            assert_eq!((s.release / 86_400 + 4) % 7, 0, "{} is a Sunday in UTC", s.id);
        }
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
        let mut g = CardFile::default();
        assert!(g.pass_gift("s1_pack_t3", 1));
        assert!(!g.pass_gift("s1_pack_t3", 1), "a pass pack only goes in once");
        assert!(g.pass_gift("s1_pack_t8", 1));
        assert_eq!(g.packs, 2);
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
