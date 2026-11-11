//! Game data from the Platinum Together Wiki (`wiki.json`): Pokémon, where
//! they live, evolutions, moves. A copy is built into the app (so it works
//! offline and on first start); a fresh one is downloaded at most once a day
//! into the cache folder and used when it's newer.

use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub const URL: &str = "https://platinumtogether.online/wiki/data/wiki.json";
/// A Pokémon's page on the wiki: `#/pokemon/<slug of its name>`.
pub const PAGE_BASE: &str = "https://platinumtogether.online/wiki/#/pokemon/";
const REFRESH_AFTER: Duration = Duration::from_secs(24 * 3600);

/// The copy built into the exe (`{}` if the build couldn't fetch one).
#[cfg(not(offline_test))]
static BUNDLED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wiki.json"));
#[cfg(offline_test)]
static BUNDLED: &[u8] = b"{}";

// ───────────────────────────── the file ─────────────────────────────

/// A string field that may be missing, null or a number in the file.
fn lenient<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        v => v.to_string(),
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WikiFile {
    #[serde(deserialize_with = "lenient")]
    pub source: String,
    #[serde(deserialize_with = "lenient")]
    pub version: String,
    /// Badge stages in game order ("Before Roark" … "After the League").
    pub stages: Vec<String>,
    pub places: BTreeMap<String, Place>,
    pub wild: Vec<Wild>,
    pub species: Vec<Mon>,
    pub other: Vec<Other>,
    pub evolutions: Vec<Evo>,
    pub unobtainable: Vec<Unobtainable>,
    pub moves: HashMap<String, Move>,
    pub learnsets: HashMap<String, Learnset>,
    /// "TM01" → move key.
    pub tms: BTreeMap<String, String>,
    pub tutors: HashMap<String, Tutor>,
    /// Short explanations the wiki shows (keys like "chances", "radar").
    pub text: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Place {
    #[serde(deserialize_with = "lenient")]
    pub name: String,
    #[serde(deserialize_with = "lenient")]
    pub group: String,
    #[serde(deserialize_with = "lenient")]
    pub stage: String,
    pub cave: bool,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
pub struct Chance {
    pub morning: u32,
    pub day: u32,
    pub night: u32,
}

/// One wild encounter slot.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Wild {
    #[serde(deserialize_with = "lenient")]
    pub species: String,
    /// Key into `places`.
    #[serde(deserialize_with = "lenient")]
    pub area: String,
    /// grass, radar, surf, old_rod, good_rod, super_rod, cave
    #[serde(deserialize_with = "lenient")]
    pub method: String,
    /// "", "more" or "modern" (only with that world option on).
    #[serde(deserialize_with = "lenient")]
    pub setting: String,
    #[serde(deserialize_with = "lenient")]
    pub levels: String,
    #[serde(deserialize_with = "lenient")]
    pub note: String,
    #[serde(deserialize_with = "lenient")]
    pub stage: String,
    pub chance: Chance,
}

/// A Pokémon.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Mon {
    /// e.g. "SCYTHER", "NIDORAN_F".
    #[serde(deserialize_with = "lenient")]
    pub id: String,
    #[serde(deserialize_with = "lenient")]
    pub name: String,
    pub dex: u32,
    pub types: Vec<String>,
    /// Added by the MODERN POKéMON option.
    pub modern: bool,
}

/// A way to get a Pokémon other than wild grass/water (gifts, eggs, trades…).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Other {
    #[serde(deserialize_with = "lenient")]
    pub species: String,
    #[serde(deserialize_with = "lenient")]
    pub how: String,
    #[serde(rename = "where", deserialize_with = "lenient")]
    pub place: String,
    #[serde(deserialize_with = "lenient")]
    pub detail: String,
    #[serde(deserialize_with = "lenient")]
    pub level: String,
    #[serde(deserialize_with = "lenient")]
    pub when: String,
    #[serde(deserialize_with = "lenient")]
    pub needs: String,
    /// false = doesn't work in the game right now.
    pub works: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Evo {
    #[serde(deserialize_with = "lenient")]
    pub species: String,
    #[serde(deserialize_with = "lenient")]
    pub from: String,
    #[serde(deserialize_with = "lenient")]
    pub method: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Unobtainable {
    #[serde(deserialize_with = "lenient")]
    pub species: String,
    #[serde(deserialize_with = "lenient")]
    pub why: String,
}

/// A move: name, type, category, power, accuracy, PP, description.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Move {
    #[serde(deserialize_with = "lenient")]
    pub n: String,
    #[serde(deserialize_with = "lenient")]
    pub t: String,
    #[serde(deserialize_with = "lenient")]
    pub c: String,
    #[serde(deserialize_with = "lenient")]
    pub p: String,
    #[serde(deserialize_with = "lenient")]
    pub a: String,
    #[serde(deserialize_with = "lenient")]
    pub pp: String,
    #[serde(deserialize_with = "lenient")]
    pub d: String,
}

/// What a Pokémon learns. `m…` lists are the MODERN POKéMON versions.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Learnset {
    pub lv: Vec<LevelMove>,
    pub tm: Vec<String>,
    pub tutor: Vec<String>,
    pub egg: Vec<String>,
    pub mlv: Vec<LevelMove>,
    pub mtm: Vec<String>,
    pub mtutor: Vec<String>,
    pub megg: Vec<String>,
}

/// `[level, move]` (level can be a word, e.g. "Evolve").
#[derive(Debug, Clone, Default)]
pub struct LevelMove {
    pub level: String,
    pub key: String,
}

impl<'de> Deserialize<'de> for LevelMove {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        let s = |v: &serde_json::Value| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => String::new(),
            v => v.to_string(),
        };
        Ok(match v.as_array() {
            Some(a) => LevelMove {
                level: a.first().map(s).unwrap_or_default(),
                key: a.get(1).map(s).unwrap_or_default(),
            },
            None => LevelMove { level: String::new(), key: s(&v) },
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Tutor {
    #[serde(rename = "where", deserialize_with = "lenient")]
    pub place: String,
    /// Shards by colour ("red", "blue", "yellow", "green").
    pub cost: BTreeMap<String, u32>,
}

// ───────────────────────────── indexed ─────────────────────────────

/// wiki.json plus quick lookups.
#[derive(Debug, Default)]
pub struct Wiki {
    pub data: WikiFile,
    /// Species, sorted by dex number.
    pub order: Vec<usize>,
    by_id: HashMap<String, usize>,
    by_dex: HashMap<u32, usize>,
    wild_of: HashMap<String, Vec<usize>>,
    other_of: HashMap<String, Vec<usize>>,
    unobtainable_of: HashMap<String, usize>,
}

impl Wiki {
    pub fn from_file(data: WikiFile) -> Self {
        let mut w = Wiki { data, ..Default::default() };
        for (i, m) in w.data.species.iter().enumerate() {
            w.by_id.insert(m.id.clone(), i);
            w.by_dex.entry(m.dex).or_insert(i);
        }
        let mut order: Vec<usize> = (0..w.data.species.len()).collect();
        order.sort_by_key(|&i| w.data.species[i].dex);
        w.order = order;
        for (i, e) in w.data.wild.iter().enumerate() {
            w.wild_of.entry(e.species.clone()).or_default().push(i);
        }
        for (i, o) in w.data.other.iter().enumerate() {
            w.other_of.entry(o.species.clone()).or_default().push(i);
        }
        for (i, u) in w.data.unobtainable.iter().enumerate() {
            w.unobtainable_of.insert(u.species.clone(), i);
        }
        w
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let f: WikiFile = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        Ok(Self::from_file(f))
    }

    pub fn is_empty(&self) -> bool {
        self.data.species.is_empty()
    }

    pub fn mon(&self, id: &str) -> Option<&Mon> {
        self.by_id.get(id).map(|&i| &self.data.species[i])
    }

    pub fn by_dex(&self, dex: u32) -> Option<&Mon> {
        self.by_dex.get(&dex).map(|&i| &self.data.species[i])
    }

    /// Every Pokémon in dex order.
    pub fn all(&self) -> impl Iterator<Item = &Mon> {
        self.order.iter().map(|&i| &self.data.species[i])
    }

    /// Wild encounter slots for a Pokémon.
    pub fn wild(&self, id: &str) -> Vec<&Wild> {
        self.wild_of.get(id).map(|v| v.iter().map(|&i| &self.data.wild[i]).collect()).unwrap_or_default()
    }

    /// Gifts, eggs, trades and the like.
    pub fn other(&self, id: &str) -> Vec<&Other> {
        self.other_of.get(id).map(|v| v.iter().map(|&i| &self.data.other[i]).collect()).unwrap_or_default()
    }

    /// Why it can't be obtained, if it can't.
    pub fn unobtainable(&self, id: &str) -> Option<&str> {
        self.unobtainable_of.get(id).map(|&i| self.data.unobtainable[i].why.as_str())
    }

    pub fn place(&self, key: &str) -> Option<&Place> {
        self.data.places.get(key)
    }

    /// 0 = "Before Roark"; unknown stages sort last.
    pub fn stage_index(&self, stage: &str) -> usize {
        self.data.stages.iter().position(|s| s == stage).unwrap_or(usize::MAX)
    }

    /// Evolves from (with how).
    pub fn evolves_from(&self, id: &str) -> Option<&Evo> {
        self.data.evolutions.iter().find(|e| e.species == id)
    }

    /// Evolves into (with how).
    pub fn evolves_into(&self, id: &str) -> Vec<&Evo> {
        self.data.evolutions.iter().filter(|e| e.from == id).collect()
    }

    /// The whole family as stages: [[root], [its evolutions], [theirs]…].
    pub fn family(&self, id: &str) -> Vec<Vec<String>> {
        let mut root = id.to_string();
        let mut guard = 0;
        while let Some(e) = self.evolves_from(&root) {
            root = e.from.clone();
            guard += 1;
            if guard > 8 {
                break;
            }
        }
        let mut stages = vec![vec![root]];
        while stages.len() < 6 {
            let next: Vec<String> = stages
                .last()
                .unwrap()
                .iter()
                .flat_map(|s| self.evolves_into(s).into_iter().map(|e| e.species.clone()))
                .collect();
            if next.is_empty() {
                break;
            }
            stages.push(next);
        }
        stages
    }

    pub fn learnset(&self, id: &str) -> Option<&Learnset> {
        self.data.learnsets.get(id)
    }

    pub fn move_info(&self, key: &str) -> Option<&Move> {
        self.data.moves.get(key)
    }

    /// The wiki page for a Pokémon.
    pub fn page_url(&self, m: &Mon) -> String {
        format!("{PAGE_BASE}{}", slug(&m.name))
    }

    /// "0.3.4" style version as numbers, for "is this one newer?".
    fn version_key(&self) -> (Vec<u32>, String) {
        let v = self.data.version.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect();
        (v, self.data.source.clone())
    }
}

/// The wiki's own URL slug, made the same way the wiki does: lower case,
/// ♀ → "-f", ♂ → "-m", é → e, every other run of non-letters → one dash.
pub fn slug(name: &str) -> String {
    let lower = name.to_lowercase().replace('♀', "-f").replace('♂', "-m").replace('é', "e");
    let mut out = String::new();
    let mut dash = false;
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    out
}

// ───────────────────────────── loading ─────────────────────────────

fn cache_file() -> PathBuf {
    crate::config::cache_dir().join("wiki.json")
}

/// Whichever is newer: the built-in copy or the downloaded one.
fn best_local() -> Wiki {
    let bundled = Wiki::parse(BUNDLED).unwrap_or_default();
    let cached = std::fs::read(cache_file()).ok().and_then(|b| Wiki::parse(&b).ok());
    match cached {
        Some(c) if !c.is_empty() && (bundled.is_empty() || c.version_key() >= bundled.version_key()) => c,
        _ => bundled,
    }
}

fn cache_is_fresh() -> bool {
    std::fs::metadata(cache_file())
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < REFRESH_AFTER)
}

fn download() -> Result<Vec<u8>, String> {
    let resp = ureq::get(URL)
        .set("User-Agent", concat!("PokeTogetherCompanion/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    Ok(text.into_bytes())
}

/// Load in the background: the best local copy first, then a fresh one if
/// the cached copy is more than a day old (or missing). Each usable copy is
/// sent down the channel as it's ready.
pub fn start() -> Receiver<Arc<Wiki>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new().name("wiki".into()).spawn(move || run(tx));
    rx
}

fn run(tx: Sender<Arc<Wiki>>) {
    let local = best_local();
    let have = local.version_key();
    let had_any = !local.is_empty();
    if had_any && tx.send(Arc::new(local)).is_err() {
        return;
    }
    if cache_is_fresh() {
        return;
    }
    let Ok(bytes) = download() else { return };
    let Ok(fresh) = Wiki::parse(&bytes) else { return };
    if fresh.is_empty() {
        return;
    }
    let path = cache_file();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, &bytes);
    if !had_any || fresh.version_key() != have {
        let _ = tx.send(Arc::new(fresh));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "source": "wt-relay-20261028 @ 8f7b70500", "version": "0.3.4",
      "stages": ["Before Roark", "Before Gardenia", "Before Byron"],
      "places": {"route_210_south": {"name": "Route 210 (south)", "group": "Route 210", "stage": "Before Gardenia", "cave": false}},
      "wild": [
        {"species": "SCYTHER", "area": "route_210_south", "method": "grass", "setting": null, "levels": "19–21", "note": "", "stage": "Before Gardenia", "chance": {"morning": 15, "day": 5, "night": 5}},
        {"species": "QWILFISH", "area": "route_210_south", "method": "good_rod", "setting": "more", "levels": 25, "note": "", "stage": "Before Byron", "chance": {"morning": 5, "day": 5, "night": 5}}
      ],
      "species": [
        {"id": "SCIZOR", "name": "Scizor", "dex": 212, "types": ["Bug", "Steel"], "modern": false},
        {"id": "SCYTHER", "name": "Scyther", "dex": 123, "types": ["Bug", "Flying"], "modern": false},
        {"id": "EEVEE", "name": "Eevee", "dex": 133, "types": ["Normal"], "modern": false},
        {"id": "VAPOREON", "name": "Vaporeon", "dex": 134, "types": ["Water"], "modern": false},
        {"id": "JOLTEON", "name": "Jolteon", "dex": 135, "types": ["Electric"], "modern": false},
        {"id": "NIDORAN_F", "name": "Nidoran♀", "dex": 29, "types": ["Poison"], "modern": false}
      ],
      "other": [
        {"species": "TOGEPI", "how": "Egg", "where": "Eterna City (Cynthia)", "detail": "x", "level": null, "when": "After the Eterna Building", "needs": "Room", "works": true},
        {"species": "SCYTHER", "how": "Trade", "where": "Somewhere", "detail": "", "level": {"min": 5}, "when": "", "needs": ""}
      ],
      "evolutions": [
        {"species": "SCIZOR", "from": "SCYTHER", "method": "Trade it holding a Metal Coat."},
        {"species": "VAPOREON", "from": "EEVEE", "method": "Water Stone."},
        {"species": "JOLTEON", "from": "EEVEE", "method": "Thunder Stone."}
      ],
      "unobtainable": [{"species": "MEWTWO", "why": "Not in the game."}],
      "moves": {"pound": {"n": "Pound", "t": "Normal", "c": "Physical", "p": 40, "a": 100, "pp": 35, "d": "Pounds."},
                "growl": {"n": "Growl", "t": "Normal", "c": "Status", "p": null, "a": 100, "pp": 40, "d": "Growls."}},
      "learnsets": {"SCYTHER": {"lv": [[1, "pound"], ["Evolve", "growl"]], "tm": ["TM06"], "tutor": [], "egg": ["counter"]}},
      "tms": {"TM01": "focus_punch"},
      "tutors": {"dive": {"where": "Route 212", "cost": {"blue": 4, "red": 2}}},
      "text": {"chances": "Chances are morning / day / night."},
      "items": [{"id": 1}]
    }"#;

    #[test]
    fn parses_and_indexes() {
        let w = Wiki::parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(w.data.version, "0.3.4");
        // dex order
        let names: Vec<&str> = w.all().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Nidoran\u{2640}", "Scyther", "Eevee", "Vaporeon", "Jolteon", "Scizor"]);
        assert_eq!(w.by_dex(212).unwrap().id, "SCIZOR");
        // nulls and numbers become text
        let wild = w.wild("QWILFISH");
        assert_eq!(wild[0].levels, "25");
        assert_eq!(wild[0].setting, "more");
        assert_eq!(w.wild("SCYTHER")[0].setting, "");
        assert_eq!(w.wild("SCYTHER")[0].chance.morning, 15);
        assert_eq!(w.other("TOGEPI")[0].level, "");
        assert_eq!(w.other("TOGEPI")[0].place, "Eterna City (Cynthia)");
        assert_eq!(w.other("SCYTHER")[0].works, None);
        assert_eq!(w.move_info("growl").unwrap().p, "");
        assert_eq!(w.move_info("pound").unwrap().p, "40");
        let ls = w.learnset("SCYTHER").unwrap();
        assert_eq!(ls.lv[0].level, "1");
        assert_eq!(ls.lv[1].level, "Evolve");
        assert_eq!(ls.lv[1].key, "growl");
        assert_eq!(w.data.tutors["dive"].cost["blue"], 4);
        assert_eq!(w.unobtainable("MEWTWO"), Some("Not in the game."));
        assert_eq!(w.stage_index("Before Byron"), 2);
        assert_eq!(w.place("route_210_south").unwrap().group, "Route 210");
    }

    #[test]
    fn families() {
        let w = Wiki::parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(w.family("SCIZOR"), vec![vec!["SCYTHER".to_string()], vec!["SCIZOR".to_string()]]);
        assert_eq!(w.family("JOLTEON")[1], vec!["VAPOREON".to_string(), "JOLTEON".to_string()]);
        assert_eq!(w.family("NIDORAN_F"), vec![vec!["NIDORAN_F".to_string()]]);
        assert_eq!(w.evolves_from("SCIZOR").unwrap().method, "Trade it holding a Metal Coat.");
    }

    #[test]
    fn slugs_match_the_wiki() {
        assert_eq!(slug("Scyther"), "scyther");
        assert_eq!(slug("Mr. Mime"), "mr-mime");
        assert_eq!(slug("Nidoran\u{2640}"), "nidoran-f");
        assert_eq!(slug("Ho-Oh"), "ho-oh");
        assert_eq!(slug("Farfetch'd"), "farfetch-d");
    }

    #[test]
    fn empty_file_is_empty() {
        let w = Wiki::parse(b"{}").unwrap();
        assert!(w.is_empty());
        assert!(w.all().next().is_none());
    }
}
