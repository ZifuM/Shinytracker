//! Pokégotchi: a pocket pet made from a shiny you caught. It has four needs
//! that go down slowly, a mood that follows them, and friendship that grows
//! with care. Hunting (read from the game's own log) earns berries to feed
//! it. It never runs away or faints: left alone it gets hungry, sleepy or
//! grumpy, and a bit of care brings it right back.
//!
//! Saved to its own file (pokegotchi.json) with the usual safe saving.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The most pets in the Box (the partner included).
pub const MAX_BOX: usize = 6;
/// Berries and treats for the very first pet.
pub const START_BERRIES: u32 = 10;
pub const START_TREATS: u32 = 2;
/// Free berries each new day you open the app.
pub const DAILY_BERRIES: u32 = 3;
/// Encounters for a berry, and rare grass encounters for a treat.
pub const ENC_PER_BERRY: u32 = 10;
pub const RARE_PER_TREAT: u32 = 10;
/// Friendship points for each level.
pub const PER_LEVEL: u32 = 30;
/// Rare grass encounters for the Rare Grass frame, and the friendship level
/// (any pet) for Distortion.
pub const RARE_FOR_GRASS: u32 = 100;
pub const LEVEL_FOR_DISTORTION: u32 = 5;
/// While the app is closed needs drop at a quarter speed and never below this.
pub const OFFLINE_FLOOR: f32 = 20.0;

/// The four needs, in the order they're kept and shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Full,
    Happy,
    Energy,
    Clean,
}

impl Need {
    pub const ALL: [Need; 4] = [Need::Full, Need::Happy, Need::Energy, Need::Clean];
    pub fn name(self) -> &'static str {
        match self {
            Need::Full => "Full",
            Need::Happy => "Happy",
            Need::Energy => "Energy",
            Need::Clean => "Clean",
        }
    }
    /// How much it drops each hour while the app is open.
    pub fn per_hour(self) -> f32 {
        match self {
            Need::Full => 6.0,
            Need::Happy => 5.0,
            Need::Energy => 4.0,
            Need::Clean => 3.0,
        }
    }
}

/// The device's look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frame {
    #[default]
    Pearl,
    Distortion,
    Grass,
}

impl Frame {
    pub const ALL: [Frame; 3] = [Frame::Pearl, Frame::Distortion, Frame::Grass];
    pub fn name(self) -> &'static str {
        match self {
            Frame::Pearl => "Sinnoh",
            Frame::Distortion => "Distortion",
            Frame::Grass => "Rare Grass",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Frame::Pearl => "A pearly shell. More colours come from the battle pass.",
            Frame::Distortion => "Black and gold, with a red glow.",
            Frame::Grass => "Leafy green, with gold sparkles.",
        }
    }
    pub fn unlock(self) -> &'static str {
        match self {
            Frame::Pearl => "Yours from the start",
            Frame::Distortion => "Reach friendship level 5 with any pet",
            Frame::Grass => "Meet 100 Pokémon on rare grass",
        }
    }
}

/// The Pearl frame's colour (each has its own name).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tint {
    #[default]
    Pink,
    Sky,
    Mint,
    /// (Saved as "Peach" for a short while.)
    #[serde(alias = "Peach")]
    Platinum,
    Midnight,
}

impl Tint {
    pub const ALL: [Tint; 5] = [Tint::Pink, Tint::Sky, Tint::Platinum, Tint::Mint, Tint::Midnight];
    pub fn name(self) -> &'static str {
        match self {
            Tint::Pink => "Sinnoh Pearl",
            Tint::Sky => "Sinnoh Diamond",
            Tint::Platinum => "Sinnoh Platinum",
            Tint::Mint => "Sinnoh Emerald",
            Tint::Midnight => "Sinnoh Onyx",
        }
    }
    pub fn colour(self) -> &'static str {
        match self {
            Tint::Pink => "pink",
            Tint::Sky => "blue",
            Tint::Platinum => "white",
            Tint::Mint => "green",
            Tint::Midnight => "black",
        }
    }
}

/// How the pet shows on the overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OvStyle {
    /// The whole device, small.
    #[default]
    Device,
    /// Just your shiny, floating on its own.
    Perch,
    /// A slim strip with its needs.
    Strip,
}

impl OvStyle {
    pub fn name(self) -> &'static str {
        match self {
            OvStyle::Device => "Mini device",
            OvStyle::Perch => "Just your shiny",
            OvStyle::Strip => "Needs strip",
        }
    }
}

/// How big the Pokégotchi is on the overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OvScale {
    #[default]
    Small,
    Medium,
    Large,
}

impl OvScale {
    pub fn name(self) -> &'static str {
        match self {
            OvScale::Small => "Small",
            OvScale::Medium => "Medium",
            OvScale::Large => "Large",
        }
    }
    /// How much bigger than small (whole steps for the pixel sprite alone,
    /// so it stays sharp).
    pub fn factor(self) -> f32 {
        match self {
            OvScale::Small => 1.0,
            OvScale::Medium => 1.5,
            OvScale::Large => 2.0,
        }
    }
    pub fn pixels(self) -> f32 {
        match self {
            OvScale::Small => 1.0,
            OvScale::Medium => 2.0,
            OvScale::Large => 3.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OvCfg {
    /// Picked in the Start overlay picker.
    pub show: bool,
    pub style: OvStyle,
    /// No longer used (it was pinned under the shiny overlay).
    pub right: bool,
    /// Where its window was left on screen (None: the default spot).
    pub pos: Option<[f32; 2]>,
    pub scale: OvScale,
}

impl Default for OvCfg {
    fn default() -> Self {
        OvCfg { show: true, style: OvStyle::Device, right: true, pos: None, scale: OvScale::Small }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pet {
    /// The species' slug (the same as the Shiny Dex).
    pub slug: String,
    /// Its nickname ("" = the species' name).
    pub nick: String,
    /// When it was adopted (unix seconds).
    pub adopted: u64,
    /// Full, Happy, Energy, Clean: 0 to 100.
    pub needs: [f32; 4],
    /// Friendship points (`PER_LEVEL` a level).
    pub friend: u32,
    /// The last time a pat counted for friendship.
    pub last_pat: u64,
}

impl Default for Pet {
    fn default() -> Self {
        Pet { slug: String::new(), nick: String::new(), adopted: 0, needs: [80.0; 4], friend: 0, last_pat: 0 }
    }
}

impl Pet {
    pub fn need(&self, n: Need) -> f32 {
        self.needs[n as usize]
    }
    fn add(&mut self, n: Need, by: f32) {
        let v = &mut self.needs[n as usize];
        *v = (*v + by).clamp(0.0, 100.0);
    }
    pub fn level(&self) -> u32 {
        1 + self.friend / PER_LEVEL
    }
    /// How far into the current level (0..1).
    pub fn level_frac(&self) -> f32 {
        (self.friend % PER_LEVEL) as f32 / PER_LEVEL as f32
    }
    pub fn mood(&self, asleep: bool) -> Mood {
        if asleep {
            return Mood::Asleep;
        }
        let low = self.needs.iter().filter(|v| **v < 30.0).count();
        if low >= 2 {
            return Mood::Grumpy;
        }
        let [full, happy, energy, clean] = self.needs;
        if full < 30.0 {
            Mood::Hungry
        } else if energy < 25.0 {
            Mood::Sleepy
        } else if clean < 30.0 {
            Mood::Messy
        } else if happy < 35.0 {
            Mood::Bored
        } else if self.needs.iter().all(|v| *v >= 60.0) {
            Mood::Happy
        } else {
            Mood::Fine
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Happy,
    Fine,
    Hungry,
    Sleepy,
    Messy,
    Bored,
    Grumpy,
    Asleep,
}

impl Mood {
    pub fn name(self) -> &'static str {
        match self {
            Mood::Happy => "Happy",
            Mood::Fine => "Doing fine",
            Mood::Hungry => "Hungry",
            Mood::Sleepy => "Sleepy",
            Mood::Messy => "Wants a clean",
            Mood::Bored => "Bored",
            Mood::Grumpy => "Grumpy",
            Mood::Asleep => "Asleep",
        }
    }
    /// What it says in its speech bubble.
    pub fn says(self) -> &'static str {
        match self {
            Mood::Happy => "♪",
            Mood::Fine => "…",
            Mood::Hungry => "hungry…",
            Mood::Sleepy => "yawn…",
            Mood::Messy => "icky…",
            Mood::Bored => "play?",
            Mood::Grumpy => "hmph!",
            Mood::Asleep => "zzz",
        }
    }
    /// What would help.
    pub fn hint(self) -> &'static str {
        match self {
            Mood::Happy => "All good. Hunting earns more berries.",
            Mood::Fine => "Doing fine. A game or a pat would cheer it up.",
            Mood::Hungry => "Feed it a berry.",
            Mood::Sleepy => "Turn the lights off for a nap.",
            Mood::Messy => "Give it a clean.",
            Mood::Bored => "Play a game with it.",
            Mood::Grumpy => "A few things are low. Check its needs.",
            Mood::Asleep => "Sleeping. It wakes up when it's rested, or turn the lights on.",
        }
    }
}

/// What a journal entry is about (its dot's colour and icon).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// Something you did together (feeding, games…).
    #[default]
    Care,
    /// Berries and treats from your hunting.
    Hunt,
    /// What your pet was thinking.
    Thought,
    /// What it got up to while you were away.
    Away,
    /// A little moment of its own.
    Fun,
    /// Something big: a shiny, a new level, a frame unlocked, a new pet.
    Big,
}

/// A line in the journal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    /// When (unix seconds).
    pub at: u64,
    pub kind: Kind,
    pub text: String,
    /// Hunting entries keep a running count (berries, treats) so a hunt
    /// makes one line, not dozens.
    pub n: (u32, u32),
}

/// The most journal lines kept.
pub const JOURNAL_MAX: usize = 300;

/// What came of a care action, for the speech bubble.
#[derive(Clone, Debug, PartialEq)]
pub enum Care {
    Done(&'static str),
    No(&'static str),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GotchiFile {
    pub version: u32,
    pub pets: Vec<Pet>,
    /// Which pet is out (the others rest in the Box).
    pub partner: usize,
    /// When the needs were last brought up to date (unix seconds).
    pub last: u64,
    /// The lights are off: it sleeps, and its needs don't drop.
    pub asleep: bool,
    pub berries: u32,
    pub treats: u32,
    /// Encounters toward the next berry, rare grass ones toward the next treat.
    pub enc_part: u32,
    pub rare_part: u32,
    /// Rare grass encounters met since the first pet.
    pub rare_met: u32,
    /// Encounters met since the first pet.
    pub met: u64,
    pub frame: Frame,
    /// The Pearl frame's colour.
    pub tint: Tint,
    pub unlocked: Vec<Frame>,
    pub overlay: OvCfg,
    /// How far each game session log has been read (file name → bytes).
    pub logs: BTreeMap<String, u64>,
    /// The last local day the app was open (for the daily berries).
    pub day: i64,
    /// The first pet's starter berries were given.
    pub started: bool,
    /// Games played, and the best score.
    pub games: u32,
    pub best: u32,
    /// Today: (day, berries earned, treats earned, games played).
    pub today: (i64, u32, u32, u32),
    /// Shinies already celebrated (so Undo can't repeat one).
    pub shiny_keys: Vec<String>,
    /// The journal, oldest first.
    pub journal: Vec<Entry>,
}

impl GotchiFile {
    /// Add a line to the journal.
    pub fn note(&mut self, at: u64, kind: Kind, text: impl Into<String>) {
        // kept in time order (lines about time away are written up later)
        let i = self.journal.partition_point(|e| e.at <= at);
        self.journal.insert(i, Entry { at, kind, text: text.into(), n: (0, 0) });
        if self.journal.len() > JOURNAL_MAX {
            let extra = self.journal.len() - JOURNAL_MAX;
            self.journal.drain(..extra);
        }
    }

    /// Hunting brought berries and treats: one line per stretch of hunting
    /// (it keeps adding up for 20 minutes).
    pub fn note_hunt(&mut self, at: u64, berries: u32, treats: u32) {
        if berries == 0 && treats == 0 {
            return;
        }
        let mut n = (berries, treats);
        if let Some(last) = self.journal.last() {
            if last.kind == Kind::Hunt && at.saturating_sub(last.at) < 20 * 60 {
                n = (last.n.0 + berries, last.n.1 + treats);
                self.journal.pop();
            }
        }
        let mut parts = Vec::new();
        if n.0 > 0 {
            parts.push(format!("{} berr{}", n.0, if n.0 == 1 { "y" } else { "ies" }));
        }
        if n.1 > 0 {
            parts.push(format!("{} treat{}", n.1, if n.1 == 1 { "" } else { "s" }));
        }
        self.journal.push(Entry { at, kind: Kind::Hunt, text: format!("Your hunting brought home {}.", parts.join(" and ")), n });
    }

    pub fn has_data(&self) -> bool {
        !self.pets.is_empty() || self.started
    }

    pub fn partner(&self) -> Option<&Pet> {
        self.pets.get(self.partner)
    }
    pub fn partner_mut(&mut self) -> Option<&mut Pet> {
        self.pets.get_mut(self.partner)
    }

    pub fn is_unlocked(&self, f: Frame) -> bool {
        f == Frame::Pearl || self.unlocked.contains(&f)
    }

    /// Bring the partner's needs up to `now`. `open`: the app was open the
    /// whole time (full speed); else it was closed (a quarter speed, and
    /// nothing drops below `OFFLINE_FLOOR`). Pets in the Box rest.
    pub fn decay(&mut self, now: u64, open: bool) {
        if self.last == 0 || now <= self.last {
            self.last = self.last.max(now);
            return;
        }
        let hours = (now - self.last) as f32 / 3600.0;
        self.last = now;
        let asleep = self.asleep;
        let Some(p) = self.pets.get_mut(self.partner) else { return };
        if asleep {
            p.add(Need::Energy, 15.0 * hours);
            if p.need(Need::Energy) >= 100.0 {
                self.asleep = false;
            }
            return;
        }
        let rate = if open { 1.0 } else { 0.25 };
        for n in Need::ALL {
            let was = p.need(n);
            let mut v = (was - n.per_hour() * rate * hours).max(0.0);
            if !open {
                v = v.max(was.min(OFFLINE_FLOOR));
            }
            p.needs[n as usize] = v;
        }
    }

    /// A new day: free berries, and friendship for another day together.
    /// Returns true on a new day.
    pub fn new_day(&mut self, day: i64) -> bool {
        if day == self.day {
            return false;
        }
        let first = self.day == 0;
        self.day = day;
        if first || self.pets.is_empty() {
            return false;
        }
        self.berries += DAILY_BERRIES;
        if let Some(p) = self.partner_mut() {
            p.friend += 5;
        }
        self.check_unlocks();
        true
    }

    fn today_mut(&mut self, day: i64) -> &mut (i64, u32, u32, u32) {
        if self.today.0 != day {
            self.today = (day, 0, 0, 0);
        }
        &mut self.today
    }

    /// Adopt a caught shiny. It becomes the partner.
    pub fn adopt(&mut self, slug: &str, now: u64) -> Result<(), &'static str> {
        if self.pets.iter().any(|p| p.slug == slug) {
            return Err("It's already in your Box.");
        }
        if self.pets.len() >= MAX_BOX {
            return Err("The Box is full. Send one back first.");
        }
        self.pets.push(Pet { slug: slug.to_string(), adopted: now, ..Default::default() });
        self.partner = self.pets.len() - 1;
        self.asleep = false;
        if self.last == 0 {
            self.last = now;
        }
        if !self.started {
            self.started = true;
            self.berries += START_BERRIES;
            self.treats += START_TREATS;
        }
        Ok(())
    }

    /// Bring a pet out of the Box.
    pub fn swap(&mut self, i: usize) {
        if i < self.pets.len() && i != self.partner {
            self.partner = i;
            self.asleep = false;
        }
    }

    /// Send a pet back (it's still your shiny; only the Pokégotchi goes).
    pub fn release(&mut self, i: usize) {
        if i >= self.pets.len() {
            return;
        }
        self.pets.remove(i);
        if self.partner > i || self.partner >= self.pets.len() {
            self.partner = self.partner.saturating_sub(1);
        }
        if self.pets.is_empty() {
            self.asleep = false;
        }
    }

    pub fn feed(&mut self, treat: bool) -> Care {
        if self.asleep {
            return Care::No("It's asleep.");
        }
        if treat && self.treats == 0 {
            return Care::No("No treats. Rare grass drops them.");
        }
        if !treat && self.berries == 0 {
            return Care::No("No berries. Hunting earns them.");
        }
        let Some(p) = self.pets.get_mut(self.partner) else { return Care::No("") };
        if p.need(Need::Full) >= 98.0 {
            return Care::No("It's full!");
        }
        let helped = p.need(Need::Full) < 90.0;
        if treat {
            self.treats -= 1;
            p.add(Need::Full, 40.0);
            p.add(Need::Happy, 10.0);
        } else {
            self.berries -= 1;
            p.add(Need::Full, 20.0);
        }
        if helped {
            p.friend += 2;
        }
        self.check_unlocks();
        Care::Done(if treat { "Yum! A treat!" } else { "Munch munch" })
    }

    pub fn clean(&mut self) -> Care {
        if self.asleep {
            return Care::No("It's asleep.");
        }
        let Some(p) = self.pets.get_mut(self.partner) else { return Care::No("") };
        if p.need(Need::Clean) >= 98.0 {
            return Care::No("Already sparkling.");
        }
        if p.need(Need::Clean) < 70.0 {
            p.friend += 2;
        }
        p.needs[Need::Clean as usize] = 100.0;
        self.check_unlocks();
        Care::Done("Sparkly clean!")
    }

    /// A pat: a little happiness; friendship at most every ten minutes.
    pub fn pat(&mut self, now: u64) -> Care {
        if self.asleep {
            return Care::No("Shh, it's asleep.");
        }
        let Some(p) = self.pets.get_mut(self.partner) else { return Care::No("") };
        p.add(Need::Happy, 5.0);
        if now >= p.last_pat + 600 {
            p.last_pat = now;
            p.friend += 1;
        }
        self.check_unlocks();
        Care::Done("♥")
    }

    /// Lights off (it sleeps) or on.
    pub fn lights(&mut self) -> Care {
        if self.pets.is_empty() {
            return Care::No("");
        }
        self.asleep = !self.asleep;
        Care::Done(if self.asleep { "Good night" } else { "Good morning!" })
    }

    /// Can it play right now?
    pub fn can_play(&self) -> Result<(), &'static str> {
        let Some(p) = self.partner() else { return Err("") };
        if self.asleep {
            Err("It's asleep.")
        } else if p.need(Need::Energy) < 10.0 {
            Err("Too tired to play. Lights off for a nap?")
        } else {
            Ok(())
        }
    }

    /// A game finished with `score` out of `HiLo::ROUNDS`.
    pub fn played(&mut self, score: u32, day: i64) {
        self.games += 1;
        self.best = self.best.max(score);
        self.today_mut(day).3 += 1;
        if let Some(p) = self.pets.get_mut(self.partner) {
            p.add(Need::Happy, 10.0 + 5.0 * score as f32);
            p.add(Need::Energy, -8.0);
            p.friend += 1 + score / 2;
        }
        self.check_unlocks();
    }

    /// Wild Pokémon met while hunting: berries and treats. Returns what was
    /// earned (berries, treats).
    pub fn met(&mut self, n: u32, rare: u32, day: i64) -> (u32, u32) {
        if self.pets.is_empty() {
            return (0, 0);
        }
        self.met += n as u64;
        self.enc_part += n;
        let b = self.enc_part / ENC_PER_BERRY;
        self.enc_part %= ENC_PER_BERRY;
        self.rare_met += rare;
        self.rare_part += rare;
        let t = self.rare_part / RARE_PER_TREAT;
        self.rare_part %= RARE_PER_TREAT;
        self.berries += b;
        self.treats += t;
        let today = self.today_mut(day);
        today.1 += b;
        today.2 += t;
        self.check_unlocks();
        (b, t)
    }

    /// A shiny was found: the partner is thrilled.
    pub fn shiny_found(&mut self) {
        if let Some(p) = self.pets.get_mut(self.partner) {
            p.add(Need::Happy, 50.0);
            p.friend += 10;
        }
        self.check_unlocks();
    }

    /// Unlock frames that have been earned. Returns the newly unlocked ones.
    pub fn check_unlocks(&mut self) -> Vec<Frame> {
        let mut new = Vec::new();
        if !self.unlocked.contains(&Frame::Distortion) && self.pets.iter().any(|p| p.level() >= LEVEL_FOR_DISTORTION) {
            new.push(Frame::Distortion);
        }
        if !self.unlocked.contains(&Frame::Grass) && self.rare_met >= RARE_FOR_GRASS {
            new.push(Frame::Grass);
        }
        self.unlocked.extend(new.iter().copied());
        new
    }
}

/// Higher or Lower: a number shows; guess if the next is higher or lower.
#[derive(Clone, Debug)]
pub struct HiLo {
    pub shown: u32,
    pub next: u32,
    pub round: u32,
    pub score: u32,
    /// The last guess was right (None before the first).
    pub last: Option<bool>,
    seed: u64,
}

impl HiLo {
    pub const ROUNDS: u32 = 5;

    pub fn new(seed: u64) -> HiLo {
        let mut g = HiLo { shown: 0, next: 0, round: 0, score: 0, last: None, seed: seed | 1 };
        g.shown = g.roll(0);
        g.next = g.roll(g.shown);
        g
    }

    fn roll(&mut self, not: u32) -> u32 {
        loop {
            // xorshift
            let mut x = self.seed;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.seed = x;
            let v = (x % 9) as u32 + 1;
            if v != not {
                return v;
            }
        }
    }

    pub fn done(&self) -> bool {
        self.round >= Self::ROUNDS
    }

    /// Guess the next one is higher (or lower). Returns if it was right.
    pub fn guess(&mut self, higher: bool) -> bool {
        if self.done() {
            return false;
        }
        let right = (self.next > self.shown) == higher;
        if right {
            self.score += 1;
        }
        self.last = Some(right);
        self.round += 1;
        self.shown = self.next;
        self.next = self.roll(self.shown);
        right
    }
}

pub fn load_from(dir: &Path) -> (GotchiFile, Option<String>) {
    crate::safefile::load(dir, "pokegotchi", "Pokégotchi")
}

pub fn save_to(dir: &Path, f: &GotchiFile, first_this_run: bool) -> Result<(), String> {
    crate::safefile::save(dir, "pokegotchi", f, first_this_run, GotchiFile::has_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_pet() -> GotchiFile {
        let mut f = GotchiFile::default();
        f.adopt("eevee", 1000).unwrap();
        f
    }

    #[test]
    fn adopting_gives_starters_once_and_box_is_capped() {
        let mut f = with_pet();
        assert_eq!((f.berries, f.treats), (START_BERRIES, START_TREATS));
        assert_eq!(f.adopt("eevee", 1001), Err("It's already in your Box."));
        for s in ["a", "b", "c", "d", "e"] {
            f.adopt(s, 1002).unwrap();
        }
        assert_eq!(f.pets.len(), MAX_BOX);
        assert!(f.adopt("f", 1003).is_err());
        assert_eq!(f.partner, MAX_BOX - 1);
        assert_eq!(f.berries, START_BERRIES);
        f.release(0);
        assert_eq!(f.partner, MAX_BOX - 2);
        assert_eq!(f.partner().unwrap().slug, "e");
    }

    #[test]
    fn needs_drop_open_and_gently_while_closed() {
        let mut f = with_pet();
        f.decay(1000 + 3600 * 5, true);
        let p = f.partner().unwrap();
        assert!((p.need(Need::Full) - 50.0).abs() < 0.01);
        assert!((p.need(Need::Clean) - 65.0).abs() < 0.01);
        // a week away: a quarter speed, never below the floor
        let mut f = with_pet();
        f.decay(1000 + 3600 * 24 * 7, false);
        for n in Need::ALL {
            assert_eq!(f.partner().unwrap().need(n), OFFLINE_FLOOR);
        }
        // already below the floor: it doesn't go back up, or further down
        f.partner_mut().unwrap().needs[0] = 10.0;
        f.decay(1000 + 3600 * 24 * 8, false);
        assert_eq!(f.partner().unwrap().need(Need::Full), 10.0);
        // two hours closed: half of an open hour's drop
        let mut f = with_pet();
        f.decay(1000 + 7200, false);
        assert!((f.partner().unwrap().need(Need::Full) - 77.0).abs() < 0.01);
    }

    #[test]
    fn sleep_restores_energy_and_wakes_up() {
        let mut f = with_pet();
        f.partner_mut().unwrap().needs[2] = 70.0;
        f.lights();
        assert!(f.asleep);
        assert_eq!(f.feed(false), Care::No("It's asleep."));
        f.decay(1000 + 3600, true);
        let p = f.partner().unwrap();
        assert!((p.need(Need::Energy) - 85.0).abs() < 0.01);
        assert_eq!(p.need(Need::Full), 80.0);
        f.decay(1000 + 3 * 3600, true);
        assert!(!f.asleep);
    }

    #[test]
    fn care_helps_and_builds_friendship() {
        let mut f = with_pet();
        assert_eq!(f.feed(false), Care::Done("Munch munch"));
        assert_eq!(f.berries, START_BERRIES - 1);
        assert_eq!(f.partner().unwrap().friend, 2);
        assert_eq!(f.partner().unwrap().need(Need::Full), 100.0);
        assert_eq!(f.feed(false), Care::No("It's full!"));
        f.partner_mut().unwrap().needs[3] = 40.0;
        f.clean();
        assert_eq!(f.partner().unwrap().need(Need::Clean), 100.0);
        f.pat(5000);
        f.pat(5001);
        assert_eq!(f.partner().unwrap().friend, 5);
        assert_eq!(f.partner().unwrap().mood(false), Mood::Happy);
        f.partner_mut().unwrap().needs = [20.0, 80.0, 80.0, 20.0];
        assert_eq!(f.partner().unwrap().mood(false), Mood::Grumpy);
        f.partner_mut().unwrap().needs = [20.0, 80.0, 80.0, 80.0];
        assert_eq!(f.partner().unwrap().mood(false), Mood::Hungry);
    }

    #[test]
    fn hunting_earns_berries_treats_and_unlocks() {
        let mut f = GotchiFile::default();
        assert_eq!(f.met(50, 50, 1), (0, 0), "nothing before the first pet");
        let mut f = with_pet();
        assert_eq!(f.met(9, 0, 1), (0, 0));
        assert_eq!(f.met(12, 12, 1), (2, 1));
        assert_eq!(f.today, (1, 2, 1, 0));
        assert!(!f.is_unlocked(Frame::Grass));
        f.met(88, 88, 2);
        assert!(f.is_unlocked(Frame::Grass));
        assert!(!f.is_unlocked(Frame::Distortion));
        f.partner_mut().unwrap().friend = PER_LEVEL * 4 - 1;
        f.pat(10_000);
        assert!(f.is_unlocked(Frame::Distortion));
    }

    #[test]
    fn new_days_give_berries_but_not_the_first() {
        let mut f = with_pet();
        assert!(!f.new_day(100));
        assert!(!f.new_day(100));
        assert!(f.new_day(101));
        assert_eq!(f.berries, START_BERRIES + DAILY_BERRIES);
        assert_eq!(f.partner().unwrap().friend, 5);
    }

    #[test]
    fn higher_or_lower_never_ties_and_ends() {
        let mut g = HiLo::new(42);
        let mut right = 0;
        while !g.done() {
            assert_ne!(g.shown, g.next);
            assert!((1..=9).contains(&g.shown));
            let higher = g.next > g.shown;
            right += g.guess(higher) as u32;
        }
        assert_eq!((right, g.score, g.round), (5, 5, 5));
        assert!(!g.guess(true));
        let mut f = with_pet();
        f.played(5, 3);
        assert_eq!((f.games, f.best, f.today.3), (1, 5, 1));
        assert_eq!(f.partner().unwrap().need(Need::Happy), 100.0);
    }

    #[test]
    fn hunting_lines_add_up_and_the_journal_is_capped() {
        let mut f = with_pet();
        f.note_hunt(100, 2, 0);
        f.note_hunt(400, 1, 1);
        assert_eq!(f.journal.len(), 1);
        assert_eq!(f.journal[0].text, "Your hunting brought home 3 berries and 1 treat.");
        f.note_hunt(100 + 3600, 1, 0);
        assert_eq!(f.journal.len(), 2);
        for i in 0..(JOURNAL_MAX + 20) {
            f.note(5000 + i as u64, Kind::Thought, "hm");
        }
        assert_eq!(f.journal.len(), JOURNAL_MAX);
        assert_eq!(f.journal.last().unwrap().at, 5000 + (JOURNAL_MAX + 19) as u64);
        // written up later, it still goes in its place
        f.note(5001, Kind::Away, "earlier");
        assert!(f.journal.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn old_and_empty_files_load() {
        let f: GotchiFile = serde_json::from_str("{}").unwrap();
        assert!(f.overlay.show && f.overlay.right);
        assert!(!f.has_data());
        let p: Pet = serde_json::from_str(r#"{"slug":"eevee"}"#).unwrap();
        let f: GotchiFile = serde_json::from_str(r#"{"tint":"Peach"}"#).unwrap();
        assert_eq!(f.tint, Tint::Platinum);
        assert_eq!(p.needs, [80.0; 4]);
    }
}
