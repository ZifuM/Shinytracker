//! Your world's options, read from the game folder's `world.ini` (read
//! only). The game writes each option as a number with a comment above it
//! ending "Now: <word>"; the word is what the option is set to (the numbers
//! don't follow the order the comment lists), so that's what's read, with
//! the number as a fallback for on / off options.

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldOptions {
    /// Each option: its key → the "Now:" word (or the raw value).
    pub raw: BTreeMap<String, String>,
    /// The world's name (`world-name.txt`).
    pub name: String,
    /// The world being played (`worlds/active.txt`).
    pub active: Option<u32>,
}

/// How the world's catch rate is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatchRate {
    #[default]
    Normal,
    Double,
    Four,
    Always,
}

impl CatchRate {
    pub fn label(self) -> &'static str {
        match self {
            CatchRate::Normal => "Normal",
            CatchRate::Double => "Double",
            CatchRate::Four => "4 times",
            CatchRate::Always => "Always",
        }
    }
    /// For the catch formula (0 normal, 1 double, 2 four times, 3 always).
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Read `world.ini` (and the world's name) from the game folder.
pub fn read(game_dir: &Path) -> Option<WorldOptions> {
    let text = std::fs::read_to_string(game_dir.join("world.ini")).ok()?;
    let mut w = parse(&text);
    w.name = std::fs::read_to_string(game_dir.join("world-name.txt")).map(|s| s.trim().to_string()).unwrap_or_default();
    w.active = std::fs::read_to_string(game_dir.join("worlds").join("active.txt")).ok().and_then(|s| s.trim().parse().ok());
    Some(w)
}

/// Another world's options (`worlds/<n>/world.ini`), when it keeps them.
pub fn read_world(game_dir: &Path, n: u32) -> Option<WorldOptions> {
    let text = std::fs::read_to_string(game_dir.join("worlds").join(n.to_string()).join("world.ini")).ok()?;
    let mut w = parse(&text);
    w.active = Some(n);
    Some(w)
}

pub fn parse(text: &str) -> WorldOptions {
    let mut raw = BTreeMap::new();
    let mut now: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(c) = line.strip_prefix('#') {
            now = c.rsplit_once("Now:").map(|(_, w)| w.trim().to_lowercase());
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim().to_string();
        raw.insert(k.trim().to_string(), now.take().filter(|w| !w.is_empty()).unwrap_or(v));
    }
    // Older worlds keep bare numbers with no "Now:" comment: name the ones
    // whose numbers are known (from worlds that do have the comments).
    for (key, words) in KNOWN {
        if let Some(v) = raw.get_mut(*key) {
            if let Some((_, w)) = v.parse::<i32>().ok().and_then(|n| words.iter().find(|(k, _)| *k == n)) {
                *v = w.to_string();
            }
        }
    }
    WorldOptions { raw, name: String::new(), active: None }
}

/// Numbers whose words are known, for world.ini files without comments.
const KNOWN: &[(&str, &[(i32, &str)])] = &[
    ("game_type", &[(0, "normal"), (1, "nuzlocke"), (2, "duallocke")]),
    ("rules", &[(1, "classic"), (3, "soul_link")]),
    ("preset", &[(0, "none")]),
    ("level_caps", &[(0, "off"), (1, "on"), (2, "no_grind")]),
    ("starter_region", &[(0, "kanto"), (1, "johto"), (2, "hoenn"), (3, "sinnoh"), (4, "lets_go")]),
    ("random_species", &[(0, "by_route")]),
];

/// A bare number left over (one whose word isn't known).
fn number(w: &str) -> bool {
    w.parse::<i64>().is_ok()
}

impl WorldOptions {
    pub fn word(&self, key: &str) -> Option<&str> {
        self.raw.get(key).map(|s| s.as_str())
    }

    /// An on / off option.
    pub fn on(&self, key: &str) -> bool {
        matches!(self.word(key), Some("on" | "yes" | "true" | "1"))
    }

    /// MODERN POKéMON: newer Pokémon, Megas, newer moves and abilities.
    pub fn modern(&self) -> bool {
        self.on("modern_types")
    }

    /// MORE POKéMON: Pokémon Platinum keeps for later turn up earlier.
    pub fn more(&self) -> bool {
        self.on("expanded_encounters")
    }

    /// Wild battles: Some(true) doubles, Some(false) singles, None mixed /
    /// asks each time.
    pub fn doubles(&self) -> Option<bool> {
        if self.raw.contains_key("double_battles") && !self.on("double_battles") {
            return Some(false);
        }
        match self.word("battle_style") {
            Some("doubles") => Some(true),
            Some("singles") => Some(false),
            _ => None,
        }
    }

    pub fn catch_rate(&self) -> CatchRate {
        match self.word("catch_rate") {
            Some("double") => CatchRate::Double,
            Some("four_times") => CatchRate::Four,
            Some("always") => CatchRate::Always,
            _ => CatchRate::Normal,
        }
    }

    /// One in-game day goes by each real hour.
    pub fn one_hour_day(&self) -> bool {
        matches!(self.word("one_hour_day"), Some("one_hour" | "on" | "1"))
    }

    /// A Nuzlocke-style run: its preset, or its game type (None: a normal
    /// world).
    pub fn challenge(&self) -> Option<String> {
        let pick = |k: &str| self.word(k).filter(|w| !matches!(*w, "none" | "normal") && !number(w)).map(|w| pretty(w));
        pick("preset").or_else(|| pick("game_type"))
    }

    /// Its Nuzlocke rules ("Classic", "Hardcore"…), when it has some.
    pub fn rules(&self) -> Option<String> {
        self.word("rules").filter(|w| *w != "none" && !number(w)).map(pretty)
    }

    /// Level caps: None off, else "On" / "No grind".
    pub fn level_caps(&self) -> Option<String> {
        self.word("level_caps").filter(|w| !matches!(*w, "off" | "0")).map(pretty)
    }

    /// What it all is in short lines, for the "Your world" card:
    /// (option, value).
    pub fn summary(&self) -> Vec<(&'static str, String)> {
        let yes = |b: bool| if b { "On".to_string() } else { "Off".to_string() };
        let mut v = vec![
            ("MODERN POKéMON", yes(self.modern())),
            ("MORE POKéMON", yes(self.more())),
            (
                "Wild battles",
                match self.doubles() {
                    Some(true) => "Doubles".into(),
                    Some(false) => "Singles".into(),
                    None => self.word("battle_style").map(pretty).unwrap_or_else(|| "—".into()),
                },
            ),
            ("Catch rate", self.catch_rate().label().into()),
            ("Day length", if self.one_hour_day() { "One hour".into() } else { "Real time".into() }),
            ("Run", match (self.challenge(), self.rules()) {
                (Some(c), Some(r)) => format!("{c} · {r} rules"),
                (Some(c), None) => c,
                _ => "Normal".into(),
            }),
            ("Level caps", self.level_caps().unwrap_or_else(|| "Off".into())),
        ];
        if let Some(r) = self.word("starter_region") {
            v.push(("Starters from", pretty(r)));
        }
        v
    }
}

/// "four_times" → "Four times", "soul_link" → "Soul link".
fn pretty(w: &str) -> String {
    let s = w.replace('_', " ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YOURS: &str = "# GAME TYPE: normal / nuzlocke / duallocke. Now: normal\ngame_type=0\n# NUZLOCKE RULES: none / easy / classic. Now: none\nrules=-1\n# LEVEL CAPS: off / on / no_grind (or on / off). Now: off\nlevel_caps=0\n# MODERN POKEMON: on / off. Now: on\nmodern_types=1\n# DOUBLE BATTLES: on / off. Now: on\ndouble_battles=1\n# BATTLE STYLE: ask / singles / doubles / both / random. Now: doubles\nbattle_style=1\n# MORE POKEMON: on / off. Now: on\nexpanded_encounters=1\n# DAY LENGTH: real_time / one_hour (or on / off). Now: real_time\none_hour_day=0\n# CATCH RATE: normal / double / four_times / always. Now: normal\ncatch_rate=1\n# STARTER REGION: kanto / johto / hoenn. Now: hoenn\nstarter_region=2\n";

    #[test]
    fn reads_the_now_words_not_the_numbers() {
        let w = parse(YOURS);
        assert!(w.modern() && w.more());
        assert_eq!(w.doubles(), Some(true));
        // catch_rate=1 but "Now: normal"
        assert_eq!(w.catch_rate(), CatchRate::Normal);
        assert!(!w.one_hour_day());
        assert_eq!(w.challenge(), None);
        assert_eq!(w.level_caps(), None);
        assert_eq!(w.word("starter_region"), Some("hoenn"));
    }

    #[test]
    fn other_settings() {
        let w = parse("# CATCH RATE: x. Now: four_times\ncatch_rate=1\n# TYPE. Now: nuzlocke\ngame_type=1\n# RULES. Now: soul_link\nrules=4\nmodern_types=0\n");
        assert_eq!(w.catch_rate(), CatchRate::Four);
        assert_eq!(w.challenge().as_deref(), Some("Nuzlocke"));
        assert_eq!(w.rules().as_deref(), Some("Soul link"));
        assert!(!w.modern());
    }

    /// Your world.ini, if it's on this machine.
    #[test]
    fn reads_your_world() {
        let dir = Path::new("/mnt/user-data/uploads/PlatinumTogether/Game");
        let Some(w) = read(dir) else { return };
        assert!(w.modern() && w.more());
        assert_eq!(w.doubles(), Some(true));
        assert_eq!(w.catch_rate(), CatchRate::Normal);
        assert!(!w.name.is_empty());
        assert_eq!(w.active, Some(1));
        let other = read_world(dir, 8).unwrap();
        assert_eq!(other.challenge().as_deref(), Some("Duallocke"));
    }
}
