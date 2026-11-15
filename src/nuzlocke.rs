//! Nuzlocke runs, from the game's own files (read only): each world keeps
//! a route log in `worlds/<n>/encounters.ini`, one line a route:
//!
//!   16 542 2 2 0130,0136,…,ffff Route 201
//!   (zone, species, level then, status, nickname in the game's letters, route)
//!
//! The status is 1 or 2: 1 still in the run, 2 out of it (fainted, or not
//! caught; the game doesn't say which here). Its options come from the
//! world's `world.ini`. A Soul Link world also keeps the friend's log as
//! last seen, `soul-friend.ini` (their name, then the same lines without the
//! route's name); the two pair up by area. Scrambled files (run.ini,
//! challenge results) aren't read.

use crate::worldini::WorldOptions;
use std::path::Path;

/// One route's Pokémon.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteMon {
    pub zone: u16,
    pub route: String,
    /// The game's species number.
    pub species: u16,
    /// Its level when it was met.
    pub level: u8,
    /// Still in the run (status 1), or out of it (2).
    pub alive: bool,
    pub status: u8,
    /// Its nickname, or its name in capitals.
    pub nickname: String,
}

/// A world the game keeps (the one being played, and others it has files for).
#[derive(Debug, Clone)]
pub struct World {
    pub id: u32,
    /// Only known for the world being played.
    pub name: String,
    pub playing: bool,
    pub opts: Option<WorldOptions>,
    pub routes: Vec<RouteMon>,
    /// Played with a friend (the world keeps a Soul Link file).
    pub soul_link: bool,
    /// The friend's side of a Soul Link, as the game last saw it.
    pub friend: Option<Friend>,
    /// Your trainer name in this world (from its save), when known.
    pub trainer: String,
}

/// The friend in a Soul Link: their name and their routes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Friend {
    pub name: String,
    pub routes: Vec<RouteMon>,
}

/// One linked pair: the same area, yours and the friend's.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub zone: u16,
    pub route: String,
    pub mine: Option<RouteMon>,
    pub theirs: Option<RouteMon>,
}

impl Pair {
    /// Still in the run: neither half is out (a linked pair goes out
    /// together). A half not caught yet doesn't put the pair out.
    pub fn alive(&self) -> bool {
        self.mine.as_ref().map_or(true, |m| m.alive) && self.theirs.as_ref().map_or(true, |t| t.alive)
    }

    /// Both halves caught.
    pub fn whole(&self) -> bool {
        self.mine.is_some() && self.theirs.is_some()
    }
}

/// `soul-friend.ini`: `name=` and the friend's route lines (zone, species,
/// level, status, nickname; no route name, so it comes from the area list).
pub fn parse_friend(text: &str) -> Friend {
    let mut f = Friend::default();
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        if let Some(n) = line.strip_prefix("name=") {
            f.name = n.trim().to_string();
        } else if !line.contains('=') {
            if let Some(mut r) = parse_line(line) {
                // The friend's log counts one higher than yours (2 in where
                // yours says 1, 3 out where yours says 2: the same pairs
                // match up that way), so only 3 is out here.
                r.alive = r.status != 3;
                if r.route.is_empty() {
                    r.route = crate::rare_calc::zone_name(r.zone).unwrap_or("").to_string();
                }
                f.routes.push(r);
            }
        }
    }
    f
}

/// Your routes and the friend's side by side, paired by area: in the order
/// you got yours, then any the friend has that you don't.
pub fn pairs(mine: &[RouteMon], theirs: &[RouteMon]) -> Vec<Pair> {
    let mut out: Vec<Pair> = mine
        .iter()
        .map(|m| Pair { zone: m.zone, route: m.route.clone(), mine: Some(m.clone()), theirs: theirs.iter().find(|t| t.zone == m.zone).cloned() })
        .collect();
    for t in theirs.iter().filter(|t| !mine.iter().any(|m| m.zone == t.zone)) {
        let route = if t.route.is_empty() { format!("Area {}", t.zone) } else { t.route.clone() };
        out.push(Pair { zone: t.zone, route, mine: None, theirs: Some(t.clone()) });
    }
    out
}

impl World {
    /// "Duallocke · Classic rules" / "Normal".
    pub fn kind(&self) -> String {
        match self.opts.as_ref().map(|o| (o.challenge(), o.rules())) {
            Some((Some(c), Some(r))) => format!("{c} · {r} rules"),
            Some((Some(c), None)) => c,
            _ if self.soul_link => "Soul Link".into(),
            _ if !self.routes.is_empty() => "Nuzlocke".into(),
            _ => "Normal".into(),
        }
    }

    /// A Nuzlocke-style run (or at least has a route log).
    pub fn is_run(&self) -> bool {
        !self.routes.is_empty() || self.opts.as_ref().is_some_and(|o| o.challenge().is_some())
    }

    pub fn title(&self) -> String {
        if self.name.is_empty() {
            format!("World {}", self.id)
        } else {
            format!("{} · World {}", self.name, self.id)
        }
    }
}

/// One line of encounters.ini.
pub fn parse_line(line: &str) -> Option<RouteMon> {
    let mut it = line.split_whitespace();
    let zone = it.next()?.parse().ok()?;
    let species = it.next()?.parse().ok()?;
    let level = it.next()?.parse().ok()?;
    let status: u8 = it.next()?.parse().ok()?;
    let nick = it.next()?;
    let codes = nick.split(',').filter_map(|h| u16::from_str_radix(h, 16).ok()).take_while(|c| *c != 0xFFFF);
    let nickname = crate::gamesave::text(codes);
    let route = it.collect::<Vec<_>>().join(" ");
    Some(RouteMon { zone, route, species, level, alive: status == 1, status, nickname })
}

pub fn parse(text: &str) -> Vec<RouteMon> {
    text.lines().filter(|l| !l.trim_start().starts_with('#')).filter_map(parse_line).collect()
}

/// Every world the game has files for, the one being played first.
pub fn worlds(game_dir: &Path, playing: Option<&WorldOptions>) -> Vec<World> {
    let active = playing.and_then(|o| o.active);
    let mut ids: Vec<u32> = std::fs::read_dir(game_dir.join("worlds"))
        .map(|rd| rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().to_str()?.parse().ok()).collect())
        .unwrap_or_default();
    if let Some(a) = active {
        if !ids.contains(&a) {
            ids.push(a);
        }
    }
    ids.sort();
    let mut out: Vec<World> = ids
        .into_iter()
        .map(|id| {
            let dir = game_dir.join("worlds").join(id.to_string());
            let is_active = Some(id) == active;
            let opts = if is_active { playing.cloned() } else { crate::worldini::read_world(game_dir, id) };
            let mut routes = std::fs::read_to_string(dir.join("encounters.ini")).map(|t| parse(&t)).unwrap_or_default();
            if routes.is_empty() && is_active {
                // the world being played may keep its log next to world.ini
                routes = std::fs::read_to_string(game_dir.join("encounters.ini")).map(|t| parse(&t)).unwrap_or_default();
            }
            let friend = std::fs::read_to_string(dir.join("soul-friend.ini")).ok().map(|t| parse_friend(&t));
            // your name from the world's save, only needed beside a friend's
            let trainer = if friend.is_some() {
                let save = if is_active { crate::gamesave::newest_save(game_dir).map(|(p, _)| p) } else { None }.unwrap_or_else(|| dir.join("save.bin"));
                crate::gamesave::read(&save).map(|s| s.trainer).unwrap_or_default()
            } else {
                String::new()
            };
            World {
                id,
                name: if is_active { playing.map(|o| o.name.clone()).unwrap_or_default() } else { String::new() },
                playing: is_active,
                opts,
                routes,
                soul_link: friend.is_some(),
                friend,
                trainer,
            }
        })
        .collect();
    out.sort_by_key(|w| !w.playing);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_route_line() {
        let r = parse_line("48 169 12 1 0137,0153,0153,0157,0158,0145,0147,014c,0149,ffff Eterna Forest").unwrap();
        assert_eq!((r.zone, r.species, r.level, r.alive), (48, 169, 12, true));
        assert_eq!(r.route, "Eterna Forest");
        assert_eq!(r.nickname, "Moostache");
        let r = parse_line("16 542 2 2 0130,0136,012f,013e,012d,0132,0136,0133,0138,0131,ffff Route 201").unwrap();
        assert!(!r.alive);
        assert_eq!(r.nickname, "FLETCHLING");
    }

    /// Your worlds, if they're on this machine.
    #[test]
    fn reads_your_worlds() {
        let dir = Path::new("/mnt/user-data/uploads/PlatinumTogether/Game");
        let Some(w) = crate::worldini::read(dir) else { return };
        let all = worlds(dir, Some(&w));
        assert!(all[0].playing);
        let duo = all.iter().find(|x| x.id == 8).unwrap();
        assert_eq!(duo.routes.len(), 40);
        assert_eq!(duo.routes.iter().filter(|r| r.alive).count(), 26);
        assert!(duo.kind().starts_with("Duallocke"));
        let soul = all.iter().find(|x| x.id == 12).unwrap();
        assert!(soul.kind().contains("Soul link"), "{}", soul.kind());
        let f = soul.friend.as_ref().unwrap();
        assert_eq!(f.name, "Varli");
        let ps = pairs(&soul.routes, &f.routes);
        assert_eq!(ps[0].route, "Route 201");
        assert_eq!(ps[0].theirs.as_ref().unwrap().species, 43);
        assert!(ps.iter().all(|p| p.mine.is_some() || p.theirs.is_some()));
        // where yours went out, theirs did too (Route 203, Oreburgh Mine)
        for z in [18, 46] {
            let p = ps.iter().find(|p| p.zone == z).unwrap();
            assert!(!p.mine.as_ref().unwrap().alive && !p.theirs.as_ref().unwrap().alive);
        }
        // world 4 keeps its options as bare numbers
        assert_eq!(all.iter().find(|x| x.id == 4).unwrap().kind(), "Nuzlocke · Classic rules");
    }

    #[test]
    fn reads_a_friend() {
        let f = parse_friend("# logs\nseat=1\nkey=AAAA-BBBB\nname=Varli\n16 43 2 3 0139,012e,ffff\n160 1 5 2 0130,ffff\n");
        assert_eq!(f.name, "Varli");
        assert_eq!(f.routes.len(), 2);
        assert_eq!(f.routes[0].route, crate::rare_calc::zone_name(16).unwrap_or(""));
        let mine = vec![parse_line("16 61 2 1 013a,ffff Route 201").unwrap(), parse_line("17 16 4 1 013a,ffff Route 202").unwrap()];
        let ps = pairs(&mine, &f.routes);
        assert_eq!(ps.len(), 3);
        assert!(!ps[0].alive(), "the friend's half is out, so the pair is");
        assert!(ps[1].alive() && ps[1].theirs.is_none());
        assert!(ps[2].mine.is_none());
    }
}
