use crate::config::{now_unix, BattleType, Config, CountSource, HuntRecord, SavedRun};
use crate::detect::{NameMatcher, PerBattle, TrackerConfig};
use crate::fonts::FontLoader;
use crate::hotkeys::{HotAction, Hotkeys};
use crate::imageproc::{prepare_names, Gray, Region, Rgba};
use crate::species::{self, Species};
use crate::sprites::SpriteStore;
use crate::theme::{self, outlined_text, GOLD, MUTED, OUTLINE, RED};
use crate::worker::{self, Status, WorkerMsg, WorkerSettings};
use crate::{capture, ocr};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod extras;
mod hunt_pages;
mod insights;
mod notices;
mod settings;
mod trainer;
mod ui_kit;

pub const FULL_SIZE: [f32; 2] = [280.0, 344.0];
pub const MINI_SIZE: [f32; 2] = [84.0, 96.0];

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Profile,
    Dex,
    Hunt,
    Counting,
    Runs,
    Trophies,
    Overlay,
    Appearance,
    Hotkeys,
    Advanced,
}

struct Shot {
    rgba: Rgba,
    tex: egui::TextureHandle,
}

struct TestRead {
    processed: Option<egui::TextureHandle>,
    processed_size: Vec2,
    text: Result<String, String>,
    names: Vec<String>,
    yellow: Vec<u32>,
}

struct Setup {
    tab: Tab,
    search: String,
    monitors: usize,
    shot: Option<Shot>,
    shot_error: Option<String>,
    drag_from: Option<Pos2>,
    drag_to: Option<Pos2>,
    zoom_fit: bool,
    test: Option<TestRead>,
    confirm_reset: bool,
    confirm_delete_run: Option<u64>,
    capture_at: Option<Instant>,
    recording: Option<HotAction>,
    bind_msg: Option<(bool, String)>,
    picking_target: bool,
    /// Picking the optional second target.
    picking_target2: bool,
    /// Backup "Save as" (true) / "Open" (false) box while it's open.
    backup_picker: Option<(bool, Receiver<Option<std::path::PathBuf>>)>,
    /// A backup read from disk, waiting for "Replace everything?".
    restore_pending: Option<(std::path::PathBuf, Box<Config>)>,
    /// Last backup/restore result (ok?, message).
    backup_msg: Option<(bool, String)>,
    show_all_methods: bool,
    welcome_step: usize,
    /// Runs page: showing the stats of this run.
    view_run: Option<RunRef>,
    /// Run stats chart: per day / 6 hours / hour.
    chart_view: hunt_pages::ChartView,
    /// Which tab of a run's stats page is open.
    run_tab: hunt_pages::RunTab,
    /// "Choose…" box for a sound file while it's open: which sound it's for
    /// (None = the alert song) and where the answer arrives.
    song_picker: Option<(Option<crate::sound::Sfx>, Receiver<Option<std::path::PathBuf>>)>,
    /// Share image maker, when open.
    share: Option<ShareView>,
    /// Short message on the trophies page (time, ok, text).
    toast: Option<(Instant, bool, String)>,
    ach_filter: trainer::AchFilter,
    /// Selected achievement category tab.
    ach_tab: usize,
    /// Trainer card image maker, when open.
    profile_card: Option<trainer::ProfileCardView>,
    /// Profile page tab.
    profile_section: trainer::ProfileSection,
    dex: extras::DexView,
    /// Hunt journal text being typed.
    note_draft: String,
    /// Picking which Pokémon a phase shiny was.
    phase_pick: bool,
    /// Series whose steps are shown expanded.
    ach_open: Option<String>,
}

/// Which run a stats page / share image is about.
#[derive(Clone, Copy, PartialEq, Debug)]
enum RunRef {
    Current,
    Saved(u64),
    /// Index into the finished-hunt history.
    Finished(usize),
}

struct ShareView {
    src: RunRef,
    tex: Option<egui::TextureHandle>,
    image: Option<crate::canvas::Canvas>,
    /// Options and sprite availability the preview was drawn with.
    drawn: Option<(crate::card::CardOptions, bool, bool)>,
    msg: Option<(bool, String)>,
    saved: Option<std::path::PathBuf>,
    back_to: Option<RunRef>,
    /// Animated version: job in progress, finished GIF and its preview frames.
    gif_job: Option<(crate::share::GifJob, (crate::card::CardOptions, bool, bool))>,
    gif: Option<GifReady>,
    changed_at: Instant,
    /// A folder / "Save as" box that's open.
    picker: Option<(hunt_pages::Pick, std::sync::mpsc::Receiver<Option<std::path::PathBuf>>)>,
}

struct GifReady {
    key: (crate::card::CardOptions, bool, bool),
    bytes: Vec<u8>,
    frames: Vec<(egui::TextureHandle, u32)>,
    saved: Option<std::path::PathBuf>,
}

pub struct App {
    cfg: Config,
    dirty: bool,
    last_save: Instant,
    started: Instant,
    species: Arc<Vec<Species>>,
    shared: Arc<Mutex<WorkerSettings>>,
    rx: Receiver<WorkerMsg>,
    sprites: SpriteStore,
    fonts: FontLoader,
    hotkeys: Hotkeys,
    status: Status,
    last_battle: Vec<usize>,
    battle_target_hits: u64,
    /// Pokémon already tallied for the current battle (screen reading can
    /// report names late).
    battle_species: Vec<usize>,
    target_flash: Option<Instant>,
    shiny_alert: Option<usize>,
    confirm_caught: bool,
    show_setup: bool,
    setup: Setup,
    applied_size: Option<[f32; 2]>,
    icon_tex: egui::TextureHandle,
    topmost_started: bool,
    setup_geom: Option<(egui::Pos2, egui::Vec2)>,
    /// Open the settings window maximized (restored from last time).
    setup_max: bool,
    /// Window position/size changed: save without flashing "✔ Saved".
    geom_dirty: bool,
    saved_flash: Option<Instant>,
    borderless_fix: Arc<std::sync::atomic::AtomicBool>,
    /// Is the game running? (None until first checked.)
    game_open: Option<bool>,
    game_check: Instant,
    /// Milestone celebration on the overlay.
    celebrate: Option<(Instant, String)>,
    /// Fonts for drawing share images (loaded when first needed).
    card_text: Option<crate::share::AbText>,
    applied_appearance: Option<crate::config::Appearance>,
    settings_rounded: bool,
    /// Encounters added by the last battle (for the undo hotkey).
    last_added: Option<u64>,
    /// Overlay notices (break, goal, stuck, update); the first is shown.
    notices: Vec<notices::Notice>,
    /// Session (by start time) already nudged for a break / goal.
    break_nudged: u64,
    goal_nudged: u64,
    /// Anything counted since the app started (for the "stuck?" hint).
    counted_since_start: bool,
    stuck_shown: bool,
    game_open_since: Option<Instant>,
    /// Auto-update state shared with the download thread.
    updater: crate::update::Shared,
    last_update_check: Option<Instant>,
    update_dismissed: bool,
    /// Everything as it was just before Caught!, for a few seconds of Undo.
    undo_caught: Option<(Instant, Box<Config>)>,
    /// Achievement / level-up pop-ups waiting to show.
    toasts: std::collections::VecDeque<trainer::Toast>,
    progress_checked: Instant,
}

fn per_battle_for(b: BattleType) -> PerBattle {
    match b {
        BattleType::Doubles => PerBattle::Fixed(2),
        BattleType::Singles => PerBattle::Fixed(1),
        BattleType::Auto => PerBattle::Auto,
    }
}

/// "2,048" style formatting for odds.
fn fmt_one_in(x: f64) -> String {
    theme::thousands(x.round().max(1.0) as u64)
}

fn worker_settings(cfg: &Config, generation: u64, game_closed: bool) -> WorkerSettings {
    WorkerSettings {
        paused: cfg.paused || game_closed,
        monitor: cfg.monitor,
        name_regions: cfg.name_regions.clone(),
        tracker: TrackerConfig {
            per_battle: per_battle_for(cfg.battle_type),
            stable_reads: cfg.stable_reads.max(1),
            rearm_reads: cfg.rearm_reads.max(1),
            ..TrackerConfig::default()
        },
        interval_ms: cfg.interval_ms,
        shiny_star_alert: cfg.shiny_star_alert,
        star_threshold: cfg.star_threshold.max(1),
        source: cfg.count_source,
        generation,
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: Config) -> Self {
        let ctx = &cc.egui_ctx;
        theme::apply(ctx);
        let fonts = FontLoader::start(ctx);
        let species = Arc::new(species::all());
        let mut cfg = cfg;
        // (Before the worker starts, so it gets the right setting.)
        // The shiny star alert is on by default. It was switched off for
        // everyone in an earlier version, so turn it back on once; after that
        // the user's own choice in Settings > Counting stands.
        if !cfg.star_alert_on3 {
            cfg.shiny_star_alert = true;
            cfg.star_alert_reset = true;
            cfg.star_alert_reset2 = true;
            cfg.star_alert_on3 = true;
        }
        let shared = Arc::new(Mutex::new(worker_settings(&cfg, 0, false)));
        let (tx, rx) = channel();
        worker::spawn(shared.clone(), tx, ctx.clone(), species.clone());
        // Existing users (anything already set up) skip the welcome.
        if !cfg.welcome_done
            && (cfg.target.is_some() || cfg.count > 0 || !cfg.name_regions.is_empty())
        {
            cfg.welcome_done = true;
        }
        let show_setup = !cfg.welcome_done
            || (cfg.count_source == CountSource::Screen && cfg.name_regions.is_empty());
        if cfg.run_id == 0 {
            cfg.run_id = new_run_id();
            cfg.run_started_unix = now_unix();
        }
        // Start stats for runs from before stats existed (no catch-up fireworks).
        let odds = cfg.odds_one_in();
        let count = cfg.count;
        cfg.run_stats.ensure_started(now_unix(), count, odds);
        for r in &mut cfg.saved_runs {
            r.stats.ensure_started(r.saved_unix.max(1), r.count, odds);
        }
        theme::set_appearance(ctx, &cfg.appearance);
        Self {
            dirty: false,
            last_save: Instant::now(),
            started: Instant::now(),
            species,
            shared,
            rx,
            sprites: SpriteStore::new(ctx),
            fonts,
            hotkeys: Hotkeys::new(&cfg.keybinds),
            status: Status::default(),
            last_battle: Vec::new(),
            battle_target_hits: 0,
            battle_species: Vec::new(),
            target_flash: None,
            shiny_alert: None,
            confirm_caught: false,
            show_setup,
            setup: Setup {
                tab: Tab::Hunt,
                search: String::new(),
                monitors: 0,
                shot: None,
                shot_error: None,
                drag_from: None,
                drag_to: None,
                zoom_fit: true,
                test: None,
                confirm_reset: false,
                confirm_delete_run: None,
                capture_at: None,
                recording: None,
                bind_msg: None,
                picking_target: false,
                picking_target2: false,
                backup_picker: None,
                restore_pending: None,
                backup_msg: None,
                show_all_methods: false,
                welcome_step: 0,
                view_run: None,
                chart_view: Default::default(),
                run_tab: Default::default(),
                song_picker: None,
                share: None,
                toast: None,
                ach_filter: trainer::AchFilter::All,
                ach_tab: 0,
                profile_card: None,
                profile_section: Default::default(),
                dex: extras::DexView::default(),
                note_draft: String::new(),
                phase_pick: false,
                ach_open: None,
            },
            applied_size: None,
            icon_tex: {
                let (px, w, h) = crate::icon::rgba();
                ctx.load_texture(
                    "app-icon",
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &px),
                    egui::TextureOptions::LINEAR,
                )
            },
            topmost_started: false,
            setup_geom: None,
            setup_max: false,
            geom_dirty: false,
            saved_flash: None,
            borderless_fix: Arc::new(std::sync::atomic::AtomicBool::new(cfg.borderless_fix)),
            game_open: None,
            game_check: Instant::now() - Duration::from_secs(10),
            celebrate: None,
            card_text: None,
            applied_appearance: Some(cfg.appearance.clone()),
            settings_rounded: false,
            last_added: None,
            notices: Vec::new(),
            break_nudged: 0,
            goal_nudged: 0,
            counted_since_start: false,
            stuck_shown: false,
            game_open_since: None,
            updater: crate::update::new_shared(),
            last_update_check: None,
            update_dismissed: false,
            undo_caught: None,
            toasts: std::collections::VecDeque::new(),
            progress_checked: Instant::now() - Duration::from_secs(60),
            cfg,
        }
    }

    fn t_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    fn target_idx(&self) -> Option<usize> {
        let slug = self.cfg.target.as_deref()?;
        self.species.iter().position(|s| s.slug == slug)
    }

    /// The optional second target of a two-target hunt.
    fn target2_idx(&self) -> Option<usize> {
        let slug = self.cfg.target2.as_deref()?;
        self.species.iter().position(|s| s.slug == slug).filter(|i| Some(*i) != self.target_idx())
    }

    /// Send current settings to the detector. `restart` clears its state.
    fn push_settings(&mut self, restart: bool) {
        if let Ok(mut s) = self.shared.lock() {
            let gen = if restart {
                s.generation + 1
            } else {
                s.generation
            };
            *s = worker_settings(&self.cfg, gen, self.game_closed());
        }
        self.dirty = true;
    }

    fn set_paused(&mut self, paused: bool) {
        self.cfg.paused = paused;
        self.push_settings(true);
    }

    fn add(&mut self, delta: i64) {
        let before = self.cfg.count;
        self.cfg.count = (self.cfg.count as i64 + delta).max(0) as u64;
        // what "undo last encounter" takes back
        self.last_added = (self.cfg.count > before).then(|| self.cfg.count - before);
        if self.cfg.count > before {
            self.counted_since_start = true;
            self.note_encounters(self.cfg.count - before);
        } else if self.cfg.count < before {
            let day = crate::stats::local_day(now_unix());
            self.cfg.run_stats.unrecord(day, before - self.cfg.count);
        }
        self.dirty = true;
    }

    /// Paused because the game isn't running (and auto-pause is on).
    fn game_closed(&self) -> bool {
        self.cfg.auto_pause_game && self.game_open == Some(false)
    }

    /// Check every couple of seconds whether the game is running.
    fn watch_game(&mut self) {
        if self.game_check.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.game_check = Instant::now();
        let open = crate::topmost::game_running();
        if self.game_open != Some(open) {
            let was_closed = self.game_closed();
            self.game_open = Some(open);
            if self.game_closed() != was_closed {
                self.push_settings(true);
            }
        }
    }

    /// Update the run's time/day/session stats and check milestones.
    fn note_encounters(&mut self, n: u64) {
        let now = now_unix();
        let odds = self.cfg.odds_one_in();
        let count = self.cfg.count;
        let st = &mut self.cfg.run_stats;
        st.ensure_started(now, count.saturating_sub(n), odds);
        let gap = self.cfg.idle_gap_secs();
        let st = &mut self.cfg.run_stats;
        st.record_gap(now, crate::stats::local_day(now), n, gap);
        let reached = st.check_milestones(now, count, odds);
        if self.cfg.celebrate_milestones {
            if let Some(r) = reached.last() {
                let text = match r {
                    crate::stats::Reached::Thousand(k) => format!("{} ENCOUNTERS!", theme::thousands(*k)),
                    crate::stats::Reached::Odds(1) => "YOU HIT THE ODDS!".to_string(),
                    crate::stats::Reached::Odds(k) => format!("{k}× THE ODDS!"),
                };
                self.celebrate = Some((Instant::now(), text));
                self.play(crate::sound::Sfx::Milestone);
                self.add_event("celebrated");
            }
        }
        self.note_time_events();
        self.maybe_check_progress();
    }

    fn on_msg(&mut self, msg: WorkerMsg) {
        match msg {
            WorkerMsg::Encounter { names, added, late } => {
                self.cfg.count += added as u64;
                self.counted_since_start = true;
                self.last_added = Some(added as u64);
                self.note_encounters(added as u64);
                if !late {
                    self.cfg.battles += 1;
                    self.battle_target_hits = 0;
                    self.battle_species.clear();
                }
                // what you're running into (each Pokémon once per battle)
                let mut already = self.battle_species.clone();
                for &i in &names {
                    if let Some(k) = already.iter().position(|&x| x == i) {
                        already.remove(k);
                    } else if let Some(sp) = self.species.get(i) {
                        *self.cfg.run_stats.species.entry(sp.dex).or_insert(0) += 1;
                        self.battle_species.push(i);
                    }
                }
                let (t1, t2) = (self.target_idx(), self.target2_idx());
                if t1.is_some() || t2.is_some() {
                    // A sighting of either target (two-target hunts count both).
                    let hits = names.iter().filter(|&&i| Some(i) == t1 || Some(i) == t2).count() as u64;
                    if hits > self.battle_target_hits {
                        self.cfg.target_seen += hits - self.battle_target_hits;
                        self.battle_target_hits = hits;
                        self.target_flash = Some(Instant::now());
                        self.cfg.run_stats.note_target_seen(self.cfg.count);
                    }
                }
                self.last_battle = names;
                self.dirty = true;
            }
            WorkerMsg::BattleOver => {}
            WorkerMsg::ShinyStar { plate, .. } => {
                // Its own fanfare, once per alert (not again while it's showing).
                if self.shiny_alert.is_none() {
                    self.shiny_fanfare();
                }
                self.shiny_alert = Some(plate);
                if self.cfg.mini {
                    self.cfg.mini = false;
                }
            }
            WorkerMsg::LogOdds { num, den } => {
                if self.cfg.last_log_odds != Some((num, den)) {
                    self.cfg.last_log_odds = Some((num, den));
                    self.dirty = true;
                }
            }
            WorkerMsg::Status(s) => self.status = s,
        }
    }

    fn on_hotkey(&mut self, a: HotAction) {
        match a {
            HotAction::ToggleHide => {
                self.cfg.mini = !self.cfg.mini;
                self.dirty = true;
            }
            HotAction::Plus => self.add(1),
            HotAction::Minus => self.add(-1),
            HotAction::Pause => self.set_paused(!self.cfg.paused),
            HotAction::Caught => {
                // opens the "Log hunt & reset?" confirmation on the overlay
                self.cfg.mini = false;
                self.confirm_caught = true;
                self.dirty = true;
            }
            HotAction::Undo => self.undo_last(),
        }
    }

    fn sync_window(&mut self, ctx: &egui::Context) {
        let want = if self.cfg.mini { MINI_SIZE } else { FULL_SIZE };
        if self.applied_size != Some(want) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                want[0], want[1],
            )));
            self.applied_size = Some(want);
        }
        // Remember where the overlay sits (ignore the first second while the
        // window is still being placed).
        if self.started.elapsed() > Duration::from_secs(1) {
            if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                let p = [r.min.x, r.min.y];
                let moved = self.cfg.window_pos.map_or(true, |q| {
                    (q[0] - p[0]).abs() > 0.5 || (q[1] - p[1]).abs() > 0.5
                });
                if moved {
                    self.cfg.window_pos = Some(p);
                    self.geom_dirty = true;
                }
            }
        }
    }

    fn autosave(&mut self) {
        if (self.dirty || self.geom_dirty) && self.last_save.elapsed() > Duration::from_millis(800) {
            self.cfg.save();
            // Only real setting changes flash "✔ Saved"; moving a window doesn't.
            if self.dirty && self.show_setup {
                self.saved_flash = Some(Instant::now());
            }
            self.dirty = false;
            self.geom_dirty = false;
            self.last_save = Instant::now();
        }
    }

    /// Save everything now, then quit the app (overlay's ✕ button).
    fn save_and_quit(&mut self, ctx: &egui::Context) {
        self.cfg.save();
        self.dirty = false;
        self.geom_dirty = false;
        self.last_save = Instant::now();
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    }

    // ───────────────────────────── sprites ─────────────────────────────

    /// Draw a sprite standing on `foot` (bottom-centre), fitting in `max`.
    fn draw_sprite(
        &mut self,
        p: &egui::Painter,
        sp: &Species,
        shiny: bool,
        foot: Pos2,
        max: Vec2,
    ) -> bool {
        self.draw_sprite_tinted(p, sp, shiny, foot, max, Color32::WHITE)
    }

    /// Like `draw_sprite`, multiplied by `tint` (black = silhouette).
    fn draw_sprite_tinted(
        &mut self,
        p: &egui::Painter,
        sp: &Species,
        shiny: bool,
        foot: Pos2,
        max: Vec2,
        tint: Color32,
    ) -> bool {
        let t = self.t_ms();
        if let Some(anim) = self.sprites.get(sp, shiny) {
            let s = anim.size;
            let fit = (max.x / s.x).min(max.y / s.y);
            let k = if anim.still {
                // Gen 6+ box sprites are small: scale them up to roughly fill
                // the space (like the animated ones), but not edge to edge.
                (fit * 0.8).min(3.0)
            } else if fit >= 1.0 {
                fit.floor() // integer scale keeps pixel art crisp
            } else {
                fit
            };
            let size = s * k;
            let rect = Rect::from_min_size(foot - egui::vec2(size.x / 2.0, size.y), size);
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            p.image(anim.frame(t).id(), rect, uv, tint);
            true
        } else {
            let msg = if self.sprites.failed(sp, shiny) {
                "no sprite"
            } else {
                "loading…"
            };
            p.text(
                foot - egui::vec2(0.0, max.y / 2.0),
                Align2::CENTER_CENTER,
                msg,
                egui::FontId::proportional(11.0),
                MUTED(),
            );
            false
        }
    }

    // ───────────────────────────── overlay ─────────────────────────────

    fn mini_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let full = ui.max_rect().shrink(2.0);
                let p = ui.painter().clone();
                let bg = theme::BG().gamma_multiply(self.cfg.opacity);
                p.rect_filled(full, 14.0, bg);
                let blink = (self.t_ms() / 350) % 2 == 0;
                let celebrating = self
                    .celebrate
                    .as_ref()
                    .is_some_and(|(at, _)| at.elapsed() < Duration::from_secs(5));
                let border = if (self.shiny_alert.is_some() || celebrating || self.toasts_pending()) && blink {
                    GOLD
                } else {
                    OUTLINE
                };
                p.rect_stroke(full, 14.0, Stroke::new(2.5f32, border));

                let sprite_area =
                    Rect::from_min_size(full.min, egui::vec2(full.width(), full.height() - 22.0));
                theme::platform(
                    &p,
                    sprite_area.center_bottom() - egui::vec2(0.0, 6.0),
                    28.0,
                    7.0,
                );
                let drew = match self.target_idx() {
                    Some(i) => {
                        let sp = self.species[i].clone();
                        self.draw_sprite(
                            &p,
                            &sp,
                            true,
                            sprite_area.center_bottom() - egui::vec2(0.0, 4.0),
                            egui::vec2(64.0, 62.0),
                        )
                    }
                    None => false,
                };
                if !drew {
                    p.image(
                        self.icon_tex.id(),
                        Rect::from_center_size(sprite_area.center(), egui::vec2(46.0, 46.0)),
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                outlined_text(
                    &p,
                    egui::pos2(full.center().x, full.max.y - 11.0),
                    Align2::CENTER_CENTER,
                    &theme::thousands(self.cfg.count),
                    9.0,
                    Color32::WHITE,
                );

                let r = ui.interact(full, ui.id().with("mini"), Sense::click_and_drag());
                if r.drag_started() && !self.cfg.pinned {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if r.clicked() {
                    self.cfg.mini = false;
                    self.dirty = true;
                }
                r.on_hover_text(format!(
                    "Click to open{}",
                    self.cfg.keybinds.hint(HotAction::ToggleHide)
                ));
            });
    }

    fn full_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let full = ui.max_rect().shrink(1.0);
                let p = ui.painter().clone();
                let t = self.t_ms();
                let blink = (t / 350) % 2 == 0;

                let target_flash = self
                    .target_flash
                    .is_some_and(|f| f.elapsed() < Duration::from_millis(1600));
                let border = if (self.shiny_alert.is_some() || target_flash) && blink {
                    GOLD
                } else {
                    OUTLINE
                };

                // Body (only below the header, so nothing dark peeks out
                // around the header's rounded top corners), then header.
                let header = Rect::from_min_size(full.min, egui::vec2(full.width(), 32.0));
                let body = Rect::from_min_max(egui::pos2(full.min.x, header.max.y - 1.0), full.max);
                p.rect_filled(
                    body,
                    egui::Rounding {
                        nw: 0.0,
                        ne: 0.0,
                        sw: 12.0,
                        se: 12.0,
                    },
                    theme::BG().gamma_multiply(self.cfg.opacity),
                );
                theme::header_bar(&p, header, 12.0);
                p.line_segment(
                    [header.left_bottom(), header.right_bottom()],
                    Stroke::new(2.0f32, theme::RED_DARK()),
                );
                p.rect_stroke(
                    full,
                    12.0,
                    Stroke::new(if border == GOLD { 2.5f32 } else { 1.5f32 }, border),
                );
                p.image(
                    self.icon_tex.id(),
                    Rect::from_center_size(
                        header.left_center() + egui::vec2(15.0, 0.0),
                        egui::vec2(18.0, 18.0),
                    ),
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
                let title_r = outlined_text(
                    &p,
                    header.left_center() + egui::vec2(28.0, 1.0),
                    Align2::LEFT_CENTER,
                    "SHINY HUNT",
                    8.5,
                    Color32::WHITE,
                );
                if self.cfg.level_on_overlay {
                    let lvl = self.cfg.level_seen.max(1);
                    theme::level_badge(&p, egui::pos2(title_r.max.x + 11.0, header.center().y), 8.0, lvl);
                }

                // Compact header controls so the title, level badge, status light
                // and four buttons all fit the narrow panel.
                let btn_w = 21.0;
                let drag_rect = Rect::from_min_max(
                    header.min,
                    egui::pos2(header.max.x - btn_w * 4.0 - 18.0, header.max.y),
                );
                let hr = ui.interact(drag_rect, ui.id().with("drag"), Sense::click_and_drag());
                if hr.drag_started() && !self.cfg.pinned {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                hr.on_hover_text(if self.cfg.pinned {
                    "Pinned: unpin to move"
                } else {
                    "Drag to move"
                });

                // Status light: green watching, gold in battle, grey paused, red problem.
                let (dot, status_text) = self.status_line();
                let dot_pos = egui::pos2(header.max.x - btn_w * 4.0 - 11.0, header.center().y);
                p.circle_filled(dot_pos, 3.5, dot);
                p.circle_stroke(dot_pos, 3.5, Stroke::new(1.0f32, OUTLINE));
                ui.interact(
                    Rect::from_center_size(dot_pos, egui::vec2(16.0, 16.0)),
                    ui.id().with("status-dot"),
                    Sense::hover(),
                )
                .on_hover_text(status_text);

                let icon_btn = |ui: &mut egui::Ui, i: usize, icon: &str, on: bool, tip: &str| {
                    let r = Rect::from_min_size(
                        egui::pos2(
                            header.max.x - 5.0 - btn_w * (4 - i) as f32,
                            header.min.y + 5.0,
                        ),
                        egui::vec2(btn_w - 1.0, 22.0),
                    );
                    let color = if on { GOLD } else { Color32::WHITE };
                    ui.put(
                        r,
                        egui::Button::new(RichText::new(icon).size(12.5).color(color)).frame(false),
                    )
                    .on_hover_text(tip)
                    .clicked()
                };
                if icon_btn(
                    ui,
                    0,
                    if self.cfg.pinned { "🔒" } else { "🔓" },
                    self.cfg.pinned,
                    "Pin in place",
                ) {
                    self.cfg.pinned = !self.cfg.pinned;
                    self.dirty = true;
                }
                if icon_btn(
                    ui,
                    1,
                    "⚙",
                    self.show_setup,
                    "Setup: target, screen boxes, odds",
                ) {
                    self.show_setup = !self.show_setup;
                }
                let hide_tip = format!(
                    "Hide to badge{}",
                    self.cfg.keybinds.hint(HotAction::ToggleHide)
                );
                if icon_btn(ui, 2, "—", false, &hide_tip) {
                    self.cfg.mini = true;
                    self.dirty = true;
                }
                // Close the whole app (saves everything first). Drawn, not a
                // font glyph, with the usual red hover.
                let close_r = Rect::from_min_size(
                    egui::pos2(header.max.x - 5.0 - btn_w, header.min.y + 5.0),
                    egui::vec2(btn_w - 1.0, 22.0),
                );
                let close = ui
                    .interact(close_r, ui.id().with("close-app"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if close.hovered() {
                    p.rect_filled(close_r, 5.0, Color32::from_rgb(196, 43, 28));
                }
                let cc = close_r.center();
                let xs = Stroke::new(1.8f32, Color32::WHITE);
                p.line_segment([cc - egui::vec2(3.5, 3.5), cc + egui::vec2(3.5, 3.5)], xs);
                p.line_segment([cc + egui::vec2(-3.5, 3.5), cc + egui::vec2(3.5, -3.5)], xs);
                if close.on_hover_text("Save and close Shiny Tracker").clicked() {
                    self.save_and_quit(ctx);
                }

                let mut y = header.max.y + 10.0;
                let x0 = full.min.x + 12.0;
                let x1 = full.max.x - 12.0;

                // Target plate
                let plate = Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + 26.0));
                theme::plate(&p, plate, theme::PLATE());
                let target = self.target_idx().map(|i| self.species[i].clone());
                let target2 = self.target2_idx().map(|i| self.species[i].clone());
                match &target {
                    Some(sp) => {
                        // Two-target hunts show both names (smaller to fit).
                        let (name, size) = match &target2 {
                            Some(t2) => (format!("{} + {}", sp.name.to_uppercase(), t2.name.to_uppercase()), 7.5),
                            None => (sp.name.to_uppercase(), 10.0),
                        };
                        let name_r = outlined_text(
                            &p,
                            plate.left_center() + egui::vec2(16.0, 1.0),
                            Align2::LEFT_CENTER,
                            &name,
                            size,
                            Color32::WHITE,
                        );
                        let _ = name_r;
                        if target_flash && blink {
                            outlined_text(
                                &p,
                                plate.right_center() - egui::vec2(16.0, -1.0),
                                Align2::RIGHT_CENTER,
                                "SPOTTED!",
                                7.0,
                                GOLD,
                            );
                        } else if !target_flash {
                            if let Some(w) = self.target_watch() {
                                let (txt, col) = if w.dry {
                                    (format!("NOT SEEN IN {}", theme::thousands(w.since.unwrap_or(0))), theme::HP_YELLOW)
                                } else {
                                    (format!("SEEN {}", theme::thousands(w.seen)), MUTED())
                                };
                                outlined_text(&p, plate.right_center() - egui::vec2(12.0, -1.0), Align2::RIGHT_CENTER, &txt, 6.5, col);
                                let tip = w.tip(sp.name);
                                ui.interact(plate, ui.id().with("target-plate"), Sense::hover()).on_hover_text(tip);
                            }
                        }
                    }
                    None => {
                        outlined_text(
                            &p,
                            plate.center(),
                            Align2::CENTER_CENTER,
                            "PICK A TARGET ⚙",
                            8.0,
                            MUTED(),
                        );
                    }
                }
                y = plate.max.y + 8.0;

                // Normal vs shiny sprites
                let box_w = (x1 - x0 - 8.0) / 2.0;
                let box_h = 118.0;
                // Normal + shiny of the target, or both targets' shinies.
                let shows: [(Option<crate::species::Species>, bool); 2] = match &target2 {
                    Some(t2) => [(target.clone(), true), (Some(t2.clone()), true)],
                    None => [(target.clone(), false), (target.clone(), true)],
                };
                for (k, (who, shiny)) in shows.into_iter().enumerate() {
                    let b = Rect::from_min_size(
                        egui::pos2(x0 + k as f32 * (box_w + 8.0), y),
                        egui::vec2(box_w, box_h),
                    );
                    p.rect_filled(b, 8.0, theme::PANEL().gamma_multiply(0.9));
                    p.rect_stroke(
                        b,
                        8.0,
                        Stroke::new(
                            1.5f32,
                            if shiny {
                                GOLD.gamma_multiply(0.8)
                            } else {
                                theme::BORDER()
                            },
                        ),
                    );
                    if shiny {
                        theme::sparkle(&p, b.right_top() + egui::vec2(-12.0, 12.0), 6.0, GOLD);
                    }
                    let foot = b.center_bottom() - egui::vec2(0.0, 14.0);
                    theme::platform(&p, foot + egui::vec2(0.0, 2.0), box_w * 0.38, 8.0);
                    if let Some(sp) = &who {
                        self.draw_sprite(
                            &p,
                            sp,
                            shiny,
                            foot + egui::vec2(0.0, 4.0),
                            egui::vec2(box_w - 12.0, box_h - 24.0),
                        );
                    }
                }
                y += box_h + 10.0;

                // Shiny alert banner over the sprites
                if let Some(plate_idx) = self.shiny_alert {
                    let banner = Rect::from_min_max(
                        egui::pos2(x0, y - box_h - 10.0 + 30.0),
                        egui::pos2(x1, y - 40.0),
                    );
                    p.rect_filled(banner, 8.0, Color32::from_rgba_unmultiplied(20, 16, 4, 235));
                    p.rect_stroke(banner, 8.0, Stroke::new(2.0f32, GOLD));
                    let c = banner.center();
                    if blink {
                        theme::sparkle(&p, c - egui::vec2(86.0, 6.0), 9.0, GOLD);
                        theme::sparkle(&p, c + egui::vec2(86.0, -6.0), 9.0, GOLD);
                    }
                    outlined_text(
                        &p,
                        c - egui::vec2(0.0, 8.0),
                        Align2::CENTER_CENTER,
                        "SHINY!",
                        16.0,
                        GOLD,
                    );
                    p.text(
                        c + egui::vec2(0.0, 14.0),
                        Align2::CENTER_CENTER,
                        format!("Star on plate {} — go catch it!", plate_idx + 1),
                        egui::FontId::proportional(11.0),
                        Color32::WHITE,
                    );
                    let dr = Rect::from_center_size(
                        egui::pos2(c.x, banner.max.y + 14.0),
                        egui::vec2(80.0, 20.0),
                    );
                    if ui
                        .put(dr, egui::Button::new("Dismiss").fill(theme::PLATE()))
                        .clicked()
                    {
                        self.shiny_alert = None;
                        crate::sound::stop_song();
                    }
                }

                // Milestone celebration over the sprites (about 5 seconds).
                if let Some((at, text)) = self.celebrate.clone() {
                    let el = at.elapsed().as_secs_f32();
                    if el > 5.0 || self.shiny_alert.is_some() {
                        self.celebrate = None;
                    } else {
                        let fade = (el / 0.25).min(1.0) * ((5.0 - el) / 0.6).min(1.0);
                        let area = Rect::from_min_max(
                            egui::pos2(x0, y - box_h - 10.0),
                            egui::pos2(x1, y - 10.0),
                        );
                        // confetti
                        let cols = [GOLD, RED(), theme::GREEN, Color32::from_rgb(90, 170, 255), Color32::WHITE];
                        for i in 0..36u32 {
                            let seed = i.wrapping_mul(2654435761);
                            let fx = (seed % 1000) as f32 / 1000.0;
                            let speed = 40.0 + (seed / 1000 % 60) as f32;
                            let fy = ((el * speed + (seed / 7 % 140) as f32) % (area.height() + 20.0)) - 10.0;
                            let wob = (el * 3.0 + i as f32).sin() * 6.0;
                            let pos = egui::pos2(area.min.x + fx * area.width() + wob, area.min.y + fy);
                            let c = cols[(i % 5) as usize].gamma_multiply(fade);
                            let sz = 2.0 + (i % 3) as f32;
                            p.rect_filled(Rect::from_center_size(pos, egui::vec2(sz * 1.6, sz)), 1.0, c);
                        }
                        let banner = Rect::from_center_size(area.center(), egui::vec2(area.width() - 8.0, 54.0));
                        let pulse = 1.0 + 0.04 * (el * 6.0).sin();
                        let banner = Rect::from_center_size(banner.center(), banner.size() * pulse);
                        p.rect_filled(banner, 10.0, Color32::from_rgba_unmultiplied(20, 16, 4, (230.0 * fade) as u8));
                        p.rect_stroke(banner, 10.0, Stroke::new(2.0f32, GOLD.gamma_multiply(fade)));
                        theme::sparkle(&p, banner.left_center() + egui::vec2(14.0, -12.0), 6.0 + 2.0 * (el * 5.0).sin().abs(), GOLD.gamma_multiply(fade));
                        theme::sparkle(&p, banner.right_center() + egui::vec2(-14.0, 12.0), 6.0 + 2.0 * (el * 5.0).cos().abs(), GOLD.gamma_multiply(fade));
                        let size = if text.chars().count() > 16 { 9.0 } else { 11.0 };
                        outlined_text(&p, banner.center() - egui::vec2(0.0, 6.0), Align2::CENTER_CENTER, &text, size, GOLD.gamma_multiply(fade));
                        p.text(
                            banner.center() + egui::vec2(0.0, 13.0),
                            Align2::CENTER_CENTER,
                            "Milestone! Keep going",
                            egui::FontId::proportional(11.0),
                            Color32::WHITE.gamma_multiply(fade),
                        );
                        ctx.request_repaint();
                    }
                }

                // Counter
                outlined_text(
                    &p,
                    egui::pos2(full.center().x, y + 14.0),
                    Align2::CENTER_CENTER,
                    &theme::thousands(self.cfg.count),
                    24.0,
                    Color32::WHITE,
                );
                y += 32.0;
                let (label, label_color) = if self.status.error.is_some() {
                    ("CHECK SETUP ⚙", theme::HP_RED)
                } else if self.cfg.paused {
                    ("PAUSED", MUTED())
                } else if self.game_closed() {
                    ("GAME CLOSED", MUTED())
                } else {
                    ("ENCOUNTERS", MUTED())
                };
                outlined_text(
                    &p,
                    egui::pos2(full.center().x, y + 4.0),
                    Align2::CENTER_CENTER,
                    label,
                    8.0,
                    label_color,
                );
                if self.cfg.show_session_timer {
                    let secs = self.today_hunt_secs();
                    let idle = self.timer_idle();
                    let txt = crate::stats::fmt_duration(secs);
                    let r = p.text(
                        egui::pos2(x1, y + 4.0),
                        Align2::RIGHT_CENTER,
                        txt,
                        egui::FontId::proportional(10.5),
                        if idle { MUTED().gamma_multiply(0.55) } else { MUTED() },
                    );
                    // a tiny clock icon
                    let c = egui::pos2(r.min.x - 8.0, y + 4.0);
                    if idle {
                        // pause bars
                        for dx in [-1.8f32, 1.8] {
                            p.rect_filled(
                                Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(2.0, 7.0)),
                                0.5,
                                MUTED().gamma_multiply(0.7),
                            );
                        }
                    } else {
                        p.circle_stroke(c, 4.0, Stroke::new(1.2f32, MUTED()));
                        p.line_segment([c, c + egui::vec2(0.0, -2.6)], Stroke::new(1.2f32, MUTED()));
                        p.line_segment([c, c + egui::vec2(2.0, 0.0)], Stroke::new(1.2f32, MUTED()));
                    }
                    ui.interact(
                        Rect::from_min_max(egui::pos2(r.min.x - 14.0, r.min.y - 2.0), r.max + egui::vec2(0.0, 2.0)),
                        ui.id().with("today-timer"),
                        Sense::hover(),
                    )
                    .on_hover_text(if idle {
                        format!(
                            "Timer paused: no encounter for over {} minutes. It carries on with the next one.",
                            self.cfg.idle_gap_secs() / 60
                        )
                    } else if self.cfg.idle_pause {
                        format!(
                            "Time hunted today. Pauses after {} minutes with no encounter (Settings > Hunt).",
                            self.cfg.idle_minutes
                        )
                    } else {
                        "Time hunted today".to_string()
                    });
                }
                y += 20.0;

                // Odds gauge: under / at / over odds (encounters ÷ odds).
                let (left, left_color, frac, right, tip) = match self.cfg.odds_one_in() {
                    None => (
                        "ODDS: AFTER 1ST BATTLE".to_string(),
                        MUTED(),
                        0.0,
                        String::new(),
                        "Waiting for the game log to report the odds (start a battle).".to_string(),
                    ),
                    Some(one_in) => {
                        let (odds_n, odds_what) = self.odds_count();
                        let mult = odds_n as f64 / one_in;
                        let m = if mult < 0.1 { format!("{mult:.2}×") } else { format!("{mult:.1}×") };
                        let (label, color) = if mult < 0.9 {
                            (format!("UNDER ODDS · {m}"), theme::GREEN)
                        } else if mult <= 1.1 {
                            (format!("AT ODDS · {m}"), theme::HP_YELLOW)
                        } else if mult < 2.0 {
                            (format!("OVER ODDS · {m}"), theme::HP_RED)
                        } else {
                            (format!("{m} OVER ODDS"), theme::HP_RED)
                        };
                        let p = 1.0 / one_in;
                        let per_battle = self.cfg.per_battle_one_in();
                        let wild = self.cfg.hunt_method.is_wild_battles();
                        let right = match per_battle {
                            Some(b) if wild => format!("1/{} a battle", fmt_one_in(b)),
                            _ if wild => format!("1/{} each", fmt_one_in(one_in)),
                            _ => format!("1/{}", fmt_one_in(one_in)),
                        };
                        let found = 1.0 - (1.0 - p).powf(odds_n as f64);
                        let mut tip = format!(
                            "{}\n{} of {} {odds_what} ({m}).\n{:.0}% of hunters would have found one by now.",
                            self.cfg.odds_caption(),
                            theme::thousands(odds_n),
                            fmt_one_in(one_in),
                            found * 100.0,
                        );
                        tip.push_str(&format!("\nEach Pokémon: 1 in {}.", fmt_one_in(one_in)));
                        if wild {
                            if let (Some(b), Some(n)) = (per_battle, self.cfg.pokemon_per_battle()) {
                                if n > 1.0 {
                                    tip.push_str(&format!(
                                        "\nEach battle ({} Pokémon): 1 in {}.",
                                        if n == 2.0 { "2".to_string() } else { format!("~{n:.1}") },
                                        fmt_one_in(b)
                                    ));
                                }
                            }
                        }
                        tip.push_str("\nThe chance never builds up between encounters.");
                        (label, color, mult.min(1.0) as f32, right, tip)
                    }
                };
                outlined_text(&p, egui::pos2(x0, y + 5.0), Align2::LEFT_CENTER, &left, 7.0, left_color);
                p.text(
                    egui::pos2(x1, y + 5.0),
                    Align2::RIGHT_CENTER,
                    right,
                    egui::FontId::proportional(11.0),
                    MUTED(),
                );
                y += 14.0;
                let bar = Rect::from_min_max(egui::pos2(x0 + 2.0, y), egui::pos2(x1 - 2.0, y + 8.0));
                theme::gauge(&p, bar, frac, left_color);
                y += 15.0;
                // What the odds are based on: method · singles/doubles · per-Pokémon odds.
                let mut caption = self.cfg.odds_caption();
                if let Some(one_in) = self.cfg.odds_one_in() {
                    if self.cfg.pokemon_per_battle().map_or(true, |n| n > 1.0) {
                        caption.push_str(&format!(" · 1/{} each", fmt_one_in(one_in)));
                    }
                }
                p.text(
                    egui::pos2(full.center().x, y),
                    Align2::CENTER_CENTER,
                    caption,
                    egui::FontId::proportional(10.5),
                    MUTED().gamma_multiply(0.85),
                );
                let hover = Rect::from_min_max(bar.min - egui::vec2(6.0, 20.0), egui::pos2(bar.max.x + 6.0, y + 7.0));
                ui.interact(hover, ui.id().with("odds"), Sense::hover()).on_hover_text(tip);
                y += 15.0;

                // Buttons
                let row = Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, full.max.y - 10.0));
                let undo_left = if self.confirm_caught { None } else { self.undo_caught_left() };
                if let Some(left) = undo_left {
                    // Just logged a Caught!: offer Undo for a few seconds.
                    let w = row.width();
                    p.text(
                        egui::pos2(row.min.x, row.center().y),
                        Align2::LEFT_CENTER,
                        "✔ Logged as caught",
                        egui::FontId::proportional(11.5),
                        theme::pal().text,
                    );
                    let at = |a: f32, b: f32| {
                        Rect::from_min_max(egui::pos2(row.min.x + w * a, row.min.y), egui::pos2(row.min.x + w * b, row.max.y))
                    };
                    if ui
                        .put(at(0.52, 0.84), egui::Button::new(RichText::new(format!("Undo ({left})")).strong().color(Color32::WHITE)).fill(theme::PLATE()))
                        .on_hover_text("Pressed Caught! by mistake? Put everything back")
                        .clicked()
                    {
                        self.undo_caught_now();
                    }
                    if ui.put(at(0.86, 1.0), egui::Button::new("OK").fill(theme::PLATE())).clicked() {
                        self.undo_caught = None;
                    }
                    ctx.request_repaint_after(Duration::from_millis(250));
                } else if self.confirm_caught {
                    let w = row.width();
                    let two = self.target2_idx().map(|i| self.species[i].name.to_string());
                    // Phase / ✕ shift right to make room for a second target button.
                    let (phase_a, phase_b, x_a) = if two.is_some() { (0.76, 0.90, 0.92) } else { (0.64, 0.86, 0.88) };
                    p.text(
                        egui::pos2(row.min.x, row.center().y),
                        Align2::LEFT_CENTER,
                        if two.is_some() { "Which?" } else { "Shiny! Was it…" },
                        egui::FontId::proportional(11.5),
                        theme::pal().text,
                    );
                    let at = |a: f32, b: f32| {
                        Rect::from_min_max(egui::pos2(row.min.x + w * a, row.min.y), egui::pos2(row.min.x + w * b, row.max.y))
                    };
                    if let Some(name2) = two {
                        // Two-target hunt: one button per target.
                        let short = |n: &str| if n.chars().count() > 8 { format!("{}…", n.chars().take(7).collect::<String>()) } else { n.to_string() };
                        let name1 = self.target_idx().map(|i| self.species[i].name.to_string()).unwrap_or_default();
                        if ui
                            .put(at(0.22, 0.47), egui::Button::new(RichText::new(short(&name1)).strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                            .on_hover_text(format!("Shiny {name1}! Logs this hunt and starts fresh"))
                            .clicked()
                        {
                            self.log_hunt_as(self.cfg.target.clone());
                            self.confirm_caught = false;
                        }
                        if ui
                            .put(at(0.49, 0.74), egui::Button::new(RichText::new(short(&name2)).strong().color(Color32::WHITE)).fill(theme::GREEN.gamma_multiply(0.7)))
                            .on_hover_text(format!("Shiny {name2}! Logs this hunt and starts fresh"))
                            .clicked()
                        {
                            self.log_hunt_as(self.cfg.target2.clone());
                            self.confirm_caught = false;
                        }
                    } else if ui
                        .put(
                            at(0.36, 0.62),
                            egui::Button::new(RichText::new("Target").strong().color(Color32::WHITE))
                                .fill(theme::GREEN.gamma_multiply(0.7)),
                        )
                        .on_hover_text("Your target! Logs this hunt and starts fresh")
                        .clicked()
                    {
                        self.log_hunt();
                        self.confirm_caught = false;
                    }
                    if ui
                        .put(
                            at(phase_a, phase_b),
                            egui::Button::new(RichText::new("Phase").strong().color(Color32::WHITE)).fill(GOLD.gamma_multiply(0.55)),
                        )
                        .on_hover_text("A shiny of a different Pokémon: pick which, and this hunt keeps going")
                        .clicked()
                    {
                        self.confirm_caught = false;
                        self.setup.phase_pick = true;
                        self.setup.search.clear();
                        self.show_setup = true;
                    }
                    if ui
                        .put(at(x_a, 1.0), egui::Button::new("✕").fill(theme::PLATE()))
                        .on_hover_text("Not a shiny after all")
                        .clicked()
                    {
                        self.confirm_caught = false;
                    }
                } else {
                    let gap = 6.0;
                    let widths = [0.18, 0.18, 0.22, 0.42];
                    let mut bx = row.min.x;
                    let total = row.width() - gap * 3.0;
                    for (i, wf) in widths.iter().enumerate() {
                        let r = Rect::from_min_size(
                            egui::pos2(bx, row.min.y),
                            egui::vec2(total * wf, row.height()),
                        );
                        bx += total * wf + gap;
                        let (label, fill, tip) = match i {
                            0 => (
                                "−1".to_string(),
                                theme::PLATE(),
                                format!("Remove one{}", self.cfg.keybinds.hint(HotAction::Minus)),
                            ),
                            1 => (
                                "+1".to_string(),
                                theme::PLATE(),
                                format!("Add one{}", self.cfg.keybinds.hint(HotAction::Plus)),
                            ),
                            2 => (
                                if self.cfg.paused { "▶" } else { "⏸" }.to_string(),
                                theme::PLATE(),
                                format!(
                                    "Pause / resume auto count{}",
                                    self.cfg.keybinds.hint(HotAction::Pause)
                                ),
                            ),
                            _ => (
                                "Caught!".to_string(),
                                RED(),
                                "Found your shiny? Log this hunt and start fresh".to_string(),
                            ),
                        };
                        // "Caught!" uses the button colour / gradient.
                        let caught_btn = i == 3;
                        let ink = if caught_btn { theme::button_ink() } else { Color32::WHITE };
                        let slot = ui.painter().add(egui::Shape::Noop);
                        let btn = egui::Button::new(RichText::new(label).strong().color(ink))
                            .fill(if caught_btn { Color32::TRANSPARENT } else { fill })
                            .rounding(6.0);
                        let resp = ui.put(r, btn);
                        if caught_btn {
                            ui.painter().set(slot, theme::button_shape(resp.rect, 6.0, resp.hovered()));
                        }
                        if resp.on_hover_text(tip).clicked() {
                            match i {
                                0 => self.add(-1),
                                1 => self.add(1),
                                2 => self.set_paused(!self.cfg.paused),
                                _ => self.confirm_caught = true,
                            }
                        }
                    }
                }

                // Notices (break, goal, "nothing counted?", update) over the sprites.
                if !self.notices.is_empty() {
                    let area = Rect::from_min_max(
                        egui::pos2(full.min.x + 8.0, header.max.y + 6.0),
                        egui::pos2(full.max.x - 8.0, header.max.y + 100.0),
                    );
                    self.draw_notice(ui, ctx, area);
                }

                // Achievement / level-up pop-up on top of everything.
                if self.cfg.toasts && !self.toasts.is_empty() {
                    let area = Rect::from_min_max(
                        egui::pos2(full.min.x + 8.0, header.max.y + 6.0),
                        egui::pos2(full.max.x - 8.0, header.max.y + 80.0),
                    );
                    let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts")));
                    self.draw_toast(ctx, &fg, area);
                }
            });
    }

    fn status_line(&self) -> (Color32, String) {
        if self.game_closed() {
            return (MUTED(), "Paused: Platinum Together isn't running".to_string());
        }
        let s = &self.status;
        if let Some(e) = &s.error {
            return (theme::HP_RED, e.clone());
        }
        if let Some(r) = s.idle_reason {
            return (MUTED(), r.to_string());
        }
        let log = self.cfg.count_source == CountSource::GameLog;
        if s.in_battle {
            (GOLD, "In battle (counted)".to_string())
        } else if log {
            (theme::GREEN, "Counting from the game log".to_string())
        } else {
            (theme::GREEN, "Watching for battles".to_string())
        }
    }

    fn log_hunt(&mut self) {
        self.log_hunt_as(self.cfg.target.clone());
    }

    /// Log the hunt as caught for `slug` (the target, or the second target
    /// of a two-target hunt) and start fresh. Undo is offered for a moment.
    fn log_hunt_as(&mut self, slug: Option<String>) {
        self.undo_caught = Some((Instant::now(), Box::new(self.cfg.clone())));
        self.play(crate::sound::Sfx::Caught);
        let target = slug
            .as_deref()
            .and_then(|s| self.species.iter().find(|sp| sp.slug == s))
            .map(|sp| sp.name.to_string())
            .unwrap_or_else(|| "(no target)".into());
        let stats = std::mem::take(&mut self.cfg.run_stats);
        self.cfg.history.push(HuntRecord {
            target,
            encounters: self.cfg.count,
            battles: self.cfg.battles,
            when_unix: now_unix(),
            slug: slug.clone(),
            started_unix: self.cfg.run_started_unix,
            target_seen: self.cfg.target_seen,
            method: self.cfg.odds_caption(),
            odds_one_in: self.cfg.odds_one_in(),
            stats: Some(stats),
            phase: false,
            run_id: self.cfg.run_id,
        });
        let id = self.cfg.run_id;
        self.cfg.saved_runs.retain(|r| r.id != id);
        self.start_fresh(self.cfg.target.clone());
        self.add_event("caught_it");
        self.check_progress();
    }

    /// Seconds left to undo the last Caught! (None = nothing to undo).
    fn undo_caught_left(&self) -> Option<u64> {
        let (at, _) = self.undo_caught.as_ref()?;
        let left = 10u64.checked_sub(at.elapsed().as_secs())?;
        (left > 0).then_some(left)
    }

    /// Put everything back as it was before the last Caught!.
    fn undo_caught_now(&mut self) {
        if let Some((_, before)) = self.undo_caught.take() {
            self.cfg = *before;
            self.reset_transient();
            self.push_settings(true);
            self.dirty = true;
        }
    }

    // ───────────────────────────── setup window ─────────────────────────────

    /// Switching target puts the current run aside and either reopens the
    /// most recent saved run for the new target or starts a fresh one.
    fn set_target(&mut self, slug: String) {
        if self.cfg.target.as_deref() == Some(&slug) {
            return;
        }
        let existing = self
            .cfg
            .saved_runs
            .iter()
            .filter(|r| r.target.as_deref() == Some(&slug) && r.id != self.cfg.run_id)
            .max_by_key(|r| r.saved_unix)
            .map(|r| r.id);
        match existing {
            Some(id) => self.resume_run(id),
            None => {
                self.park_current();
                self.cfg.target2 = None;
                self.start_fresh(Some(slug));
            }
        }
    }

    fn run_is_empty(&self) -> bool {
        self.cfg.count == 0 && self.cfg.battles == 0 && self.cfg.target_seen == 0
    }

    /// Save the active run into the saved list (empty runs aren't kept).
    fn park_current(&mut self) {
        if self.run_is_empty() {
            return;
        }
        let run = SavedRun {
            id: self.cfg.run_id,
            target: self.cfg.target.clone(),
            count: self.cfg.count,
            battles: self.cfg.battles,
            target_seen: self.cfg.target_seen,
            started_unix: self.cfg.run_started_unix,
            saved_unix: now_unix(),
            stats: self.cfg.run_stats.clone(),
            target2: self.cfg.target2.clone(),
        };
        match self.cfg.saved_runs.iter_mut().find(|r| r.id == run.id) {
            Some(r) => *r = run,
            None => self.cfg.saved_runs.push(run),
        }
        self.dirty = true;
    }

    fn start_fresh(&mut self, target: Option<String>) {
        self.cfg.target = target;
        self.cfg.count = 0;
        self.cfg.battles = 0;
        self.cfg.target_seen = 0;
        self.cfg.run_id = new_run_id();
        self.cfg.run_started_unix = now_unix();
        self.cfg.run_stats = Default::default();
        let odds = self.cfg.odds_one_in();
        self.cfg.run_stats.ensure_started(now_unix(), 0, odds);
        self.reset_transient();
    }

    /// Put the current run aside and reopen a saved one.
    fn resume_run(&mut self, id: u64) {
        if id == self.cfg.run_id {
            return;
        }
        self.park_current();
        if let Some(pos) = self.cfg.saved_runs.iter().position(|r| r.id == id) {
            let r = self.cfg.saved_runs.remove(pos);
            self.cfg.target = r.target;
            self.cfg.count = r.count;
            self.cfg.battles = r.battles;
            self.cfg.target_seen = r.target_seen;
            self.cfg.run_id = r.id;
            self.cfg.run_started_unix = r.started_unix;
            self.cfg.run_stats = r.stats;
            self.cfg.target2 = r.target2;
            self.add_event("comeback");
        }
        self.reset_transient();
    }

    fn reset_transient(&mut self) {
        self.battle_target_hits = 0;
        self.battle_species.clear();
        self.target_flash = None;
        self.shiny_alert = None;
        self.confirm_caught = false;
        self.celebrate = None;
        self.dirty = true;
    }

    /// One numbered step header: a red number bubble that turns into a green
    /// tick once the step is done.
    fn step(ui: &mut egui::Ui, n: usize, done: bool, title: &str, body: &str) {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), Sense::hover());
            let p = ui.painter();
            p.circle_filled(r.center(), 14.0, if done { theme::GREEN } else { RED() });
            p.circle_stroke(r.center(), 14.0, Stroke::new(2.0f32, OUTLINE));
            let label = if done {
                "✔".to_string()
            } else {
                n.to_string()
            };
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                label,
                ui_kit::font(16.0),
                Color32::WHITE,
            );
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(title)
                        .size(ui_kit::fs(17.0))
                        .strong()
                        .color(theme::pal().text),
                );
                if !body.is_empty() {
                    ui.label(RichText::new(body).size(ui_kit::fs(13.5)).color(ui_kit::MUTED()));
                }
            });
        });
    }

    /// Small drawing showing where the box goes on a name plate.
    fn draw_box_example(ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(360.0, 70.0), Sense::hover());
        let p = ui.painter_at(rect);
        let plate = Rect::from_min_size(rect.min + egui::vec2(4.0, 6.0), egui::vec2(300.0, 34.0));
        p.rect_filled(plate, 4.0, Color32::from_rgb(52, 46, 84));
        let name = outlined_text(
            &p,
            plate.left_center() + egui::vec2(14.0, 1.0),
            Align2::LEFT_CENTER,
            "RIOLU",
            12.0,
            Color32::WHITE,
        );
        p.text(
            name.right_center() + egui::vec2(78.0, 0.0),
            Align2::LEFT_CENTER,
            "♂",
            ui_kit::font(18.0),
            Color32::from_rgb(90, 170, 255),
        );
        outlined_text(
            &p,
            name.right_center() + egui::vec2(104.0, 1.0),
            Align2::LEFT_CENTER,
            "Lv.24",
            10.0,
            GOLD,
        );
        // the box: from just before the name to just before ♂ (room for a ★)
        let boxr = Rect::from_min_max(
            egui::pos2(name.min.x - 5.0, plate.min.y + 3.0),
            egui::pos2(name.right_center().x + 70.0, plate.max.y - 3.0),
        );
        p.rect_stroke(boxr, 2.0, Stroke::new(2.5f32, theme::REGION));
        p.text(
            egui::pos2(boxr.center().x, rect.max.y - 10.0),
            Align2::CENTER_CENTER,
            "your box (leave room for the ★)",
            ui_kit::font(12.0),
            theme::REGION,
        );
        p.text(
            egui::pos2(plate.max.x + 6.0, plate.center().y),
            Align2::LEFT_CENTER,
            "✖ not Lv.",
            ui_kit::font(12.0),
            theme::HP_RED,
        );
    }

    fn take_picture(&mut self, ctx: &egui::Context) {
        match capture::grab(self.cfg.monitor) {
            Ok(rgba) => {
                let img = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.w as usize, rgba.h as usize],
                    &rgba.px,
                );
                let tex = ctx.load_texture("setup-shot", img, egui::TextureOptions::LINEAR);
                self.setup.shot = Some(Shot { rgba, tex });
                self.setup.shot_error = None;
                self.setup.test = None;
            }
            Err(e) => self.setup.shot_error = Some(e),
        }
    }

    /// The 4-step name box setup (used on the Counting page).
    fn screen_wizard(&mut self, ui: &mut egui::Ui) {
        // Countdown so there's time to click back into the game.
        if let Some(at) = self.setup.capture_at {
            if Instant::now() >= at {
                self.setup.capture_at = None;
                self.take_picture(ui.ctx());
            } else {
                ui.ctx().request_repaint_after(Duration::from_millis(100));
            }
        }
        if self.setup.monitors == 0 {
            self.setup.monitors = capture::monitor_count().max(1);
        }
        let has_shot = self.setup.shot.is_some();
        let n_boxes = self.cfg.name_regions.len();
        let works = self
            .setup
            .test
            .as_ref()
            .is_some_and(|t| !t.names.is_empty());

        {
            ui.label(
                RichText::new(
                    "Draw a box over each enemy name once, so the app knows where to look.",
                )
                .size(ui_kit::fs(13.0))
                .color(ui_kit::MUTED()),
            );

            // ── Step 1
            Self::step(ui, 1, has_shot, "Start a battle", "In the game, use Sweet Scent so the enemy names are showing (top-left of the battle).");

            // ── Step 2
            Self::step(
                ui,
                2,
                has_shot,
                "Take a picture of your screen",
                "Click the button, then click on the game. The picture is taken 3 seconds later.",
            );
            ui.indent("step2", |ui| {
                ui.horizontal(|ui| {
                    if self.setup.monitors > 1 {
                        ui.label("Game is on screen:");
                        for m in 0..self.setup.monitors {
                            if ui
                                .selectable_label(self.cfg.monitor == m, format!(" {} ", m + 1))
                                .clicked()
                                && self.cfg.monitor != m
                            {
                                self.cfg.monitor = m;
                                self.setup.shot = None;
                                self.setup.test = None;
                                self.push_settings(true);
                            }
                        }
                        ui.add_space(10.0);
                    }
                    match self.setup.capture_at {
                        Some(at) => {
                            let left = at.saturating_duration_since(Instant::now()).as_secs() + 1;
                            ui.label(
                                RichText::new(format!("📸 Click on the game now… {left}"))
                                    .size(ui_kit::fs(15.0))
                                    .strong()
                                    .color(GOLD),
                            );
                        }
                        None => {
                            let label = if has_shot {
                                "📸 Retake picture"
                            } else {
                                "📸 Take picture"
                            };
                            if ui
                                .add(
                                    egui::Button::new(RichText::new(label).size(ui_kit::fs(15.0)).strong().color(theme::button_ink()))
                                        .fill(theme::BUTTON()).rounding(ui_kit::primary_round()),
                                )
                                .clicked()
                            {
                                self.setup.capture_at =
                                    Some(Instant::now() + Duration::from_secs(3));
                            }
                        }
                    }
                });
                if let Some(e) = &self.setup.shot_error {
                    ui.colored_label(theme::HP_RED, e);
                }
            });

            // ── Step 3
            Self::step(
                ui,
                3,
                n_boxes > 0,
                "Draw a box over each enemy name",
                "Click and drag on the picture. Start at the first letter and stop just before the ♂/♀ sign. One box per enemy (two in a double battle).",
            );
            ui.indent("step3", |ui| {
                Self::draw_box_example(ui);
                if has_shot {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Boxes drawn: {n_boxes}"))
                                .strong()
                                .color(theme::REGION),
                        );
                        if n_boxes > 0 {
                            if ui.button("↩ Undo last box").clicked() {
                                self.cfg.name_regions.pop();
                                self.setup.test = None;
                                self.push_settings(true);
                            }
                            if ui.button("Start over").clicked() {
                                self.cfg.name_regions.clear();
                                self.setup.test = None;
                                self.push_settings(true);
                            }
                        }
                        let mut full = !self.setup.zoom_fit;
                        if ui
                            .checkbox(&mut full, "Show picture at full size")
                            .changed()
                        {
                            self.setup.zoom_fit = !full;
                        }
                    });
                    self.screenshot_canvas(ui);
                    egui::CollapsingHeader::new("Fine-tune boxes (optional)").show(ui, |ui| {
                        self.region_list(ui);
                    });
                } else {
                    ui.label(
                        RichText::new("Your picture will appear here after step 2.")
                            .italics()
                            .color(ui_kit::MUTED()),
                    );
                }
            });

            // ── Step 4
            Self::step(
                ui,
                4,
                works,
                "Check it works",
                "Click Test. You should see the enemy names appear in green.",
            );
            ui.indent("step4", |ui| {
                let ready = has_shot && n_boxes > 0;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(RichText::new("🔎 Test").size(ui_kit::fs(15.0)).strong().color(theme::button_ink())).fill(theme::BUTTON()).rounding(ui_kit::primary_round()),
                    )
                    .on_disabled_hover_text("Do steps 2 and 3 first")
                    .clicked()
                {
                    self.run_test(ui.ctx());
                }
                self.test_panel(ui);
            });

            if works {
                ui.add_space(16.0);
                ui.label(
                    RichText::new("🎉 All set! Name reading is working.")
                        .size(ui_kit::fs(16.0))
                        .strong()
                        .color(theme::GREEN),
                );
            }
            ui.add_space(8.0);
        }
    }

    /// The captured screenshot; drag on it to add a name box.
    fn screenshot_canvas(&mut self, ui: &mut egui::Ui) {
        let Some(shot) = &self.setup.shot else { return };
        let (iw, ih) = (shot.rgba.w as f32, shot.rgba.h as f32);
        let tex_id = shot.tex.id();

        let mut new_region: Option<Region> = None;
        let fit = self.setup.zoom_fit;
        egui::ScrollArea::both()
            .id_salt("screenshot-canvas")
            .drag_to_scroll(false)
            .max_height(440.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let scale = if fit {
                    (ui.available_width() / iw).min(1.0)
                } else {
                    1.0
                };
                let size = egui::vec2(iw * scale, ih * scale);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::drag());
                let p = ui.painter_at(rect);
                p.image(
                    tex_id,
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );

                let to_screen = |r: &Region| {
                    Rect::from_min_size(
                        rect.min + egui::vec2(r.x as f32 * scale, r.y as f32 * scale),
                        egui::vec2(r.w as f32 * scale, r.h as f32 * scale),
                    )
                };
                for (i, r) in self.cfg.name_regions.iter().enumerate() {
                    let sr = to_screen(r);
                    p.rect_stroke(sr, 2.0, Stroke::new(2.0f32, theme::REGION));
                    p.text(
                        sr.left_top() + egui::vec2(2.0, -2.0),
                        Align2::LEFT_BOTTOM,
                        format!("Name {}", i + 1),
                        ui_kit::font(12.0),
                        theme::REGION,
                    );
                }

                if resp.drag_started() {
                    self.setup.drag_from = ui
                        .input(|i| i.pointer.press_origin())
                        .or(resp.interact_pointer_pos());
                }
                if resp.dragged() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        self.setup.drag_to = Some(pos);
                    }
                }
                if let (Some(a), Some(b)) = (self.setup.drag_from, self.setup.drag_to) {
                    let sel = Rect::from_two_pos(a, b).intersect(rect);
                    p.rect_stroke(sel, 0.0, Stroke::new(2.0f32, GOLD));
                    if resp.drag_stopped() {
                        let r = Region {
                            x: ((sel.min.x - rect.min.x) / scale).round().max(0.0) as u32,
                            y: ((sel.min.y - rect.min.y) / scale).round().max(0.0) as u32,
                            w: (sel.width() / scale).round() as u32,
                            h: (sel.height() / scale).round() as u32,
                        };
                        if r.w >= 6 && r.h >= 6 {
                            new_region = Some(r);
                        }
                    }
                }
                if !resp.dragged() {
                    self.setup.drag_from = None;
                    self.setup.drag_to = None;
                }
                if self.cfg.name_regions.is_empty() && self.setup.drag_from.is_none() {
                    p.text(
                        rect.center_top() + egui::vec2(0.0, 18.0),
                        Align2::CENTER_CENTER,
                        "Drag here to draw a box",
                        ui_kit::font(16.0),
                        GOLD,
                    );
                }
            });
        if let Some(r) = new_region {
            self.cfg.name_regions.push(r);
            self.setup.test = None;
            self.push_settings(true);
        }
    }

    fn region_list(&mut self, ui: &mut egui::Ui) {
        if self.cfg.name_regions.is_empty() {
            return;
        }
        ui.add_space(6.0);
        let mut remove = None;
        let mut changed = false;
        for (i, r) in self.cfg.name_regions.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Name {}", i + 1)).color(theme::REGION));
                for (label, v) in [
                    ("x", &mut r.x),
                    ("y", &mut r.y),
                    ("w", &mut r.w),
                    ("h", &mut r.h),
                ] {
                    ui.label(label);
                    changed |= ui.add(egui::DragValue::new(v).range(0..=8000)).changed();
                }
                if ui.button("🗑").on_hover_text("Delete box").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            self.cfg.name_regions.remove(i);
            changed = true;
        }
        if changed {
            self.setup.test = None;
            self.push_settings(true);
        }
    }

    fn run_test(&mut self, ctx: &egui::Context) {
        let Some(shot) = &self.setup.shot else { return };
        let frame = shot.rgba.clone();
        let regions = self.cfg.name_regions.clone();
        let threshold = self.cfg.star_threshold;
        // OCR runs on its own thread (WinRT prefers it off the UI thread).
        let result = std::thread::spawn(move || {
            let processed = prepare_names(&frame, &regions);
            let text = match &processed {
                None => Err("Boxes are outside the screenshot".to_string()),
                Some(img) => ocr::Ocr::new().and_then(|o| o.read(img)),
            };
            let yellow: Vec<u32> = regions
                .iter()
                .map(|r| frame.crop(*r).and_then(|c| c.find_star((threshold / 4).max(12))).map_or(0, |f| f.2))
                .collect();
            (processed, text, yellow)
        })
        .join();
        let Ok((processed, text, yellow)) = result else {
            self.setup.test = Some(TestRead {
                processed: None,
                processed_size: Vec2::ZERO,
                text: Err("OCR crashed".into()),
                names: vec![],
                yellow: vec![],
            });
            return;
        };
        let matcher = NameMatcher::new(&self.species);
        let names = match &text {
            Ok(t) => matcher
                .find(t)
                .into_iter()
                .map(|i| self.species[i].name.to_string())
                .collect(),
            Err(_) => vec![],
        };
        let (tex, size) = match processed {
            Some(g) => {
                let size = egui::vec2(g.w as f32, g.h as f32);
                (Some(load_gray(ctx, &g)), size)
            }
            None => (None, Vec2::ZERO),
        };
        self.setup.test = Some(TestRead {
            processed: tex,
            processed_size: size,
            text,
            names,
            yellow,
        });
    }

    fn test_panel(&mut self, ui: &mut egui::Ui) {
        let Some(t) = &self.setup.test else { return };
        ui.add_space(6.0);
        if let Err(e) = &t.text {
            ui.colored_label(
                theme::HP_RED,
                RichText::new(format!("✖ Couldn't read the screen: {e}")).size(ui_kit::fs(14.0)),
            );
        } else if t.names.is_empty() {
            ui.colored_label(
                theme::HP_RED,
                RichText::new("✖ Couldn't find a Pokémon name in your boxes.")
                    .size(ui_kit::fs(15.0))
                    .strong(),
            );
            ui.label("• Make sure the picture shows a battle with the enemy names visible.");
            ui.label("• Each box should cover just the name, with a little space around it.");
        } else {
            let per = match per_battle_for(self.cfg.battle_type) {
                PerBattle::Auto => t.names.len() as u32,
                PerBattle::Fixed(n) => n,
            };
            ui.colored_label(
                theme::GREEN,
                RichText::new(format!(
                    "✔ It works! Reading: {}",
                    t.names.join(" + ").to_uppercase()
                ))
                .size(ui_kit::fs(16.0))
                .strong(),
            );
            ui.label(
                RichText::new(format!("A battle like this adds {per} to your count.")).color(ui_kit::MUTED()),
            );
            let stars: Vec<usize> = t
                .yellow
                .iter()
                .enumerate()
                .filter(|(_, y)| **y > 0)
                .map(|(i, _)| i + 1)
                .collect();
            if stars.is_empty() {
                ui.label(
                    RichText::new("✔ No shiny star in this battle (that's normal).").color(ui_kit::MUTED()),
                );
            } else {
                let which: Vec<String> = stars.iter().map(|n| n.to_string()).collect();
                ui.colored_label(
                    GOLD,
                    format!("★ Shiny star seen in box {}.", which.join(" & ")),
                );
                ui.label(
                    RichText::new("Not actually a shiny? Tell us! Lowering the star size setting makes it pick up smaller stars, raising it ignores them.")
                        .color(ui_kit::MUTED())
                        .size(ui_kit::fs(12.0)),
                );
            }
        }
        egui::CollapsingHeader::new("Details")
            .id_salt("test-details")
            .show(ui, |ui| {
                if let Some(tex) = &t.processed {
                    ui.label(RichText::new("What the reader sees:").color(ui_kit::MUTED()));
                    let k = (360.0 / t.processed_size.x).min(1.0);
                    let (r, _) = ui.allocate_exact_size(t.processed_size * k, Sense::hover());
                    ui.painter().image(
                        tex.id(),
                        r,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                if let Ok(txt) = &t.text {
                    ui.label(format!("Raw text: {:?}", txt.trim()));
                }
                for (i, y) in t.yellow.iter().enumerate() {
                    ui.label(if *y > 0 {
                        format!("Box {}: ★ shape found ({} pixels)", i + 1, y)
                    } else {
                        format!("Box {}: no ★ shape", i + 1)
                    });
                }
            });
    }

    fn apply_keybinds(&mut self) {
        self.hotkeys.apply(&self.cfg.keybinds);
        self.dirty = true;
    }

    fn target_name(&self, slug: Option<&str>) -> String {
        slug.and_then(|t| self.species.iter().find(|s| s.slug == t))
            .map(|s| s.name.to_string())
            .unwrap_or_else(|| "(no target)".into())
    }
}

fn new_run_id() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(1)
        .max(1)
}

fn load_gray(ctx: &egui::Context, g: &Gray) -> egui::TextureHandle {
    let img = egui::ColorImage::from_rgba_unmultiplied([g.w as usize, g.h as usize], &g.to_rgba());
    ctx.load_texture("setup-test", img, egui::TextureOptions::NEAREST)
}

fn days_ago(unix: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(unix);
    match now.saturating_sub(unix) / 86_400 {
        0 => "today".into(),
        1 => "yesterday".into(),
        d => format!("{d} days ago"),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Stay above the game (background thread, started once).
        self.borderless_fix.store(
            self.cfg.borderless_fix,
            std::sync::atomic::Ordering::Relaxed,
        );
        crate::topmost::ensure_started(&mut self.topmost_started, frame, &self.borderless_fix);
        self.fonts.poll(ctx);
        self.sprites.poll(ctx);
        for a in self.hotkeys.poll() {
            self.on_hotkey(a);
        }
        while let Ok(msg) = self.rx.try_recv() {
            self.on_msg(msg);
        }
        self.watch_game();
        self.tick_notices(ctx);
        if self.applied_appearance.as_ref() != Some(&self.cfg.appearance) {
            theme::set_appearance(ctx, &self.cfg.appearance);
            self.applied_appearance = Some(self.cfg.appearance.clone());
            if self.cfg.appearance != crate::config::Appearance::default() {
                self.add_event("stylish");
            }
        }
        if self.progress_checked.elapsed() > Duration::from_secs(20) {
            self.check_progress();
        }
        self.sync_window(ctx);
        if self.cfg.mini {
            self.mini_ui(ctx);
        } else {
            self.full_ui(ctx);
        }
        if self.show_setup {
            // Open next to the overlay, on the side with more room (same monitor).
            if self.setup_geom.is_none() {
                let ppp = ctx
                    .native_pixels_per_point()
                    .unwrap_or_else(|| ctx.pixels_per_point());
                // Reopen where the user last left it (if that spot is still on a
                // monitor); otherwise beside the overlay.
                let remembered = match (self.cfg.settings_pos, self.cfg.settings_size) {
                    (Some(pos), Some(size))
                        if crate::topmost::point_on_screen([pos[0] + 60.0, pos[1] + 16.0], ppp) =>
                    {
                        Some((
                            egui::pos2(pos[0], pos[1]),
                            egui::vec2(size[0].max(640.0), size[1].max(460.0)),
                        ))
                    }
                    _ => None,
                };
                self.setup_geom = remembered.or_else(|| {
                    crate::topmost::settings_placement(frame, [980.0, 700.0], [640.0, 460.0], ppp)
                        .map(|(pos, size)| {
                            (egui::pos2(pos[0], pos[1]), egui::vec2(size[0], size[1]))
                        })
                });
                self.setup_max = self.cfg.settings_maximized;
            }
            self.setup_viewport(ctx);
        } else {
            self.setup_geom = None;
            self.settings_rounded = false;
        }
        self.autosave();
        // Keeps sprites animating and hotkeys responsive.
        ctx.request_repaint_after(Duration::from_millis(40));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4] // transparent window corners
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.cfg.save();
    }
}
