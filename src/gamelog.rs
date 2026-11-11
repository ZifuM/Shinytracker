//! Reads Platinum Together's own session log (Game\logs\session-*.log) to
//! count encounters exactly. The game writes one line per wild Pokémon when a
//! battle starts, e.g.
//!
//!   07:07:16 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)
//!
//! followed by a "BATTLE encounter effect" line. A double battle writes two
//! WILD lines. Only the log text is read; nothing in the game is touched.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, PartialEq)]
pub struct WildLine {
    /// National Dex number.
    pub species: u32,
    pub level: u32,
    /// Shiny chance as written by the game: `chance_num` in `chance_den`.
    pub chance_num: u32,
    pub chance_den: u32,
    /// e.g. "sweet scent on rare grass".
    pub method: String,
}

fn number_after<'a>(s: &'a str, key: &str) -> Option<(u32, &'a str)> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let n = rest[..end].parse().ok()?;
    Some((n, &rest[end..]))
}

/// Parse one log line; `None` if it isn't a WILD encounter line.
pub fn parse_wild(line: &str) -> Option<WildLine> {
    let i = line.find(" WILD ")?;
    let body = &line[i + 6..];
    let (chance_num, rest) = number_after(body, "shiny chance ")?;
    let (chance_den, rest) = number_after(rest, " in ")?;
    let (species, rest) = number_after(rest, "species ")?;
    let level = number_after(rest, "level ").map(|(n, _)| n).unwrap_or(0);
    let method = match (rest.find('('), rest.rfind(')')) {
        (Some(a), Some(b)) if b > a => rest[a + 1..b].to_string(),
        _ => String::new(),
    };
    if chance_den == 0 || species == 0 {
        return None;
    }
    Some(WildLine {
        species,
        level,
        chance_num,
        chance_den,
        method,
    })
}

/// A Pokémon the game handed over outside battle (so far: the starter from
/// the briefcase), e.g.
///
///   07:31:32 INFO  STARTER  starter given: species 7 shiny 0 (case shiny 0)
#[derive(Debug, Clone, PartialEq)]
pub struct GiftLine {
    /// National Dex number.
    pub species: u32,
    pub shiny: bool,
}

/// Parse one log line; `None` if it isn't a "starter given" line.
pub fn parse_gift(line: &str) -> Option<GiftLine> {
    let i = line.find(" STARTER ")?;
    let body = &line[i + 9..];
    let (species, rest) = number_after(body, "starter given: species ")?;
    let shiny = number_after(rest, " shiny ").map_or(false, |(n, _)| n > 0);
    (species > 0).then_some(GiftLine { species, shiny })
}

/// What the log says about how you're hunting (used to switch the hunting
/// style automatically).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodHint {
    /// An encounter tagged "(sweet scent on rare grass)".
    RareGrass,
    /// A starter taken from the briefcase.
    Starter,
    /// Went back to the last save, then met a single Pokémon soon after.
    StaticAfterReset,
}

/// "07:01:03 INFO  RESET    going back to the last save" (the first of the
/// pair the game writes for each reset).
pub fn is_reset(line: &str) -> bool {
    line.contains(" RESET ") && line.trim_end().ends_with("going back to the last save")
}

/// A reset only counts towards "static" if the encounter comes this soon.
const STATIC_AFTER_RESET: Duration = Duration::from_secs(120);

pub fn is_battle_start(line: &str) -> bool {
    line.contains(" BATTLE ") && line.contains("encounter effect")
}

/// Groups WILD lines into battles.
#[derive(Default)]
pub struct Grouper {
    pending: Vec<WildLine>,
    since: Option<Instant>,
}

impl Grouper {
    /// Feed one line; returns a finished battle when its start line is seen.
    pub fn push(&mut self, line: &str, now: Instant) -> Option<Vec<WildLine>> {
        if let Some(w) = parse_wild(line) {
            if self.pending.is_empty() {
                self.since = Some(now);
            }
            self.pending.push(w);
            return None;
        }
        if is_battle_start(line) && !self.pending.is_empty() {
            self.since = None;
            return Some(std::mem::take(&mut self.pending));
        }
        None
    }

    /// If the battle-start line never came, release the group anyway.
    pub fn flush_stale(&mut self, now: Instant, after: Duration) -> Option<Vec<WildLine>> {
        match self.since {
            Some(t) if now.duration_since(t) >= after && !self.pending.is_empty() => {
                self.since = None;
                Some(std::mem::take(&mut self.pending))
            }
            _ => None,
        }
    }
}

/// Where the game keeps its logs on this PC.
pub fn default_log_dir() -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let d = base.join("PlatinumTogether").join("Game").join("logs");
    d.is_dir().then_some(d)
}

/// The log folder to follow: the one picked by hand if it's there,
/// otherwise the game's usual one. A picked folder may be the logs folder
/// itself or one above it (the Game folder, or the PlatinumTogether folder).
pub fn log_dir(custom: Option<&Path>) -> Option<PathBuf> {
    if let Some(c) = custom {
        let tries = [c.to_path_buf(), c.join("logs"), c.join("Game").join("logs")];
        // First one that actually holds session logs, else the folder as given.
        if let Some(d) = tries.iter().find(|d| newest_log(d).is_some()) {
            return Some(d.clone());
        }
        if c.is_dir() {
            return Some(c.to_path_buf());
        }
    }
    default_log_dir()
}

fn newest_log(dir: &Path) -> Option<(PathBuf, SystemTime)> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("session-") && n.ends_with(".log")
        })
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.modified().ok()?)))
        .max_by_key(|(_, t)| *t)
}

/// Follows the newest session log, like `tail -f`.
pub struct LogTail {
    dir: PathBuf,
    file: Option<PathBuf>,
    pos: u64,
    partial: String,
    last_scan: Option<Instant>,
    first_pick: bool,
    grouper: Grouper,
    gifts: Vec<GiftLine>,
    hints: Vec<MethodHint>,
    last_reset: Option<Instant>,
}

impl LogTail {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            file: None,
            pos: 0,
            partial: String::new(),
            last_scan: None,
            first_pick: true,
            grouper: Grouper::default(),
            gifts: Vec::new(),
            hints: Vec::new(),
            last_reset: None,
        }
    }

    /// The folder being followed.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn file_name(&self) -> Option<String> {
        self.file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
    }

    fn rescan(&mut self) -> Result<(), String> {
        let (newest, _) = newest_log(&self.dir).ok_or("No game log found yet. Start the game.")?;
        if self.file.as_ref() != Some(&newest) {
            // At startup skip what's already in the log (old battles); a log
            // that appears later is a new game session, so read it from the top.
            self.pos = if self.first_pick {
                std::fs::metadata(&newest).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };
            self.first_pick = false;
            self.partial.clear();
            self.grouper = Grouper::default();
            self.file = Some(newest);
        }
        Ok(())
    }

    /// What a battle says about the hunting style.
    fn note_battle(&mut self, group: &[WildLine], now: Instant) {
        if group.iter().any(|w| w.method.contains("rare grass")) {
            self.hints.push(MethodHint::RareGrass);
        } else if group.len() == 1
            && self.last_reset.is_some_and(|t| now.duration_since(t) <= STATIC_AFTER_RESET)
        {
            self.hints.push(MethodHint::StaticAfterReset);
        }
        self.last_reset = None;
    }

    /// Hunting-style hints since the last call.
    pub fn take_hints(&mut self) -> Vec<MethodHint> {
        std::mem::take(&mut self.hints)
    }

    /// Starters (gifts) handed over since the last call.
    pub fn take_gifts(&mut self) -> Vec<GiftLine> {
        std::mem::take(&mut self.gifts)
    }

    /// New battles since the last call (each = the WILD lines of one battle).
    pub fn poll(&mut self) -> Result<Vec<Vec<WildLine>>, String> {
        let now = Instant::now();
        if self.file.is_none()
            || self
                .last_scan
                .map_or(true, |t| now.duration_since(t) > Duration::from_secs(3))
        {
            self.last_scan = Some(now);
            self.rescan()?;
        }
        let Some(path) = &self.file else {
            return Ok(Vec::new());
        };
        let mut f = File::open(path).map_err(|e| format!("Can't open the game log: {e}"))?;
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.pos {
            self.pos = 0; // log was recreated
        }
        let mut out = Vec::new();
        if len > self.pos {
            f.seek(SeekFrom::Start(self.pos))
                .map_err(|e| e.to_string())?;
            let mut buf = Vec::with_capacity((len - self.pos) as usize);
            f.take(len - self.pos)
                .read_to_end(&mut buf)
                .map_err(|e| e.to_string())?;
            self.pos += buf.len() as u64;
            self.partial.push_str(&String::from_utf8_lossy(&buf));
            while let Some(nl) = self.partial.find('\n') {
                let line: String = self.partial.drain(..=nl).collect();
                if let Some(g) = parse_gift(line.trim_end()) {
                    self.gifts.push(g);
                    self.hints.push(MethodHint::Starter);
                    self.last_reset = None;
                }
                if is_reset(&line) {
                    self.last_reset = Some(now);
                }
                if let Some(group) = self.grouper.push(line.trim_end(), now) {
                    self.note_battle(&group, now);
                    out.push(group);
                }
            }
        }
        if let Some(group) = self.grouper.flush_stale(now, Duration::from_millis(1500)) {
            self.note_battle(&group, now);
            out.push(group);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SAMPLE: &str = "\
07:07:11 INFO  DISPLAY  context field
07:07:16 INFO  RARE     sweet scent: evolve roll 1 of 25, species 447 becomes 448
07:07:16 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)
07:07:16 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)
07:07:17 INFO  BATTLE   encounter effect took 705 ms
07:07:17 INFO  DISPLAY  context battle
07:07:31 INFO  WILD     shiny chance 4 in 8192 for species 447 at level 24 (sweet scent on rare grass)
07:07:31 INFO  WILD     shiny chance 4 in 8192 for species 447 at level 24 (sweet scent on rare grass)
07:07:31 INFO  BATTLE   encounter effect took 175 ms
";

    #[test]
    fn spots_resets() {
        assert!(is_reset("07:01:03 INFO  RESET    going back to the last save"));
        assert!(is_reset("07:01:03 INFO  RESET    going back to the last save\r\n"));
        assert!(!is_reset("07:01:04 INFO  RESET    going back to the last save without a restart"));
        assert!(!is_reset("07:01:09 WARN  RESET    back to the last save not done: something stayed open"));
    }

    #[test]
    fn hints_from_battles() {
        let mut t = LogTail::new(std::env::temp_dir());
        let now = Instant::now();
        let rare = parse_wild("07:07:16 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)").unwrap();
        let plain = parse_wild("16:54:24 INFO  WILD     shiny chance 2 in 8192 for species 399 at level 4").unwrap();
        t.note_battle(&[rare.clone(), rare], now);
        assert_eq!(t.take_hints(), vec![MethodHint::RareGrass]);
        // A plain single encounter is only "static" right after a reset.
        t.note_battle(&[plain.clone()], now);
        assert!(t.take_hints().is_empty());
        t.last_reset = Some(now);
        t.note_battle(&[plain.clone()], now + Duration::from_secs(30));
        assert_eq!(t.take_hints(), vec![MethodHint::StaticAfterReset]);
        // ...and only once per reset; doubles don't count.
        t.note_battle(&[plain.clone()], now + Duration::from_secs(40));
        t.last_reset = Some(now);
        t.note_battle(&[plain.clone(), plain], now + Duration::from_secs(10));
        assert!(t.take_hints().is_empty());
    }

    #[test]
    fn parses_starter_line() {
        assert_eq!(
            parse_gift("07:31:32 INFO  STARTER  starter given: species 7 shiny 0 (case shiny 0)"),
            Some(GiftLine { species: 7, shiny: false })
        );
        assert_eq!(
            parse_gift("07:31:32 INFO  STARTER  starter given: species 390 shiny 1 (case shiny 1)"),
            Some(GiftLine { species: 390, shiny: true })
        );
        assert_eq!(parse_gift("07:31:32 INFO  STARTER  starter given as species 7 (story starter 393)"), None);
        assert_eq!(parse_gift("07:31:31 INFO  STARTER  the briefcase's YES: 1, 4, 7 kept for the gift (pool type -1)"), None);
    }

    #[test]
    fn parses_wild_line() {
        let w = parse_wild("07:07:16 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)").unwrap();
        assert_eq!(
            w,
            WildLine {
                species: 448,
                level: 24,
                chance_num: 4,
                chance_den: 8192,
                method: "sweet scent on rare grass".into()
            }
        );
        assert!(parse_wild("07:07:17 INFO  BATTLE   encounter effect took 705 ms").is_none());
        assert!(parse_wild(
            "07:07:16 INFO  RARE     sweet scent: evolve roll 1 of 25, species 447 becomes 448"
        )
        .is_none());
    }

    #[test]
    fn groups_double_battles() {
        let mut g = Grouper::default();
        let now = Instant::now();
        let groups: Vec<Vec<WildLine>> = SAMPLE.lines().filter_map(|l| g.push(l, now)).collect();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[0][0].species, 448);
        assert_eq!(groups[1][1].species, 447);
    }

    #[test]
    fn stale_group_is_released() {
        let mut g = Grouper::default();
        let t0 = Instant::now();
        assert!(g
            .push(
                "07:00:00 INFO  WILD     shiny chance 1 in 8192 for species 25 at level 5 (grass)",
                t0
            )
            .is_none());
        assert!(g.flush_stale(t0, Duration::from_millis(1500)).is_none());
        let late = g
            .flush_stale(t0 + Duration::from_secs(2), Duration::from_millis(1500))
            .unwrap();
        assert_eq!(late.len(), 1);
    }

    #[test]
    fn tails_new_lines_only() {
        let dir = std::env::temp_dir().join(format!("gamelog-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session-20261109-070656.log");
        // Old battle already in the log before the tracker starts: not counted.
        std::fs::write(&path, SAMPLE).unwrap();
        let mut t = LogTail::new(dir.clone());
        assert!(t.poll().unwrap().is_empty());
        // A new battle written in two chunks (line split mid-way).
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(
            f,
            "07:09:02 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet sc"
        )
        .unwrap();
        f.flush().unwrap();
        assert!(t.poll().unwrap().is_empty());
        write!(f, "ent on rare grass)\n07:09:02 INFO  WILD     shiny chance 4 in 8192 for species 448 at level 24 (sweet scent on rare grass)\n07:09:02 INFO  BATTLE   encounter effect took 170 ms\n").unwrap();
        f.flush().unwrap();
        let got = t.poll().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].len(), 2);
        assert_eq!(
            t.file_name().as_deref(),
            Some("session-20261109-070656.log")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
