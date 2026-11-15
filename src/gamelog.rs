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

/// Egg news from the log (game 0.4.0 on).
#[derive(Debug, Clone, PartialEq)]
pub enum EggEvent {
    /// "WISH an Egg is waiting at the Day-Care".
    Waiting,
    /// An Egg hatched. The species is only known when the naming page opened
    /// (or its picture was loaded for the first time this session), e.g.
    /// "EEVEE"; the log never says whether it's shiny.
    Hatched { species: Option<String> },
}

/// "07:02:55 INFO  WISH     an Egg is waiting at the Day-Care"
pub fn is_egg_waiting(line: &str) -> bool {
    line.contains(" WISH ") && line.contains("Egg is waiting")
}

/// Seconds into the day from the clock at the start of a line ("07:07:39 …").
fn line_secs(line: &str) -> Option<u32> {
    let b = line.as_bytes();
    if b.len() < 8 || b[2] != b':' || b[5] != b':' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| line.get(r)?.parse::<u32>().ok();
    Some(n(0..2)? * 3600 + n(3..5)? * 60 + n(6..8)?)
}

/// "EEVEE’s nickname?" / "RHYHORN_female/0 ready" → "EEVEE" / "RHYHORN".
fn log_species_name(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.split('_').next().unwrap_or(s);
    (!s.is_empty()).then(|| s.to_string())
}

/// The longest a hatch takes from its scene starting to the new Pokémon
/// being put in the party (the naming page can sit open a while).
const HATCH_WINDOW_SECS: u32 = 180;

/// Spots a hatch. The game has no hatch line, but every hatch writes the
/// same steps, with or without a nickname:
///
///   07:07:39 INFO  DISPLAY  scene wide 1 sides 0x8          (the hatch scene)
///   07:07:40 INFO  GEN5     EEVEE/0 ready …                  (first time only)
///   07:07:42 INFO  NAMING   the naming page opens: EEVEE’s nickname? …  (only if "Yes")
///   07:07:44 INFO  ORIGIN   1 Pokemon stamped with origin 1  (it joins you)
///
/// A battle in between calls it off, so caught Pokémon never count.
#[derive(Default)]
pub struct HatchWatch {
    /// When the scene started (seconds into the day) and the species so far.
    open: Option<(u32, Option<String>)>,
}

impl HatchWatch {
    pub fn push(&mut self, line: &str) -> Option<EggEvent> {
        if line.contains(" DISPLAY ") && line.contains("scene wide 1 sides 0x8") {
            self.open = Some((line_secs(line).unwrap_or(0), None));
            return None;
        }
        let (start, species) = self.open.as_mut()?;
        if let (Some(now), true) = (line_secs(line), *start > 0) {
            if (now + 86_400 - *start) % 86_400 > HATCH_WINDOW_SECS {
                self.open = None;
                return None;
            }
        }
        if parse_wild(line).is_some() || is_battle_start(line) || line.contains(" CATCH ") {
            self.open = None;
            return None;
        }
        if let Some(i) = line.find("the naming page opens: ") {
            let rest = &line[i + "the naming page opens: ".len()..];
            let end = rest.find(['’', '\'']).unwrap_or(rest.len());
            if let Some(n) = log_species_name(&rest[..end]) {
                *species = Some(n);
            }
            return None;
        }
        if species.is_none() && line.contains(" GEN5 ") && line.contains(" ready in ") {
            if let Some(i) = line.find(" GEN5 ") {
                let rest = line[i + 6..].trim_start();
                if let Some(slash) = rest.find('/') {
                    *species = log_species_name(&rest[..slash]);
                }
            }
            return None;
        }
        if line.contains(" ORIGIN ") && line.contains("Pokemon stamped") {
            let species = species.take();
            self.open = None;
            return Some(EggEvent::Hatched { species });
        }
        None
    }
}

/// How a wild battle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleEnd {
    /// The wild Pokémon fainted ("FAINT battler 1 state 0").
    Knocked,
    /// It was caught ("CATCH caught battler …").
    Caught,
    /// Neither: you ran (or it fled).
    Ran,
}

/// What the party is worked out from (with `world-card.ini`, written when
/// the game opens). The log gives box/party moves as slot numbers only.
#[derive(Debug, Clone, PartialEq)]
pub enum PartyEvent {
    /// "BOXES move 881 -> 3": 0-5 are party slots, bigger numbers box
    /// places. `seen` = Pokémon pictures the box page loaded since the last
    /// move (a hint at what came out of a box).
    Move { from: u32, to: u32, seen: Vec<String> },
    /// A wild Pokémon was caught (game's species number).
    Caught { species: u32 },
    /// The game's own number for a Pokémon, paired with the picture it
    /// loaded for it (for numbers past #493, which aren't Pokédex numbers).
    Named { species: u32, name: String },
}

/// "TALONFLAME/0", "EEVEE_female/0" → "TALONFLAME", "EEVEE".
pub fn picture_name(raw: &str) -> String {
    let n = raw.split('/').next().unwrap_or(raw);
    let n = n.strip_suffix("_female").or_else(|| n.strip_suffix("_male")).unwrap_or(n);
    n.to_string()
}

#[derive(Default)]
pub struct PartyWatch {
    box_open: bool,
    seen: Vec<String>,
    /// A wild Pokémon past #493 waiting for its picture line.
    wild: Option<u32>,
}

impl PartyWatch {
    pub fn push(&mut self, line: &str) -> Option<PartyEvent> {
        if line.contains(" UI ") && line.contains("box page: open") {
            self.box_open = true;
            self.seen.clear();
            return None;
        }
        if line.contains(" UI ") && line.contains("box page: closed") {
            self.box_open = false;
            self.seen.clear();
            return None;
        }
        if line.contains(" GEN5 ") && line.contains(" ready in ") {
            let raw = line.split(" GEN5 ").nth(1)?.split_whitespace().next()?;
            let name = picture_name(raw);
            if let Some(sp) = self.wild.take() {
                return Some(PartyEvent::Named { species: sp, name });
            }
            if self.box_open && !self.seen.contains(&name) {
                self.seen.push(name);
            }
            return None;
        }
        if line.contains(" WILD ") {
            // only a lone Pokémon past #493 can be paired with its picture
            let sp = line.split("for species ").nth(1).and_then(|r| r.split_whitespace().next()).and_then(|n| n.parse::<u32>().ok());
            self.wild = match (self.wild, sp) {
                (None, Some(n)) if n > 493 => Some(n),
                _ => None,
            };
            return None;
        }
        if line.contains(" BATTLE ") {
            self.wild = None;
        }
        if line.contains(" BOXES ") && line.contains(": done") {
            let r = line.split("move ").nth(1)?;
            let mut it = r.split(" -> ");
            let from = it.next()?.trim().parse().ok()?;
            let to = it.next()?.split(':').next()?.trim().parse().ok()?;
            return Some(PartyEvent::Move { from, to, seen: std::mem::take(&mut self.seen) });
        }
        if line.contains(" CATCH ") && line.contains("caught battler") {
            let sp = line.split("species ").nth(1)?.split_whitespace().next()?.parse().ok()?;
            return Some(PartyEvent::Caught { species: sp });
        }
        None
    }
}

/// `world-card.ini` (written by the game when it opens): the party then, as
/// the game's species numbers in slot order.
pub fn read_world_card(path: &Path) -> Option<Vec<u32>> {
    let s = std::fs::read_to_string(path).ok()?;
    let line = s.lines().find(|l| l.contains('\t'))?;
    let list = line.split('\t').nth(5)?;
    let party: Vec<u32> = list.split(',').filter_map(|e| e.split('.').next()?.trim().parse().ok()).collect();
    (!party.is_empty() && party.len() <= 6).then_some(party)
}

/// What a Poké Radar chain is worked out from. The game writes no chain
/// line, so the app counts it from these.
#[derive(Debug, Clone, PartialEq)]
pub enum RadarEvent {
    /// "bag page: use item 431" (or the Y wheel's "used item 431"):
    /// the Poké Radar was used.
    Used,
    /// One wild battle, from its WILD line(s) to "BATTLE HUD fade 16".
    Battle { species: u32, multi: bool, end: BattleEnd },
}

/// The Poké Radar is item 431 in Platinum.
pub fn is_radar_use(line: &str) -> bool {
    line.contains(" UI ") && (line.contains("use item 431 ") || line.contains("used item 431 ") || line.trim_end().ends_with("item 431"))
}

/// Follows wild battles from start to end, for the Poké Radar chain.
///
///   08:20:36 INFO  WILD     shiny chance 2 in 8192 for species 280 at level 17
///   08:20:36 INFO  BATTLE   encounter effect took 169 ms
///   08:20:45 INFO  FAINT    battler 1 state 0
///   08:20:48 INFO  BATTLE   HUD fade 16 towards 0000      (the battle is over)
#[derive(Default)]
pub struct RadarWatch {
    species: Vec<u32>,
    started: bool,
    knocked: bool,
    caught: bool,
}

impl RadarWatch {
    pub fn push(&mut self, line: &str) -> Option<RadarEvent> {
        if is_radar_use(line) {
            return Some(RadarEvent::Used);
        }
        if let Some(w) = parse_wild(line) {
            if self.started {
                // a new battle while the last one never ended: start over
                *self = Self::default();
            }
            self.species.push(w.species);
            return None;
        }
        if self.species.is_empty() {
            return None;
        }
        if is_battle_start(line) {
            self.started = true;
        } else if line.contains(" FAINT ") && (line.contains("battler 1 state 0") || line.contains("battler 3 state 0")) {
            // the wild side is battlers 1 and 3
            self.knocked = true;
        } else if line.contains(" CATCH ") && line.contains("caught battler") {
            self.caught = true;
        } else if self.started && line.contains(" BATTLE ") && line.contains("HUD fade 16") {
            let end = if self.caught {
                BattleEnd::Caught
            } else if self.knocked {
                BattleEnd::Knocked
            } else {
                BattleEnd::Ran
            };
            let ev = RadarEvent::Battle { species: self.species[0], multi: self.species.len() > 1, end };
            *self = Self::default();
            return Some(ev);
        }
        None
    }
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

/// A Poké Ball thrown: its item number.
///   12:50:10 INFO  CATCH    ball 15 from battler 2 at battler 1 (picked 1)
pub fn parse_ball_throw(line: &str) -> Option<u16> {
    let r = line.split(" CATCH ").nth(1)?.trim_start().strip_prefix("ball ")?;
    if !r.contains(" from battler ") {
        return None;
    }
    r.split_whitespace().next()?.parse().ok()
}

/// A Shiny Lure lit (true) or worn off (false), from the game's messages:
///   13:02:11 INFO  MESSAGE  field message: You used a Shiny Lure!
///   13:31:40 INFO  MESSAGE  field message: The Shiny Lure wore off...
pub fn parse_lure(line: &str) -> Option<bool> {
    let m = line.split(" MESSAGE ").nth(1)?;
    if m.contains("used a Shiny Lure") {
        Some(true)
    } else if m.contains("Shiny Lure wore off") {
        Some(false)
    } else {
        None
    }
}

/// Follows the newest session log, like `tail -f`.
pub struct LogTail {
    dir: PathBuf,
    /// The game folder setting it came from, to look again when the folder
    /// has no logs (yet).
    root: Option<PathBuf>,
    file: Option<PathBuf>,
    pos: u64,
    partial: String,
    last_scan: Option<Instant>,
    first_pick: bool,
    grouper: Grouper,
    gifts: Vec<GiftLine>,
    hints: Vec<MethodHint>,
    last_reset: Option<Instant>,
    hatch: HatchWatch,
    eggs: Vec<EggEvent>,
    radar: RadarWatch,
    radar_events: Vec<RadarEvent>,
    party: PartyWatch,
    /// (seconds into the day the line was written, event)
    party_events: Vec<(u32, PartyEvent)>,
    /// Poké Balls thrown: (seconds into the day, item number).
    balls: Vec<(u32, u16)>,
    /// The Shiny Lure's newest state, not yet passed on.
    lure: Option<bool>,
}

impl LogTail {
    pub fn new(dir: PathBuf, root: Option<PathBuf>) -> Self {
        Self {
            dir,
            root,
            file: None,
            pos: 0,
            partial: String::new(),
            last_scan: None,
            first_pick: true,
            grouper: Grouper::default(),
            gifts: Vec::new(),
            hints: Vec::new(),
            last_reset: None,
            hatch: HatchWatch::default(),
            eggs: Vec::new(),
            radar: RadarWatch::default(),
            radar_events: Vec::new(),
            party: PartyWatch::default(),
            party_events: Vec::new(),
            balls: Vec::new(),
            lure: None,
        }
    }

    /// The game's `world-card.ini` (next to the logs folder).
    pub fn world_card(&self) -> Option<PathBuf> {
        self.dir.parent().map(|d| d.join("world-card.ini"))
    }

    /// Poké Balls thrown since the last call: (seconds into the day, item).
    pub fn take_balls(&mut self) -> Vec<(u32, u16)> {
        std::mem::take(&mut self.balls)
    }

    /// The Shiny Lure lit or worn off since the last call (the newest).
    pub fn take_lure(&mut self) -> Option<bool> {
        self.lure.take()
    }

    /// Party moves / catches since the last call.
    pub fn take_party(&mut self) -> Vec<(u32, PartyEvent)> {
        std::mem::take(&mut self.party_events)
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
        if newest_log(&self.dir).is_none() {
            // Nothing here (yet): the logs folder may only now exist, so
            // work the folder out again.
            if let Some(d) = log_dir(self.root.as_deref()) {
                self.dir = d;
            }
        }
        let Some((newest, _)) = newest_log(&self.dir) else {
            // No logs when we started looking: the first one to appear is a
            // new game session, read from the top.
            self.first_pick = false;
            return Err("No game log found yet. Start the game.".into());
        };
        if self.file.as_ref() != Some(&newest) {
            // At startup skip what's already in the log (old battles); a log
            // that appears later is a new game session, so read it from the top.
            self.party = PartyWatch::default();
            self.pos = if self.first_pick {
                // Old battles are skipped, but this session's party moves are
                // read: the party is worked out from when the game opened.
                if let Ok(text) = std::fs::read_to_string(&newest) {
                    for line in text.lines() {
                        if let Some(e) = self.party.push(line) {
                            self.party_events.push((line_secs(line).unwrap_or(0), e));
                        }
                        // this session's throws too: the save may not have them yet
                        if let Some(b) = parse_ball_throw(line) {
                            self.balls.push((line_secs(line).unwrap_or(0), b));
                        }
                        // and whether a Shiny Lure is lit
                        if let Some(l) = parse_lure(line) {
                            self.lure = Some(l);
                        }
                    }
                }
                std::fs::metadata(&newest).map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };
            self.first_pick = false;
            self.partial.clear();
            self.grouper = Grouper::default();
            self.hatch = HatchWatch::default();
            self.radar = RadarWatch::default();
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

    /// Poké Radar uses and finished wild battles since the last call.
    pub fn take_radar(&mut self) -> Vec<RadarEvent> {
        std::mem::take(&mut self.radar_events)
    }

    /// Eggs waiting / hatched since the last call.
    pub fn take_eggs(&mut self) -> Vec<EggEvent> {
        std::mem::take(&mut self.eggs)
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
                if is_egg_waiting(&line) {
                    self.eggs.push(EggEvent::Waiting);
                }
                if let Some(e) = self.hatch.push(line.trim_end()) {
                    self.eggs.push(e);
                }
                if let Some(e) = self.radar.push(line.trim_end()) {
                    self.radar_events.push(e);
                }
                if let Some(e) = self.party.push(line.trim_end()) {
                    self.party_events.push((line_secs(&line).unwrap_or(0), e));
                }
                if let Some(b) = parse_ball_throw(line.trim_end()) {
                    self.balls.push((line_secs(&line).unwrap_or(0), b));
                }
                if let Some(l) = parse_lure(line.trim_end()) {
                    self.lure = Some(l);
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

/// Read what's new in every session log in `dir` since the byte positions
/// in `cursors` (file name → bytes read), and move them on. Returns each
/// wild Pokémon met (National Dex number, on rare grass) and whether the
/// cursors changed. The first read (no cursors yet) starts from the end of
/// every log, so nothing played before is counted; a log that turns up
/// later (a new session) is read from its top. Only whole lines are taken.
pub fn read_new_wild(dir: &Path, cursors: &mut std::collections::BTreeMap<String, u64>) -> Option<(Vec<(u32, bool)>, bool)> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<(String, PathBuf, u64)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (n.starts_with("session-") && n.ends_with(".log")).then(|| Some((n, e.path(), e.metadata().ok()?.len())))?
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let first = cursors.is_empty();
    let mut found = Vec::new();
    let mut moved = false;
    for (name, path, len) in &files {
        let at = cursors.get(name).copied();
        let from = match at {
            None if first => *len,
            None => 0,
            Some(p) if p > *len => 0,
            Some(p) => p,
        };
        if from >= *len {
            if at != Some(*len) {
                cursors.insert(name.clone(), *len);
                moved = true;
            }
            continue;
        }
        let Ok(mut f) = File::open(path) else { continue };
        if f.seek(SeekFrom::Start(from)).is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if f.take(len - from).read_to_end(&mut buf).is_err() {
            continue;
        }
        let upto = buf.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        for line in String::from_utf8_lossy(&buf[..upto]).lines() {
            if let Some(w) = parse_wild(line) {
                found.push((w.species, w.method.contains("rare grass")));
            }
        }
        cursors.insert(name.clone(), from + upto as u64);
        moved = true;
    }
    // forget logs that are gone (the game tidies old ones away)
    let before = cursors.len();
    cursors.retain(|k, _| files.iter().any(|f| &f.0 == k));
    moved |= cursors.len() != before;
    Some((found, moved))
}

/// What Shiny Bingo reads from the logs.
#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    /// One battle's wild Pokémon (two lines for a double), with the hour on
    /// the line's clock.
    Battle { mons: Vec<WildLine>, hour: u8 },
    /// "RARE sweet scent: evolve roll …, species X becomes Y".
    Evolve,
}

/// The hour at the start of a log line ("07:07:16 INFO …" → 7).
fn line_hour(line: &str) -> Option<u8> {
    let h: u8 = line.get(..2)?.parse().ok()?;
    (line.as_bytes().get(2) == Some(&b':') && h < 24).then_some(h)
}

/// Like `read_new_wild`, but grouped into battles (with the hour) plus
/// Sweet Scent evolutions, each with the session (log file) it came from.
/// A battle's lines still being written at the end of a file are left for
/// the next read, so a double is never split in two.
pub fn read_new_events(dir: &Path, cursors: &mut std::collections::BTreeMap<String, u64>) -> Option<(Vec<(String, LogEvent)>, bool)> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<(String, PathBuf, u64)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (n.starts_with("session-") && n.ends_with(".log")).then(|| Some((n, e.path(), e.metadata().ok()?.len())))?
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let first = cursors.is_empty();
    let mut found = Vec::new();
    let mut moved = false;
    for (name, path, len) in &files {
        let at = cursors.get(name).copied();
        let from = match at {
            None if first => *len,
            None => 0,
            Some(p) if p > *len => 0,
            Some(p) => p,
        };
        if from >= *len {
            if at != Some(*len) {
                cursors.insert(name.clone(), *len);
                moved = true;
            }
            continue;
        }
        let Ok(mut f) = File::open(path) else { continue };
        if f.seek(SeekFrom::Start(from)).is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if f.take(len - from).read_to_end(&mut buf).is_err() {
            continue;
        }
        let upto = buf.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
        // whole lines, keeping where each one starts (in bytes)
        let mut pos = 0usize;
        let mut battle: Vec<WildLine> = Vec::new();
        let mut battle_hour = 0u8;
        let mut battle_from = 0usize;
        for raw in buf[..upto].split_inclusive(|b| *b == b'\n') {
            let start = pos;
            pos += raw.len();
            let text = String::from_utf8_lossy(raw);
            let line = text.trim_end();
            if let Some(w) = parse_wild(line) {
                if battle.is_empty() {
                    battle_from = start;
                    battle_hour = line_hour(line).unwrap_or(12);
                }
                battle.push(w);
                continue;
            }
            if !battle.is_empty() {
                found.push((name.clone(), LogEvent::Battle { mons: std::mem::take(&mut battle), hour: battle_hour }));
            }
            if line.contains(" RARE ") && line.contains("evolve roll") && line.contains("becomes") {
                found.push((name.clone(), LogEvent::Evolve));
            }
        }
        // a battle still being written in the newest log: read it again next
        // time (an older log is finished, so it counts now)
        let newest = files.last().is_some_and(|f| &f.0 == name);
        let keep = if !battle.is_empty() && newest {
            battle_from
        } else {
            if !battle.is_empty() {
                found.push((name.clone(), LogEvent::Battle { mons: std::mem::take(&mut battle), hour: battle_hour }));
            }
            upto
        };
        cursors.insert(name.clone(), from + keep as u64);
        moved = true;
    }
    let before = cursors.len();
    cursors.retain(|k, _| files.iter().any(|f| &f.0 == k));
    moved |= cursors.len() != before;
    Some((found, moved))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hatches(text: &str) -> Vec<EggEvent> {
        let mut w = HatchWatch::default();
        text.lines().filter_map(|l| w.push(l.trim_end())).collect()
    }

    /// The real lines from a hatch where the nickname page was opened
    /// (game 0.4.1, 15 Nov 2026).
    const HATCH_NAMED: &str = "07:07:38 INFO  MAP      wide view: 35 wall tiles on filler block 586
07:07:39 INFO  DISPLAY  context app
07:07:39 INFO  DISPLAY  layout wide (held, black), context app, brightness 8010 8010
07:07:39 INFO  DISPLAY  scene wide 1 sides 0x8
07:07:39 INFO  DISPLAY  layout wide, context app, brightness 8010 8010
07:07:40 INFO  GEN5     EEVEE/0 ready in 0.4 ms (63 frames, 26 pictures, 49x49)
07:07:42 INFO  DISPLAY  scene wide 0 sides 0x0
07:07:42 INFO  DISPLAY  context field
07:07:42 INFO  NAMING   the naming page opens: EEVEE\u{2019}s nickname? (10 letters)
07:07:42 INFO  NAMING   music under the page: the song playing, turned down
07:07:44 INFO  NAMING   the name is \"\"
07:07:44 INFO  FIELD    effect pictures: room for 256 (field memory free 1031172, 368 each)
07:07:44 INFO  ORIGIN   1 Pokemon stamped with origin 1
07:07:45 INFO  UI       keys: a page";

    /// A hatch with "No" to the nickname (12 Nov 2026).
    const HATCH_UNNAMED: &str = "14:18:57 INFO  DISPLAY  context app
14:18:57 INFO  DISPLAY  scene wide 1 sides 0x8
14:18:57 INFO  DISPLAY  layout wide, context app, brightness 8010 8010
14:19:01 INFO  DISPLAY  scene wide 0 sides 0x0
14:19:01 INFO  DISPLAY  context field
14:19:01 INFO  FIELD    effect pictures: room for 256 (field memory free 1037316, 368 each)
14:19:01 INFO  ORIGIN   1 Pokemon stamped with origin 1
14:19:03 INFO  SAVE     autosaved";

    /// A catch: named mid-battle and stamped, but no hatch scene.
    const CATCH: &str = "07:14:04 INFO  WILD     shiny chance 2 in 8192 for species 280 at level 17
07:14:05 INFO  BATTLE   encounter effect took 671 ms
07:14:28 INFO  CATCH    caught battler 1 slot 0 species 280
07:14:30 INFO  CATCH    naming the Pokemon caught mid-battle (1 of 1)
07:14:30 INFO  NAMING   the naming page opens: RALTS\u{2019}s nickname? (10 letters)
07:14:48 INFO  ORIGIN   1 Pokemon stamped with origin 1";

    #[test]
    fn hatch_with_a_nickname_page_knows_the_species() {
        assert_eq!(hatches(HATCH_NAMED), vec![EggEvent::Hatched { species: Some("EEVEE".into()) }]);
    }

    #[test]
    fn hatch_without_a_nickname_still_counts() {
        assert_eq!(hatches(HATCH_UNNAMED), vec![EggEvent::Hatched { species: None }]);
    }

    #[test]
    fn catches_are_not_hatches() {
        assert!(hatches(CATCH).is_empty());
    }

    #[test]
    fn hatches_in_a_row_count_once_each() {
        let two = format!("{HATCH_UNNAMED}\n{HATCH_NAMED}");
        assert_eq!(hatches(&two).len(), 2);
        // a stamp long after a scene is something else
        let late = "10:00:00 INFO  DISPLAY  scene wide 1 sides 0x8\n10:05:00 INFO  ORIGIN   1 Pokemon stamped with origin 1";
        assert!(hatches(late).is_empty());
        // a battle in between calls it off
        let fight = "10:00:00 INFO  DISPLAY  scene wide 1 sides 0x8\n10:00:10 INFO  BATTLE   encounter effect took 600 ms\n10:00:30 INFO  ORIGIN   1 Pokemon stamped with origin 1";
        assert!(hatches(fight).is_empty());
    }

    #[test]
    fn hatch_species_from_the_picture_and_forms() {
        let s = "09:00:00 INFO  DISPLAY  scene wide 1 sides 0x8\n09:00:01 INFO  GEN5     RHYHORN_female/0 ready in 0.5 ms (76 frames)\n09:00:04 INFO  ORIGIN   1 Pokemon stamped with origin 1";
        assert_eq!(hatches(s), vec![EggEvent::Hatched { species: Some("RHYHORN".into()) }]);
    }


    /// The real 7-Ralts chain from 15 Nov 2026 (other lines left out).
    const RADAR_CHAIN: &str = "08:20:03 INFO  UI       bag page: use item 431 (request 20431)
08:20:05 INFO  WILD     shiny chance 2 in 8192 for species 315 at level 19
08:20:05 INFO  BATTLE   encounter effect took 167 ms
08:20:08 INFO  BATTLE   HUD fade 16 towards 0000
08:20:33 INFO  UI       bag page: use item 431 (request 20431)
08:20:36 INFO  WILD     shiny chance 2 in 8192 for species 280 at level 17
08:20:36 INFO  BATTLE   encounter effect took 169 ms
08:20:45 INFO  FAINT    battler 1 state 0
08:20:45 INFO  FAINT    battler 1 state 4
08:20:48 INFO  EXP      whole party's Exp. Share: foe gives 170; 1 fought get 170 each, the rest 85 each
08:20:48 INFO  BATTLE   HUD fade 2 towards 0000
08:20:48 INFO  BATTLE   HUD fade 16 towards 0000
08:20:54 INFO  WILD     shiny chance 2 in 8192 for species 280 at level 17
08:20:54 INFO  BATTLE   encounter effect took 167 ms
08:21:00 INFO  CATCH    caught battler 1 slot 0 species 280
08:21:04 INFO  BATTLE   HUD fade 16 towards 0000
08:22:37 INFO  WILD     shiny chance 2 in 8192 for species 400 at level 18
08:22:37 INFO  BATTLE   encounter effect took 167 ms
08:22:40 INFO  FAINT    battler 1 state 0
08:22:42 INFO  BATTLE   HUD fade 16 towards 0000
08:30:00 INFO  WILD     shiny chance 2 in 8192 for species 22 at level 49
08:30:00 INFO  WILD     shiny chance 2 in 8192 for species 75 at level 49
08:30:00 INFO  BATTLE   encounter effect took 177 ms
08:30:04 INFO  BATTLE   HUD fade 16 towards 0000
08:31:00 INFO  UI       Y wheel: used item 431 (1) once";

    #[test]
    fn radar_battles_and_how_they_ended() {
        let mut w = RadarWatch::default();
        let got: Vec<RadarEvent> = RADAR_CHAIN.lines().filter_map(|l| w.push(l)).collect();
        use BattleEnd::*;
        assert_eq!(
            got,
            vec![
                RadarEvent::Used,
                RadarEvent::Battle { species: 315, multi: false, end: Ran },
                RadarEvent::Used,
                RadarEvent::Battle { species: 280, multi: false, end: Knocked },
                RadarEvent::Battle { species: 280, multi: false, end: Caught },
                RadarEvent::Battle { species: 400, multi: false, end: Knocked },
                RadarEvent::Battle { species: 22, multi: true, end: Ran },
                RadarEvent::Used,
            ]
        );
        assert!(!is_radar_use("08:06:37 INFO  UI       Y wheel: used item 450 (1)"));
    }

    #[test]
    fn ball_throws() {
        assert_eq!(parse_ball_throw("12:50:10 INFO  CATCH    ball 15 from battler 2 at battler 1 (picked 1)"), Some(15));
        assert_eq!(parse_ball_throw("12:50:07 INFO  CATCH    battler 0 chose a ball (15): which wild Pokemon"), None);
        assert_eq!(parse_ball_throw("12:50:08 INFO  CAPSULE  burst: capsule 3, seals -1, ball 4"), None);
        assert_eq!(parse_lure("13:02:11 INFO  MESSAGE  field message: You used a Shiny Lure!"), Some(true));
        assert_eq!(parse_lure("13:31:40 INFO  MESSAGE  field message: The Shiny Lure wore off..."), Some(false));
        assert_eq!(parse_lure("13:31:40 INFO  MESSAGE  field message: You used a Repel!"), None);
    }

    #[test]
    fn egg_waiting_line() {
        assert!(is_egg_waiting("07:02:55 INFO  WISH     an Egg is waiting at the Day-Care"));
        assert!(!is_egg_waiting("07:02:55 INFO  UI       Y wheel: BICYCLE"));
    }

    /// Every hatch in the real logs (set PT_LOGS to a logs folder).
    #[test]
    #[ignore]
    fn real_logs_hatches() {
        let dir = std::env::var("PT_LOGS").unwrap();
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
        names.sort();
        for p in names {
            let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).to_string();
            let mut w = HatchWatch::default();
            for l in text.lines() {
                if is_egg_waiting(l) {
                    println!("{}  {}  egg waiting", p.file_name().unwrap().to_string_lossy(), &l[..8]);
                }
                if let Some(e) = w.push(l.trim_end()) {
                    println!("{}  {}  {:?}", p.file_name().unwrap().to_string_lossy(), &l[..8], e);
                }
            }
        }
    }
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
        let mut t = LogTail::new(std::env::temp_dir(), None);
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
    fn events_group_doubles_and_wait_for_a_battle_being_written() {
        let dir = std::env::temp_dir().join(format!("st-ev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("session-1.log");
        std::fs::write(&p, "06:00:00 INFO  DISPLAY  context field\n").unwrap();
        let mut cur = std::collections::BTreeMap::new();
        let (ev, _) = read_new_events(&dir, &mut cur).unwrap();
        assert!(ev.is_empty(), "the first read starts at the end");
        let mut f = std::fs::OpenOptions::new().append(true).open(&p).unwrap();
        write!(f, "{SAMPLE}").unwrap();
        // then a double still being written (no BATTLE line yet)
        writeln!(f, "21:10:00 INFO  WILD     shiny chance 4 in 8192 for species 399 at level 9 (grass)").unwrap();
        writeln!(f, "21:10:00 INFO  WILD     shiny chance 4 in 8192 for species 399 at level 9 (grass)").unwrap();
        let (ev, _) = read_new_events(&dir, &mut cur).unwrap();
        let battles: Vec<&LogEvent> = ev.iter().map(|e| &e.1).filter(|e| matches!(e, LogEvent::Battle { .. })).collect();
        assert_eq!(battles.len(), 2, "the half-written one waits");
        assert!(battles.iter().all(|b| matches!(b, LogEvent::Battle { mons, hour: 7 } if mons.len() == 2)));
        assert_eq!(ev.iter().filter(|e| e.1 == LogEvent::Evolve).count(), 1);
        writeln!(f, "07:07:32 INFO  BATTLE   encounter effect took 705 ms").unwrap();
        let (ev, _) = read_new_events(&dir, &mut cur).unwrap();
        assert!(matches!(&ev[..], [(_, LogEvent::Battle { mons, hour: 21 })] if mons.len() == 2 && mons[0].species == 399), "{ev:?}");
        let (ev, _) = read_new_events(&dir, &mut cur).unwrap();
        assert!(ev.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tails_new_lines_only() {
        let dir = std::env::temp_dir().join(format!("gamelog-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session-20261109-070656.log");
        // Old battle already in the log before the tracker starts: not counted.
        std::fs::write(&path, SAMPLE).unwrap();
        let mut t = LogTail::new(dir.clone(), None);
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
