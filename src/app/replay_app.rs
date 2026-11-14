//! Shiny Replay: the page (a big player for the newest clip, the recorder
//! beside it, older clips on a shelf, then the recording settings), and the
//! glue to the recorder (`crate::replay`).

use super::notices::{Notice, NoticeAction};
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::App;
use crate::apps::AppId;
use crate::replay::{self, Clip, ClipKind, Quality, RecEvent, RecState, Recorder, ReplayFile, Res, SaveReq};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Shiny Replay's colour: a recording red.
pub(super) const ROSE: Color32 = Color32::from_rgb(240, 86, 108);
const ROSE2: Color32 = Color32::from_rgb(255, 138, 107);
const GOLD: Color32 = Color32::from_rgb(236, 186, 80);
/// Preview pictures shown per second while a clip's preview plays.
const PREVIEW_FPS: f32 = 6.0;

/// What's known about a clip while it's being written.
struct Pending {
    kind: ClipKind,
    mon: String,
    dex: u32,
    hunt: String,
    encounters: u64,
    when: u64,
}

#[derive(Default)]
pub(super) struct ReplayState {
    pub file: Option<ReplayFile>,
    pub load_error: Option<String>,
    dirty: Option<Instant>,
    saved_once: bool,
    pub save_error: Option<String>,
    rec: Option<Recorder>,
    pending: HashMap<String, Pending>,
    /// The clip in the big player (None: the newest).
    pub selected: Option<String>,
    /// When the big player's preview started playing.
    playing: Option<Instant>,
    /// Preview sheets: id → texture (None if it couldn't be read).
    sheets: HashMap<String, Option<TextureHandle>>,
    pub renaming: Option<(String, String)>,
    pub confirm_delete: Option<String>,
    /// A line under the buttons ("Copied", errors).
    pub msg: Option<(Instant, String, bool)>,
    picking_folder: Option<Receiver<Option<PathBuf>>>,
    /// The settings window (the cog) is open.
    pub settings_open: bool,
    /// The last shiny alert a clip was asked for (one clip per alert).
    alert_saved: bool,
}

impl ReplayState {
    pub fn file(&mut self) -> &mut ReplayFile {
        if self.file.is_none() {
            let (f, note) = replay::load_from(&crate::config::data_dir());
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(1500) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match replay::save_to(&crate::config::data_dir(), f, !self.saved_once) {
            Ok(()) => {
                self.saved_once = true;
                self.dirty = None;
                self.save_error = None;
            }
            Err(e) => {
                self.save_error = Some(e);
                self.dirty = Some(Instant::now());
            }
        }
    }

    pub fn status(&self) -> replay::RecStatus {
        self.rec.as_ref().map(|r| r.status()).unwrap_or_default()
    }
}

/// "Today, 1:42 pm", "Yesterday, 9:15 pm" or "12 Nov, 9:15 pm".
pub(super) fn when_label(unix: u64) -> String {
    let local = unix as i64 + crate::stats::utc_offset_secs();
    let day = local.div_euclid(86_400);
    let today = (crate::stats::now_unix() as i64 + crate::stats::utc_offset_secs()).div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (h, m) = (secs / 3600, secs % 3600 / 60);
    let t = format!("{}:{m:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "am" } else { "pm" });
    match today - day {
        0 => format!("Today, {t}"),
        1 => format!("Yesterday, {t}"),
        _ => format!("{}, {t}", crate::stats::fmt_day_short(day)),
    }
}

/// "Today" / "12 Nov".
pub(super) fn day_label(unix: u64) -> String {
    when_label(unix).split(',').next().unwrap_or("").to_string()
}

fn secs_label(s: f32) -> String {
    let s = s.round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

fn mb(bytes: u64) -> String {
    let m = bytes as f64 / 1_000_000.0;
    if m >= 10.0 {
        format!("{m:.0} MB")
    } else {
        format!("{m:.1} MB")
    }
}

/// Open a file with its usual program (the video player for an MP4).
fn open_file(path: &std::path::Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(windows))]
    let _ = path;
}

impl App {
    /// Every frame: keep the recorder in step with the settings and the
    /// overlay, and take in finished clips.
    pub(super) fn replay_tick(&mut self, ctx: &egui::Context) {
        if !self.has_app(AppId::Replay) {
            if let Some(r) = self.setup.replay.rec.as_mut() {
                r.set(&self.cfg.replay, false);
            }
            return;
        }
        if self.setup.replay.rec.is_none() {
            self.setup.replay.rec = Some(Recorder::spawn(Some(ctx.clone())));
        }
        // (SHINY_REPLAY_DEMO: record without the overlay, for testing)
        let running = self.overlay_on || std::env::var_os("SHINY_REPLAY_DEMO").is_some();
        let events = {
            let r = self.setup.replay.rec.as_mut().expect("spawned");
            r.set(&self.cfg.replay, running);
            r.poll()
        };
        for ev in events {
            self.replay_event(ev);
        }
        if self.shiny_alert.is_none() {
            self.setup.replay.alert_saved = false;
        }
        // a folder picked in the settings
        if let Some(rx) = &self.setup.replay.picking_folder {
            if let Ok(pick) = rx.try_recv() {
                self.setup.replay.picking_folder = None;
                if let Some(dir) = pick {
                    self.cfg.replay.folder = Some(dir);
                    self.dirty = true;
                }
            }
        }
        if self.setup.replay.status().saving > 0 {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        self.setup.replay.save_tick(false);
    }

    /// The shiny alert just fired: save the moment (once per alert).
    pub(super) fn replay_on_shiny(&mut self, plate: usize) {
        if !self.has_app(AppId::Replay) || !self.cfg.replay.on || self.setup.replay.alert_saved {
            return;
        }
        self.setup.replay.alert_saved = true;
        let sp = if plate == super::GIFT_PLATE { None } else { self.status.names.get(plate).and_then(|&i| self.species.get(i)).cloned() };
        let (mon, dex) = sp.map(|s| (s.name.to_string(), s.dex)).unwrap_or_default();
        self.replay_request(ClipKind::Shiny, mon, dex);
    }

    /// "Save replay now" (button or hotkey).
    pub(super) fn replay_save_now(&mut self) {
        if !self.has_app(AppId::Replay) {
            return;
        }
        self.replay_request(ClipKind::Hand, String::new(), 0);
        self.setup.replay.msg = Some((Instant::now(), format!("Saving: the next {} s are recorded first", self.cfg.replay.after), false));
    }

    fn replay_request(&mut self, kind: ClipKind, mon: String, dex: u32) {
        if self.setup.replay.rec.is_none() {
            return;
        }
        let when = replay::now_unix();
        let (name, mut id) = replay::clip_names(kind, &mon, when);
        while self.setup.replay.pending.contains_key(&id) || self.setup.replay.file().clips.iter().any(|c| c.id == id) {
            id.push('b');
        }
        let path = self.cfg.replay.folder().join(name);
        let hunt = self.target_label();
        self.setup.replay.pending.insert(id.clone(), Pending { kind, mon, dex, hunt, encounters: self.cfg.count, when });
        if let Some(rec) = self.setup.replay.rec.as_ref() {
            rec.save(SaveReq { id, path });
        }
    }

    fn replay_event(&mut self, ev: RecEvent) {
        match ev {
            RecEvent::Saved { id, path, secs, alert_at, w, h, fps, bytes, previews, alert_frame } => {
                let p = self.setup.replay.pending.remove(&id).unwrap_or(Pending { kind: ClipKind::Hand, mon: String::new(), dex: 0, hunt: String::new(), encounters: 0, when: replay::now_unix() });
                let (fw, fh) = previews.first().map(|t| (t.w, t.h)).unwrap_or((0, 0));
                if let Some(png) = replay::preview_sheet_png(&previews) {
                    let _ = std::fs::create_dir_all(replay::previews_dir());
                    let _ = std::fs::write(replay::preview_path(&id), png);
                }
                let clip = Clip {
                    id: id.clone(),
                    file: path,
                    kind: p.kind,
                    mon: p.mon,
                    dex: p.dex,
                    hunt: p.hunt,
                    encounters: p.encounters,
                    when: p.when,
                    secs,
                    alert_at,
                    w,
                    h,
                    fps,
                    bytes,
                    frames: previews.len() as u32,
                    alert_frame: alert_frame as u32,
                    fw,
                    fh,
                    title: String::new(),
                };
                let what = format!("{} · {} s", if clip.kind == ClipKind::Shiny { format!("Shiny {}", clip.name()) } else { clip.name() }, secs.round() as u32);
                self.setup.replay.file().add(clip);
                self.setup.replay.selected = None;
                self.setup.replay.playing = None;
                // too many hand-saved clips: the oldest go (shinies never)
                let keep = self.cfg.replay.keep;
                for old in self.setup.replay.file().over_keep(keep) {
                    let _ = std::fs::remove_file(&old.file);
                    let _ = std::fs::remove_file(replay::preview_path(&old.id));
                    self.setup.replay.file().remove(&old.id);
                }
                self.setup.replay.touch();
                self.notify(Notice {
                    id: "replay",
                    icon: "🎬",
                    title: "Shiny Replay saved".into(),
                    body: what.clone(),
                    progress: None,
                    action: Some(("Watch", NoticeAction::WatchReplay)),
                    expires: Some(Instant::now() + Duration::from_secs(10)),
                    accent: ROSE,
                });
                self.app_toast(format!("Shiny Replay saved: {what}"));
            }
            RecEvent::Failed { id, msg } => {
                self.setup.replay.pending.remove(&id);
                self.setup.replay.msg = Some((Instant::now(), format!("Couldn't save the clip: {msg}"), true));
                self.notify(Notice {
                    id: "replay",
                    icon: "⚠",
                    title: "Shiny Replay couldn't save".into(),
                    body: msg,
                    progress: None,
                    action: None,
                    expires: Some(Instant::now() + Duration::from_secs(12)),
                    accent: ROSE,
                });
            }
        }
    }

    /// Open the newest clip in the video player (the notice's Watch).
    pub(super) fn replay_watch_latest(&mut self) {
        self.clear_notice("replay");
        if let Some(c) = self.setup.replay.file().clips.first().cloned() {
            open_file(&c.file);
        }
    }

    /// The preview sheet of a clip, loaded once.
    pub(super) fn replay_sheet(&mut self, ctx: &egui::Context, id: &str) -> Option<TextureHandle> {
        if let Some(t) = self.setup.replay.sheets.get(id) {
            return t.clone();
        }
        let tex = std::fs::read(replay::preview_path(id)).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|img| {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            ctx.load_texture(format!("replay-{id}"), egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), TextureOptions::LINEAR)
        });
        self.setup.replay.sheets.insert(id.to_string(), tex.clone());
        tex
    }

    /// Draw preview frame `i` of a clip into `r` (or a plain stand-in).
    pub(super) fn replay_frame(&mut self, ui: &egui::Ui, r: Rect, c: &Clip, i: u32, rounding: f32) {
        let p = ui.painter();
        p.rect_filled(r, rounding, theme::blend(kit::CARD(), Color32::BLACK, 0.35));
        if c.frames == 0 {
            p.text(r.center(), Align2::CENTER_CENTER, "🎬", egui::FontId::proportional(r.height() * 0.3), MUTED());
            return;
        }
        let Some(tex) = self.replay_sheet(ui.ctx(), &c.id) else {
            p.text(r.center(), Align2::CENTER_CENTER, "🎬", egui::FontId::proportional(r.height() * 0.3), MUTED());
            return;
        };
        let cols = 8.min(c.frames).max(1);
        let rows = c.frames.div_ceil(cols).max(1);
        let i = i.min(c.frames - 1);
        let (cx, cy) = ((i % cols) as f32, (i / cols) as f32);
        let uv = Rect::from_min_size(pos2(cx / cols as f32, cy / rows as f32), vec2(1.0 / cols as f32, 1.0 / rows as f32));
        // keep the picture's shape inside the box
        let aspect = if c.fh > 0 { c.fw as f32 / c.fh as f32 } else { 16.0 / 9.0 };
        let mut ir = r;
        if r.width() / r.height() > aspect {
            ir = Rect::from_center_size(r.center(), vec2(r.height() * aspect, r.height()));
        } else {
            ir = Rect::from_center_size(ir.center(), vec2(r.width(), r.width() / aspect));
        }
        p.image(tex.id(), ir, uv, Color32::WHITE);
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_replay(&mut self, ui: &mut egui::Ui) {
        let st = self.setup.replay.status();
        ui.horizontal(|ui| {
            ui.vertical(|ui| kit::page_header(ui, "SHINY REPLAY", "The moment you found it, saved as a clip."));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                super::app_settings::cog_button(ui, &mut self.setup.replay.settings_open, "Recording settings: quality, frame rate, where clips are saved");
                ui.add_space(10.0);
                let (text, col, live) = self.replay_state_line(&st);
                rec_dot(ui, &text, col, live);
            });
        });
        for e in [self.setup.replay.load_error.clone(), self.setup.replay.save_error.clone()].into_iter().flatten() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(6.0);
        }
        ui.add_space(4.0);
        let clips = self.setup.replay.file().clips.clone();
        let sel = self.setup.replay.selected.clone().and_then(|id| clips.iter().find(|c| c.id == id).cloned()).or_else(|| clips.first().cloned());
        let side = 250.0;
        let gap = 16.0;
        let main_w = (ui.available_width() - side - gap).max(320.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(vec2(main_w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(main_w);
                match &sel {
                    Some(c) => self.replay_player(ui, c),
                    None => self.replay_empty(ui, &st),
                }
            });
            ui.allocate_ui_with_layout(vec2(side, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(side);
                self.replay_side(ui, &st, &clips);
            });
        });
        if clips.len() > 1 {
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(if clips.len() > 1 { "Your clips" } else { "" }).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new("Newest first · click one to play it above").size(12.0).color(MUTED())));
            });
            ui.add_space(8.0);
            self.replay_shelf(ui, &clips, sel.as_ref().map(|c| c.id.clone()));
        }
        ui.add_space(12.0);
        // the settings window (the cog)
        let mut open = self.setup.replay.settings_open;
        let ctx = ui.ctx().clone();
        super::app_settings::window(&ctx, "replay", "Shiny Replay settings", "How clips are recorded, and where they go.", ROSE, &mut open, |ui| self.replay_settings(ui, &st));
        // (closed too when a setting took you to another page)
        self.setup.replay.settings_open = open && self.setup.tab == super::Tab::Replay;
    }

    fn replay_state_line(&self, st: &replay::RecStatus) -> (String, Color32, bool) {
        if !self.cfg.replay.on {
            return ("Recording off".into(), MUTED(), false);
        }
        match &st.state {
            RecState::Recording => ("Recording the game window".into(), ROSE, true),
            RecState::NoGame(_) => ("Waiting for the game window".into(), GOLD, false),
            RecState::Unavailable(_) => ("Recorder not in this build".into(), GOLD, false),
            RecState::Error(_) => ("Recorder problem: retrying".into(), kit::C_RED, false),
            RecState::Off => (if self.overlay_on { "Starting…".into() } else { "Starts with the overlay".into() }, MUTED(), false),
        }
    }

    /// No clips yet: what happens and how.
    fn replay_empty(&mut self, ui: &mut egui::Ui, st: &replay::RecStatus) {
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            let w = ui.available_width();
            let (r, _) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0 * 0.62), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, 12.0, theme::blend(kit::CARD(), Color32::BLACK, 0.3));
            p.text(r.center() - vec2(0.0, 26.0), Align2::CENTER_CENTER, "🎬", egui::FontId::proportional(44.0), ROSE);
            p.text(r.center() + vec2(0.0, 22.0), Align2::CENTER_CENTER, "Your first shiny will play here", crate::fonts::sans_bold_font(17.0), TEXT());
            p.text(r.center() + vec2(0.0, 46.0), Align2::CENTER_CENTER, format!("When the shiny alert fires, the {} s before it and {} s after are saved as an MP4.", self.cfg.replay.before, self.cfg.replay.after), crate::fonts::sans_font(12.5), MUTED());
            ui.add_space(10.0);
            let why = match &st.state {
                RecState::Unavailable(e) | RecState::NoGame(e) | RecState::Error(e) => Some(e.clone()),
                _ if !self.overlay_on => Some("It records while the overlay is running: start the overlay and the game, and the last seconds are always ready.".into()),
                _ => None,
            };
            if let Some(w) = why {
                ui.label(RichText::new(w).size(12.5).color(MUTED()));
            }
        });
    }

    /// The big player: the clip's preview, its moment, and what to do with it.
    fn replay_player(&mut self, ui: &mut egui::Ui, c: &Clip) {
        let mut act: Option<&'static str> = None;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let title = if c.kind == ClipKind::Shiny { format!("✨ Shiny {}", c.name()) } else { c.name() };
                ui.label(RichText::new(title).font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                if c.encounters > 0 && c.kind == ClipKind::Shiny {
                    super::home::chip_col(ui, &format!("{} encounters", crate::stats::thousands(c.encounters)), GOLD);
                }
                if !c.hunt.is_empty() && c.hunt != "No target yet" {
                    super::home::chip(ui, &c.hunt);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(when_label(c.when)).size(12.0).color(MUTED())));
            });
            ui.add_space(8.0);
            // the picture
            let w = ui.available_width();
            let aspect = if c.fh > 0 { c.fw as f32 / c.fh as f32 } else { 16.0 / 9.0 };
            let h = (w / aspect).min(560.0);
            // the picture's own size, in the middle
            let (row, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            let r = Rect::from_center_size(row.center(), vec2((h * aspect).min(w), h));
            let resp = ui.interact(r, ui.id().with(("replay-pic", &c.id)), Sense::click());
            let playing = self.setup.replay.playing.filter(|t| c.frames > 0 && t.elapsed().as_secs_f32() < c.frames as f32 / PREVIEW_FPS);
            let frame = match playing {
                Some(t) => (t.elapsed().as_secs_f32() * PREVIEW_FPS) as u32,
                None => c.alert_frame,
            };
            self.replay_frame(ui, r, c, frame, 12.0);
            let p = ui.painter();
            if playing.is_some() {
                ui.ctx().request_repaint_after(Duration::from_millis(50));
            } else {
                // a play mark over the picture
                let hov = resp.hovered();
                p.circle_filled(r.center(), 30.0, Color32::from_black_alpha(if hov { 170 } else { 120 }));
                let s = 12.0;
                p.add(egui::Shape::convex_polygon(vec![r.center() + vec2(-s * 0.6, -s), r.center() + vec2(s, 0.0), r.center() + vec2(-s * 0.6, s)], Color32::WHITE, Stroke::NONE));
            }
            if !c.exists() {
                let t = "The video file was moved or deleted";
                let tr = Rect::from_min_size(r.left_top() + vec2(10.0, 10.0), vec2(250.0, 24.0));
                p.rect_filled(tr, 12.0, Color32::from_black_alpha(190));
                p.text(tr.center(), Align2::CENTER_CENTER, t, crate::fonts::sans_bold_font(11.5), Color32::from_rgb(255, 150, 150));
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Play the preview here (Watch opens the real video)").clicked() {
                self.setup.replay.playing = if playing.is_some() { None } else { Some(Instant::now()) };
            }
            // the moment: progress, the alert, the times
            ui.add_space(6.0);
            let (br, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
            let p = ui.painter();
            let bar = Rect::from_min_size(pos2(br.left(), br.top() + 12.0), vec2(br.width(), 4.0));
            p.rect_filled(bar, 2.0, LINE());
            let done = match playing {
                Some(t) => (t.elapsed().as_secs_f32() * PREVIEW_FPS / c.frames.max(1) as f32).clamp(0.0, 1.0),
                None => 0.0,
            };
            if done > 0.0 {
                p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * done, 4.0)), 2.0, ROSE);
            }
            if c.secs > 0.0 {
                let ax = bar.left() + bar.width() * (c.alert_at / c.secs).clamp(0.0, 1.0);
                p.line_segment([pos2(ax, bar.top() - 7.0), pos2(ax, bar.bottom() + 3.0)], Stroke::new(2.0f32, GOLD));
                let label = if c.kind == ClipKind::Shiny { "✨ alert" } else { "saved" };
                p.text(pos2(ax, bar.top() - 9.0), Align2::CENTER_BOTTOM, label, crate::fonts::sans_bold_font(10.5), GOLD);
            }
            p.text(pos2(br.left(), br.bottom()), Align2::LEFT_BOTTOM, "0:00", crate::fonts::sans_font(10.5), MUTED());
            p.text(pos2(br.right(), br.bottom()), Align2::RIGHT_BOTTOM, secs_label(c.secs), crate::fonts::sans_font(10.5), MUTED());
            ui.add_space(6.0);
            // what to do with it
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let watch = egui::Button::new(RichText::new("▶  Watch").size(13.0).color(Color32::WHITE)).fill(ROSE).rounding(18.0).min_size(vec2(110.0, 36.0));
                if ui.add_enabled(c.exists(), watch).on_hover_text("Open the video in your video player").clicked() {
                    act = Some("watch");
                }
                if super::home::sec_button(ui, "Show in folder", "Open the clips folder with this video selected") {
                    act = Some("folder");
                }
                if super::home::sec_button(ui, "Copy file", "Copy the video, then paste it into Discord or a folder") {
                    act = Some("copy");
                }
                if super::home::sec_button(ui, "Rename", "Give the clip a name") {
                    act = Some("rename");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if super::home::sec_button(ui, "Delete", "Delete the clip and its video file") {
                        act = Some("delete");
                    }
                    let info = format!("{}×{} · {} fps · {}", c.w, c.h, c.fps, mb(c.bytes));
                    ui.label(RichText::new(info).size(11.5).color(MUTED()));
                });
            });
            // rename / delete / messages
            self.replay_inline(ui, c);
        });
        match act {
            Some("watch") => open_file(&c.file),
            Some("folder") => crate::share::reveal(&c.file),
            Some("copy") => {
                let r = crate::share::copy_file(&c.file);
                self.setup.replay.msg = Some((Instant::now(), match &r { Ok(()) => "Copied: paste it into Discord or a folder".into(), Err(e) => format!("Couldn't copy: {e}") }, r.is_err()));
            }
            Some("rename") => self.setup.replay.renaming = Some((c.id.clone(), c.name())),
            Some("delete") => self.setup.replay.confirm_delete = Some(c.id.clone()),
            _ => {}
        }
    }

    /// Rename box, delete check, and the last message, under the buttons.
    fn replay_inline(&mut self, ui: &mut egui::Ui, c: &Clip) {
        if self.setup.replay.renaming.as_ref().is_some_and(|(id, _)| *id == c.id) {
            ui.add_space(8.0);
            let mut done = None;
            ui.horizontal(|ui| {
                let (_, text) = self.setup.replay.renaming.as_mut().expect("checked");
                let r = ui.add(egui::TextEdit::singleline(text).hint_text("Name").desired_width(260.0));
                r.request_focus();
                if super::home::sec_button(ui, "Save", "Keep this name") || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    done = Some(true);
                }
                if super::home::sec_button(ui, "Cancel", "Keep the old name") || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    done = Some(false);
                }
            });
            if let Some(keep) = done {
                let (id, text) = self.setup.replay.renaming.take().expect("checked");
                if keep {
                    if let Some(x) = self.setup.replay.file().clips.iter_mut().find(|x| x.id == id) {
                        x.title = text.trim().chars().take(60).collect();
                    }
                    self.setup.replay.touch();
                }
            }
        }
        if self.setup.replay.confirm_delete.as_deref() == Some(c.id.as_str()) {
            ui.add_space(8.0);
            let mut done = None;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Delete this clip and its video file? This can't be undone.").size(12.5).color(kit::C_RED));
                if ui.add(egui::Button::new(RichText::new("Delete").size(12.5).color(Color32::WHITE)).fill(kit::C_RED).rounding(14.0)).clicked() {
                    done = Some(true);
                }
                if super::home::sec_button(ui, "Keep it", "Don't delete") {
                    done = Some(false);
                }
            });
            if let Some(yes) = done {
                self.setup.replay.confirm_delete = None;
                if yes {
                    let _ = std::fs::remove_file(&c.file);
                    let _ = std::fs::remove_file(replay::preview_path(&c.id));
                    self.setup.replay.file().remove(&c.id);
                    self.setup.replay.sheets.remove(&c.id);
                    self.setup.replay.selected = None;
                    self.setup.replay.touch();
                }
            }
        }
        if let Some((t, m, bad)) = self.setup.replay.msg.clone() {
            if t.elapsed() < Duration::from_secs(8) {
                ui.add_space(6.0);
                ui.label(RichText::new(m).size(12.0).color(if bad { kit::C_RED } else { MUTED() }));
            } else {
                self.setup.replay.msg = None;
            }
        }
    }

    /// Beside the player: what's ready to save, and the clips.
    fn replay_side(&mut self, ui: &mut egui::Ui, st: &replay::RecStatus, clips: &[Clip]) {
        let mut save = false;
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("READY TO SAVE").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(4.0);
            let recording = st.state == RecState::Recording && self.cfg.replay.on;
            let ready = if recording { st.ready } else { 0.0 };
            ui.label(RichText::new(format!("{ready:.1} s")).font(crate::fonts::sans_bold_font(28.0)).color(if recording { TEXT() } else { MUTED() }));
            let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, kit::sunken());
            let f = (ready / self.cfg.replay.before.max(1) as f32).clamp(0.0, 1.0);
            if f > 0.0 {
                let fr = Rect::from_min_size(r.min, vec2(r.width() * f, r.height()));
                ui.painter().add(theme::gradient_rect(fr, [3.0; 4], ROSE, ROSE2));
            }
            ui.add_space(6.0);
            let line = if recording {
                format!("{:.0} fps · {}×{} · {:.1} Mbps · {} in memory", st.fps.min(self.cfg.replay.fps() as f32), st.w, st.h, st.kbps as f32 / 1000.0, mb(st.mem))
            } else {
                match &st.state {
                    RecState::NoGame(e) | RecState::Unavailable(e) | RecState::Error(e) if self.cfg.replay.on => e.clone(),
                    _ if !self.cfg.replay.on => "Recording is off: turn it on in the settings (⚙ at the top right).".into(),
                    _ => "Starts when the overlay is running.".into(),
                }
            };
            ui.label(RichText::new(line).size(11.5).color(MUTED()));
            if st.saving > 0 {
                ui.label(RichText::new(format!("Writing {} clip{}…", st.saving, if st.saving == 1 { "" } else { "s" })).size(11.5).color(ROSE));
            }
            ui.add_space(8.0);
            let b = egui::Button::new(RichText::new("⏺  Save replay now").size(12.5).color(TEXT())).fill(kit::CARD()).stroke(Stroke::new(1.0f32, LINE())).rounding(16.0).min_size(vec2(0.0, 32.0));
            let hint = self.cfg.keybinds.hint(crate::hotkeys::HotAction::SaveReplay);
            if ui.add_enabled(recording, b).on_hover_text(format!("Save the last {} s and the next {} s, for near misses and funny moments{hint}", self.cfg.replay.before, self.cfg.replay.after)).clicked() {
                save = true;
            }
        });
        ui.add_space(12.0);
        kit::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("CLIPS").font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
            ui.add_space(2.0);
            ui.label(RichText::new(clips.len().to_string()).font(crate::fonts::sans_bold_font(22.0)).color(TEXT()));
            let total: u64 = clips.iter().map(|c| c.bytes).sum();
            let shinies = clips.iter().filter(|c| c.kind == ClipKind::Shiny).count();
            ui.label(RichText::new(format!("{shinies} shin{} · {}", if shinies == 1 { "y" } else { "ies" }, mb(total))).size(11.5).color(MUTED()));
            let folder = self.cfg.replay.folder();
            ui.add(egui::Label::new(RichText::new(folder.display().to_string()).size(11.0).color(MUTED())).truncate()).on_hover_text(folder.display().to_string());
            ui.add_space(6.0);
            if super::home::sec_button(ui, "Open folder", "Open the clips folder") {
                crate::share::open_dir(&folder);
            }
        });
        if save {
            self.replay_save_now();
        }
    }

    /// Older clips, four to a row.
    fn replay_shelf(&mut self, ui: &mut egui::Ui, clips: &[Clip], sel: Option<String>) {
        let cols = 4usize;
        let gap = 12.0;
        let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        for row in clips.chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for c in row {
                    let on = sel.as_deref() == Some(c.id.as_str());
                    let (r, resp) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0 + 40.0), Sense::click());
                    let pic = Rect::from_min_size(r.min, vec2(w, w * 9.0 / 16.0));
                    self.replay_frame(ui, pic, c, c.alert_frame, 8.0);
                    let p = ui.painter();
                    if on || resp.hovered() {
                        p.rect_stroke(pic.expand(2.0), 10.0, Stroke::new(2.0f32, if on { ROSE } else { LINE() }));
                    }
                    let dur = secs_label(c.secs);
                    let dr = Rect::from_min_size(pic.right_bottom() - vec2(44.0, 24.0), vec2(38.0, 18.0));
                    p.rect_filled(dr, 5.0, Color32::from_black_alpha(170));
                    p.text(dr.center(), Align2::CENTER_CENTER, dur, crate::fonts::sans_bold_font(10.5), Color32::WHITE);
                    if c.kind == ClipKind::Shiny {
                        p.text(pic.left_top() + vec2(8.0, 6.0), Align2::LEFT_TOP, "✨", egui::FontId::proportional(15.0), GOLD);
                    }
                    p.text(pos2(r.left(), pic.bottom() + 8.0), Align2::LEFT_TOP, c.name(), crate::fonts::sans_bold_font(12.5), TEXT());
                    p.text(pos2(r.left(), pic.bottom() + 25.0), Align2::LEFT_TOP, day_label(c.when), crate::fonts::sans_font(11.5), MUTED());
                    if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.setup.replay.selected = Some(c.id.clone());
                        self.setup.replay.playing = Some(Instant::now());
                        self.setup.replay.renaming = None;
                        self.setup.replay.confirm_delete = None;
                    }
                }
            });
            ui.add_space(10.0);
        }
    }

    /// Recording settings (in the cog's window): on/off, size, frame rate,
    /// bitrate, where clips go, how many are kept, the hotkey.
    fn replay_settings(&mut self, ui: &mut egui::Ui, st: &replay::RecStatus) {
        use super::app_settings::{group, row};
        let before = self.cfg.replay.clone();
        let mut pick = false;
        let mut hotkeys = false;
        let hk = self.cfg.keybinds.get(crate::hotkeys::HotAction::SaveReplay).map(|k| k.text());
        let s = &mut self.cfg.replay;
        group(ui, "Recording");
        row(ui, "Record while hunting", "Keeps the last seconds of the game window ready while the overlay runs. Only the game window, never the rest of your screen.", |ui| {
            kit::toggle(ui, &mut s.on);
        });
        row(ui, "Resolution", "1080p at most. A smaller game window is recorded at its own size.", |ui| {
            kit::segmented(ui, &mut s.res, &Res::ALL.map(|r| (r, r.label())));
        });
        row(ui, "Frame rate", "60 fps is smoother. 30 fps is lighter on your PC and makes smaller files.", |ui| {
            let mut fps = s.fps();
            if kit::segmented(ui, &mut fps, &[(30u32, "30 fps"), (60u32, "60 fps")]) {
                s.fps = fps;
            }
        });
        row(ui, "Bitrate", "Higher is sharper and makes bigger files. The presets follow the resolution and frame rate.", |ui| {
            kit::segmented(ui, &mut s.quality, &Quality::ALL.map(|q| (q, q.label())));
        });
        if s.quality == Quality::Custom {
            row(ui, "Custom bitrate", "Megabits per second.", |ui| {
                ui.add(egui::Slider::new(&mut s.custom_mbps, replay::MBPS_MIN..=replay::MBPS_MAX).suffix(" Mbps").step_by(0.5));
            });
        }
        // what that makes
        let size = if st.w > 0 && st.state == RecState::Recording { (st.w, st.h) } else { s.size_for((3840, 2160)) };
        let kbps = s.kbps(size);
        let clip = kbps as f32 * s.clip_secs() as f32 / 8.0 / 1000.0;
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{}×{} · {:.1} Mbps · about {:.0} MB per {} s clip", size.0, size.1, kbps as f32 / 1000.0, clip.max(1.0), s.clip_secs())).size(12.0).color(MUTED()));
            if clip > 10.0 {
                ui.label(RichText::new("· bigger than Discord's free 10 MB upload").size(12.0).color(GOLD));
            }
        });
        group(ui, "Saving");
        let folder = s.folder();
        row(ui, "Save clips to", &folder.display().to_string(), |ui| {
            if super::home::sec_button(ui, "Change…", "Pick any folder for new clips") {
                pick = true;
            }
            if super::home::sec_button(ui, "Open", "Open the folder") {
                crate::share::open_dir(&folder);
            }
            if s.folder.is_some() && super::home::sec_button(ui, "Default", "Back to Videos\\Shiny Replays") {
                s.folder = None;
            }
        });
        row(ui, "Keep at most", "Hand-saved clips past this are deleted, oldest first. Shiny clips are never deleted.", |ui| {
            kit::segmented(ui, &mut s.keep, &[(20u32, "20"), (50u32, "50"), (0u32, "All")]);
        });
        group(ui, "Hotkey");
        row(ui, "Save replay now", "Saves the last seconds by hand, right from the game.", |ui| {
            super::home::chip(ui, hk.as_deref().unwrap_or("Not set"));
            if super::home::sec_button(ui, "Set in Hotkeys", "Pick the key combination on the Hotkeys page") {
                hotkeys = true;
            }
        });
        if self.cfg.replay != before {
            self.dirty = true;
        }
        if pick && self.setup.replay.picking_folder.is_none() {
            self.setup.replay.picking_folder = Some(crate::share::pick_folder(self.cfg.replay.folder(), ui.ctx().clone()));
        }
        if hotkeys {
            self.setup.replay.settings_open = false;
            self.go_to(super::Tab::Hotkeys);
        }
    }
}

/// "● Recording the game window": a blinking dot when live.
fn rec_dot(ui: &mut egui::Ui, text: &str, col: Color32, live: bool) {
    let t = ui.input(|i| i.time) as f32;
    ui.label(RichText::new(text).size(12.5).color(if live { TEXT() } else { col }));
    let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    let a = if live { 0.55 + 0.45 * (t * 4.0).sin().abs() } else { 1.0 };
    ui.painter().circle_filled(r.center(), if live { 8.0 } else { 0.0 }, col.gamma_multiply(0.18));
    ui.painter().circle_filled(r.center(), 4.5, col.gamma_multiply(a));
    if live {
        ui.ctx().request_repaint_after(Duration::from_millis(120));
    }
}
