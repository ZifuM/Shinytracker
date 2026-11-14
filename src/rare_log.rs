//! Rare Grass: a history of what's been on rare grass each day (one entry
//! per date on the player's clock, when the game's rare grass changes), so the app can show a glossary of every Pokémon seen
//! there: how many days, when last, and where. Today's list comes from the
//! Map's own reading (the shared daily list plus the game's log); each day
//! the app sees is kept here. The first days are bundled in.
//!
//! Nothing here says what will come next: it only remembers what was there.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Days kept (about a year).
pub const DAYS_MAX: usize = 400;

/// One Pokémon on one day: species slug, rare grass tiles, and where
/// (place name, tiles there).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DayMon {
    pub slug: String,
    pub tiles: u32,
    pub places: Vec<(String, u32)>,
}

/// One day: its date (days since 1970), and what was out.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Day {
    pub day: i64,
    /// Test builds keyed days by the Central European reset (unix
    /// seconds); moved over to `day` when read.
    #[serde(skip_serializing_if = "is_zero")]
    pub reset: i64,
    pub mons: Vec<DayMon>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RareFile {
    pub version: u32,
    /// Oldest first.
    pub days: Vec<Day>,
}

/// What the glossary shows for one Pokémon.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub slug: String,
    /// The dates it was out, oldest first.
    pub days: Vec<i64>,
    /// Tiles over every day.
    pub tiles: u32,
}

fn is_zero(v: &i64) -> bool {
    *v == 0
}

/// The days bundled with the app: (date, [(slug, tiles, [(place, tiles)])]).
#[allow(clippy::type_complexity)]
const SEED: &[(i64, &[(&str, u32, &[(&str, u32)])])] = &[
    // 2026-11-12
    (20_769, &[
        ("bulbasaur", 2, &[]),
        ("squirtle", 1, &[]),
        ("abra", 2, &[]),
        ("onix", 2, &[]),
        ("kangaskhan", 2, &[]),
        ("lapras", 1, &[]),
        ("dratini", 1, &[]),
        ("chikorita", 1, &[]),
        ("cyndaquil", 1, &[]),
        ("togepi", 2, &[]),
        ("misdreavus", 1, &[]),
        ("heracross", 1, &[]),
        ("skarmory", 2, &[]),
        ("magby", 1, &[]),
        ("larvitar", 2, &[]),
        ("treecko", 3, &[]),
        ("torchic", 1, &[]),
        ("mudkip", 1, &[]),
        ("ralts", 1, &[]),
        ("bagon", 1, &[]),
        ("chimchar", 4, &[]),
        ("piplup", 1, &[]),
        ("spiritomb", 1, &[]),
        ("gible", 1, &[]),
        ("munchlax", 1, &[]),
        ("greninja", 1, &[]),
        ("delphox", 1, &[]),
        ("emboar", 1, &[]),
        ("tynamo", 1, &[]),
        ("falinks", 1, &[]),
        ("frigibax", 2, &[]),
        ("arctibax", 1, &[]),
    ]),
    // 2026-11-13
    (20_770, &[
        ("bulbasaur", 11, &[]),
        ("growlithe", 2, &[]),
        ("gastly", 6, &[]),
        ("onix", 7, &[]),
        ("scyther", 9, &[]),
        ("tauros", 3, &[]),
        ("lapras", 5, &[]),
        ("porygon", 9, &[]),
        ("snorlax", 9, &[]),
        ("dratini", 6, &[]),
        ("totodile", 1, &[]),
        ("pichu", 2, &[]),
        ("misdreavus", 9, &[]),
        ("heracross", 5, &[]),
        ("sneasel", 8, &[]),
        ("tyrogue", 6, &[]),
        ("elekid", 5, &[]),
        ("magby", 9, &[]),
        ("larvitar", 13, &[]),
        ("torchic", 7, &[]),
        ("mudkip", 1, &[]),
        ("ralts", 9, &[]),
        ("duskull", 2, &[]),
        ("bagon", 1, &[]),
        ("gible", 4, &[]),
        ("munchlax", 4, &[]),
        ("riolu", 7, &[]),
        ("froakie", 9, &[]),
        ("frogadier", 10, &[]),
        ("chespin", 9, &[]),
        ("fennekin", 1, &[]),
        ("delphox", 4, &[]),
        ("audino", 6, &[]),
        ("tynamo", 9, &[]),
        ("arctibax", 3, &[]),
    ]),
    // 2026-11-14
    (20_771, &[
        ("bulbasaur", 5, &[("Route 225", 3), ("Valley Windworks", 2)]),
        ("squirtle", 9, &[("Route 222", 9)]),
        ("vulpix", 9, &[("Route 209", 9)]),
        ("onix", 5, &[("Route 215", 5)]),
        ("lickitung", 6, &[("Route 224", 6)]),
        ("tangela", 9, &[("Route 221", 9)]),
        ("kangaskhan", 5, &[("Route 222", 5)]),
        ("pinsir", 9, &[("Route 211", 8), ("Route 217", 1)]),
        ("lapras", 9, &[("Route 228", 9)]),
        ("eevee", 2, &[("Route 212", 2)]),
        ("aerodactyl", 8, &[("Route 221", 6), ("Acuity Lakefront", 2)]),
        ("dratini", 14, &[("Route 230", 5), ("Route 204", 3), ("Route 212", 3), ("Route 227", 3)]),
        ("chikorita", 11, &[("Route 229", 8), ("Trophy Garden", 3)]),
        ("cyndaquil", 3, &[("Route 226", 3)]),
        ("yanma", 3, &[("Trophy Garden", 3)]),
        ("murkrow", 9, &[("Stark Mountain", 9)]),
        ("misdreavus", 8, &[("Acuity Lakefront", 8)]),
        ("gligar", 9, &[("Route 212", 9)]),
        ("heracross", 4, &[("Route 225", 4)]),
        ("sneasel", 3, &[("Route 225", 3)]),
        ("tyrogue", 9, &[("Route 208", 9)]),
        ("elekid", 2, &[("Route 214", 2)]),
        ("larvitar", 5, &[("Route 204", 3), ("Eterna Forest", 2)]),
        ("treecko", 6, &[("Route 202", 6)]),
        ("torchic", 3, &[("Route 213", 3)]),
        ("ralts", 3, &[("Route 204", 3)]),
        ("trapinch", 10, &[("Sendoff Spring", 5), ("Route 214", 3), ("Route 201", 2)]),
        ("swablu", 3, &[("Route 204", 3)]),
        ("duskull", 6, &[("Route 228", 6)]),
        ("absol", 3, &[("Route 212", 3)]),
        ("bagon", 2, &[("Route 221", 2)]),
        ("piplup", 11, &[("Acuity Lakefront", 9), ("Trophy Garden", 2)]),
        ("cranidos", 7, &[("Valor Lakefront", 7)]),
        ("gible", 6, &[("Route 227", 6)]),
        ("munchlax", 5, &[("Route 224", 3), ("Mt. Coronet", 2)]),
        ("chespin", 20, &[("Mt. Coronet", 9), ("Lake Verity", 6), ("Route 225", 3), ("Pal Park", 1), ("Route 201", 1)]),
        ("drampa", 2, &[("Route 225", 2)]),
        ("glimmet", 11, &[("Eterna Forest", 9), ("Pal Park", 1), ("Sendoff Spring", 1)]),
        ("frigibax", 3, &[("Acuity Lakefront", 3)]),
    ]),
];

impl RareFile {
    pub fn has_data(&self) -> bool {
        !self.days.is_empty()
    }

    /// Move days from test builds over to dates, and add the bundled days
    /// this file doesn't have yet.
    pub fn seed(&mut self) -> bool {
        let mut changed = false;
        for d in self.days.iter_mut() {
            if d.day == 0 && d.reset != 0 {
                d.day = crate::bingo::card_day(d.reset);
                d.reset = 0;
                changed = true;
            }
        }
        for (day, mons) in SEED {
            if self.days.iter().any(|d| d.day == *day) {
                continue;
            }
            let mons = mons.iter().map(|(s, t, p)| DayMon { slug: s.to_string(), tiles: *t, places: p.iter().map(|(n, k)| (n.to_string(), *k)).collect() }).collect();
            self.days.push(Day { day: *day, reset: 0, mons });
            changed = true;
        }
        if changed {
            self.days.sort_by_key(|d| d.day);
        }
        changed
    }

    /// Note what's out on date `day`: (slug, place, tiles there), one entry
    /// per Pokémon and place. Keeps the most tiles seen for each. True if
    /// anything changed.
    pub fn record(&mut self, day: i64, spots: &[(String, String, u32)]) -> bool {
        if spots.is_empty() {
            return false;
        }
        let i = match self.days.iter().position(|d| d.day == day) {
            Some(i) => i,
            None => {
                self.days.push(Day { day, reset: 0, mons: Vec::new() });
                self.days.sort_by_key(|d| d.day);
                if self.days.len() > DAYS_MAX {
                    let cut = self.days.len() - DAYS_MAX;
                    self.days.drain(..cut);
                }
                match self.days.iter().position(|d| d.day == day) {
                    Some(i) => i,
                    None => return false,
                }
            }
        };
        let before = self.days[i].clone();
        let today = &mut self.days[i];
        for (slug, place, n) in spots {
            let m = match today.mons.iter().position(|m| &m.slug == slug) {
                Some(k) => &mut today.mons[k],
                None => {
                    today.mons.push(DayMon { slug: slug.clone(), ..Default::default() });
                    today.mons.last_mut().expect("just pushed")
                }
            };
            match m.places.iter_mut().find(|p| &p.0 == place) {
                Some(p) => p.1 = p.1.max(*n),
                None => m.places.push((place.clone(), *n)),
            }
            let sum: u32 = m.places.iter().map(|p| p.1).sum();
            m.tiles = m.tiles.max(sum);
        }
        today.mons.sort_by(|a, b| a.slug.cmp(&b.slug));
        for m in today.mons.iter_mut() {
            m.places.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        }
        *today != before
    }

    /// Put a whole day's list in, replacing what was kept for that date (the
    /// app works each day out in full, so the newest list is the right one).
    /// True if anything changed.
    pub fn set_day(&mut self, day: i64, spots: &[(String, String, u32)]) -> bool {
        let mut fresh = RareFile::default();
        if !fresh.record(day, spots) {
            return false;
        }
        let new_day = fresh.days.remove(0);
        match self.days.iter_mut().find(|d| d.day == day) {
            Some(d) if *d == new_day => false,
            Some(d) => {
                *d = new_day;
                true
            }
            None => self.record(day, spots),
        }
    }

    /// Every Pokémon ever seen, with its days.
    pub fn glossary(&self) -> Vec<Seen> {
        let mut out: Vec<Seen> = Vec::new();
        for d in &self.days {
            for m in &d.mons {
                match out.iter_mut().find(|s| s.slug == m.slug) {
                    Some(s) => {
                        if !s.days.contains(&d.day) {
                            s.days.push(d.day);
                        }
                        s.tiles += m.tiles;
                    }
                    None => out.push(Seen { slug: m.slug.clone(), days: vec![d.day], tiles: m.tiles }),
                }
            }
        }
        out
    }

    /// Date `day`'s entry.
    #[cfg(test)]
    pub fn on(&self, day: i64) -> Option<&Day> {
        self.days.iter().find(|d| d.day == day)
    }
}

pub fn load_from(dir: &Path) -> (RareFile, Option<String>) {
    crate::safefile::load(dir, "rare-grass", "Rare Grass")
}

pub fn save_to(dir: &Path, f: &RareFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "rare-grass", f, first_this_run, RareFile::has_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_once_and_record_keeps_the_most() {
        let mut f = RareFile::default();
        assert!(f.seed());
        assert!(!f.seed(), "seeding twice adds nothing");
        assert_eq!(f.days.len(), SEED.len());
        let n = f.glossary().len();
        assert!(n >= 60, "{n} Pokémon in the bundled days");
        let reset = 20_772;
        let spots = vec![("dratini".to_string(), "Route 204".to_string(), 3), ("dratini".to_string(), "Route 212".to_string(), 2), ("lapras".to_string(), "Route 228".to_string(), 9)];
        assert!(f.record(reset, &spots));
        assert!(!f.record(reset, &spots), "the same again changes nothing");
        // a later read with fewer tiles keeps the most
        assert!(!f.record(reset, &[("lapras".to_string(), "Route 228".to_string(), 4)]));
        let d = f.on(reset).unwrap();
        let dr = d.mons.iter().find(|m| m.slug == "dratini").unwrap();
        assert_eq!(dr.tiles, 5);
        assert_eq!(dr.places[0], ("Route 204".to_string(), 3));
        let g = f.glossary();
        let lap = g.iter().find(|s| s.slug == "lapras").unwrap();
        assert_eq!(*lap.days.last().unwrap(), reset);
        assert!(lap.days.len() >= 2);
        assert!(!f.record(reset, &[]));
        // a whole day put in replaces the old list for that date
        let whole = vec![("eevee".to_string(), "Route 212 (south)".to_string(), 2)];
        assert!(f.set_day(reset, &whole));
        assert!(!f.set_day(reset, &whole), "the same list again changes nothing");
        let d = f.days.iter().find(|d| d.day == reset).unwrap();
        assert_eq!(d.mons.len(), 1);
        assert_eq!(d.mons[0].places, vec![("Route 212 (south)".to_string(), 2)]);
        // a test build's day (keyed by the Central European reset) moves over
        let mut old: RareFile = serde_json::from_str(r#"{"days":[{"reset":1794610800,"mons":[{"slug":"eevee","tiles":2,"places":[]}]}]}"#).unwrap();
        old.seed();
        assert!(old.days.iter().any(|d| d.day == 20_771 && d.mons.iter().any(|m| m.slug == "eevee")));
        assert_eq!(old.days.len(), SEED.len(), "it joins the bundled day of the same date");
    }
}
