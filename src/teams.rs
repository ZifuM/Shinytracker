//! Team Planner teams: saved in their own file (`teams.json` in the data
//! folder), written whole then swapped in, with rolling copies in
//! `backups/teams-*.json`. A file that can't be read is set aside, never
//! overwritten. Showdown's text format in and out.

use crate::battle::{self, World};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn default_ivs() -> [u16; 6] {
    [31; 6]
}

fn default_level() -> u8 {
    50
}

fn default_happy() -> u8 {
    255
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Member {
    /// Species name ("Garchomp", "Rotom-Wash").
    pub species: String,
    pub nickname: String,
    #[serde(default = "default_level")]
    pub level: u8,
    /// 'M', 'F' or none.
    pub gender: Option<char>,
    pub shiny: bool,
    pub item: String,
    pub ability: String,
    pub nature: String,
    pub evs: [u16; 6],
    #[serde(default = "default_ivs")]
    pub ivs: [u16; 6],
    /// Up to 4 move names.
    pub moves: Vec<String>,
    #[serde(default = "default_happy")]
    pub happiness: u8,
    /// Build checklist: keys of the steps ticked off.
    pub done: Vec<String>,
    /// Your own notes for this member.
    pub notes: String,
}

impl Default for Member {
    fn default() -> Self {
        Member {
            species: String::new(),
            nickname: String::new(),
            level: 50,
            gender: None,
            shiny: false,
            item: String::new(),
            ability: String::new(),
            nature: "Hardy".into(),
            evs: [0; 6],
            ivs: [31; 6],
            moves: Vec::new(),
            happiness: 255,
            done: Vec::new(),
            notes: String::new(),
        }
    }
}

impl Member {
    pub fn new(species: &str, world: World) -> Self {
        let mut m = Member { species: species.to_string(), ..Default::default() };
        if let Some(s) = battle::data().species(species) {
            m.species = s.name.clone();
            m.ability = s.abilities_in(world).first().cloned().unwrap_or_default();
            m.item = s.item.clone();
        }
        m
    }

    pub fn display_name(&self) -> &str {
        if self.nickname.trim().is_empty() {
            &self.species
        } else {
            &self.nickname
        }
    }

    pub fn final_stats(&self, world: World) -> Option<[u16; 6]> {
        let d = battle::data();
        let s = d.species(&self.species)?;
        Some(battle::stats(s.base_stats(world), self.ivs, self.evs, self.level, d.nature(&self.nature), s.id == "shedinja"))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Team {
    pub id: u64,
    pub name: String,
    /// A folder to group teams in ("" = none).
    pub folder: String,
    pub world: World,
    pub members: Vec<Member>,
    pub notes: String,
    pub created: u64,
    pub updated: u64,
}

impl Team {
    pub fn new(id: u64, name: &str, world: World) -> Self {
        let now = now();
        Team { id, name: name.to_string(), world, created: now, updated: now, ..Default::default() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct TeamFile {
    pub version: u32,
    pub teams: Vec<Team>,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn file_in(dir: &Path) -> PathBuf {
    crate::safefile::file_in(dir, "teams")
}

pub fn file() -> PathBuf {
    file_in(&crate::config::data_dir())
}

/// Load the teams, and a note when the file couldn't be read: then it's
/// moved aside (kept, never overwritten) and the newest good copy is used.
pub fn load_from(dir: &Path) -> (TeamFile, Option<String>) {
    crate::safefile::load(dir, "teams", "teams")
}

pub fn load() -> (TeamFile, Option<String>) {
    load_from(&crate::config::data_dir())
}

#[cfg(test)]
fn backups_in(dir: &Path) -> Vec<PathBuf> {
    crate::safefile::backups_in(dir, "teams")
}

/// Save the teams. Before the first save of each run, the file on disk is
/// copied to `backups/teams-<time>.json` (the newest 5 are kept). An empty
/// list never replaces one with teams in it without a copy.
pub fn save_to(dir: &Path, f: &TeamFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "teams", f, first_this_run, |t: &TeamFile| !t.teams.is_empty())
}

pub fn save(f: &TeamFile, first_this_run: bool) -> Result<(), String> {
    save_to(&crate::config::data_dir(), f, first_this_run)
}

// ───────────────────────────── Showdown text ─────────────────────────────

const EV_KEYS: [&str; 6] = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];

fn stat_key(s: &str) -> Option<usize> {
    match s.trim().to_lowercase().as_str() {
        "hp" => Some(0),
        "atk" | "attack" => Some(1),
        "def" | "defense" => Some(2),
        "spa" | "spatk" | "sp. atk" => Some(3),
        "spd" | "spdef" | "sp. def" => Some(4),
        "spe" | "speed" => Some(5),
        _ => None,
    }
}

/// One member in Showdown's export format.
pub fn export_member(m: &Member) -> String {
    let mut out = String::new();
    let mut head = if m.nickname.trim().is_empty() || m.nickname == m.species { m.species.clone() } else { format!("{} ({})", m.nickname, m.species) };
    if let Some(g) = m.gender {
        head.push_str(&format!(" ({g})"));
    }
    if !m.item.is_empty() {
        head.push_str(&format!(" @ {}", m.item));
    }
    out.push_str(&head);
    out.push('\n');
    if !m.ability.is_empty() {
        out.push_str(&format!("Ability: {}\n", m.ability));
    }
    if m.level != 100 {
        out.push_str(&format!("Level: {}\n", m.level));
    }
    if m.shiny {
        out.push_str("Shiny: Yes\n");
    }
    if m.happiness != 255 {
        out.push_str(&format!("Happiness: {}\n", m.happiness));
    }
    let evs: Vec<String> = (0..6).filter(|&i| m.evs[i] > 0).map(|i| format!("{} {}", m.evs[i], EV_KEYS[i])).collect();
    if !evs.is_empty() {
        out.push_str(&format!("EVs: {}\n", evs.join(" / ")));
    }
    if !m.nature.is_empty() {
        out.push_str(&format!("{} Nature\n", m.nature));
    }
    let ivs: Vec<String> = (0..6).filter(|&i| m.ivs[i] != 31).map(|i| format!("{} {}", m.ivs[i], EV_KEYS[i])).collect();
    if !ivs.is_empty() {
        out.push_str(&format!("IVs: {}\n", ivs.join(" / ")));
    }
    for mv in m.moves.iter().filter(|m| !m.is_empty()) {
        out.push_str(&format!("- {mv}\n"));
    }
    out
}

pub fn export_team(t: &Team) -> String {
    t.members.iter().map(export_member).collect::<Vec<_>>().join("\n")
}

/// Members from Showdown text (one team, or a single Pokémon). Lines it
/// doesn't know are skipped.
pub fn import(text: &str) -> Vec<Member> {
    let mut out: Vec<Member> = Vec::new();
    let mut cur: Option<Member> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            if let Some(m) = cur.take() {
                out.push(m);
            }
            continue;
        }
        if line.starts_with("===") {
            continue;
        }
        if cur.is_none() {
            cur = Some(parse_head(line));
            continue;
        }
        let m = cur.as_mut().expect("member");
        if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("~ ")) {
            let mv = rest.trim().replace("[", "").replace("]", "");
            if m.moves.len() < 4 {
                m.moves.push(mv);
            }
        } else if let Some(v) = line.strip_prefix("Ability:") {
            m.ability = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("Level:") {
            m.level = v.trim().parse().unwrap_or(m.level).clamp(1, 100);
        } else if let Some(v) = line.strip_prefix("Shiny:") {
            m.shiny = v.trim().eq_ignore_ascii_case("yes");
        } else if let Some(v) = line.strip_prefix("Happiness:") {
            m.happiness = v.trim().parse().unwrap_or(255);
        } else if let Some(v) = line.strip_prefix("EVs:") {
            m.evs = [0; 6];
            for part in v.split('/') {
                let mut it = part.split_whitespace();
                if let (Some(n), Some(k)) = (it.next(), it.next()) {
                    if let (Ok(n), Some(i)) = (n.parse::<u16>(), stat_key(k)) {
                        m.evs[i] = n.min(255);
                    }
                }
            }
        } else if let Some(v) = line.strip_prefix("IVs:") {
            for part in v.split('/') {
                let mut it = part.split_whitespace();
                if let (Some(n), Some(k)) = (it.next(), it.next()) {
                    if let (Ok(n), Some(i)) = (n.parse::<u16>(), stat_key(k)) {
                        m.ivs[i] = n.min(31);
                    }
                }
            }
        } else if let Some(n) = line.strip_suffix(" Nature") {
            m.nature = n.trim().to_string();
        }
    }
    if let Some(m) = cur.take() {
        out.push(m);
    }
    out.retain(|m| !m.species.is_empty());
    out
}

/// "Nick (Species) (M) @ Item"
fn parse_head(line: &str) -> Member {
    let mut m = Member { level: 100, ..Default::default() };
    let (left, item) = match line.split_once(" @ ") {
        Some((a, b)) => (a.trim(), b.trim().to_string()),
        None => (line.trim(), String::new()),
    };
    m.item = item;
    let mut name = left.to_string();
    for g in [" (M)", " (F)"] {
        if let Some(n) = name.strip_suffix(g) {
            m.gender = g.chars().nth(2);
            name = n.to_string();
        }
    }
    // "Nick (Species)"
    if let (Some(a), true) = (name.rfind(" ("), name.ends_with(')')) {
        let inner = name[a + 2..name.len() - 1].to_string();
        if battle::data().species(&inner).is_some() {
            m.nickname = name[..a].trim().to_string();
            name = inner;
        }
    }
    m.species = battle::data().species(&name).map(|s| s.name.clone()).unwrap_or(name);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &str = "Chompy (Garchomp) (M) @ Choice Scarf\nAbility: Sand Veil\nShiny: Yes\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\nIVs: 30 HP\n- Outrage\n- Earthquake\n- Stone Edge\n- Fire Fang\n\nRotom-Wash @ Leftovers\nAbility: Levitate\nLevel: 50\nEVs: 252 HP / 252 Def / 4 SpA\nBold Nature\n- Hydro Pump\n- Will-O-Wisp\n";

    #[test]
    fn showdown_text_round_trip() {
        let ms = import(SET);
        assert_eq!(ms.len(), 2);
        let g = &ms[0];
        assert_eq!((g.species.as_str(), g.nickname.as_str(), g.gender, g.item.as_str()), ("Garchomp", "Chompy", Some('M'), "Choice Scarf"));
        assert!(g.shiny);
        assert_eq!(g.evs, [0, 252, 0, 0, 4, 252]);
        assert_eq!(g.ivs[0], 30);
        assert_eq!(g.level, 100);
        assert_eq!(g.moves.len(), 4);
        assert_eq!(ms[1].level, 50);
        assert_eq!(ms[1].species, "Rotom-Wash");
        let again = import(&ms.iter().map(export_member).collect::<Vec<_>>().join("\n"));
        assert_eq!(again, ms);
    }

    #[test]
    fn safe_files() {
        let dir = std::env::temp_dir().join(format!("teams-test-{}", now() ^ 0x5eed));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let mut f = TeamFile { version: 1, teams: vec![Team::new(1, "Rain", World::Platinum)] };
        f.teams[0].members = import(SET);
        save_to(&dir, &f, true).expect("save");
        assert_eq!(load_from(&dir).0.teams, f.teams);
        // the second save of a run backs the first one up
        save_to(&dir, &f, true).expect("save");
        assert_eq!(backups_in(&dir).len(), 1);
        // a broken file is set aside, and the backup is used
        std::fs::write(file_in(&dir), "{ not json").expect("write");
        let back = load_from(&dir);
        assert_eq!(back.0.teams.len(), 1);
        assert!(back.1.is_some());
        assert!(std::fs::read_dir(&dir).expect("ls").flatten().any(|e| e.file_name().to_string_lossy().starts_with("teams.unreadable-")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
