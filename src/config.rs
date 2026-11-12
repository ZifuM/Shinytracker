use crate::hotkeys::KeyBinds;
use crate::imageproc::Region;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
/// grass doubles it (patch notes + the game's log: 4 in 8192).
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
            HuntMethod::RareGrassSweetScent => "Sweet Scent on rare grass",
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
                "1 in 2,048. Double chance on rare grass (patch notes, and confirmed by the game's log)."
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

/// Overlay layout: the original fixed Legacy overlay, or the Modular one
/// whose sections can each be turned on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OverlayStyle {
    #[default]
    Modular,
    Legacy,
}

/// Size preset for the modular overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OverlaySize {
    #[default]
    Full,
    Compact,
    Bar,
}

/// Sections of the modular overlay. What the original overlay had is on by
/// default; the newer extras start off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayModules {
    pub style: OverlayStyle,
    pub size: OverlaySize,
    /// Clicks go through to the game (toggle back with the hotkey).
    pub click_through: bool,
    // Header
    pub status_dot: bool,
    // Hunt
    pub target: bool,
    pub sprites: bool,
    /// Just the shiny sprite (one big box) instead of normal + shiny.
    pub single_sprite: bool,
    pub counter: bool,
    pub buttons: bool,
    // Odds & luck
    pub odds_bar: bool,
    pub method_line: bool,
    pub luck: bool,
    pub milestone: bool,
    // Session
    pub session: bool,
    pub sparkline: bool,
    pub last_seen: bool,
    pub phases: bool,
    // Progress
    pub xp: bool,
    pub daily_goal: bool,
    pub streak: bool,
    pub best: bool,
}

impl Default for OverlayModules {
    fn default() -> Self {
        Self {
            style: OverlayStyle::Modular,
            size: OverlaySize::Full,
            click_through: false,
            status_dot: true,
            target: true,
            sprites: true,
            single_sprite: false,
            counter: true,
            buttons: true,
            odds_bar: true,
            method_line: true,
            luck: false,
            milestone: false,
            session: false,
            sparkline: false,
            last_seen: false,
            phases: false,
            xp: false,
            daily_goal: false,
            streak: false,
            best: false,
        }
    }
}

impl OverlayModules {
    pub fn legacy(&self) -> bool {
        self.style == OverlayStyle::Legacy
    }
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
    /// The modern look for the Companion window (the default since 0.5).
    /// Off = the clean or classic look above.
    pub modern_ui: bool,
    /// Modern look: the sidebar shows icons only.
    pub nav_compact: bool,
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
            modern_ui: true,
            nav_compact: false,
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
    /// Third target onwards.
    #[serde(default)]
    pub more_targets: Vec<String>,
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
    /// Nickname and gender for each hunt's share image, by hunt.
    pub card_names: std::collections::HashMap<String, (String, crate::card::Gender)>,
    /// Where share images are saved (None = Pictures\Shiny Tracker).
    pub share_folder: Option<String>,
    /// Show a "Save as" box every time instead of saving straight away.
    pub share_ask: bool,
    /// Rare grass infographic in the original sea-blue look (else the anime one).
    pub poster_classic: bool,
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
    /// Switch between Rare grass and Soft resets by what the game log shows.
    pub auto_method: bool,
    /// Open the Shiny Hunter overlay as soon as the Companion starts.
    pub overlay_autostart: bool,
    /// Keep counting encounters while the overlay is stopped.
    pub count_in_background: bool,

    // Overlay window
    pub window_pos: Option<[f32; 2]>,
    /// Where the overlay was last, for each layout ("full", "compact",
    /// "bar", "legacy", "badge"): each one remembers its own spot.
    pub overlay_positions: std::collections::HashMap<String, [f32; 2]>,
    /// Optional second target (two-target hunts, e.g. Scyther and Scizor).
    pub target2: Option<String>,
    /// Third target onwards (several Pokémon hunted on one route).
    pub more_targets: Vec<String>,
    /// Gentle nudge to take a break after this long hunting in one session.
    pub break_reminder: bool,
    pub break_minutes: u32,
    /// "Stop at X encounters" goal for a session (0 = off).
    pub session_goal: u64,
    /// Check GitHub for new versions and download them.
    pub auto_update: bool,
    /// The release this copy was last updated to (stops a re-download loop
    /// if that release's .exe reports an older version number).
    pub installed_update: Option<String>,
    /// Settings window position / size (egui points) and maximized state,
    /// restored next time it opens.
    pub settings_pos: Option<[f32; 2]>,
    pub settings_size: Option<[f32; 2]>,
    pub settings_maximized: bool,
    pub pinned: bool,
    pub mini: bool,
    pub opacity: f32,
    /// Which overlay to show (Modular sections, or the Legacy layout) and
    /// what the modular one includes.
    pub overlay: OverlayModules,
    /// Keep the overlay visible over the game's borderless/fullscreen mode.
    pub borderless_fix: bool,
    /// Flash the overlay's border gold when a target is spotted.
    pub spot_flash: bool,
    /// Liquid layout: the overlay's width can be dragged, and its sections stretch.
    pub liquid: bool,
    pub liquid_width: f32,
    /// The game's folder, when chosen by hand in Advanced settings (None =
    /// the usual place). Counting from the game log and the Map both use it.
    /// (Older versions saved a log folder here as `log_dir`; that still
    /// works, the game's folder is found from it.)
    #[serde(alias = "log_dir")]
    pub game_dir: Option<String>,
    /// Where the running game was last found (used when no folder is chosen).
    pub game_dir_found: Option<String>,

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
    /// One-time switch to the modular overlay (original sections on, the
    /// new extras off).
    pub overlay_reset1: bool,
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
            card_names: Default::default(),
            share_folder: None,
            share_ask: false,
            poster_classic: false,
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
            auto_method: true,
            overlay_autostart: false,
            count_in_background: true,
            star_threshold: 60,
            window_pos: None,
            overlay_positions: Default::default(),
            target2: None,
            more_targets: Vec::new(),
            break_reminder: false,
            break_minutes: 90,
            session_goal: 0,
            auto_update: true,
            installed_update: None,
            settings_pos: None,
            settings_size: None,
            settings_maximized: false,
            overlay: OverlayModules::default(),
            pinned: false,
            mini: false,
            opacity: 0.94,
            borderless_fix: true,
            spot_flash: true,
            liquid: false,
            liquid_width: 280.0,
            game_dir: None,
            game_dir_found: None,
            history: Vec::new(),
            keybinds: KeyBinds::default(),
            welcome_done: false,
            star_alert_reset: false,
            star_alert_reset2: false,
            star_alert_on3: false,
            overlay_reset1: false,
        }
    }
}

// ───────────────────────────── data folder ─────────────────────────────
//
// Saves (config.json) normally live in %APPDATA%\ShinyTracker and downloads
// (sprites, font) in %LOCALAPPDATA%\ShinyTracker. The user can move both to a
// folder of their choice (Settings → Advanced → Data folder): everything then
// lives in <chosen>\ShinyTracker, with downloads in its "cache" subfolder. The
// choice is remembered in a tiny pointer file that always stays in the
// default folder, so the app can find the data again on the next start.

/// The default saves folder (%APPDATA%\ShinyTracker).
pub fn default_data_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShinyTracker")
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShinyTracker")
}

/// Name of the pointer file (in the default folder) holding a moved location.
const POINTER: &str = "data-location.txt";

/// The moved data folder, if the user moved it (read once, then kept here).
fn custom_dir() -> Option<PathBuf> {
    custom_cell().read().ok().and_then(|g| g.clone())
}

fn custom_cell() -> &'static std::sync::RwLock<Option<PathBuf>> {
    static CELL: std::sync::OnceLock<std::sync::RwLock<Option<PathBuf>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        let p = std::fs::read_to_string(default_data_dir().join(POINTER))
            .ok()
            .map(|t| PathBuf::from(t.trim()))
            .filter(|p| !p.as_os_str().is_empty() && p.is_dir());
        std::sync::RwLock::new(p)
    })
}

/// Where saves live now.
pub fn data_dir() -> PathBuf {
    custom_dir().unwrap_or_else(default_data_dir)
}

/// Where downloads (sprites, font) live now.
pub fn cache_dir() -> PathBuf {
    match custom_dir() {
        Some(d) => d.join("cache"),
        None => default_cache_dir(),
    }
}

/// Has the user moved the data folder?
pub fn data_dir_moved() -> bool {
    custom_dir().is_some()
}

/// Move all saves and downloads to `<target>\ShinyTracker` (or back to the
/// default folders with `None`). Copies everything first and only removes
/// the old copies once that worked. Save the config right after.
pub fn move_data_dir(target: Option<PathBuf>) -> Result<PathBuf, String> {
    let (old_data, old_cache) = (data_dir(), cache_dir());
    let (new_data, new_cache) = match &target {
        Some(t) => {
            let d = if t.file_name().is_some_and(|n| n.eq_ignore_ascii_case("ShinyTracker")) {
                t.clone()
            } else {
                t.join("ShinyTracker")
            };
            let c = d.join("cache");
            (d, c)
        }
        None => (default_data_dir(), default_cache_dir()),
    };
    move_between(&old_data, &old_cache, &new_data, &new_cache)?;
    // Remember the choice.
    let pointer = default_data_dir().join(POINTER);
    match &target {
        Some(_) => {
            let _ = std::fs::create_dir_all(default_data_dir());
            std::fs::write(&pointer, new_data.display().to_string())
                .map_err(|e| format!("Moved, but couldn't remember the new place ({e})"))?;
        }
        None => {
            let _ = std::fs::remove_file(&pointer);
        }
    }
    if let Ok(mut g) = custom_cell().write() {
        *g = target.map(|_| new_data.clone());
    }
    Ok(new_data)
}

/// Copy saves + downloads from the old folders to the new ones, then delete
/// the old copies. (Separate from the pointer handling so it can be tested.)
fn move_between(old_data: &Path, old_cache: &Path, new_data: &Path, new_cache: &Path) -> Result<(), String> {
    let same = |a: &Path, b: &Path| {
        let n = |p: &Path| p.display().to_string().to_lowercase().trim_end_matches(['\\', '/']).to_string();
        n(a) == n(b)
    };
    if same(old_data, new_data) {
        return Err("Your data is already in that folder".into());
    }
    let inside = |a: &Path, b: &Path| {
        let (a, b) = (a.display().to_string().to_lowercase(), b.display().to_string().to_lowercase());
        a.starts_with(&format!("{}\\", b.trim_end_matches('\\'))) || a.starts_with(&format!("{}/", b.trim_end_matches('/')))
    };
    if inside(new_data, old_data) || inside(old_data, new_data) {
        return Err("Pick a folder outside the current data folder".into());
    }
    std::fs::create_dir_all(new_data).map_err(|e| format!("Couldn't create the folder ({e})"))?;
    let probe = new_data.join(".write-test");
    std::fs::write(&probe, b"ok").map_err(|_| "Can't save in that folder. Pick one in your own files".to_string())?;
    let _ = std::fs::remove_file(&probe);

    // 1. copy (saves: everything except the pointer and an inner cache folder)
    let mut copied: Vec<PathBuf> = Vec::new();
    copy_tree(old_data, new_data, &mut copied, &|rel| rel == Path::new(POINTER) || rel.starts_with("cache"))?;
    if !same(old_cache, new_cache) && old_cache.is_dir() {
        copy_tree(old_cache, new_cache, &mut copied, &|_| false)?;
    }
    // 2. only now remove the old copies (files we copied, then empty folders)
    for f in &copied {
        let _ = std::fs::remove_file(f);
    }
    for dir in [old_cache, old_data] {
        remove_empty_dirs(dir);
    }
    Ok(())
}

/// Copy every file under `from` into `to`, skipping relative paths `skip`
/// says to. Records each copied source file in `copied`.
fn copy_tree(from: &Path, to: &Path, copied: &mut Vec<PathBuf>, skip: &dyn Fn(&Path) -> bool) -> Result<(), String> {
    fn walk(root: &Path, dir: &Path, to: &Path, copied: &mut Vec<PathBuf>, skip: &dyn Fn(&Path) -> bool) -> Result<(), String> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Ok(()) };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
            if skip(&rel) {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, to, copied, skip)?;
            } else {
                let dest = to.join(&rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't copy {} ({e})", rel.display()))?;
                }
                std::fs::copy(&p, &dest).map_err(|e| format!("Couldn't copy {} ({e})", rel.display()))?;
                copied.push(p);
            }
        }
        Ok(())
    }
    walk(from, from, to, copied, skip)
}

/// Remove folders under (and including) `dir` that are now empty.
fn remove_empty_dirs(dir: &Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                remove_empty_dirs(&e.path());
            }
        }
    }
    let _ = std::fs::remove_dir(dir); // only succeeds when empty
}

#[cfg(test)]
mod data_dir_tests {
    use super::*;

    #[test]
    fn moves_saves_and_cache_then_cleans_up() {
        let base = std::env::temp_dir().join(format!("st-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (od, oc) = (base.join("roaming/ShinyTracker"), base.join("local/ShinyTracker"));
        std::fs::create_dir_all(oc.join("sprites/bw")).unwrap();
        std::fs::create_dir_all(&od).unwrap();
        std::fs::write(od.join("config.json"), "{}").unwrap();
        std::fs::write(od.join(POINTER), "x").unwrap();
        std::fs::write(oc.join("sprites/bw/abra.gif"), "gif").unwrap();
        let nd = base.join("new/ShinyTracker");
        move_between(&od, &oc, &nd, &nd.join("cache")).unwrap();
        assert!(nd.join("config.json").is_file());
        assert!(nd.join("cache/sprites/bw/abra.gif").is_file());
        assert!(!nd.join(POINTER).exists());
        assert!(!od.join("config.json").exists());
        assert!(od.join(POINTER).exists(), "pointer stays in the default folder");
        assert!(!oc.exists(), "old cache removed");
        // and back again
        move_between(&nd, &nd.join("cache"), &od, &oc).unwrap();
        assert!(od.join("config.json").is_file());
        assert!(oc.join("sprites/bw/abra.gif").is_file());
        assert!(!nd.exists());
        // refusals
        assert!(move_between(&od, &oc, &od, &oc).is_err());
        assert!(move_between(&od, &oc, &od.join("sub"), &oc).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
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

    /// The game folder to look in: the one chosen in Advanced settings, else
    /// where the running game was last found (None = the usual place).
    pub fn game_dir_setting(&self) -> Option<PathBuf> {
        // (A found folder that's gone again is skipped until the game is seen.)
        let found = self.game_dir_found.as_ref().filter(|d| Path::new(d.as_str()).is_dir());
        self.game_dir.as_ref().or(found).map(PathBuf::from)
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
