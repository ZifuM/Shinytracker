use crate::hotkeys::KeyBinds;
use crate::imageproc::Region;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::card::CardOptions;
pub use crate::stats::RunStats;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HuntRecord {
    /// Display name.
    pub target: String,
    pub encounters: u64,
    pub battles: u64,
    /// When it was caught.
    pub when_unix: u64,
    // Added later (older records won't have these):
    pub slug: Option<String>,
    pub started_unix: u64,
    pub target_seen: u64,
    /// "Rare grass · Doubles"
    pub method: String,
    pub odds_one_in: Option<f64>,
    pub stats: Option<RunStats>,
    /// A shiny of another Pokémon found during a hunt (the hunt kept going).
    pub phase: bool,
    /// The run this came from (0 = unknown).
    pub run_id: u64,
}

/// How the shiny is being hunted; decides the odds. Rates are for
/// Platinum Together: modern 1 in 4,096 base rate, and Sweet Scent on rare
/// (sparkling) grass doubles it (patch notes + the game's log: 4 in 8192).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HuntMethod {
    FullOdds,
    SoftReset,
    SweetScent,
    RareGrassSweetScent,
    PokeRadar,
    Masuda,
    FromGameLog,
    Custom,
}

pub const BASE_ONE_IN: f64 = 4096.0;

impl HuntMethod {
    pub const ALL: [HuntMethod; 8] = [
        HuntMethod::RareGrassSweetScent,
        HuntMethod::SweetScent,
        HuntMethod::FullOdds,
        HuntMethod::SoftReset,
        HuntMethod::PokeRadar,
        HuntMethod::Masuda,
        HuntMethod::FromGameLog,
        HuntMethod::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HuntMethod::FullOdds => "Full odds (walking / random encounters)",
            HuntMethod::SoftReset => "Soft resets (starter, legendary, gift)",
            HuntMethod::SweetScent => "Sweet Scent (normal grass)",
            HuntMethod::RareGrassSweetScent => "Sweet Scent on rare (sparkling) grass",
            HuntMethod::PokeRadar => "Poké Radar chaining",
            HuntMethod::Masuda => "Masuda Method (breeding)",
            HuntMethod::FromGameLog => "Exact odds from the game log",
            HuntMethod::Custom => "Something else (enter the odds)",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            HuntMethod::FullOdds => "1 in 4,096. Platinum Together uses the modern base rate.",
            HuntMethod::SoftReset => "1 in 4,096 per reset (the base rate; not separately confirmed).",
            HuntMethod::SweetScent => "1 in 4,096. Only rare grass gets the double chance.",
            HuntMethod::RareGrassSweetScent => {
                "1 in 2,048. Double chance on sparkling grass (patch notes, and confirmed by the game's log)."
            }
            HuntMethod::PokeRadar => {
                "Gen 4 chain curve, never worse than the base rate: about 1 in 200 at chain 40. Not confirmed for this mod."
            }
            HuntMethod::Masuda => {
                "Gen 4 gives eggs 5 chances at the base rate: about 1 in 820. Not confirmed for this mod."
            }
            HuntMethod::FromGameLog => {
                "Uses the exact odds the game writes for each wild encounter (includes any bonuses). Updates after your first battle."
            }
            HuntMethod::Custom => "Type the odds below.",
        }
    }

    /// Whether Shiny Charms add chances on top (the log/custom values already include everything).
    pub fn uses_charms(self) -> bool {
        !matches!(self, HuntMethod::FromGameLog | HuntMethod::Custom)
    }

    /// Short name for the overlay.
    pub fn short(self) -> &'static str {
        match self {
            HuntMethod::FullOdds => "Full odds",
            HuntMethod::SoftReset => "Soft resets",
            HuntMethod::SweetScent => "Sweet Scent",
            HuntMethod::RareGrassSweetScent => "Rare grass",
            HuntMethod::PokeRadar => "Poké Radar",
            HuntMethod::Masuda => "Masuda",
            HuntMethod::FromGameLog => "Game log odds",
            HuntMethod::Custom => "Custom odds",
        }
    }

    /// Whether the hunt is wild battles, where singles/doubles changes the
    /// chance per battle. Soft resets and eggs are always one at a time.
    pub fn is_wild_battles(self) -> bool {
        !matches!(self, HuntMethod::SoftReset | HuntMethod::Masuda)
    }
}

/// A daily goal or weekly challenge that was completed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GoalDone {
    /// "day:20767" or "week:2967:enc3k"
    pub key: String,
    pub xp: u64,
    pub unix: u64,
}

/// Sound effect switches and volume.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundSettings {
    pub enabled: bool,
    pub volume: f32,
    pub achievement: bool,
    pub level_up: bool,
    pub milestone: bool,
    pub caught: bool,
    pub goal: bool,
    /// Shiny star spotted on screen.
    pub shiny: bool,
    /// "Alert song": play the user's own song file instead of the shiny
    /// sound once a hunt is long enough. Off by default.
    pub alert_song: bool,
    pub alert_song_path: String,
    /// Only for shinies spotted at this many encounters or more.
    pub alert_song_after: u64,
    /// The user's own sound files, by effect (`Sfx::key`). Missing = built-in sound.
    pub custom: std::collections::BTreeMap<String, String>,
    /// Output device name ("" = Windows default).
    pub device: String,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.5,
            achievement: true,
            level_up: true,
            milestone: true,
            caught: true,
            goal: true,
            shiny: true,
            alert_song: false,
            alert_song_path: String::new(),
            alert_song_after: 3000,
            custom: Default::default(),
            device: String::new(),
        }
    }
}

impl SoundSettings {
    pub fn on(&self, s: crate::sound::Sfx) -> bool {
        use crate::sound::Sfx;
        self.enabled
            && match s {
                Sfx::Achievement => self.achievement,
                Sfx::LevelUp => self.level_up,
                Sfx::Milestone => self.milestone,
                Sfx::Caught => self.caught,
                Sfx::Goal => self.goal,
                Sfx::Shiny => self.shiny,
            }
    }
    /// The user's own file for this effect, if they picked one.
    pub fn custom_for(&self, s: crate::sound::Sfx) -> Option<&str> {
        self.custom.get(s.key()).map(|p| p.as_str()).filter(|p| !p.is_empty())
    }
    pub fn flag(&mut self, s: crate::sound::Sfx) -> &mut bool {
        use crate::sound::Sfx;
        match s {
            Sfx::Achievement => &mut self.achievement,
            Sfx::LevelUp => &mut self.level_up,
            Sfx::Milestone => &mut self.milestone,
            Sfx::Caught => &mut self.caught,
            Sfx::Goal => &mut self.goal,
            Sfx::Shiny => &mut self.shiny,
        }
    }
}

/// An achievement that's been earned.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Unlocked {
    pub id: String,
    pub unix: u64,
}

/// Overall brightness of the app.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UiMode {
    Light,
    SemiDark,
    Dark,
}

/// How the app looks: mode plus an accent colour, optionally a gradient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub mode: UiMode,
    pub accent: [u8; 3],
    pub gradient: bool,
    pub accent2: [u8; 3],
    /// Buttons get their own colour (otherwise they follow the header).
    pub separate_buttons: bool,
    pub button: [u8; 3],
    pub button_gradient: bool,
    pub button2: [u8; 3],
    /// Flat "clean" look for the settings window (neutral greys, pill
    /// buttons, system font); the default. Off = the classic look.
    pub clean_settings: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: UiMode::Dark,
            accent: [214, 52, 58],
            gradient: false,
            accent2: [240, 120, 60],
            separate_buttons: false,
            button: [52, 116, 218],
            button_gradient: false,
            button2: [128, 78, 208],
            clean_settings: true,
        }
    }
}

/// Singles or doubles: how many Pokémon each battle has.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BattleType {
    Doubles,
    Singles,
    Auto,
}

/// Where encounters are counted from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CountSource {
    /// Read the enemy names on screen (OCR).
    Screen,
    /// Read the game's own session log.
    GameLog,
}

/// A paused hunt that can be reopened later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRun {
    pub id: u64,
    pub target: Option<String>,
    pub count: u64,
    pub battles: u64,
    pub target_seen: u64,
    pub started_unix: u64,
    pub saved_unix: u64,
    #[serde(default)]
    pub stats: RunStats,
    /// Second target of a two-target hunt.
    #[serde(default)]
    pub target2: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // Current hunt
    pub count: u64,
    pub battles: u64,
    pub target_seen: u64,
    /// Target species slug, e.g. "riolu".
    pub target: Option<String>,
    /// Odds denominator for HuntMethod::Custom ("1 in N").
    pub odds: u32,
    pub hunt_method: HuntMethod,
    pub battle_type: BattleType,
    pub count_source: CountSource,
    /// Last shiny chance seen in the game log (numerator, denominator).
    pub last_log_odds: Option<(u32, u32)>,
    /// Current Poké Radar chain length (odds stop improving at 40).
    pub radar_chain: u32,
    /// Shiny Charms in the bag: each gives one more chance to be shiny.
    pub shiny_charms: u32,
    pub paused: bool,
    /// Id of the run currently being counted (0 = not assigned yet).
    pub run_id: u64,
    pub run_started_unix: u64,
    /// Runs put aside when switching targets.
    pub saved_runs: Vec<SavedRun>,
    /// Time/day/session stats for the current run.
    pub run_stats: RunStats,
    /// Pause counting while the game isn't running.
    pub auto_pause_game: bool,
    /// Little celebrations at 1,000s and at 1×/2×/3× the odds.
    pub celebrate_milestones: bool,
    /// Odds bar counts only encounters with the target.
    pub odds_target_only: bool,
    /// Pause the hunt timer after `idle_minutes` with no encounter.
    pub idle_pause: bool,
    pub idle_minutes: u32,
    /// Last-used share image options.
    pub card: CardOptions,
    /// Where share images are saved (None = Pictures\Shiny Tracker).
    pub share_folder: Option<String>,
    /// Show a "Save as" box every time instead of saving straight away.
    pub share_ask: bool,
    /// Light/dark mode and accent colours.
    pub appearance: Appearance,
    /// Trainer profile name (also used on share images).
    pub trainer_name: String,
    /// Achievements unlocked, with when.
    pub achievements: Vec<Unlocked>,
    /// One-off things that happened, for fun achievements.
    pub events: Vec<String>,
    /// Last level the trainer was told about (0 = progress not set up yet).
    pub level_seen: u32,
    /// Achievement / level-up pop-ups on the overlay.
    pub toasts: bool,
    /// Small level badge in the overlay header.
    pub level_on_overlay: bool,
    /// Shinies marked by hand in the Shiny Dex (caught before using the app).
    pub dex_manual: Vec<String>,
    /// Daily encounter goal.
    pub daily_goal: u64,
    /// Daily goals and weekly challenges completed (for XP and streaks).
    pub goals_done: Vec<GoalDone>,
    pub sounds: SoundSettings,
    /// Today's hunting time on the overlay.
    pub show_session_timer: bool,
    /// Trainer card image: include the shiny showcase / recent achievements.
    pub profile_card_shinies: bool,
    pub profile_card_achievements: bool,

    // Screen reading
    pub monitor: usize,
    pub name_regions: Vec<Region>,
    pub stable_reads: u32,
    pub rearm_reads: u32,
    pub interval_ms: u64,
    pub shiny_star_alert: bool,
    pub star_threshold: u32,

    // Overlay window
    pub window_pos: Option<[f32; 2]>,
    /// Optional second target (two-target hunts, e.g. Scyther and Scizor).
    pub target2: Option<String>,
    /// Gentle nudge to take a break after this long hunting in one session.
    pub break_reminder: bool,
    pub break_minutes: u32,
    /// "Stop at X encounters" goal for a session (0 = off).
    pub session_goal: u64,
    /// Check GitHub for new versions and download them.
    pub auto_update: bool,
    /// Settings window position / size (egui points) and maximized state,
    /// restored next time it opens.
    pub settings_pos: Option<[f32; 2]>,
    pub settings_size: Option<[f32; 2]>,
    pub settings_maximized: bool,
    pub pinned: bool,
    pub mini: bool,
    pub opacity: f32,
    /// Keep the overlay visible over the game's borderless/fullscreen mode.
    pub borderless_fix: bool,

    pub history: Vec<HuntRecord>,
    pub keybinds: KeyBinds,
    /// First-run welcome finished (existing users are migrated to true).
    pub welcome_done: bool,
    /// One-time switch-off of the experimental shiny star alert.
    pub star_alert_reset: bool,
    /// Second switch-off, when the stricter ★ detector arrived.
    pub star_alert_reset2: bool,
    /// One-time switch back on of the shiny star alert (now on by default).
    pub star_alert_on3: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            count: 0,
            battles: 0,
            target_seen: 0,
            target: None,
            odds: 8192,
            hunt_method: HuntMethod::RareGrassSweetScent,
            battle_type: BattleType::Doubles,
            count_source: CountSource::GameLog,
            last_log_odds: None,
            radar_chain: 40,
            shiny_charms: 0,
            paused: false,
            run_id: 0,
            run_started_unix: 0,
            saved_runs: Vec::new(),
            run_stats: RunStats::default(),
            auto_pause_game: true,
            celebrate_milestones: true,
            odds_target_only: false,
            idle_pause: true,
            idle_minutes: 5,
            card: CardOptions::default(),
            share_folder: None,
            share_ask: false,
            appearance: Appearance::default(),
            trainer_name: String::new(),
            achievements: Vec::new(),
            events: Vec::new(),
            level_seen: 0,
            toasts: true,
            level_on_overlay: true,
            dex_manual: Vec::new(),
            daily_goal: 500,
            goals_done: Vec::new(),
            sounds: SoundSettings::default(),
            show_session_timer: true,
            profile_card_shinies: true,
            profile_card_achievements: true,
            monitor: 0,
            name_regions: Vec::new(),
            stable_reads: 2,
            rearm_reads: 3,
            interval_ms: 200,
            shiny_star_alert: true,
            star_threshold: 60,
            window_pos: None,
            target2: None,
            break_reminder: false,
            break_minutes: 90,
            session_goal: 0,
            auto_update: true,
            settings_pos: None,
            settings_size: None,
            settings_maximized: false,
            pinned: false,
            mini: false,
            opacity: 0.94,
            borderless_fix: true,
            history: Vec::new(),
            keybinds: KeyBinds::default(),
            welcome_done: false,
            star_alert_reset: false,
            star_alert_reset2: false,
            star_alert_on3: false,
        }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShinyTracker")
}

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShinyTracker")
}

impl Config {
    /// Longest wait between encounters still counted as hunting time.
    pub fn idle_gap_secs(&self) -> u64 {
        if self.idle_pause {
            self.idle_minutes.clamp(1, 60) as u64 * 60
        } else {
            crate::stats::SESSION_GAP
        }
    }

    /// Shiny odds per Pokémon as "1 in N" (N can be fractional), if known.
    pub fn odds_one_in(&self) -> Option<f64> {
        // Chance of at least one success in `rolls` tries at 1-in-`one_in`.
        let rolls = |one_in: f64, rolls: f64| 1.0 / (1.0 - (1.0 - 1.0 / one_in).powf(rolls));
        let charms = if self.hunt_method.uses_charms() {
            self.shiny_charms as f64
        } else {
            0.0
        };
        let base = match self.hunt_method {
            HuntMethod::FullOdds | HuntMethod::SoftReset | HuntMethod::SweetScent => BASE_ONE_IN,
            HuntMethod::RareGrassSweetScent => BASE_ONE_IN / 2.0,
            HuntMethod::Masuda => rolls(BASE_ONE_IN, 5.0),
            // Gen 4 radar: chance per patch = (65535 / (8200 - chain*200)) / 65536,
            // chain capped at 40; never worse than the base rate.
            HuntMethod::PokeRadar => {
                let c = self.radar_chain.min(40) as f64;
                ((8200.0 - c * 200.0) * 65536.0 / 65535.0).min(BASE_ONE_IN)
            }
            HuntMethod::FromGameLog => {
                let (n, d) = self.last_log_odds?;
                if n == 0 || d == 0 {
                    return None;
                }
                d as f64 / n as f64
            }
            HuntMethod::Custom => self.odds.max(1) as f64,
        };
        Some(if charms > 0.0 {
            rolls(base, 1.0 + charms)
        } else {
            base
        })
    }

    /// Pokémon per battle: 2 for doubles, 1 for singles (and for soft resets
    /// and eggs), or the measured average for "Not sure" once there are battles.
    pub fn pokemon_per_battle(&self) -> Option<f64> {
        if !self.hunt_method.is_wild_battles() {
            return Some(1.0);
        }
        match self.battle_type {
            BattleType::Doubles => Some(2.0),
            BattleType::Singles => Some(1.0),
            BattleType::Auto => (self.battles > 0 && self.count > 0)
                .then(|| (self.count as f64 / self.battles as f64).clamp(1.0, 2.0)),
        }
    }

    /// Chance of at least one shiny per battle, as "1 in N".
    pub fn per_battle_one_in(&self) -> Option<f64> {
        let one_in = self.odds_one_in()?;
        let n = self.pokemon_per_battle()?;
        Some(1.0 / (1.0 - (1.0 - 1.0 / one_in).powf(n)))
    }

    /// "Rare grass · Doubles" style summary of what the odds are based on.
    pub fn odds_caption(&self) -> String {
        let mut s = self.hunt_method.short().to_string();
        if self.hunt_method == HuntMethod::PokeRadar {
            s.push_str(&format!(" (chain {})", self.radar_chain.min(40)));
        }
        if self.hunt_method.uses_charms() && self.shiny_charms > 0 {
            s.push_str(" + Charm");
        }
        if self.hunt_method.is_wild_battles() {
            s.push_str(match self.battle_type {
                BattleType::Doubles => " · Doubles",
                BattleType::Singles => " · Singles",
                BattleType::Auto => " · Auto",
            });
        }
        s
    }

    fn path() -> PathBuf {
        data_dir().join("config.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            // Write then rename, so a crash mid-write never loses the count.
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
