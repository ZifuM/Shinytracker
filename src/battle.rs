//! Battle data for the Team Planner and the damage calculator: base stats,
//! abilities, natures, move details and held items, built into the app
//! (`assets/battle.json`, from Pokémon Showdown's open data, MIT licence,
//! with Platinum's own stats where they differ from later games). Plus the
//! Platinum stat formulas and Hidden Power.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

static RAW: &[u8] = include_bytes!("../assets/battle.json");

/// Which game rules a team or a calculation uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, Deserialize, Hash)]
pub enum World {
    /// Platinum's own Pokémon, stats and abilities.
    #[default]
    Platinum,
    /// MODERN POKéMON on: newer Pokémon, Megas, newer abilities and stats.
    Modern,
}

impl World {
    pub fn label(self) -> &'static str {
        match self {
            World::Platinum => "Platinum",
            World::Modern => "MODERN",
        }
    }
}

pub const STATS: [&str; 6] = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];
pub const STAT_NAMES: [&str; 6] = ["HP", "Attack", "Defense", "Sp. Atk", "Sp. Def", "Speed"];

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Species {
    pub id: String,
    pub name: String,
    pub num: u32,
    pub stats: [u16; 6],
    pub stats_plat: Option<[u16; 6]>,
    pub types: Vec<String>,
    pub types_plat: Option<Vec<String>>,
    pub weight: f32,
    /// Every ability in newer games (hidden one last when there is one).
    pub abilities: Vec<String>,
    pub hidden: String,
    /// Platinum's abilities, when they differ from `abilities` minus the hidden one.
    pub abilities_plat: Option<Vec<String>>,
    /// The game wiki's id ("NIDORAN_F"); empty for forms.
    pub wiki: String,
    /// For forms: the base species' id ("garchomp" for "garchompmega").
    pub base: String,
    /// Megas: the stone.
    pub item: String,
}

impl Species {
    pub fn base_stats(&self, w: World) -> [u16; 6] {
        match (w, self.stats_plat) {
            (World::Platinum, Some(s)) => s,
            _ => self.stats,
        }
    }

    pub fn types_in(&self, w: World) -> &[String] {
        match (w, &self.types_plat) {
            (World::Platinum, Some(t)) => t,
            _ => &self.types,
        }
    }

    pub fn abilities_in(&self, w: World) -> Vec<String> {
        match w {
            World::Modern => self.abilities.clone(),
            World::Platinum => match &self.abilities_plat {
                Some(a) => a.clone(),
                None => self.abilities.iter().filter(|a| **a != self.hidden).cloned().collect(),
            },
        }
    }

    pub fn is_mega(&self) -> bool {
        self.name.contains("-Mega")
    }

    /// Only in MODERN worlds: a newer Pokémon or a Mega.
    pub fn modern_only(&self) -> bool {
        self.num > 493 || self.is_mega()
    }

    pub fn bst(&self, w: World) -> u32 {
        self.base_stats(w).iter().map(|s| *s as u32).sum()
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct MoveFx {
    /// Name.
    pub n: String,
    /// Flags: "contact,punch,sound,bite…".
    pub f: String,
    /// Hits: "2-5" or "2".
    pub hits: String,
    /// Recoil "1/3".
    pub recoil: String,
    pub crash: u8,
    pub pri: i8,
    pub drain: String,
    pub crit: u8,
    /// Target ("allAdjacentFoes" = spread).
    pub t: String,
    pub sec: u8,
    pub ignd: u8,
    pub ovo: String,
    pub ovd: String,
    /// 9: only in newer games (MODERN).
    pub g: u8,
}

impl MoveFx {
    pub fn has(&self, flag: &str) -> bool {
        self.f.split(',').any(|f| f == flag)
    }
    pub fn spread(&self) -> bool {
        self.t == "allAdjacentFoes" || self.t == "allAdjacent"
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ItemInfo {
    pub id: String,
    pub n: String,
    /// Natural Gift "Fire/80".
    pub ng: String,
    pub berry: u8,
    /// Mega stone: the Mega's name.
    pub mega: String,
    pub g: u8,
}

/// A ready-made competitive set (Smogon's Platinum-era sets).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct SetInfo {
    /// Species name ("Rotom-Heat").
    pub s: String,
    /// Set name ("OU Choice Scarf").
    pub n: String,
    pub item: String,
    pub ab: String,
    pub nat: String,
    pub ev: [u16; 6],
    pub iv: [u16; 6],
    pub m: Vec<String>,
    pub lv: u8,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct Raw {
    source: String,
    natures: Vec<[String; 3]>,
    species: Vec<Species>,
    moves: HashMap<String, MoveFx>,
    items: Vec<ItemInfo>,
    sets: Vec<SetInfo>,
}

#[derive(Debug, Clone)]
pub struct Nature {
    pub name: String,
    /// Index into the 6 stats (1..=5) raised / lowered; equal = neutral.
    pub plus: usize,
    pub minus: usize,
}

impl Nature {
    pub fn neutral(&self) -> bool {
        self.plus == self.minus
    }
    /// ×1.1, ×0.9 or ×1 for stat `i`.
    pub fn mult(&self, i: usize) -> f32 {
        if self.neutral() {
            1.0
        } else if i == self.plus {
            1.1
        } else if i == self.minus {
            0.9
        } else {
            1.0
        }
    }
}

pub struct BattleData {
    pub source: String,
    pub natures: Vec<Nature>,
    pub species: Vec<Species>,
    pub moves: HashMap<String, MoveFx>,
    pub items: Vec<ItemInfo>,
    pub sets: Vec<SetInfo>,
    by_id: HashMap<String, usize>,
}

/// Showdown's ids: lower case letters and digits only ("Mr. Mime" → "mrmime").
pub fn to_id(s: &str) -> String {
    s.to_lowercase()
        .replace('♀', "f")
        .replace('♂', "m")
        .replace('é', "e")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// Old move names the game uses → the newer names in the data.
pub fn move_alias(id: &str) -> &str {
    match id {
        "smellingsalt" => "smellingsalts",
        "hijumpkick" => "highjumpkick",
        "vicegrip" => "visegrip",
        "faintattack" => "feintattack",
        other => other,
    }
}

fn stat_index(s: &str) -> usize {
    match s {
        "atk" => 1,
        "def" => 2,
        "spa" => 3,
        "spd" => 4,
        "spe" => 5,
        _ => 0,
    }
}

impl BattleData {
    fn load() -> Self {
        let raw: Raw = serde_json::from_slice(RAW).unwrap_or_default();
        let natures = raw.natures.iter().map(|n| Nature { name: n[0].clone(), plus: stat_index(&n[1]), minus: stat_index(&n[2]) }).collect();
        let by_id = raw.species.iter().enumerate().map(|(i, s)| (s.id.clone(), i)).collect();
        BattleData { source: raw.source, natures, species: raw.species, moves: raw.moves, items: raw.items, sets: raw.sets, by_id }
    }

    /// By Showdown id, name ("Rotom-Wash", "Mr. Mime") or the wiki's id ("MR_MIME").
    pub fn species(&self, name: &str) -> Option<&Species> {
        self.by_id.get(&to_id(name)).map(|&i| &self.species[i])
    }

    pub fn nature(&self, name: &str) -> Option<&Nature> {
        self.natures.iter().find(|n| n.name.eq_ignore_ascii_case(name.trim()))
    }

    /// Move details by name or key ("Focus Punch", "focus_punch").
    pub fn move_fx(&self, name: &str) -> Option<&MoveFx> {
        let id = to_id(name);
        self.moves.get(move_alias(&id))
    }

    pub fn item(&self, name: &str) -> Option<&ItemInfo> {
        let id = to_id(name);
        self.items.iter().find(|i| i.id == id)
    }

    /// Forms of a species (Megas, Rotom's appliances, Deoxys…).
    pub fn forms(&self, base_id: &str) -> Vec<&Species> {
        self.species.iter().filter(|s| s.base == base_id).collect()
    }

    pub fn sets_for(&self, species_name: &str) -> Vec<&SetInfo> {
        let id = to_id(species_name);
        self.sets.iter().filter(|s| to_id(&s.s) == id).collect()
    }
}

pub fn data() -> &'static BattleData {
    static D: OnceLock<BattleData> = OnceLock::new();
    D.get_or_init(BattleData::load)
}

// ───────────────────────────── stats ─────────────────────────────

/// One stat at a level (Platinum's formula). `i` 0 is HP.
pub fn stat(i: usize, base: u16, iv: u16, ev: u16, level: u8, nature: Option<&Nature>, shedinja: bool) -> u16 {
    let (b, iv, ev, l) = (base as u32, iv.min(31) as u32, ev.min(255) as u32, level.clamp(1, 100) as u32);
    let core = (2 * b + iv + ev / 4) * l / 100;
    if i == 0 {
        if shedinja {
            1
        } else {
            (core + l + 10) as u16
        }
    } else {
        let v = core + 5;
        let v = match nature {
            Some(n) if !n.neutral() && n.plus == i => v * 110 / 100,
            Some(n) if !n.neutral() && n.minus == i => v * 90 / 100,
            _ => v,
        };
        v as u16
    }
}

pub fn stats(base: [u16; 6], ivs: [u16; 6], evs: [u16; 6], level: u8, nature: Option<&Nature>, shedinja: bool) -> [u16; 6] {
    let mut out = [0u16; 6];
    for i in 0..6 {
        out[i] = stat(i, base[i], ivs[i], evs[i], level, nature, shedinja);
    }
    out
}

/// The 16 Hidden Power types in their order (no Normal, no Fairy).
pub const HP_TYPES: [&str; 16] = [
    "Fighting", "Flying", "Poison", "Ground", "Rock", "Bug", "Ghost", "Steel", "Fire", "Water", "Grass", "Electric", "Psychic", "Ice", "Dragon", "Dark",
];

/// Hidden Power's type and power from the IVs (Platinum: power 30–70).
pub fn hidden_power(ivs: [u16; 6]) -> (&'static str, u16) {
    // bit order: HP, Atk, Def, Spe, SpA, SpD
    let order = [0usize, 1, 2, 5, 3, 4];
    let mut t = 0u32;
    let mut p = 0u32;
    for (k, &i) in order.iter().enumerate() {
        t += ((ivs[i] as u32) & 1) << k;
        p += (((ivs[i] as u32) >> 1) & 1) << k;
    }
    (HP_TYPES[(t * 15 / 63) as usize], (p * 40 / 63 + 30) as u16)
}

/// EVs you can still spend (510 total).
pub fn ev_left(evs: [u16; 6]) -> i32 {
    510 - evs.iter().map(|e| *e as i32).sum::<i32>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_loads() {
        let d = data();
        assert!(d.species.len() > 700);
        assert_eq!(d.natures.len(), 25);
        assert!(d.moves.len() > 600);
        let pika = d.species("Pikachu").expect("pikachu");
        // Platinum's Pikachu had 30 Defense and 40 Sp. Def (raised in gen 6)
        assert_eq!(pika.base_stats(World::Platinum), [35, 55, 30, 50, 40, 90]);
        assert_eq!(pika.base_stats(World::Modern), [35, 55, 40, 50, 50, 90]);
        let gengar = d.species("gengar").expect("gengar");
        assert_eq!(gengar.abilities_in(World::Platinum), vec!["Levitate".to_string()]);
        assert!(d.species("Garchomp-Mega").is_some_and(|m| m.is_mega() && m.item == "Garchompite"));
        assert!(d.species("MR_MIME").is_some());
        assert!(d.move_fx("faint_attack").is_some());
        assert!(d.move_fx("Close Combat").is_some_and(|m| m.has("contact")));
        // every species in the game has stats
        let Ok(bytes) = std::fs::read("assets/wiki.json") else { return };
        let w = crate::wiki::Wiki::parse(&bytes).expect("wiki");
        for m in &w.data.species {
            assert!(d.species(&m.name).is_some(), "{}", m.name);
        }
        for k in w.data.moves.keys() {
            assert!(d.move_fx(k).is_some(), "{k}");
        }
    }

    #[test]
    fn stat_formula() {
        let d = data();
        let adamant = d.nature("Adamant");
        let chomp = d.species("garchomp").expect("chomp").base_stats(World::Platinum);
        // 252 Atk Adamant Garchomp at 100: 394 Atk; HP 31 IV 0 EV: 357
        let s = stats(chomp, [31; 6], [0, 252, 0, 0, 4, 252], 100, adamant, false);
        assert_eq!(s[1], 394);
        assert_eq!(s[0], 357);
        assert_eq!(s[5], 303);
        // level 50, neutral
        let s50 = stats(chomp, [31; 6], [0; 6], 50, d.nature("Hardy"), false);
        assert_eq!(s50[0], 183);
        assert_eq!(stat(0, 1, 31, 252, 100, None, true), 1);
    }

    #[test]
    fn hidden_power_types() {
        assert_eq!(hidden_power([31; 6]), ("Dark", 70));
        // HP Fire: 31/30/31/30/31/30 (Atk, SpA... classic spread) → Fire
        assert_eq!(hidden_power([31, 30, 31, 30, 31, 30]).0, "Fire");
        assert_eq!(hidden_power([31, 30, 30, 31, 31, 31]).0, "Ice");
        assert_eq!(ev_left([252, 252, 4, 0, 0, 0]), 2);
    }
}
