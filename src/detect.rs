//! Turns OCR text into species names, and a stream of readings into encounter
//! events. Pure logic with an injected clock, so it is fully unit tested.

use crate::species::Species;

/// Words that show up near the name plates but are never species names.
const STOPWORDS: &[&str] = &[
    "WILD", "AND", "APPEARED", "LV", "HP", "THE", "FOE", "WHAT", "WILL", "DO", "GO", "FIGHT",
    "BAG", "RUN", "POKEMON", "POKMON", "GOT", "AWAY", "SAFELY", "SENT", "OUT", "A", "AN",
];

pub struct NameMatcher {
    keys: Vec<(String, usize)>,
}

impl NameMatcher {
    pub fn new(species: &[Species]) -> Self {
        Self {
            keys: species
                .iter()
                .enumerate()
                .map(|(i, s)| (s.key.clone(), i))
                .collect(),
        }
    }

    /// Allowed edit distance for a key of this length.
    fn tolerance(len: usize) -> usize {
        match len {
            0..=4 => 0,
            5..=7 => 1,
            _ => 2,
        }
    }

    /// Best species for one candidate string: (species index, distance).
    fn best(&self, cand: &str) -> Option<(usize, usize)> {
        if cand.len() < 3 {
            return None;
        }
        let mut best: Option<(usize, usize)> = None;
        for (key, idx) in &self.keys {
            let tol = Self::tolerance(key.len());
            if key.len().abs_diff(cand.len()) > tol {
                continue;
            }
            let d = levenshtein(cand, key);
            if d <= tol && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((*idx, d));
                if d == 0 {
                    break;
                }
            }
        }
        best
    }

    /// Every species name found in `text`, in reading order (duplicates kept,
    /// so "RIOLU RIOLU" gives two entries).
    pub fn find(&self, text: &str) -> Vec<usize> {
        let tokens = tokenize(text);
        let mut out = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            let t = &tokens[i];
            if is_stop(t) {
                i += 1;
                continue;
            }
            // Two-word names first: "MR MIME", "HO OH", "PORYGON Z", "IRON MOTH".
            let single = self.best(t);
            if let Some(next) = tokens.get(i + 1) {
                if !is_stop(next) {
                    let joined = format!("{t}{next}");
                    if let Some((idx, d2)) = self.best(&joined) {
                        if single.map_or(true, |(_, d1)| d2 <= d1) {
                            out.push(idx);
                            i += 2;
                            continue;
                        }
                    }
                }
            }
            if let Some((idx, _)) = single {
                out.push(idx);
            }
            i += 1;
        }
        out
    }
}

fn is_stop(t: &str) -> bool {
    STOPWORDS.contains(&t)
}

/// Uppercase words; apostrophes dropped ("FARFETCH'D" -> "FARFETCHD"), common
/// OCR digit/letter swaps fixed inside words ("RI0LU" -> "RIOLU"), pure
/// numbers (levels, HP) dropped.
pub fn tokenize(text: &str) -> Vec<String> {
    let cleaned: String = text
        .chars()
        .filter(|c| *c != '\'' && *c != '’' && *c != '`')
        .map(|c| match c {
            'é' | 'É' => 'E',
            c => c.to_ascii_uppercase(),
        })
        .collect();
    cleaned
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .filter(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
        .map(|w| {
            w.chars()
                .map(|c| match c {
                    '0' => 'O',
                    '1' => 'I',
                    '5' => 'S',
                    '8' => 'B',
                    c => c,
                })
                .collect()
        })
        .collect()
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(not(offline_test), derive(serde::Serialize, serde::Deserialize))]
pub enum PerBattle {
    /// Count however many names were read (1 in singles, 2 in doubles).
    Auto,
    /// Always count this many per battle once a battle is seen.
    Fixed(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct TrackerConfig {
    pub per_battle: PerBattle,
    /// Identical readings in a row needed before trusting a battle start.
    pub stable_reads: u32,
    /// Empty readings in a row needed before the battle counts as over.
    pub rearm_reads: u32,
    /// After a battle starts, a second name that appears late (plate sliding
    /// in) can still be added within this window.
    pub late_name_window_ms: u64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            per_battle: PerBattle::Auto,
            stable_reads: 2,
            rearm_reads: 3,
            late_name_window_ms: 2500,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackerEvent {
    /// A new battle: `added` Pokémon to add to the counter.
    Encounter {
        names: Vec<usize>,
        added: u32,
    },
    /// More names showed up just after the battle started.
    LateNames {
        names: Vec<usize>,
        added: u32,
    },
    BattleOver,
}

#[derive(Debug, PartialEq)]
enum State {
    Idle,
    InBattle {
        started_ms: u64,
        counted: u32,
        empty_reads: u32,
    },
}

pub struct Tracker {
    pub cfg: TrackerConfig,
    state: State,
    last: Vec<usize>,
    stable: u32,
}

impl Tracker {
    pub fn new(cfg: TrackerConfig) -> Self {
        Self {
            cfg,
            state: State::Idle,
            last: Vec::new(),
            stable: 0,
        }
    }

    pub fn in_battle(&self) -> bool {
        matches!(self.state, State::InBattle { .. })
    }

    pub fn battle_age_ms(&self, now_ms: u64) -> Option<u64> {
        match self.state {
            State::InBattle { started_ms, .. } => Some(now_ms.saturating_sub(started_ms)),
            State::Idle => None,
        }
    }

    pub fn reset(&mut self) {
        self.state = State::Idle;
        self.last.clear();
        self.stable = 0;
    }

    fn per_battle(&self, n_names: usize) -> u32 {
        match self.cfg.per_battle {
            PerBattle::Auto => (n_names as u32).clamp(1, 4),
            PerBattle::Fixed(n) => n.max(1),
        }
    }

    /// Feed one reading (species indices read from the screen).
    pub fn feed(&mut self, names: Vec<usize>, now_ms: u64) -> Option<TrackerEvent> {
        if !names.is_empty() && names == self.last {
            self.stable += 1;
        } else {
            self.stable = u32::from(!names.is_empty());
        }
        self.last = names.clone();
        let stable = self.stable >= self.cfg.stable_reads;

        match &mut self.state {
            State::Idle => {
                if stable {
                    let added = self.per_battle(names.len());
                    self.state = State::InBattle {
                        started_ms: now_ms,
                        counted: added,
                        empty_reads: 0,
                    };
                    return Some(TrackerEvent::Encounter { names, added });
                }
                None
            }
            State::InBattle {
                started_ms,
                counted,
                empty_reads,
            } => {
                if names.is_empty() {
                    *empty_reads += 1;
                    if *empty_reads >= self.cfg.rearm_reads {
                        self.state = State::Idle;
                        return Some(TrackerEvent::BattleOver);
                    }
                    return None;
                }
                *empty_reads = 0;
                let late_ok = now_ms.saturating_sub(*started_ms) <= self.cfg.late_name_window_ms;
                if late_ok && stable && self.cfg.per_battle == PerBattle::Auto {
                    let n = (names.len() as u32).clamp(1, 4);
                    if n > *counted {
                        let added = n - *counted;
                        *counted = n;
                        return Some(TrackerEvent::LateNames { names, added });
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::species;

    fn names(m: &NameMatcher, all: &[Species], text: &str) -> Vec<&'static str> {
        m.find(text).into_iter().map(|i| all[i].name).collect()
    }

    #[test]
    fn reads_plates() {
        let all = species::all();
        let m = NameMatcher::new(&all);
        assert_eq!(names(&m, &all, "RIOLU\nRIOLU"), vec!["Riolu", "Riolu"]);
        assert_eq!(names(&m, &all, "GRENIHJA *"), vec!["Greninja"]);
        assert_eq!(
            names(&m, &all, "RIOLU  Lv. 24\nRI0LU Lv.24"),
            vec!["Riolu", "Riolu"]
        );
        assert_eq!(
            names(&m, &all, "MR. MIME\nHO-OH"),
            vec!["Mr. Mime", "Ho-Oh"]
        );
        assert_eq!(names(&m, &all, "PORYGON-Z"), vec!["Porygon-Z"]);
        assert_eq!(names(&m, &all, "PORYGON2"), vec!["Porygon2"]);
        assert_eq!(names(&m, &all, "PORYGON"), vec!["Porygon"]);
        assert_eq!(names(&m, &all, "FARFETCH'D"), vec!["Farfetch'd"]);
        assert_eq!(
            names(&m, &all, "Wild ZUBAT and GEODUDE appeared!"),
            vec!["Zubat", "Geodude"]
        );
        assert_eq!(names(&m, &all, "IRON MOTH"), vec!["Iron Moth"]);
        assert_eq!(names(&m, &all, "MEW"), vec!["Mew"]);
        assert!(names(&m, &all, "What will STOMPY do?").is_empty());
        assert!(names(&m, &all, "HP 324/324").is_empty());
    }

    #[test]
    fn counts_each_pokemon_in_doubles_once() {
        let mut t = Tracker::new(TrackerConfig::default());
        let two = vec![447, 447];
        assert_eq!(t.feed(vec![], 0), None);
        assert_eq!(t.feed(two.clone(), 100), None); // needs 2 identical reads
        assert_eq!(
            t.feed(two.clone(), 200),
            Some(TrackerEvent::Encounter {
                names: two.clone(),
                added: 2
            })
        );
        for k in 0..20 {
            assert_eq!(t.feed(two.clone(), 300 + k * 100), None); // whole battle: no recount
        }
        assert_eq!(t.feed(vec![], 3000), None);
        assert_eq!(t.feed(vec![], 3100), None);
        assert_eq!(t.feed(vec![], 3200), Some(TrackerEvent::BattleOver));
        // next sweet scent
        assert_eq!(t.feed(two.clone(), 5000), None);
        assert!(matches!(
            t.feed(two, 5100),
            Some(TrackerEvent::Encounter { added: 2, .. })
        ));
    }

    #[test]
    fn late_second_plate_is_added() {
        let mut t = Tracker::new(TrackerConfig::default());
        t.feed(vec![1], 0);
        assert!(matches!(
            t.feed(vec![1], 100),
            Some(TrackerEvent::Encounter { added: 1, .. })
        ));
        t.feed(vec![1, 2], 200);
        assert!(matches!(
            t.feed(vec![1, 2], 300),
            Some(TrackerEvent::LateNames { added: 1, .. })
        ));
        assert_eq!(t.feed(vec![1, 2], 400), None);
    }

    #[test]
    fn ocr_flicker_does_not_end_battle() {
        let mut t = Tracker::new(TrackerConfig::default());
        t.feed(vec![5, 6], 0);
        t.feed(vec![5, 6], 100);
        assert_eq!(t.feed(vec![], 200), None);
        assert_eq!(t.feed(vec![5, 6], 300), None);
        assert_eq!(t.feed(vec![], 400), None);
        assert_eq!(t.feed(vec![], 500), None);
        assert_eq!(t.feed(vec![5, 6], 600), None);
        assert!(t.in_battle());
    }

    #[test]
    fn fixed_mode_always_counts_two() {
        let mut t = Tracker::new(TrackerConfig {
            per_battle: PerBattle::Fixed(2),
            ..Default::default()
        });
        t.feed(vec![9], 0);
        assert!(matches!(
            t.feed(vec![9], 100),
            Some(TrackerEvent::Encounter { added: 2, .. })
        ));
        t.feed(vec![9, 10], 200);
        assert_eq!(t.feed(vec![9, 10], 300), None);
    }
}
