//! Breeding rules for the Breeding Planner.
//!
//! Platinum Together breeds by the newer games' rules (its patch notes):
//! Everstone, Destiny Knot, the mother's Ability and Poké Ball, egg moves
//! from either parent, Power items pass their stat, the Rift Knot is an
//! Everstone and a Destiny Knot in one, the Egg Warmer hatches Eggs 2× (4×
//! with Flame Body), and shiny Eggs get 3 tries with the Shiny Charm and 8
//! with the Masuda Method too. Wynaut needs a Lax Incense, and Partner
//! starters can never be bred.
//!
//! Anything the notes don't spell out uses the newer games' numbers and adds
//! a line to `notes`, so the screens can say it isn't confirmed. Every such
//! number is in this file, so they're easy to correct.
//!
//! Data: `assets/breed.json` (egg groups, gender ratio, egg cycles, incense
//! babies and offspring rules, from Pokémon Essentials v21's newer-games
//! data) for every Pokémon in the game's wiki. Abilities come from the
//! Battle Sim's data, learnsets and evolutions from the wiki.
#![allow(dead_code)] // the Breeding Planner screens arrive in stage 2

use crate::battle::{self, World};
use crate::wiki::Wiki;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

static RAW: &[u8] = include_bytes!("../assets/breed.json");

/// The base shiny rate in Platinum Together.
pub const SHINY_BASE: f64 = 4096.0;
/// Steps in one egg cycle (Platinum's own count).
pub const CYCLE_STEPS: u32 = 255;
pub const DITTO: &str = "DITTO";
pub const NO_EGGS: &str = "No Eggs";

const NOTE_EGG_RATE: &str = "How often an Egg turns up uses the newer games' numbers. Not confirmed for this mod.";
const NOTE_ABILITY: &str = "Ability chances use the newer games' numbers (80% for the same Ability, 60% for a hidden one). Not confirmed for this mod.";
const NOTE_BALL_DITTO: &str = "With Ditto the Egg gets the other parent's ball, as in the newer games. Not confirmed for this mod.";
const NOTE_BALL_SAME: &str = "Two parents of the same species: the ball comes from either one, as in the newer games. Not confirmed for this mod.";
const NOTE_STEPS: &str = "Steps use 255 per egg cycle, as in Platinum. Not confirmed for this mod.";
const NOTE_MASUDA: &str = "Masuda alone is 6 tries: worked out from the notes' 8 with the Shiny Charm too.";
const NOTE_TWO_POWER: &str = "Two Power items: one of them works, picked at random, as in the newer games. Not confirmed for this mod.";
const NOTE_LEVEL_MOVES: &str = "Moves both parents know that the baby learns by level may pass too, as in the newer games. Not confirmed for this mod.";
const NOTE_TOO_MANY: &str = "More than 4 moves would pass: the baby keeps 4 of them.";

// ───────────────────────────── data ─────────────────────────────

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SpeciesBreed {
    /// Egg groups ("Monster", "Water 1", "No Eggs", "Ditto"…).
    #[serde(rename = "g")]
    pub groups: Vec<String>,
    /// Female chance in eighths (0 = always male, 8 = always female),
    /// -1 = genderless.
    #[serde(rename = "r")]
    pub ratio: i8,
    /// Egg cycles.
    #[serde(rename = "c")]
    pub cycles: u16,
    /// For incense babies (Wynaut…): the incense a parent must hold.
    #[serde(rename = "inc", default)]
    pub incense: Option<String>,
    /// What the Egg can be when it isn't just the baby (Nidoran♀ gives
    /// either Nidoran, Manaphy gives Phione).
    #[serde(rename = "off", default)]
    pub offspring: Vec<String>,
}

impl SpeciesBreed {
    pub fn genderless(&self) -> bool {
        self.ratio < 0
    }
    pub fn no_eggs(&self) -> bool {
        self.groups.iter().any(|g| g == NO_EGGS)
    }
    /// Chance a hatched one is female (None: genderless).
    pub fn female_chance(&self) -> Option<f64> {
        (self.ratio >= 0).then(|| self.ratio as f64 / 8.0)
    }
    pub fn gender_label(&self) -> &'static str {
        match self.ratio {
            -1 => "Genderless",
            0 => "Always male",
            1 => "87.5% male",
            2 => "75% male",
            4 => "50% male, 50% female",
            6 => "75% female",
            7 => "87.5% female",
            8 => "Always female",
            _ => "Unknown",
        }
    }
}

#[derive(Deserialize, Default)]
struct RawFile {
    species: HashMap<String, SpeciesBreed>,
}

/// Breeding data by the wiki's species id ("NIDORAN_F").
pub fn info(id: &str) -> Option<&'static SpeciesBreed> {
    static D: OnceLock<HashMap<String, SpeciesBreed>> = OnceLock::new();
    D.get_or_init(|| serde_json::from_slice::<RawFile>(RAW).map(|f| f.species).unwrap_or_default()).get(id)
}

/// Every egg group in the game, in a sensible order.
pub const GROUPS: [&str; 15] = [
    "Monster", "Water 1", "Bug", "Flying", "Field", "Fairy", "Grass", "Human-Like", "Water 3", "Mineral", "Amorphous",
    "Water 2", "Ditto", "Dragon", NO_EGGS,
];

// ───────────────────────────── parents ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
pub enum Gender {
    Male,
    #[default]
    Female,
    Genderless,
}

/// Which of its species' Abilities a Pokémon has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
pub enum Slot {
    #[default]
    First,
    Second,
    Hidden,
}

/// What a parent holds, as far as breeding cares.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
pub enum Held {
    #[default]
    Nothing,
    Everstone,
    DestinyKnot,
    /// Everstone and Destiny Knot in one (Rift Shop).
    RiftKnot,
    /// Power Weight/Bracer/Belt/Lens/Band/Anklet: passes that stat (0 = HP).
    Power(usize),
    /// An incense, by name ("Lax Incense").
    Incense(String),
    Other(String),
}

pub const POWER_ITEMS: [&str; 6] = ["Power Weight", "Power Bracer", "Power Belt", "Power Lens", "Power Band", "Power Anklet"];

impl Held {
    pub fn from_name(name: &str) -> Held {
        let n = name.trim();
        if n.is_empty() {
            return Held::Nothing;
        }
        if let Some(i) = POWER_ITEMS.iter().position(|p| p.eq_ignore_ascii_case(n)) {
            return Held::Power(i);
        }
        match n.to_ascii_lowercase().as_str() {
            "everstone" => Held::Everstone,
            "destiny knot" => Held::DestinyKnot,
            "rift knot" => Held::RiftKnot,
            l if l.ends_with(" incense") => Held::Incense(n.to_string()),
            _ => Held::Other(n.to_string()),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Held::Nothing => String::new(),
            Held::Everstone => "Everstone".into(),
            Held::DestinyKnot => "Destiny Knot".into(),
            Held::RiftKnot => "Rift Knot".into(),
            Held::Power(i) => POWER_ITEMS[(*i).min(5)].into(),
            Held::Incense(n) | Held::Other(n) => n.clone(),
        }
    }

    /// Passes the holder's nature.
    pub fn keeps_nature(&self) -> bool {
        matches!(self, Held::Everstone | Held::RiftKnot)
    }
    /// Five stats instead of three.
    pub fn knot(&self) -> bool {
        matches!(self, Held::DestinyKnot | Held::RiftKnot)
    }
    pub fn power(&self) -> Option<usize> {
        match self {
            Held::Power(i) => Some((*i).min(5)),
            _ => None,
        }
    }
}

/// One parent at the Day Care.
#[derive(Debug, Clone, Default, serde::Serialize, Deserialize)]
#[serde(default)]
pub struct Parent {
    /// The wiki's species id ("CROAGUNK").
    pub species: String,
    pub gender: Gender,
    pub nature: Option<String>,
    /// HP, Atk, Def, SpA, SpD, Spe. None = not known.
    pub ivs: [Option<u8>; 6],
    pub ability: Slot,
    pub held: Held,
    /// "Poké Ball", "Dusk Ball"…
    pub ball: String,
    /// Move keys as the wiki writes them ("ice_punch").
    pub moves: Vec<String>,
    /// From another trainer (a trade or the GTS): the Day Care counts
    /// different trainers.
    pub other_trainer: bool,
    /// Counts for the Masuda Method: traded from another player since
    /// 0.3.2 (your own from other worlds don't).
    pub masuda: bool,
    /// A Partner starter: can never be bred.
    pub partner: bool,
}

impl Parent {
    pub fn new(species: &str, gender: Gender) -> Parent {
        Parent { species: species.to_string(), gender, ball: "Poké Ball".into(), ..Default::default() }
    }
    pub fn is_ditto(&self) -> bool {
        self.species == DITTO
    }
    /// Its gender as the species allows (genderless species are always
    /// genderless, one-gender species always that gender).
    pub fn gender(&self) -> Gender {
        match info(&self.species).map(|i| i.ratio) {
            Some(-1) => Gender::Genderless,
            Some(0) => Gender::Male,
            Some(8) => Gender::Female,
            _ if self.gender == Gender::Genderless => Gender::Female,
            _ => self.gender,
        }
    }
}

// ───────────────────────────── compatibility ─────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Why {
    Unknown(String),
    Partner,
    TwoDitto,
    NoEggs(String),
    SameGender,
    GenderlessNeedsDitto,
    NoSharedGroup,
}

impl Why {
    pub fn text(&self) -> String {
        match self {
            Why::Unknown(s) => format!("No breeding data for {s}."),
            Why::Partner => "Partner starters can never be bred.".into(),
            Why::TwoDitto => "Two Ditto can't make an Egg.".into(),
            Why::NoEggs(n) => format!("{n} can't make Eggs at all."),
            Why::SameGender => "They're the same gender.".into(),
            Why::GenderlessNeedsDitto => "A genderless Pokémon can only breed with Ditto.".into(),
            Why::NoSharedGroup => "They don't share an egg group.".into(),
        }
    }
}

/// How well two Pokémon get along: decides how often an Egg turns up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Compat {
    Low,
    Medium,
    High,
}

impl Compat {
    /// The Day Care man's words.
    pub fn line(self) -> &'static str {
        match self {
            Compat::High => "The two seem to get along very well.",
            Compat::Medium => "The two seem to get along.",
            Compat::Low => "The two don't seem to like each other much.",
        }
    }
    /// Chance of an Egg every 256 steps (newer games; not confirmed).
    pub fn egg_chance(self, oval_charm: bool) -> f64 {
        match (self, oval_charm) {
            (Compat::High, false) => 0.70,
            (Compat::Medium, false) => 0.50,
            (Compat::Low, false) => 0.20,
            (Compat::High, true) => 0.88,
            (Compat::Medium, true) => 0.80,
            (Compat::Low, true) => 0.40,
        }
    }
    /// About how many steps per Egg.
    pub fn steps_per_egg(self, oval_charm: bool) -> u32 {
        (256.0 / self.egg_chance(oval_charm)).round() as u32
    }
}

fn display(id: &str, wiki: Option<&Wiki>) -> String {
    wiki.and_then(|w| w.mon(id)).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string())
}

/// Whether two Pokémon can breed, and how well they get along.
pub fn compat(a: &Parent, b: &Parent, wiki: Option<&Wiki>) -> Result<Compat, Why> {
    let ia = info(&a.species).ok_or_else(|| Why::Unknown(display(&a.species, wiki)))?;
    let ib = info(&b.species).ok_or_else(|| Why::Unknown(display(&b.species, wiki)))?;
    if a.partner || b.partner {
        return Err(Why::Partner);
    }
    if a.is_ditto() && b.is_ditto() {
        return Err(Why::TwoDitto);
    }
    for (p, i) in [(a, ia), (b, ib)] {
        if i.no_eggs() {
            return Err(Why::NoEggs(display(&p.species, wiki)));
        }
    }
    let ditto = a.is_ditto() || b.is_ditto();
    if !ditto {
        let (ga, gb) = (a.gender(), b.gender());
        if ga == Gender::Genderless || gb == Gender::Genderless {
            return Err(Why::GenderlessNeedsDitto);
        }
        if ga == gb {
            return Err(Why::SameGender);
        }
        if !ia.groups.iter().any(|g| ib.groups.contains(g)) {
            return Err(Why::NoSharedGroup);
        }
    }
    let same_species = !ditto && a.species == b.species;
    let other_trainer = a.other_trainer || b.other_trainer;
    Ok(match (same_species, other_trainer) {
        (true, true) => Compat::High,
        (true, false) | (false, true) => Compat::Medium,
        (false, false) => Compat::Low,
    })
}

// ───────────────────────────── the Egg ─────────────────────────────

/// What the Egg will be like.
#[derive(Debug, Clone, Default)]
pub struct Egg {
    /// What it hatches into, with chances (usually one).
    pub species: Vec<(String, f64)>,
    /// The parent whose species, Ability and ball the Egg takes: 0 or 1.
    pub base: usize,
    /// "Hold a Lax Incense to get Wynaut instead": the baby you'd miss.
    pub incense_missing: Option<(String, String)>,
    /// Chance it's female (None: genderless).
    pub female: Option<f64>,
    /// Stats that come from the parents (3, or 5 with a knot).
    pub inherited: usize,
    /// Chance each stat is 31.
    pub perfect: [f64; 6],
    /// Natures it can get from a parent, with chances (None = that
    /// parent's nature isn't entered). Empty: random, 1 in 25 each.
    pub nature: Vec<(Option<String>, f64)>,
    /// Abilities with chances.
    pub abilities: Vec<(String, Slot, f64)>,
    /// Balls with chances.
    pub ball: Vec<(String, f64)>,
    /// Egg moves it will know.
    pub moves: Vec<String>,
    /// Moves that may pass too (both parents know them, learned by level).
    pub maybe_moves: Vec<String>,
    pub cycles: u16,
    /// How well the parents get along.
    pub compat: Option<Compat>,
    /// Shiny odds as "1 in N".
    pub shiny_one_in: f64,
    /// Rules that aren't confirmed for this mod, as they apply here.
    pub notes: Vec<&'static str>,
}

/// The settings that change what an Egg is like or how long it takes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Setup {
    pub world: World,
    /// The world has hidden Abilities.
    pub hidden_abilities: bool,
    pub shiny_charm: bool,
    pub oval_charm: bool,
    pub egg_warmer: bool,
    /// Flame Body (or Magma Armor) in the party.
    pub flame_body: bool,
}

/// The chance a value comes out as `want` when it's `v` (None: not known,
/// so random).
fn p_is(v: Option<u8>, want: u8) -> f64 {
    match v {
        Some(x) => f64::from(u8::from(x == want)),
        None => 1.0 / 32.0,
    }
}

/// The chance the Egg gets every wanted IV (e.g. `[Some(31), None, …]`):
/// three stats (five with a Destiny Knot or Rift Knot) come from a parent,
/// each from either at random; a Power item makes its stat one of them,
/// from its holder. The rest are random.
pub fn iv_chance(a: &Parent, b: &Parent, want: &[Option<u8>; 6]) -> f64 {
    let n = if a.held.knot() || b.held.knot() { 5 } else { 3 };
    let forced: Vec<(f64, Option<(usize, &Parent)>)> = match (a.held.power(), b.held.power()) {
        (Some(s), Some(t)) => vec![(0.5, Some((s, a))), (0.5, Some((t, b)))],
        (Some(s), None) => vec![(1.0, Some((s, a)))],
        (None, Some(t)) => vec![(1.0, Some((t, b)))],
        (None, None) => vec![(1.0, None)],
    };
    let mut total = 0.0;
    for (pf, f) in forced {
        let k = n - f.is_some() as usize;
        let pool: Vec<usize> = (0..6).filter(|s| f.map_or(true, |(fs, _)| fs != *s)).collect();
        let masks: Vec<u32> = (0u32..(1 << pool.len())).filter(|m| m.count_ones() as usize == k).collect();
        let each = 1.0 / masks.len() as f64;
        for m in masks {
            let picked = |s: usize| pool.iter().position(|p| *p == s).is_some_and(|i| m & (1 << i) != 0);
            let mut p = 1.0;
            for (s, w) in want.iter().enumerate() {
                let Some(w) = *w else { continue };
                p *= match f {
                    Some((fs, holder)) if fs == s => p_is(holder.ivs[s], w),
                    _ if picked(s) => 0.5 * p_is(a.ivs[s], w) + 0.5 * p_is(b.ivs[s], w),
                    _ => 1.0 / 32.0,
                };
            }
            total += pf * each * p;
        }
    }
    total
}

/// How many Eggs for a `confidence` chance of at least one with chance `p`.
pub fn eggs_for(p: f64, confidence: f64) -> Option<u32> {
    if p <= 0.0 {
        return None;
    }
    if p >= 1.0 {
        return Some(1);
    }
    Some(((1.0 - confidence).ln() / (1.0 - p).ln()).ceil().max(1.0) as u32)
}

/// Shiny tries per Egg: 1, 3 with the Shiny Charm, 6 with the Masuda
/// Method, 8 with both.
pub fn shiny_rolls(charm: bool, masuda: bool) -> u32 {
    1 + if charm { 2 } else { 0 } + if masuda { 5 } else { 0 }
}

/// Shiny odds as "1 in N" for that many tries.
pub fn shiny_one_in(rolls: u32) -> f64 {
    1.0 / (1.0 - (1.0 - 1.0 / SHINY_BASE).powi(rolls as i32))
}

/// About how many steps an Egg takes to hatch.
pub fn hatch_steps(cycles: u16, egg_warmer: bool, flame_body: bool) -> u32 {
    let speed = if flame_body { 2 } else { 1 } * if egg_warmer { 2 } else { 1 };
    (cycles as u32 * CYCLE_STEPS).div_ceil(speed)
}

/// The Abilities a species has, without the hidden one.
pub fn normal_abilities(id: &str, world: World) -> Vec<String> {
    let Some(sp) = battle::data().species(id) else { return vec![] };
    match world {
        World::Platinum => sp.abilities_in(World::Platinum),
        World::Modern => {
            let mut a = sp.abilities.clone();
            if !sp.hidden.is_empty() && a.len() > 1 && a.last() == Some(&sp.hidden) {
                a.pop();
            }
            a
        }
    }
}

/// The hidden Ability, if the species has one.
pub fn hidden_ability(id: &str) -> Option<String> {
    battle::data().species(id).map(|s| s.hidden.clone()).filter(|h| !h.is_empty())
}

/// Egg moves a species can get (in that kind of world).
pub fn egg_moves(wiki: &Wiki, id: &str, world: World) -> Vec<String> {
    let Some(ls) = wiki.learnset(id) else { return vec![] };
    let mut v = ls.egg.clone();
    if world == World::Modern {
        for k in &ls.megg {
            if !v.contains(k) {
                v.push(k.clone());
            }
        }
    }
    v
}

fn level_moves(wiki: &Wiki, id: &str, world: World) -> Vec<String> {
    let Some(ls) = wiki.learnset(id) else { return vec![] };
    let list = if world == World::Modern && !ls.mlv.is_empty() { &ls.mlv } else { &ls.lv };
    list.iter().map(|l| l.key.clone()).collect()
}

/// The family's first stage and the path down to `id` ([root, …, id]).
fn path_to_root(wiki: &Wiki, id: &str) -> Vec<String> {
    let mut path = vec![id.to_string()];
    while path.len() < 8 {
        match wiki.evolves_from(path.last().unwrap()) {
            Some(e) if !e.from.is_empty() => path.push(e.from.clone()),
            _ => break,
        }
    }
    path.reverse();
    path
}

/// What hatches from a parent of species `id`: the family's first stage,
/// unless it's an incense baby and nobody holds the incense.
fn hatches(wiki: &Wiki, id: &str, held: [&Held; 2]) -> (Vec<(String, f64)>, Option<(String, String)>) {
    let path = path_to_root(wiki, id);
    let mut baby = path[0].clone();
    let mut missing = None;
    if let Some(inc) = info(&baby).and_then(|i| i.incense.clone()) {
        let has = held.iter().any(|h| matches!(h, Held::Incense(n) if n.eq_ignore_ascii_case(&inc)));
        if !has && path.len() > 1 {
            missing = Some((inc, baby.clone()));
            baby = path[1].clone();
        }
    }
    let off = info(&baby).map(|i| i.offspring.clone()).unwrap_or_default();
    let species = if off.is_empty() {
        vec![(baby, 1.0)]
    } else {
        let each = 1.0 / off.len() as f64;
        off.into_iter().map(|s| (s, each)).collect()
    };
    (species, missing)
}

fn normal_ball(ball: &str) -> String {
    let b = ball.trim();
    if b.is_empty() || b.eq_ignore_ascii_case("Master Ball") || b.eq_ignore_ascii_case("Cherish Ball") {
        "Poké Ball".to_string()
    } else {
        b.to_string()
    }
}

/// What an Egg from these two will be like, or why there won't be one.
pub fn egg(a: &Parent, b: &Parent, setup: &Setup, wiki: &Wiki) -> Result<Egg, Why> {
    let how_well = compat(a, b, Some(wiki))?;
    let parents = [a, b];
    let mut e = Egg {
        compat: Some(how_well),
        shiny_one_in: egg_shiny_one_in(a, b, setup),
        notes: vec![NOTE_EGG_RATE],
        ..Default::default()
    };

    // The parent that decides the species, Ability and ball: the non-Ditto
    // one, else the mother.
    e.base = if a.is_ditto() {
        1
    } else if b.is_ditto() {
        0
    } else if b.gender() == Gender::Female {
        1
    } else {
        0
    };
    let base = parents[e.base];
    let ditto = a.is_ditto() || b.is_ditto();

    let (species, missing) = hatches(wiki, &base.species, [&a.held, &b.held]);
    e.species = species;
    e.incense_missing = missing;
    let main = e.species[0].0.clone();
    let mi = info(&main);
    e.cycles = mi.map_or(20, |i| i.cycles);
    e.female = if e.species.len() > 1 {
        // Nidoran♀/♂, Volbeat/Illumise: the species decides the gender.
        let f: f64 = e.species.iter().filter(|(s, _)| info(s).is_some_and(|i| i.ratio == 8)).map(|(_, p)| p).sum();
        Some(f)
    } else {
        mi.and_then(|i| i.female_chance())
    };

    // Stats.
    e.inherited = if a.held.knot() || b.held.knot() { 5 } else { 3 };
    for s in 0..6 {
        let mut want = [None; 6];
        want[s] = Some(31);
        e.perfect[s] = iv_chance(a, b, &want);
    }
    if a.held.power().is_some() && b.held.power().is_some() {
        e.notes.push(NOTE_TWO_POWER);
    }

    // Nature.
    let keepers: Vec<&Parent> = parents.iter().copied().filter(|p| p.held.keeps_nature()).collect();
    let each = 1.0 / keepers.len().max(1) as f64;
    for p in keepers {
        match e.nature.iter_mut().find(|(n, _)| *n == p.nature && n.is_some()) {
            Some(x) => x.1 += each,
            None => e.nature.push((p.nature.clone(), each)),
        }
    }

    // Ability: from the base parent, slot for slot.
    let normal = normal_abilities(&main, setup.world);
    let hidden = if setup.hidden_abilities { hidden_ability(&main) } else { None };
    let mut add = |name: String, slot: Slot, p: f64| {
        if p <= 0.0 || name.is_empty() {
            return;
        }
        match e.abilities.iter_mut().find(|(n, _, _)| *n == name) {
            Some(x) => x.2 += p,
            None => e.abilities.push((name, slot, p)),
        }
    };
    let first = normal.first().cloned().unwrap_or_default();
    let second = normal.get(1).cloned().unwrap_or_else(|| first.clone());
    match (base.ability, &hidden) {
        (Slot::Hidden, Some(h)) => {
            add(h.clone(), Slot::Hidden, 0.6);
            add(first, Slot::First, 0.2);
            add(second, Slot::Second, 0.2);
        }
        (Slot::Second, _) => {
            add(second, Slot::Second, 0.8);
            add(first, Slot::First, 0.2);
        }
        _ => {
            add(first, Slot::First, 0.8);
            add(second, Slot::Second, 0.2);
        }
    }
    e.notes.push(NOTE_ABILITY);

    // Ball.
    if !ditto && a.species == b.species {
        let (ba, bb) = (normal_ball(&a.ball), normal_ball(&b.ball));
        if ba == bb {
            e.ball = vec![(ba, 1.0)];
        } else {
            e.ball = vec![(normal_ball(&base.ball), 0.5), (normal_ball(&parents[1 - e.base].ball), 0.5)];
            e.notes.push(NOTE_BALL_SAME);
        }
    } else {
        e.ball = vec![(normal_ball(&base.ball), 1.0)];
        if ditto && base.gender() != Gender::Female {
            e.notes.push(NOTE_BALL_DITTO);
        }
    }

    // Moves: egg moves from either parent; shared level-up moves may pass.
    let eggs = egg_moves(wiki, &main, setup.world);
    for p in parents {
        for m in &p.moves {
            if eggs.contains(m) && !e.moves.contains(m) {
                e.moves.push(m.clone());
            }
        }
    }
    let lv = level_moves(wiki, &main, setup.world);
    for m in &a.moves {
        if b.moves.contains(m) && lv.contains(m) && !e.moves.contains(m) && !e.maybe_moves.contains(m) {
            e.maybe_moves.push(m.clone());
        }
    }
    if !e.maybe_moves.is_empty() {
        e.notes.push(NOTE_LEVEL_MOVES);
    }
    if e.moves.len() + e.maybe_moves.len() > 4 {
        e.notes.push(NOTE_TOO_MANY);
    }

    if a.masuda || b.masuda {
        e.notes.push(NOTE_MASUDA);
    }
    e.notes.push(NOTE_STEPS);
    Ok(e)
}

impl Egg {
    /// The chance of one nature (1 in 25 when it's random).
    pub fn nature_chance(&self, want: &str) -> f64 {
        if self.nature.is_empty() {
            return 1.0 / 25.0;
        }
        self.nature.iter().filter(|(n, _)| n.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(want))).map(|(_, p)| p).sum()
    }

    pub fn ability_chance(&self, want: &str) -> f64 {
        self.abilities.iter().filter(|(n, _, _)| n.eq_ignore_ascii_case(want)).map(|(_, _, p)| p).sum()
    }

    pub fn hatch_steps(&self, setup: &Setup) -> u32 {
        hatch_steps(self.cycles, setup.egg_warmer, setup.flame_body)
    }
}

/// Shiny odds for Eggs from these parents.
pub fn egg_shiny_one_in(a: &Parent, b: &Parent, setup: &Setup) -> f64 {
    shiny_one_in(shiny_rolls(setup.shiny_charm, a.masuda || b.masuda))
}

/// Every Pokémon a parent of `id` can make Eggs with (the species ids),
/// Ditto included. Empty when it can't breed. Newer Pokémon only in
/// MODERN worlds.
pub fn partners(id: &str, wiki: &Wiki, world: World) -> Vec<String> {
    let allowed = |m: &crate::wiki::Mon| world == World::Modern || !m.modern;
    let Some(me) = info(id) else { return vec![] };
    if me.no_eggs() {
        return vec![];
    }
    if id == DITTO {
        return wiki
            .all()
            .filter(|m| allowed(m) && m.id != DITTO && info(&m.id).is_some_and(|i| !i.no_eggs()))
            .map(|m| m.id.clone())
            .collect();
    }
    let mut out = vec![DITTO.to_string()];
    if me.genderless() {
        return out;
    }
    for m in wiki.all() {
        let Some(i) = info(&m.id) else { continue };
        if !allowed(m) || m.id == DITTO || i.no_eggs() || i.genderless() {
            continue;
        }
        // Two one-gender species of the same gender can't pair.
        if (me.ratio == 0 && i.ratio == 0) || (me.ratio == 8 && i.ratio == 8) {
            continue;
        }
        if i.groups.iter().any(|g| me.groups.contains(g)) {
            out.push(m.id.clone());
        }
    }
    out
}

// ───────────────────────────── tests ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn wiki() -> &'static Wiki {
        static W: OnceLock<Wiki> = OnceLock::new();
        W.get_or_init(|| Wiki::parse(include_bytes!("../assets/wiki.json")).expect("wiki"))
    }

    fn setup() -> Setup {
        Setup { world: World::Platinum, hidden_abilities: true, ..Default::default() }
    }

    #[test]
    fn every_wiki_species_has_breeding_data() {
        let w = wiki();
        let missing: Vec<&str> = w.all().filter(|m| info(&m.id).is_none()).map(|m| m.id.as_str()).collect();
        assert!(missing.is_empty(), "{missing:?}");
        let ditto = info(DITTO).unwrap();
        assert_eq!(ditto.groups, vec!["Ditto"]);
        assert!(info("MEW").unwrap().no_eggs());
        assert_eq!(info("BULBASAUR").unwrap().ratio, 1);
        assert_eq!(info("BULBASAUR").unwrap().cycles, 20);
        assert_eq!(info("WYNAUT").unwrap().incense.as_deref(), Some("Lax Incense"));
    }

    #[test]
    fn who_can_breed() {
        let w = Some(wiki());
        let f = |s: &str| Parent::new(s, Gender::Female);
        let m = |s: &str| Parent::new(s, Gender::Male);
        assert_eq!(compat(&f("CHARMANDER"), &m("CHARMANDER"), w), Ok(Compat::Medium));
        let mut traded = m("CHARMANDER");
        traded.other_trainer = true;
        assert_eq!(compat(&f("CHARMANDER"), &traded, w), Ok(Compat::High));
        assert_eq!(compat(&f("CHARMANDER"), &m("BAGON"), w), Ok(Compat::Low));
        assert_eq!(compat(&f("CHARMANDER"), &f("CHARMANDER"), w), Err(Why::SameGender));
        assert_eq!(compat(&f("CHARMANDER"), &m("PIKACHU"), w), Err(Why::NoSharedGroup));
        assert_eq!(compat(&m("DITTO"), &m("DITTO"), w), Err(Why::TwoDitto));
        assert!(matches!(compat(&f("PICHU"), &m("PIKACHU"), w), Err(Why::NoEggs(_))));
        assert_eq!(compat(&m("BRONZOR"), &f("CHARMANDER"), w), Err(Why::GenderlessNeedsDitto));
        assert_eq!(compat(&m("BRONZOR"), &m("DITTO"), w), Ok(Compat::Low));
        let mut partner = f("CHARMANDER");
        partner.partner = true;
        assert_eq!(compat(&partner, &m("CHARMANDER"), w), Err(Why::Partner));
        // Genderless species are genderless whatever is entered.
        assert_eq!(f("BRONZOR").gender(), Gender::Genderless);
        assert_eq!(f("TAUROS").gender(), Gender::Male);
    }

    #[test]
    fn what_hatches() {
        let w = wiki();
        let s = setup();
        let egg = |a: &Parent, b: &Parent| egg(a, b, &s, w).unwrap();
        // The mother's family's first stage.
        let e = egg(&Parent::new("GARCHOMP", Gender::Female), &Parent::new("CHARIZARD", Gender::Male));
        assert_eq!(e.species, vec![("GIBLE".to_string(), 1.0)]);
        // Ditto with a male: the male's family.
        let e = egg(&Parent::new("DITTO", Gender::Male), &Parent::new("LUCARIO", Gender::Male));
        assert_eq!(e.species[0].0, "RIOLU");
        // Wynaut only with the Lax Incense.
        let mut wob = Parent::new("WOBBUFFET", Gender::Female);
        let e = egg(&wob, &Parent::new("DITTO", Gender::Male));
        assert_eq!(e.species[0].0, "WOBBUFFET");
        assert_eq!(e.incense_missing, Some(("Lax Incense".to_string(), "WYNAUT".to_string())));
        wob.held = Held::from_name("Lax Incense");
        let e = egg(&wob, &Parent::new("DITTO", Gender::Male));
        assert_eq!(e.species[0].0, "WYNAUT");
        assert!(e.incense_missing.is_none());
        // Nidoran: either one, and the species sets the gender.
        let e = egg(&Parent::new("NIDOKING", Gender::Male), &Parent::new("DITTO", Gender::Male));
        assert_eq!(e.species.len(), 2);
        assert!((e.female.unwrap() - 0.5).abs() < 1e-9);
        // The baby's own egg cycles (Croagunk 10, not Toxicroak's 20).
        let e = egg(&Parent::new("TOXICROAK", Gender::Female), &Parent::new("DITTO", Gender::Male));
        assert_eq!((e.species[0].0.as_str(), e.cycles), ("CROAGUNK", 10));
        assert_eq!(e.compat, Some(Compat::Low));
        assert_eq!(e.shiny_one_in, 4096.0);
        // Nidorina and Nidoqueen can't breed at all.
        assert!(matches!(compat(&Parent::new("NIDOQUEEN", Gender::Female), &Parent::new("NIDOKING", Gender::Male), Some(w)), Err(Why::NoEggs(_))));
        // A traded parent: Masuda odds, with and without the Charm.
        let mut traded = Parent::new("DITTO", Gender::Male);
        traded.masuda = true;
        let e = egg(&Parent::new("TOXICROAK", Gender::Female), &traded);
        assert_eq!(e.shiny_one_in.round(), 683.0);
        let charm = Setup { shiny_charm: true, ..s };
        let e = super::egg(&Parent::new("TOXICROAK", Gender::Female), &traded, &charm, w).unwrap();
        assert_eq!(e.shiny_one_in.round(), 512.0);
        // Manaphy gives Phione.
        let e = egg(&Parent::new("MANAPHY", Gender::Genderless), &Parent::new("DITTO", Gender::Male));
        assert_eq!(e.species[0].0, "PHIONE");
        assert!(e.female.is_none());
    }

    #[test]
    fn iv_inheritance_adds_up() {
        let mut a = Parent::new("GARCHOMP", Gender::Female);
        let mut b = Parent::new("DITTO", Gender::Male);
        // Nothing perfect and nothing held: 1 in 32 per stat.
        a.ivs = [Some(0); 6];
        b.ivs = [Some(0); 6];
        let s = setup();
        let e = egg(&a, &b, &s, wiki()).unwrap();
        // 3 of 6 come from a parent (never 31), the other 3 are random:
        // P = 0.5 × 1/32.
        for p in e.perfect {
            assert!((p - 0.5 / 32.0).abs() < 1e-12);
        }
        // A 6IV Ditto with a Destiny Knot: each stat is 5/6 inherited, half
        // of those from the Ditto.
        b.ivs = [Some(31); 6];
        b.held = Held::DestinyKnot;
        let e = egg(&a, &b, &s, wiki()).unwrap();
        for p in e.perfect {
            assert!((p - (5.0 / 6.0 * 0.5 + 1.0 / 6.0 / 32.0)).abs() < 1e-12);
        }
        // Both parents 6IV with a knot: every stat inherited is 31.
        a.ivs = [Some(31); 6];
        let all = iv_chance(&a, &b, &[Some(31); 6]);
        assert!((all - 1.0 / 32.0).abs() < 1e-12, "{all}");
        // A Power item makes its stat certain from the holder.
        a.ivs = [Some(0); 6];
        a.ivs[5] = Some(31);
        a.held = Held::Power(5);
        b.held = Held::Nothing;
        b.ivs = [Some(0); 6];
        let mut want = [None; 6];
        want[5] = Some(31);
        assert!((iv_chance(&a, &b, &want) - 1.0).abs() < 1e-12);
        // Unknown IVs count as random.
        let c = Parent::new("GARCHOMP", Gender::Female);
        let d = Parent::new("DITTO", Gender::Male);
        assert!((iv_chance(&c, &d, &want) - 1.0 / 32.0).abs() < 1e-12);
        // Probabilities never leave [0, 1] and the set is no likelier than
        // any one stat in it.
        let mut two = [None; 6];
        two[0] = Some(31);
        two[1] = Some(31);
        let mut one = [None; 6];
        one[0] = Some(31);
        a.ivs = [Some(31); 6];
        b.ivs = [Some(31), Some(0), Some(31), Some(0), Some(31), Some(0)];
        assert!(iv_chance(&a, &b, &two) <= iv_chance(&a, &b, &one) + 1e-12);
    }

    #[test]
    fn iv_chance_matches_a_simulation() {
        // Monte Carlo check of the exact sum, with two Power items and a knot.
        let mut a = Parent::new("GARCHOMP", Gender::Female);
        let mut b = Parent::new("DITTO", Gender::Male);
        a.ivs = [Some(31), Some(31), Some(0), Some(31), Some(0), Some(31)];
        b.ivs = [Some(0), Some(31), Some(31), Some(0), Some(31), Some(31)];
        a.held = Held::Power(0);
        b.held = Held::Power(2);
        let want = [Some(31); 6];
        let exact = iv_chance(&a, &b, &want);
        let mut seed: u64 = 0x9e3779b97f4a7c15;
        let mut rnd = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        let trials = 400_000;
        let mut hits = 0;
        for _ in 0..trials {
            let (fs, holder) = if rnd(2) == 0 { (0usize, &a) } else { (2usize, &b) };
            let mut iv = [None::<u8>; 6];
            iv[fs] = holder.ivs[fs];
            let mut left: Vec<usize> = (0..6).filter(|s| *s != fs).collect();
            for _ in 0..2 {
                let i = rnd(left.len() as u64) as usize;
                let s = left.remove(i);
                iv[s] = if rnd(2) == 0 { a.ivs[s] } else { b.ivs[s] };
            }
            let ok = (0..6).all(|s| match iv[s] {
                Some(v) => v == 31,
                None => rnd(32) == 31,
            });
            hits += ok as u32;
        }
        let sim = hits as f64 / trials as f64;
        assert!((sim - exact).abs() < 0.002, "exact {exact} sim {sim}");
    }

    #[test]
    fn nature_ability_ball_and_moves() {
        let w = wiki();
        let s = setup();
        let mut mom = Parent::new("CHARMANDER", Gender::Female);
        mom.nature = Some("Adamant".into());
        mom.held = Held::Everstone;
        mom.ball = "Dusk Ball".into();
        mom.ability = Slot::Hidden;
        let mut dad = Parent::new("BAGON", Gender::Male);
        dad.moves = vec!["dragon_rush".into(), "outrage".into(), "ember".into()];
        dad.ball = "Ultra Ball".into();
        let e = egg(&mom, &dad, &s, w).unwrap();
        assert!((e.nature_chance("Adamant") - 1.0).abs() < 1e-12);
        assert!((e.ability_chance("Solar Power") - 0.6).abs() < 1e-12);
        assert_eq!(e.ball, vec![("Dusk Ball".to_string(), 1.0)]);
        // Egg moves from the father, not his other moves.
        assert!(e.moves.contains(&"dragon_rush".to_string()));
        assert!(e.moves.contains(&"outrage".to_string()));
        assert!(!e.moves.contains(&"ember".to_string()));
        // No hidden Abilities in this world: never the hidden one.
        let e2 = egg(&mom, &dad, &Setup { hidden_abilities: false, ..s }, w).unwrap();
        assert_eq!(e2.ability_chance("Solar Power"), 0.0);
        assert!((e2.ability_chance("Blaze") - 1.0).abs() < 1e-12);
        // A random nature without an Everstone.
        mom.held = Held::Nothing;
        let e3 = egg(&mom, &dad, &s, w).unwrap();
        assert!((e3.nature_chance("Adamant") - 0.04).abs() < 1e-12);
        // A Master Ball isn't passed on.
        mom.ball = "Master Ball".into();
        let e4 = egg(&mom, &dad, &s, w).unwrap();
        assert_eq!(e4.ball[0].0, "Poké Ball");
    }

    #[test]
    fn shiny_and_hatching_numbers() {
        assert_eq!(shiny_rolls(false, false), 1);
        assert_eq!(shiny_rolls(true, false), 3);
        assert_eq!(shiny_rolls(false, true), 6);
        assert_eq!(shiny_rolls(true, true), 8);
        assert!((shiny_one_in(1) - 4096.0).abs() < 1e-6);
        assert_eq!(shiny_one_in(6).round(), 683.0);
        assert_eq!(shiny_one_in(8).round(), 512.0);
        assert_eq!(hatch_steps(20, false, false), 5100);
        assert_eq!(hatch_steps(20, true, true), 1275);
        assert_eq!(eggs_for(0.5, 0.5), Some(1));
        assert_eq!(eggs_for(1.0 / 32.0, 0.5), Some(22));
        assert_eq!(eggs_for(0.0, 0.5), None);
    }

    #[test]
    fn partner_lists() {
        let w = wiki();
        let p = partners("CHARMANDER", w, World::Platinum);
        // Newer Pokémon only in MODERN worlds.
        assert!(!p.contains(&"FRIGIBAX".to_string()));
        assert!(partners("CHARMANDER", w, World::Modern).contains(&"FRIGIBAX".to_string()));
        assert!(p.contains(&DITTO.to_string()));
        assert!(p.contains(&"BAGON".to_string()));
        assert!(!p.contains(&"PIKACHU".to_string()));
        assert_eq!(partners("BRONZOR", w, World::Platinum), vec![DITTO.to_string()]);
        assert!(partners("MEW", w, World::Platinum).is_empty());
        // Two always-male species can't pair.
        assert!(!partners("TAUROS", w, World::Platinum).contains(&"NIDOKING".to_string()));
        assert!(partners("TAUROS", w, World::Platinum).contains(&"MILTANK".to_string()));
    }
}
