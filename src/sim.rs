//! Battle Sim: what moves do in battle, catch rates and Poké Balls
//! (`assets/sim.json`), Platinum's catch formula, and your saved battle
//! setups (opponents and trainers, in `battle-sim.json`).

use crate::battle::World;
use crate::teams::Member;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

static RAW: &[u8] = include_bytes!("../assets/sim.json");

// ───────────────────────────── move effects ─────────────────────────────

/// A stat change: (stat, stages). Stats: atk, def, spa, spd, spe, accuracy, evasion.
pub type Boosts = HashMap<String, i8>;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Secondary {
    /// % chance.
    pub chance: Option<u8>,
    pub status: Option<String>,
    #[serde(rename = "volatileStatus")]
    pub volatile: Option<String>,
    pub boosts: Option<Boosts>,
    #[serde(rename = "self")]
    pub on_self: Option<SelfEffect>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct SelfEffect {
    pub boosts: Option<Boosts>,
    #[serde(rename = "volatileStatus")]
    pub volatile: Option<String>,
}

/// What a move does besides its damage (from Showdown's data).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MoveEffect {
    /// Accuracy; 0 = never misses.
    pub acc: u8,
    pub pri: i8,
    pub flags: String,
    /// "normal" when missing; "self", "allAdjacentFoes", "foeSide"…
    pub target: String,
    pub secondary: Option<Secondary>,
    pub secondaries: Option<Vec<Secondary>>,
    #[serde(rename = "self")]
    pub on_self: Option<SelfEffect>,
    #[serde(rename = "selfBoost")]
    pub self_boost: Option<SelfEffect>,
    #[serde(rename = "volatileStatus")]
    pub volatile: Option<String>,
    pub boosts: Option<Boosts>,
    pub status: Option<String>,
    /// A number, or [min, max].
    pub multihit: Option<serde_json::Value>,
    #[serde(rename = "critRatio")]
    pub crit_ratio: Option<u8>,
    /// [numerator, denominator].
    pub drain: Option<[u8; 2]>,
    pub recoil: Option<[u8; 2]>,
    pub heal: Option<[u8; 2]>,
    #[serde(rename = "selfSwitch")]
    pub self_switch: Option<serde_json::Value>,
    #[serde(rename = "forceSwitch")]
    pub force_switch: bool,
    pub weather: Option<String>,
    #[serde(rename = "sideCondition")]
    pub side_condition: Option<String>,
    #[serde(rename = "pseudoWeather")]
    pub pseudo_weather: Option<String>,
    pub selfdestruct: Option<String>,
    pub ohko: Option<serde_json::Value>,
    /// Fixed damage: "level" or a number.
    pub damage: Option<serde_json::Value>,
    #[serde(rename = "stallingMove")]
    pub stalling: bool,
    #[serde(rename = "breaksProtect")]
    pub breaks_protect: bool,
    #[serde(rename = "willCrit")]
    pub will_crit: bool,
    #[serde(rename = "thawsTarget")]
    pub thaws_target: bool,
    #[serde(rename = "hasCrashDamage")]
    pub crash: bool,
    #[serde(rename = "sleepUsable")]
    pub sleep_usable: bool,
}

impl MoveEffect {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.split(',').any(|f| f == flag)
    }

    /// Every secondary effect (one or several).
    pub fn secondaries(&self) -> Vec<&Secondary> {
        match (&self.secondaries, &self.secondary) {
            (Some(v), _) => v.iter().collect(),
            (None, Some(s)) => vec![s],
            _ => Vec::new(),
        }
    }

    /// (min, max) hits.
    pub fn hits(&self) -> (u8, u8) {
        match &self.multihit {
            Some(serde_json::Value::Number(n)) => {
                let n = n.as_u64().unwrap_or(1) as u8;
                (n, n)
            }
            Some(serde_json::Value::Array(a)) if a.len() == 2 => (a[0].as_u64().unwrap_or(1) as u8, a[1].as_u64().unwrap_or(1) as u8),
            _ => (1, 1),
        }
    }

    /// Hits both foes (and maybe the ally) in a double battle.
    pub fn spread(&self) -> bool {
        self.target == "allAdjacentFoes" || self.target == "allAdjacent"
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Raw {
    source: String,
    moves: HashMap<String, MoveEffect>,
    /// Species id → "M" / "F" / "N", or the share that's male.
    gender: HashMap<String, serde_json::Value>,
    /// Dex number → catch rate.
    catch: HashMap<String, u16>,
    /// Dex number → growth rate ("medium-slow"…).
    growth: HashMap<String, String>,
}

pub struct SimData {
    pub source: String,
    pub moves: HashMap<String, MoveEffect>,
    gender: HashMap<String, serde_json::Value>,
    catch: HashMap<u32, u16>,
    growth: HashMap<u32, String>,
}

/// Who can be which gender.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gender {
    /// Share that's male (0.5 for most).
    Ratio(f32),
    Male,
    Female,
    None,
}

impl SimData {
    fn load() -> Self {
        let raw: Raw = serde_json::from_slice(RAW).unwrap_or_default();
        SimData {
            source: raw.source,
            moves: raw.moves,
            gender: raw.gender,
            catch: raw.catch.into_iter().filter_map(|(k, v)| k.parse().ok().map(|k| (k, v))).collect(),
            growth: raw.growth.into_iter().filter_map(|(k, v)| k.parse().ok().map(|k| (k, v))).collect(),
        }
    }

    /// A move's effects by name or key.
    pub fn effect(&self, name: &str) -> Option<&MoveEffect> {
        let id = crate::battle::to_id(name);
        self.moves.get(crate::battle::move_alias(&id))
    }

    pub fn catch_rate(&self, dex: u32) -> Option<u16> {
        self.catch.get(&dex).copied()
    }

    pub fn growth(&self, dex: u32) -> Option<&str> {
        self.growth.get(&dex).map(String::as_str)
    }

    pub fn gender(&self, species: &str) -> Gender {
        let d = crate::battle::data();
        let Some(sp) = d.species(species) else { return Gender::Ratio(0.5) };
        let v = self.gender.get(&sp.id).or_else(|| self.gender.get(&sp.base));
        match v {
            Some(serde_json::Value::String(s)) if s == "M" => Gender::Male,
            Some(serde_json::Value::String(s)) if s == "F" => Gender::Female,
            Some(serde_json::Value::String(_)) => Gender::None,
            Some(serde_json::Value::Number(n)) => Gender::Ratio(n.as_f64().unwrap_or(0.5) as f32),
            _ => Gender::Ratio(0.5),
        }
    }
}

pub fn data() -> &'static SimData {
    static D: OnceLock<SimData> = OnceLock::new();
    D.get_or_init(SimData::load)
}

// ───────────────────────────── catching ─────────────────────────────

/// The Poké Balls in Platinum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ball {
    Poke,
    Great,
    Ultra,
    Master,
    Safari,
    Net,
    Dive,
    Nest,
    Repeat,
    Timer,
    Luxury,
    Premier,
    Dusk,
    Heal,
    Quick,
    Cherish,
}

pub const BALLS: [Ball; 16] = [
    Ball::Poke,
    Ball::Great,
    Ball::Ultra,
    Ball::Quick,
    Ball::Timer,
    Ball::Dusk,
    Ball::Net,
    Ball::Dive,
    Ball::Nest,
    Ball::Repeat,
    Ball::Luxury,
    Ball::Heal,
    Ball::Premier,
    Ball::Cherish,
    Ball::Safari,
    Ball::Master,
];

impl Ball {
    pub fn name(self) -> &'static str {
        match self {
            Ball::Poke => "Poké Ball",
            Ball::Great => "Great Ball",
            Ball::Ultra => "Ultra Ball",
            Ball::Master => "Master Ball",
            Ball::Safari => "Safari Ball",
            Ball::Net => "Net Ball",
            Ball::Dive => "Dive Ball",
            Ball::Nest => "Nest Ball",
            Ball::Repeat => "Repeat Ball",
            Ball::Timer => "Timer Ball",
            Ball::Luxury => "Luxury Ball",
            Ball::Premier => "Premier Ball",
            Ball::Dusk => "Dusk Ball",
            Ball::Heal => "Heal Ball",
            Ball::Quick => "Quick Ball",
            Ball::Cherish => "Cherish Ball",
        }
    }

    /// When it's better than a Poké Ball.
    pub fn when(self) -> &'static str {
        match self {
            Ball::Poke | Ball::Luxury | Ball::Premier | Ball::Heal | Ball::Cherish => "",
            Ball::Great => "always ×1.5",
            Ball::Ultra => "always ×2",
            Ball::Master => "always catches",
            Ball::Safari => "×1.5, Great Marsh only",
            Ball::Net => "×3 on Water and Bug types",
            Ball::Dive => "×3.5 while surfing or fishing",
            Ball::Nest => "better the lower its level (below 30)",
            Ball::Repeat => "×3 if you've caught one before",
            Ball::Timer => "better every turn, up to ×4 after 30",
            Ball::Dusk => "×3.5 at night or in caves",
            Ball::Quick => "×4 on the first turn",
        }
    }
}

/// What the catch depends on besides HP and status.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CatchSituation {
    pub level: u8,
    /// Turns already gone (0 = the first throw).
    pub turn: u32,
    pub night_or_cave: bool,
    pub water: bool,
    pub caught_before: bool,
    pub water_or_bug: bool,
}

/// The ball's multiplier in Platinum.
pub fn ball_mult(b: Ball, s: &CatchSituation) -> f64 {
    match b {
        Ball::Great | Ball::Safari => 1.5,
        Ball::Ultra => 2.0,
        Ball::Master => 255.0,
        Ball::Net if s.water_or_bug => 3.0,
        Ball::Dive if s.water => 3.5,
        Ball::Nest => ((40.0 - s.level as f64) / 10.0).max(1.0),
        Ball::Repeat if s.caught_before => 3.0,
        Ball::Timer => ((s.turn as f64 + 10.0) / 10.0).min(4.0),
        Ball::Dusk if s.night_or_cave => 3.5,
        Ball::Quick if s.turn == 0 => 4.0,
        _ => 1.0,
    }
}

/// Status the catch counts: sleep and freeze ×2, the rest ×1.5.
pub fn status_mult(status: crate::calc::Status) -> f64 {
    use crate::calc::Status as S;
    match status {
        S::Sleep | S::Freeze => 2.0,
        S::Burn | S::Paralysis | S::Poison | S::Toxic => 1.5,
        S::None => 1.0,
    }
}

fn isqrt(n: u64) -> u64 {
    (n as f64).sqrt().floor() as u64
}

/// The shake value b (out of 65,536) each of the four checks must beat,
/// or None when the ball can't fail. X is the catch value.
fn shake_value(rate: u16, max_hp: u32, hp: u32, ball: f64, status: f64) -> Result<u64, bool> {
    if ball >= 255.0 {
        return Err(true);
    }
    let m = max_hp.max(1) as f64;
    let h = hp.clamp(1, max_hp.max(1)) as f64;
    let x = ((((3.0 * m - 2.0 * h) * rate as f64 * ball) / (3.0 * m)).floor() * status).floor();
    if x >= 255.0 {
        return Err(true);
    }
    if x < 1.0 {
        return Err(false);
    }
    let b = 1_048_560u64 / isqrt(isqrt(16_711_680u64 / x as u64)).max(1);
    if b >= 65_536 {
        return Err(true);
    }
    Ok(b)
}

/// The chance (0–1) one ball catches it, by Platinum's formula: the catch
/// value X, then four shake checks.
pub fn catch_chance(rate: u16, max_hp: u32, hp: u32, ball: f64, status: f64) -> f64 {
    match shake_value(rate, max_hp, hp, ball, status) {
        Err(true) => 1.0,
        Err(false) => 0.0,
        Ok(b) => (b as f64 / 65_536.0).powi(4),
    }
}

/// One throw: how many times the ball shakes (0–3) and whether it's caught.
pub fn throw(rate: u16, max_hp: u32, hp: u32, ball: f64, status: f64, rng: &mut Rng) -> (u8, bool) {
    match shake_value(rate, max_hp, hp, ball, status) {
        Err(true) => (3, true),
        Err(false) => (0, false),
        Ok(b) => {
            for i in 0..4u8 {
                if rng.below(65_536) as u64 >= b {
                    return (i.min(3), false);
                }
            }
            (3, true)
        }
    }
}

/// How many balls to have a `want` chance (0–1) of a catch by then.
pub fn balls_for(p: f64, want: f64) -> Option<u32> {
    if p >= 1.0 {
        return Some(1);
    }
    if p <= 0.0 {
        return None;
    }
    Some(((1.0 - want).ln() / (1.0 - p).ln()).ceil().max(1.0) as u32)
}

// ───────────────────────────── wild Pokémon ─────────────────────────────

/// A small random number generator (xorshift64*), seeded so a roll can be
/// repeated.
#[derive(Clone, Copy, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1) ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn from_time() -> Self {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
        Rng::new(t)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0..n
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as u32
        }
    }

    /// true with chance p (0–1).
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64 * 1.0 < p
    }
}

/// "17–20" or "35" → (17, 20).
pub fn parse_levels(s: &str) -> (u8, u8) {
    let nums: Vec<u8> = s.split(|c: char| !c.is_ascii_digit()).filter_map(|x| x.parse().ok()).collect();
    match nums.as_slice() {
        [a, b, ..] => ((*a).min(*b).max(1), (*a).max(*b).min(100)),
        [a] => ((*a).max(1), (*a).max(1)),
        _ => (5, 5),
    }
}

/// The moves a wild Pokémon knows: the last four it learned by levelling
/// up at or below its level (oldest first, no repeats).
pub fn wild_moves(learnset: &[(String, String)], level: u8) -> Vec<String> {
    let mut known: Vec<String> = Vec::new();
    for (lv, key) in learnset {
        let Ok(l) = lv.trim().parse::<u8>() else { continue };
        if l > level || key.is_empty() {
            continue;
        }
        known.retain(|k| k != key);
        known.push(key.clone());
    }
    let n = known.len();
    known.split_off(n.saturating_sub(4))
}

/// Random IVs (0–31 each).
pub fn random_ivs(rng: &mut Rng) -> [u16; 6] {
    let mut v = [0u16; 6];
    for x in &mut v {
        *x = rng.below(32) as u16;
    }
    v
}

/// A gender for a wild one.
pub fn random_gender(g: Gender, rng: &mut Rng) -> Option<char> {
    match g {
        Gender::Male => Some('M'),
        Gender::Female => Some('F'),
        Gender::None => None,
        Gender::Ratio(m) => Some(if rng.chance(m as f64) { 'M' } else { 'F' }),
    }
}

// ───────────────────────────── saved setups ─────────────────────────────

/// What kind of battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Wild,
    Trainer,
    Catch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Format {
    #[default]
    Singles,
    Doubles,
}

/// How a trainer picks its moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Ai {
    /// Any move at random (wild Pokémon).
    Random,
    /// Strong hits, KOs and type matchups, like a gym leader.
    #[default]
    Smart,
    /// Smart, and switches out of bad matchups.
    Expert,
}

/// A trainer you made.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Trainer {
    pub id: u64,
    pub name: String,
    /// "Ace Trainer", "Gym Leader"…
    pub class: String,
    pub world: World,
    pub ai: Ai,
    pub members: Vec<Member>,
    /// Items it can use: (name, how many).
    pub items: Vec<(String, u8)>,
    pub doubles: bool,
    pub notes: String,
    pub updated: u64,
}

/// One opponent saved on its own.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SavedFoe {
    pub id: u64,
    pub member: Member,
    pub world: World,
    pub updated: u64,
}

/// A battle as it was set up, to start again.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Setup {
    pub mode: Mode,
    pub format: Format,
    /// Your team (Team Planner id) and which members.
    pub team: Option<u64>,
    pub picks: Vec<usize>,
    /// The wild Pokémon (or the one to catch).
    pub foes: Vec<Member>,
    /// Where it was rolled from, when it came from a route.
    pub place: String,
    pub method: String,
    pub trainer: Option<u64>,
    pub world: World,
    pub when: u64,
    /// The last auto-sim of this setup: (won, battles).
    pub last_sim: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SimFile {
    pub version: u32,
    pub trainers: Vec<Trainer>,
    pub foes: Vec<SavedFoe>,
    /// The last few setups, newest first.
    pub recent: Vec<Setup>,
}

impl SimFile {
    fn has_data(&self) -> bool {
        !self.trainers.is_empty() || !self.foes.is_empty() || !self.recent.is_empty()
    }

    /// Remember a setup (newest first, the same one only once, 12 at most).
    pub fn remember(&mut self, s: Setup) {
        self.recent.retain(|r| !(r.mode == s.mode && r.format == s.format && r.team == s.team && r.picks == s.picks && r.foes == s.foes && r.trainer == s.trainer));
        self.recent.insert(0, s);
        self.recent.truncate(12);
    }
}

pub fn load_from(dir: &Path) -> (SimFile, Option<String>) {
    crate::safefile::load(dir, "battle-sim", "Battle Sim")
}

pub fn load() -> (SimFile, Option<String>) {
    load_from(&crate::config::data_dir())
}

pub fn save_to(dir: &Path, f: &SimFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "battle-sim", f, first_this_run, SimFile::has_data)
}

pub fn save(f: &SimFile, first_this_run: bool) -> Result<(), String> {
    save_to(&crate::config::data_dir(), f, first_this_run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_loads() {
        let d = data();
        assert!(d.moves.len() > 900);
        let tb = d.effect("Thunderbolt").expect("tbolt");
        assert_eq!(tb.acc, 100);
        assert_eq!(tb.secondaries()[0].status.as_deref(), Some("par"));
        let cc = d.effect("Close Combat").expect("cc");
        assert_eq!(cc.on_self.as_ref().and_then(|s| s.boosts.as_ref()).and_then(|b| b.get("def")).copied(), Some(-1));
        assert_eq!(d.effect("Protect").map(|m| (m.pri, m.stalling)), Some((4, true)));
        assert_eq!(d.effect("Bullet Seed").map(|m| m.hits()), Some((2, 5)));
        assert_eq!(d.effect("Aerial Ace").map(|m| m.acc), Some(0));
        // the game's old spellings
        assert!(d.effect("Hi Jump Kick").is_some());
        assert_eq!(d.catch_rate(1), Some(45));
        assert_eq!(d.catch_rate(150), Some(3));
        assert_eq!(d.gender("Tauros"), Gender::Male);
        assert_eq!(d.gender("Magnemite"), Gender::None);
        assert_eq!(d.gender("Bulbasaur"), Gender::Ratio(0.875));
        assert_eq!(d.gender("Pidgey"), Gender::Ratio(0.5));
        // every species the planner knows has a catch rate
        for s in &crate::battle::data().species {
            assert!(d.catch_rate(s.num).is_some(), "{}", s.name);
        }
    }

    #[test]
    fn catching() {
        // rate 45, full HP, Poké Ball: X = 15 → b = 32767 → about 6.25%
        let p = catch_chance(45, 100, 100, 1.0, 1.0);
        assert!((p - 0.0625).abs() < 0.001, "{p}");
        // lower HP and sleep only ever help
        assert!(catch_chance(45, 100, 1, 1.0, 2.0) > catch_chance(45, 100, 50, 1.0, 2.0));
        assert!(catch_chance(45, 100, 50, 1.0, 2.0) > catch_chance(45, 100, 50, 1.0, 1.0));
        assert_eq!(catch_chance(255, 100, 1, 2.0, 2.0), 1.0);
        assert_eq!(catch_chance(3, 300, 300, 255.0, 1.0), 1.0);
        let s = CatchSituation { level: 10, turn: 0, ..Default::default() };
        assert_eq!(ball_mult(Ball::Quick, &s), 4.0);
        assert_eq!(ball_mult(Ball::Nest, &s), 3.0);
        assert_eq!(ball_mult(Ball::Timer, &CatchSituation { turn: 40, ..s }), 4.0);
        assert_eq!(balls_for(0.5, 0.9), Some(4));
        // throws land near the odds
        let mut r = Rng::new(3);
        let p = catch_chance(45, 100, 30, 2.0, 1.5);
        let n = 20_000;
        let caught = (0..n).filter(|_| throw(45, 100, 30, 2.0, 1.5, &mut r).1).count() as f64 / n as f64;
        assert!((caught - p).abs() < 0.015, "{caught} vs {p}");
    }

    #[test]
    fn wild() {
        assert_eq!(parse_levels("32–35"), (32, 35));
        assert_eq!(parse_levels("7"), (7, 7));
        let ls: Vec<(String, String)> = [("1", "tackle"), ("3", "growl"), ("7", "leech_seed"), ("9", "vine_whip"), ("13", "poison_powder"), ("13", "sleep_powder"), ("15", "take_down")]
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        assert_eq!(wild_moves(&ls, 2), vec!["tackle"]);
        assert_eq!(wild_moves(&ls, 13), vec!["leech_seed", "vine_whip", "poison_powder", "sleep_powder"]);
        let mut r = Rng::new(42);
        let ivs = random_ivs(&mut r);
        assert!(ivs.iter().all(|v| *v <= 31));
        let mut males = 0;
        for _ in 0..1000 {
            if random_gender(Gender::Ratio(0.875), &mut r) == Some('M') {
                males += 1;
            }
        }
        assert!((800..950).contains(&males), "{males}");
    }

    #[test]
    fn safe_files() {
        let dir = std::env::temp_dir().join(format!("sim-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut f = SimFile::default();
        f.trainers.push(Trainer { id: 1, name: "Roark".into(), ..Default::default() });
        save_to(&dir, &f, true).expect("save");
        assert_eq!(load_from(&dir).0.trainers.len(), 1);
        std::fs::write(crate::safefile::file_in(&dir, "battle-sim"), "nope").expect("write");
        save_to(&dir, &f, true).expect("save");
        let back = load_from(&dir);
        assert_eq!(back.0.trainers.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
