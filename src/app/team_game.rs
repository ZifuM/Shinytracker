//! Team Planner checklist from your game: which Pokémon in your party or
//! boxes each team member is, and which of its steps are already done there.
//!
//! Read only, like Pokémon Boxes: the save (re-read each time the game
//! saves, every few minutes) for species, moves, Ability, nature, item,
//! level and EVs, and its Bag for TMs, stones, held items and the Key Stone;
//! the log for catches, the moment they happen. Nothing is written to the
//! game.

use super::team_check::{chain_of, Step};
use super::team_build::wiki_id;
use super::App;
use crate::battle::to_id;
use crate::teams::{GamePick, Member, Team};
use crate::wiki::Wiki;
use std::sync::Arc;

/// One Pokémon in your party or boxes, as a team member.
#[derive(Clone, Debug)]
pub(super) struct Owned {
    pub pid: u32,
    /// "Party, place 1", "BOX 3, place 17".
    pub at: String,
    pub party: bool,
    pub m: Member,
    /// Its species' wiki id.
    pub wiki: String,
}

/// What your game has, as of its last save (and the log since).
#[derive(Clone, Debug, Default)]
pub(super) struct Snap {
    /// When the save was written (unix ms).
    pub ms: u64,
    pub owned: Vec<Owned>,
    /// The Bag: (item id, its name).
    pub bag: Vec<(String, String)>,
    /// Species caught since the save (wiki ids), from the log.
    pub caught: Vec<String>,
    /// The world it's from (its name in the game).
    pub world: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Not done in the game (maybe a note on how far along).
    No,
    /// Done in your game.
    Game,
    /// On its way: what it needs is in your Bag.
    Half,
}

#[derive(Clone, Debug)]
pub(super) struct Mark {
    pub kind: Kind,
    /// "from your game", "TM26 in your Bag", "Atk 96/252 · Spe 40/252".
    pub note: String,
}

impl Mark {
    fn no() -> Self {
        Mark { kind: Kind::No, note: String::new() }
    }
    fn game(note: impl Into<String>) -> Self {
        Mark { kind: Kind::Game, note: note.into() }
    }
    fn half(note: impl Into<String>) -> Self {
        Mark { kind: Kind::Half, note: note.into() }
    }
    fn progress(note: impl Into<String>) -> Self {
        Mark { kind: Kind::No, note: note.into() }
    }
}

/// One team member in your game.
#[derive(Clone, Debug, Default)]
pub(super) struct Found {
    /// The Pokémon it is (index into `Snap::owned`).
    pub pick: Option<usize>,
    /// Every Pokémon of its family you have, best first.
    pub family: Vec<usize>,
    /// One per step.
    pub marks: Vec<Mark>,
}

impl Found {
    pub fn kind(&self, i: usize) -> Kind {
        self.marks.get(i).map_or(Kind::No, |m| m.kind)
    }
}

const STATS: [&str; 6] = ["HP", "Atk", "Def", "SpA", "SpD", "Spe"];

fn same(a: &str, b: &str) -> bool {
    !a.is_empty() && to_id(a) == to_id(b)
}

/// How well `o` fits `m` (higher is better).
fn score(o: &Owned, m: &Member, stage: usize) -> i32 {
    let mut s = stage as i32 * 100;
    if o.m.species == m.species {
        s += 50;
    }
    s += m.moves.iter().filter(|mv| o.m.moves.iter().any(|x| same(x, mv))).count() as i32 * 10;
    for (a, b) in [(&o.m.ability, &m.ability), (&o.m.nature, &m.nature), (&o.m.item, &m.item)] {
        if same(a, b) {
            s += 10;
        }
    }
    if o.party {
        s += 5;
    }
    s
}

/// A move step's key against a move it knows ("move:hiddenpowerfire" and
/// "Hidden Power").
fn knows(key_id: &str, moves: &[String]) -> bool {
    moves.iter().any(|x| {
        let id = to_id(x);
        id == key_id || (id == "hiddenpower" && key_id.starts_with("hiddenpower"))
    })
}

/// Which of `m`'s steps your game already has done.
pub(super) fn find(w: &Wiki, m: &Member, steps: &[Step], snap: &Snap) -> Found {
    let mut f = Found { marks: vec![Mark::no(); steps.len()], ..Default::default() };
    let Some(id) = wiki_id(&m.species) else { return f };
    let chain = chain_of(w, &id);
    let stage_of = |wid: &str| chain.iter().position(|c| c == wid);
    let mut fam: Vec<(i32, usize)> = snap.owned.iter().enumerate().filter_map(|(i, o)| stage_of(&o.wiki).map(|st| (score(o, m, st), i))).collect();
    fam.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    f.family = fam.iter().map(|x| x.1).collect();
    f.pick = match m.game_pick {
        GamePick::Auto => f.family.first().copied(),
        GamePick::Pid(p) => snap.owned.iter().position(|o| o.pid == p),
        GamePick::Nothing => None,
    };
    let pick = f.pick.map(|i| &snap.owned[i]);
    let bag_has = |item: &str| -> Option<String> {
        let id = to_id(item);
        snap.bag.iter().find(|(b, _)| !id.is_empty() && *b == id).map(|(_, n)| n.clone())
    };
    let caught_now = snap.caught.iter().any(|c| chain.contains(c));
    for (k, s) in steps.iter().enumerate() {
        let key = s.key.as_str();
        let mark = if key == "catch" {
            if let Some(o) = pick {
                Mark::game(o.at.clone())
            } else if caught_now {
                Mark::game("Caught just now")
            } else {
                Mark::no()
            }
        } else if let Some(c) = key.strip_prefix("evolve:") {
            let need = stage_of(c).unwrap_or(usize::MAX);
            match pick {
                Some(o) if stage_of(&o.wiki).is_some_and(|st| st >= need) => Mark::game(""),
                Some(o) => {
                    let name = o.m.display_name();
                    // a stone or other item it needs, in your Bag
                    let method = to_id(&s.detail);
                    let held = snap.bag.iter().find(|(b, _)| b.len() >= 5 && method.contains(b.as_str()));
                    if let Some((_, n)) = held {
                        Mark::half(format!("{n} in your Bag"))
                    } else if s.detail.to_lowercase().contains("level") {
                        Mark::progress(format!("{name} is Lv {}", o.m.level))
                    } else {
                        Mark::progress(format!("{name} is a {}", o.m.species))
                    }
                }
                None => Mark::no(),
            }
        } else if key.starts_with("form:") {
            match pick {
                Some(o) if o.m.species == m.species => Mark::game(""),
                _ => Mark::no(),
            }
        } else if key == "keystone" {
            match bag_has("Key Stone") {
                Some(_) => Mark::game("In your Bag"),
                None => Mark::no(),
            }
        } else if let Some(mv) = key.strip_prefix("move:") {
            if pick.is_some_and(|o| knows(mv, &o.m.moves)) {
                Mark::game("")
            } else if let Some(tm) = s.links.iter().map(|l| &l.0).find(|l| (l.starts_with("TM") || l.starts_with("HM")) && bag_has(l).is_some()) {
                Mark::half(format!("{tm} in your Bag"))
            } else {
                Mark::no()
            }
        } else if key == "item" {
            if pick.is_some_and(|o| same(&o.m.item, &m.item)) {
                Mark::game("Holding it")
            } else if bag_has(&m.item).is_some() {
                Mark::half("In your Bag")
            } else {
                Mark::no()
            }
        } else if let Some(a) = key.strip_prefix("ability:") {
            match pick {
                Some(o) if to_id(&o.m.ability) == a => Mark::game(""),
                _ => Mark::no(),
            }
        } else if let Some(n) = key.strip_prefix("nature:") {
            match pick {
                Some(o) if o.m.nature.to_lowercase() == n => Mark::game(""),
                _ => Mark::no(),
            }
        } else if key == "evs" {
            match pick {
                Some(o) => {
                    let short: Vec<String> = (0..6).filter(|&i| m.evs[i] > 0 && o.m.evs[i] < m.evs[i]).map(|i| format!("{} {}/{}", STATS[i], o.m.evs[i], m.evs[i])).collect();
                    if short.is_empty() {
                        Mark::game("")
                    } else {
                        Mark::progress(short.join(" · "))
                    }
                }
                None => Mark::no(),
            }
        } else if let Some(l) = key.strip_prefix("level:") {
            let want: u8 = l.parse().unwrap_or(100);
            match pick {
                Some(o) if o.m.level >= want => Mark::game(""),
                Some(o) => Mark::progress(format!("Lv {} now", o.m.level)),
                None => Mark::no(),
            }
        } else {
            Mark::no()
        };
        f.marks[k] = mark;
    }
    f
}

/// (done, total) for a member: ticked by you, or done in your game.
pub(super) fn member_done(m: &Member, steps: &[Step], f: Option<&Found>) -> usize {
    steps.iter().enumerate().filter(|(i, s)| m.done.contains(&s.key) || f.is_some_and(|f| f.kind(*i) == Kind::Game)).count()
}

/// Cached between frames: rebuilt when the game saves or catches something.
#[derive(Default)]
pub(super) struct TeamGame {
    snap: Option<Arc<Snap>>,
    key: (u64, usize),
    /// The world last read (its name and Trainer ID), and when the app
    /// first saw it: catches the log shows before that were in another world.
    world: Option<(String, u16)>,
    world_since: u32,
}

/// Seconds since local midnight (how the log's times are kept).
fn day_secs(ms: u64) -> u32 {
    ((ms / 1000) as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400) as u32
}

impl App {
    /// What your game has, for the checklist (None: no save read yet).
    pub(super) fn team_snap(&mut self) -> Option<Arc<Snap>> {
        let save = self.setup.boxes.save.as_ref()?;
        let ms = self.setup.boxes.save_ms;
        let n_events = self.setup.party_events.len();
        if self.setup.team_game.snap.is_some() && self.setup.team_game.key == (ms, n_events) {
            return self.setup.team_game.snap.clone();
        }
        let save = save.clone();
        let text = self.setup.boxes.text.clone().unwrap_or_default();
        let world = self.team_world();
        let mut owned = Vec::new();
        let mut add = |me: &App, mon: &crate::gamesave::Mon, at: String, party: bool| {
            if mon.egg {
                return;
            }
            let (Some(m), Some(wiki)) = (me.boxes_member(mon, &text, world), crate::breed_save::species_id(mon.species)) else { return };
            owned.push(Owned { pid: mon.pid, at, party, m, wiki });
        };
        for (i, mon) in save.party.iter().enumerate() {
            add(self, mon, format!("Party, place {}", i + 1), true);
        }
        for (b, bx) in save.boxes.iter().enumerate() {
            let name = if bx.name.trim().is_empty() { format!("Box {}", b + 1) } else { bx.name.trim().to_string() };
            for (k, mon) in bx.slots.iter().enumerate() {
                if let Some(mon) = mon {
                    add(self, mon, format!("{name}, place {}", k + 1), false);
                }
            }
        }
        let bag = save
            .bag
            .iter()
            .filter(|b| b.2 > 0 && (b.1 as usize) < text.items.len())
            .map(|b| {
                let n = text.item_name(b.1);
                (to_id(&n), n)
            })
            .collect();
        // the world being played: the save is only ever this one's
        let world = std::fs::read_to_string(self.game_root().join("world-name.txt")).map(|s| s.trim().to_string()).unwrap_or_default();
        let key = (world.clone(), save.tid);
        if self.setup.team_game.world.as_ref() != Some(&key) {
            if self.setup.team_game.world.is_some() {
                self.setup.team_game.world_since = day_secs(crate::config::now_unix() * 1000);
            }
            self.setup.team_game.world = Some(key);
        }
        let from = day_secs(ms).max(self.setup.team_game.world_since);
        let caught = self
            .setup
            .party_events
            .iter()
            .filter(|(at, _)| *at >= from)
            .filter_map(|(_, e)| match e {
                crate::gamelog::PartyEvent::Caught { species } => crate::breed_save::species_id(*species as u16),
                _ => None,
            })
            .collect();
        let snap = Arc::new(Snap { ms, owned, bag, caught, world });
        self.setup.team_game.snap = Some(snap.clone());
        self.setup.team_game.key = (ms, n_events);
        Some(snap)
    }

    /// Each member of `t` in your game (None when the team is ticked by
    /// hand only, or there's no save).
    pub(super) fn team_found(&mut self, w: &Wiki, t: &Team, all: &[Vec<Step>]) -> Option<(Arc<Snap>, Vec<Found>)> {
        if t.hand_only {
            return None;
        }
        let snap = self.team_snap()?;
        let f = t.members.iter().zip(all).map(|(m, ss)| find(w, m, ss, &snap)).collect();
        Some((snap, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::World;

    fn wiki() -> Wiki {
        Wiki::parse(include_bytes!("../../assets/wiki.json")).expect("wiki")
    }

    fn own(pid: u32, species: &str, level: u8, moves: &[&str], party: bool) -> Owned {
        let mut m = Member::new(species, World::Platinum);
        m.level = level;
        m.nature = "Jolly".into();
        m.moves = moves.iter().map(|s| s.to_string()).collect();
        Owned { pid, at: if party { "Party, place 1".into() } else { "Box 3".into() }, party, wiki: wiki_id(species).unwrap(), m }
    }

    fn plan() -> Member {
        let mut m = Member::new("Garchomp", World::Platinum);
        m.level = 60;
        m.nature = "Jolly".into();
        m.ability = "Rough Skin".into();
        m.moves = vec!["Earthquake".into(), "Dragon Claw".into()];
        m.evs = [0, 252, 0, 0, 0, 252];
        m
    }

    fn mark<'a>(f: &'a Found, ss: &[Step], key: &str) -> &'a Mark {
        let keys: Vec<&str> = ss.iter().map(|s| s.key.as_str()).collect();
        &f.marks[ss.iter().position(|s| s.key == key).unwrap_or_else(|| panic!("no {key} in {keys:?} (gible is {:?})", wiki_id("Gible")))]
    }

    #[test]
    fn a_gabite_in_a_box_ticks_what_it_has() {
        let w = wiki();
        let m = plan();
        let ss = super::super::team_check::steps(&w, World::Platinum, &m, None);
        let mut gab = own(7, "Gabite", 41, &["Dragon Claw", "Dig"], false);
        gab.m.evs = [0, 96, 0, 0, 0, 40];
        let snap = Snap { owned: vec![own(1, "Bidoof", 10, &[], true), gab], bag: vec![("tm26".into(), "TM26".into())], ..Default::default() };
        let f = find(&w, &m, &ss, &snap);
        assert_eq!(f.pick, Some(1));
        assert_eq!(mark(&f, &ss, "catch").kind, Kind::Game);
        assert_eq!(mark(&f, &ss, "catch").note, "Box 3");
        assert_eq!(mark(&f, &ss, "evolve:GABITE").kind, Kind::Game);
        let g = mark(&f, &ss, "evolve:GARCHOMP");
        assert_eq!(g.kind, Kind::No);
        assert!(g.note.contains("Lv 41"), "{}", g.note);
        assert_eq!(mark(&f, &ss, "move:dragonclaw").kind, Kind::Game);
        let eq = mark(&f, &ss, "move:earthquake");
        assert_eq!(eq.kind, Kind::Half, "{eq:?}");
        assert!(eq.note.contains("TM26"));
        assert_eq!(mark(&f, &ss, "nature:jolly").kind, Kind::Game);
        let ev = mark(&f, &ss, "evs");
        assert_eq!(ev.kind, Kind::No);
        assert_eq!(ev.note, "Atk 96/252 · Spe 40/252");
        assert_eq!(member_done(&m, &ss, Some(&f)), 4);
    }

    #[test]
    fn the_one_furthest_along_counts_and_a_pick_wins() {
        let w = wiki();
        let mut m = plan();
        let ss = super::super::team_check::steps(&w, World::Platinum, &m, None);
        let snap = Snap { owned: vec![own(1, "Gible", 5, &[], false), own(2, "Garchomp", 70, &["Earthquake"], true), own(3, "Gible", 30, &["Dragon Claw"], false)], ..Default::default() };
        let f = find(&w, &m, &ss, &snap);
        assert_eq!(f.pick, Some(1));
        assert_eq!(f.family, vec![1, 2, 0]);
        assert_eq!(mark(&f, &ss, "level:60").kind, Kind::Game);
        m.game_pick = GamePick::Pid(3);
        let f = find(&w, &m, &ss, &snap);
        assert_eq!(f.pick, Some(2));
        assert_eq!(mark(&f, &ss, "evolve:GABITE").kind, Kind::No);
        m.game_pick = GamePick::Nothing;
        let f = find(&w, &m, &ss, &snap);
        assert_eq!(f.pick, None);
        assert_eq!(mark(&f, &ss, "catch").kind, Kind::No);
    }

    #[test]
    fn a_catch_the_save_has_not_seen_yet() {
        let w = wiki();
        let m = plan();
        let ss = super::super::team_check::steps(&w, World::Platinum, &m, None);
        let snap = Snap { caught: vec!["GIBLE".into()], ..Default::default() };
        let f = find(&w, &m, &ss, &snap);
        assert_eq!(mark(&f, &ss, "catch").note, "Caught just now");
        assert_eq!(mark(&f, &ss, "catch").kind, Kind::Game);
        // someone else's family doesn't count
        let snap = Snap { caught: vec!["BIDOOF".into()], ..Default::default() };
        assert_eq!(mark(&find(&w, &m, &ss, &snap), &ss, "catch").kind, Kind::No);
    }
}
