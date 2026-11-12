//! Shared parts of the overlay themes (Settings > Overlay > Theme): what
//! the overlay shows, worked out once a frame (`OvView`), the buttons and
//! what they do (`Act`), the header controls, dragging, the window shape and
//! the notices underneath, plus drawing helpers. Each theme has its own
//! layout in its own file; the themed Bar and the hidden badge are here,
//! drawn from each theme's `Look`.

use super::overlay_mod::{Odds, OvStats, BAR_H, BAR_POPUP_W, BELOW_H, COMPACT_W, FLAME, LIQUID_MAX, LIQUID_MIN};
use super::{App, FULL_SIZE};
use crate::config::{now_unix, OverlayModules, OverlaySize, OverlayTheme};
use crate::hotkeys::HotAction;
use crate::species::Species;
use crate::theme::{self, GOLD};
use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke};
use std::time::Duration;

// ───────────────────────────── what the overlay shows ─────────────────────────────

/// Everything a theme needs to draw the overlay this frame.
pub(super) struct OvView {
    pub m: OverlayModules,
    pub compact: bool,
    pub width: f32,
    pub liquid: bool,
    pub target: Option<Species>,
    pub all: Vec<Species>,
    pub multi: bool,
    /// "Eevee" / "Eevee + Pikachu" (None = no target yet).
    pub title: Option<String>,
    pub count: u64,
    pub counter_label: &'static str,
    pub counter_col: Color32,
    pub odds: Odds,
    pub method: String,
    pub luck: Option<(f64, Color32, String)>,
    pub milestone: (String, String, String),
    /// Encounters and seconds this session.
    pub session: (u64, u64),
    pub pace_hr: u64,
    pub pace: Vec<u64>,
    /// Last encounter's names and how long ago.
    pub last: (String, Option<String>),
    pub chips: Vec<(String, Color32, bool, String)>,
    pub st: OvStats,
    pub goal: u64,
    /// Today's hunting time, idle, and its tooltip.
    pub timer: Option<(String, bool, String)>,
    pub status: (Color32, String),
    pub ghost: bool,
    /// A shiny alert (plate index) and its text.
    pub alert: Option<(usize, String)>,
    /// A milestone being celebrated: seconds in, text.
    pub celebrate: Option<(f32, String)>,
    pub blink: bool,
    /// The target was just seen.
    pub flash: bool,
    /// "SEEN 12" / "NOT SEEN IN 300", dry, tooltip.
    pub watch: Option<(String, bool, String)>,
    pub level: Option<u32>,
    pub t: f32,
}

impl OvView {
    /// Show the sprites (or their space for the alerts).
    pub fn sprites(&self) -> bool {
        self.m.sprites || self.alert.is_some() || self.celebrate.is_some()
    }
    pub fn session_rate(&self) -> String {
        let (enc, secs) = self.session;
        if secs >= 60 {
            format!("{}/h", theme::thousands(enc * 3600 / secs))
        } else {
            "—".to_string()
        }
    }
    pub fn session_time(&self) -> String {
        if self.session.1 > 0 {
            crate::stats::fmt_duration(self.session.1)
        } else {
            "—".to_string()
        }
    }
    /// Odds as "0.42×" (or "—").
    pub fn mult_text(&self) -> String {
        match self.odds.mult {
            Some(v) if v < 0.1 => format!("{v:.2}×"),
            Some(v) => format!("{v:.1}×"),
            None => "—".to_string(),
        }
    }
    /// "UNDER ODDS" / "AT ODDS" / "OVER ODDS" / "ODDS".
    pub fn odds_word(&self) -> &'static str {
        match self.odds.mult {
            Some(v) if v < 0.9 => "UNDER ODDS",
            Some(v) if v <= 1.1 => "AT ODDS",
            Some(_) => "OVER ODDS",
            None => "ODDS",
        }
    }
}

/// What a button does.
#[derive(Clone, PartialEq)]
pub(super) enum Act {
    Minus,
    Plus,
    Pause,
    Caught,
    /// It was this target (None = the main one).
    Log(Option<String>),
    Phase,
    NotShiny,
    Choose,
    Undo,
    UndoOk,
    Dismiss,
}

/// How a button should look: an everyday one, the main one (Caught!), a
/// "yes, it was this one" and the gold Phase.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Plain,
    Main,
    Good,
    Gold,
}

pub(super) struct ActBtn {
    pub act: Act,
    pub label: String,
    pub tip: String,
    pub kind: Kind,
    /// Share of the row's width.
    pub weight: f32,
}

/// The buttons showing now, with a question in front when one is asked.
pub(super) struct Actions {
    pub prompt: Option<String>,
    pub btns: Vec<ActBtn>,
    /// The usual −1 / +1 / pause / Caught! row.
    pub normal: bool,
}

/// Header controls.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Ctrl {
    Pin,
    Setup,
    Hide,
    Close,
}

impl App {
    /// The overlay's numbers and states for this frame.
    pub(super) fn ov_view(&mut self) -> OvView {
        let mut m = self.cfg.overlay.clone();
        let compact = m.size == OverlaySize::Compact;
        if compact {
            let keep = m.clone();
            m = OverlayModules {
                target: false,
                buttons: false,
                odds_bar: false,
                method_line: false,
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
                ..keep
            };
        }
        let liquid = self.cfg.liquid && !compact;
        let width = if compact {
            COMPACT_W + 24.0
        } else if liquid {
            self.cfg.liquid_width.clamp(LIQUID_MIN, LIQUID_MAX).max(280.0)
        } else {
            FULL_SIZE[0] + 30.0
        };
        let st = self.ov_stats();
        let target = self.target_idx().map(|i| self.species[i].clone());
        let all: Vec<Species> = self.all_target_idx().into_iter().map(|i| self.species[i].clone()).collect();
        let multi = all.len() > 1;
        let (counter_label, counter_col) = self.counter_label();
        let mut method = self.cfg.odds_caption();
        if let Some(one_in) = self.cfg.odds_one_in() {
            if self.cfg.pokemon_per_battle().map_or(true, |n| n > 1.0) {
                method.push_str(&format!(" · 1/{} each", super::fmt_one_in(one_in)));
            }
        }
        let session = self.cfg.run_stats.current_session(now_unix()).map_or((0, 0), |s| (s.encounters, s.secs));
        let names: Vec<&str> = self.last_battle.iter().filter_map(|&i| self.species.get(i)).map(|s| s.name).collect();
        let last_text = if names.is_empty() { "—".to_string() } else { names.join(" · ") };
        let ago = self.last_enc_at.map(|t| {
            let s = t.elapsed().as_secs();
            if s < 60 {
                format!("{s}s ago")
            } else if s < 3600 {
                format!("{}m ago", s / 60)
            } else {
                format!("{}h ago", s / 3600)
            }
        });
        let timer = self.cfg.show_session_timer.then(|| {
            let idle = self.timer_idle();
            let tip = if idle {
                format!("Timer paused: no encounter for over {} minutes. It carries on with the next one.", self.cfg.idle_gap_secs() / 60)
            } else {
                "Time hunted today".to_string()
            };
            (crate::stats::fmt_duration(self.today_hunt_secs()), idle, tip)
        });
        let t = self.t_ms();
        let flash = self.target_flash.is_some_and(|f| f.elapsed() < Duration::from_millis(1600));
        let watch = if !flash && !compact {
            self.target_watch().map(|w| {
                let text = if w.dry { format!("NOT SEEN IN {}", theme::thousands(w.since.unwrap_or(0))) } else { format!("SEEN {}", theme::thousands(w.seen)) };
                let tip = target.as_ref().map(|s| w.tip(s.name)).unwrap_or_default();
                (text, w.dry, tip)
            })
        } else {
            None
        };
        let celebrate = self.celebrate.clone().and_then(|(at, text)| {
            let el = at.elapsed().as_secs_f32();
            if el > 5.0 || self.shiny_alert.is_some() {
                self.celebrate = None;
                None
            } else {
                Some((el, text))
            }
        });
        OvView {
            compact,
            width,
            liquid,
            title: self.targets_title(),
            count: self.cfg.count,
            counter_label,
            counter_col,
            odds: self.ov_odds(),
            method,
            luck: self.ov_luck(),
            milestone: self.ov_next_milestone(),
            session,
            pace_hr: self.ov_pace_per_hour(),
            pace: self.ov_pace(20),
            last: (last_text, ago),
            chips: self.ov_chips(&m, &st),
            goal: self.cfg.daily_goal,
            st,
            timer,
            status: self.status_line(),
            ghost: self.click_through_active(),
            alert: self.shiny_alert.map(|i| (i, super::shiny_alert_text(i))),
            celebrate,
            blink: (t / 350) % 2 == 0,
            flash,
            watch,
            level: (self.cfg.level_on_overlay).then(|| self.cfg.level_seen.max(1)),
            t: t as f32 / 1000.0,
            target,
            all,
            multi,
            m,
        }
    }

    /// The buttons to show now.
    pub(super) fn ov_actions(&self, buttons_on: bool) -> Actions {
        let undo_left = if self.confirm_caught { None } else { self.undo_caught_left() };
        let btn = |act: Act, label: &str, tip: &str, kind: Kind, weight: f32| ActBtn { act, label: label.to_string(), tip: tip.to_string(), kind, weight };
        if let Some(left) = undo_left {
            return Actions {
                prompt: Some("✔ Logged as caught".to_string()),
                btns: vec![
                    btn(Act::Undo, &format!("Undo ({left})"), "Pressed Caught! by mistake? Put everything back", Kind::Plain, 2.0),
                    btn(Act::UndoOk, "OK", "Keep it", Kind::Plain, 1.0),
                ],
                normal: false,
            };
        }
        if self.confirm_caught {
            let all = self.all_target_idx();
            let mut btns = Vec::new();
            let short = |n: &str| if n.chars().count() > 8 { format!("{}…", n.chars().take(7).collect::<String>()) } else { n.to_string() };
            if all.len() > 3 {
                btns.push(btn(Act::Choose, "Choose…", "Pick which of your targets it was", Kind::Good, 2.0));
            } else if all.len() <= 1 {
                btns.push(btn(Act::Log(self.cfg.target.clone()), "Target", "Your target! Logs this hunt and starts fresh", Kind::Good, 2.0));
            } else {
                for &i in &all {
                    let sp = &self.species[i];
                    btns.push(btn(Act::Log(Some(sp.slug.clone())), &short(sp.name), &format!("Shiny {}! Logs this hunt and starts fresh", sp.name), Kind::Good, 2.0));
                }
            }
            btns.push(btn(Act::Phase, "Phase", "A shiny of a different Pokémon: pick which, and this hunt keeps going", Kind::Gold, 1.6));
            btns.push(btn(Act::NotShiny, "✖", "Not a shiny after all", Kind::Plain, 0.8));
            return Actions { prompt: Some(if all.len() > 1 { "Which?" } else { "Shiny! Was it…" }.to_string()), btns, normal: false };
        }
        if !buttons_on {
            return Actions { prompt: None, btns: Vec::new(), normal: false };
        }
        let k = &self.cfg.keybinds;
        Actions {
            prompt: None,
            btns: vec![
                btn(Act::Minus, "−1", &format!("Remove one{}", k.hint(HotAction::Minus)), Kind::Plain, 0.18),
                btn(Act::Plus, "+1", &format!("Add one{}", k.hint(HotAction::Plus)), Kind::Plain, 0.18),
                btn(
                    Act::Pause,
                    if self.cfg.paused { "▶" } else { "⏸" },
                    &format!("Pause / resume auto count{}", k.hint(HotAction::Pause)),
                    Kind::Plain,
                    0.22,
                ),
                btn(Act::Caught, "Caught!", "Found your shiny? Log this hunt and start fresh", Kind::Main, 0.42),
            ],
            normal: true,
        }
    }

    /// Do what a button says.
    pub(super) fn ov_do(&mut self, act: &Act) {
        match act {
            Act::Minus => self.add(-1),
            Act::Plus => self.add(1),
            Act::Pause => self.set_paused(!self.cfg.paused),
            Act::Caught => self.confirm_caught = true,
            Act::Log(slug) => {
                self.log_hunt_as(slug.clone());
                self.confirm_caught = false;
            }
            Act::Phase => {
                self.confirm_caught = false;
                self.setup.phase_pick = true;
                self.setup.search.clear();
                self.show_setup = true;
            }
            Act::NotShiny => self.confirm_caught = false,
            Act::Choose => {
                self.setup.caught_pick = true;
                self.show_setup = true;
            }
            Act::Undo => self.undo_caught_now(),
            Act::UndoOk => self.undo_caught = None,
            Act::Dismiss => {
                self.shiny_alert = None;
                crate::sound::stop_song();
            }
        }
    }

    /// A themed button: `draw(painter, rect, label, kind, hovered, down)`
    /// paints it. Returns true when clicked (and does the action).
    pub(super) fn ov_button(&mut self, ui: &mut egui::Ui, p: &Painter, r: Rect, b: &ActBtn, draw: &dyn Fn(&Painter, Rect, &str, Kind, bool, bool)) -> bool {
        let id = ui.id().with(("ovb", &b.label, r.min.x as i32, r.min.y as i32));
        let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        draw(p, r, &b.label, b.kind, resp.hovered(), resp.is_pointer_button_down_on());
        let clicked = resp.on_hover_text(&b.tip).clicked();
        if clicked {
            self.ov_do(&b.act);
        }
        if b.act == Act::Undo || b.act == Act::UndoOk {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        clicked
    }

    /// A header control: handles the click; returns (icon, on, hovered) to draw.
    pub(super) fn ov_ctrl(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, r: Rect, c: Ctrl) -> (bool, bool) {
        let resp = ui.interact(r, ui.id().with(("ov-ctrl", c as u8)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let hovered = resp.hovered();
        let tip = match c {
            Ctrl::Pin => "Pin in place".to_string(),
            Ctrl::Setup => "Setup: target, screen boxes, odds, overlay sections".to_string(),
            Ctrl::Hide => format!("Hide to badge{}", self.cfg.keybinds.hint(HotAction::ToggleHide)),
            Ctrl::Close => "Close the overlay (the Companion keeps running)".to_string(),
        };
        if resp.on_hover_text(tip).clicked() {
            match c {
                Ctrl::Pin => {
                    self.cfg.pinned = !self.cfg.pinned;
                    self.dirty = true;
                }
                Ctrl::Setup => self.show_setup = !self.show_setup,
                Ctrl::Hide => {
                    self.cfg.mini = true;
                    self.dirty = true;
                }
                Ctrl::Close => self.save_and_quit(ctx),
            }
        }
        let on = match c {
            Ctrl::Pin => self.cfg.pinned,
            Ctrl::Setup => self.show_setup,
            _ => false,
        };
        (on, hovered)
    }

    /// Drag the overlay by `r` (unless pinned).
    pub(super) fn ov_drag(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, r: Rect, id: &str) {
        let resp = ui.interact(r, ui.id().with(id), Sense::click_and_drag());
        if resp.drag_started() && !self.cfg.pinned {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        resp.on_hover_text(if self.cfg.pinned { "Pinned: unpin to move" } else { "Drag to move" });
    }

    /// The window's shape (`shapes`: rounded rects the overlay covers),
    /// notices and pop-ups underneath, the liquid width grip and the window
    /// size. `full` is the overlay's outline.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_finish(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView, full: Rect, shapes: Vec<[f32; 5]>, bottom: f32, grip: Color32) {
        let win_top = ui.max_rect().min.y;
        let below = !self.notices.is_empty() || self.toasts_pending();
        self.ov_shape = shapes;
        if below {
            let area = Rect::from_min_max(egui::pos2(full.min.x, bottom + 6.0), egui::pos2(full.max.x, bottom + 6.0 + BELOW_H - 8.0));
            if !self.notices.is_empty() {
                self.draw_notice(ui, ctx, area, self.cfg.overlay.theme);
            }
            if !self.toasts.is_empty() {
                let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts")));
                self.draw_toast(ctx, &fg, area, self.cfg.overlay.theme);
            }
            let shape = self.popup_shape(area, self.cfg.overlay.theme);
            self.ov_shape.push(shape);
        }
        if v.liquid {
            let g = Rect::from_min_max(egui::pos2(full.max.x - 7.0, full.min.y + 40.0), egui::pos2(full.max.x, full.max.y - 14.0));
            let gr = ui.interact(g, ui.id().with("liquid-edge"), Sense::drag());
            if gr.hovered() || gr.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                let x = full.max.x - 3.0;
                ui.painter().line_segment([egui::pos2(x, g.center().y - 16.0), egui::pos2(x, g.center().y + 16.0)], Stroke::new(3.0f32, grip));
            }
            if gr.dragged() {
                self.cfg.liquid_width = (v.width + gr.drag_delta().x).clamp(LIQUID_MIN, LIQUID_MAX);
            }
            if gr.drag_stopped() {
                self.dirty = true;
            }
            gr.on_hover_text("Drag to resize the overlay");
        }
        let w = (full.max.x - ui.max_rect().min.x + 1.0).max(v.width);
        self.ov_size = Some([w.ceil(), (bottom - win_top + 1.0 + if below { BELOW_H + 6.0 } else { 0.0 }).ceil()]);
    }

    /// Draw the overlay in a theme (Full / Compact).
    pub(super) fn themed_ui(&mut self, ctx: &egui::Context, theme: OverlayTheme) {
        if self.cfg.overlay.size == OverlaySize::Bar {
            self.themed_bar(ctx, theme);
            return;
        }
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let v = self.ov_view();
            match theme {
                OverlayTheme::Anime => self.anime_ui(ui, ctx, &v),
                OverlayTheme::Glass => self.glass_ui(ui, ctx, &v),
                OverlayTheme::Retro => self.retro_ui(ui, ctx, &v),
                OverlayTheme::Neon => self.neon_ui(ui, ctx, &v),
                OverlayTheme::Poketch => self.poketch_ui(ui, ctx, &v),
                OverlayTheme::Journal => self.journal_ui(ui, ctx, &v),
                OverlayTheme::Holo => self.holo_ui(ui, ctx, &v),
                OverlayTheme::Broadcast => self.broadcast_ui(ui, ctx, &v),
                OverlayTheme::Terminal => self.terminal_ui(ui, ctx, &v),
                OverlayTheme::Classic => {}
            }
            if v.celebrate.is_some() || v.alert.is_some() {
                ctx.request_repaint();
            }
        });
    }
}

// ───────────────────────────── drawing helpers ─────────────────────────────

/// Somewhere to draw shapes: the painter, or a list kept to put under
/// what's drawn later (a background whose size is only known at the end).
pub(super) trait Sink {
    fn put(&self, s: Shape);
}

impl Sink for Painter {
    fn put(&self, s: Shape) {
        self.add(s);
    }
}

#[derive(Default)]
pub(super) struct Rec(pub std::cell::RefCell<Vec<Shape>>);

impl Sink for Rec {
    fn put(&self, s: Shape) {
        self.0.borrow_mut().push(s);
    }
}

impl Rec {
    pub fn shape(self) -> Shape {
        Shape::Vec(self.0.into_inner())
    }
}

pub(super) fn rrect(s: &dyn Sink, r: Rect, rad: f32, fill: Color32) {
    s.put(Shape::rect_filled(r, rad, fill));
}

pub(super) fn rstroke(s: &dyn Sink, r: Rect, rad: f32, stroke: Stroke) {
    s.put(Shape::rect_stroke(r, rad, stroke));
}

pub(super) fn circle(s: &dyn Sink, c: Pos2, r: f32, fill: Color32) {
    s.put(Shape::circle_filled(c, r, fill));
}

pub(super) fn line(s: &dyn Sink, a: Pos2, b: Pos2, stroke: Stroke) {
    s.put(Shape::line_segment([a, b], stroke));
}

pub(super) fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// `c` at `a` of its opacity.
pub(super) fn fade(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

pub(super) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    theme::blend(a, b, t)
}

pub(super) fn text_w(p: &Painter, s: &str, font: FontId) -> f32 {
    p.layout_no_wrap(s.to_string(), font, Color32::WHITE).size().x
}

/// The largest size from `max` down to `min` at which `s` fits `w`.
pub(super) fn fit(p: &Painter, s: &str, font: fn(f32) -> FontId, max: f32, min: f32, w: f32) -> FontId {
    let mut size = max;
    while size > min && text_w(p, s, font(size)) > w {
        size -= 0.5;
    }
    font(size)
}

/// Shorten with "…" to fit `w`.
pub(super) fn clip_text(p: &Painter, s: &str, font: &FontId, w: f32) -> String {
    if text_w(p, s, font.clone()) <= w {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let t: String = chars.iter().collect::<String>().trim_end().to_string() + "…";
        if text_w(p, &t, font.clone()) <= w {
            return t;
        }
    }
    "…".into()
}

fn ring(n: usize, r: f32) -> Vec<egui::Vec2> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            egui::vec2(a.cos() * r, a.sin() * r)
        })
        .collect()
}

/// Text with a thick outline (and a hard shadow): comic lettering.
#[allow(clippy::too_many_arguments)]
pub(super) fn lettering(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, fill: Color32, ink: Color32, t: f32, shadow: Option<Color32>) -> Rect {
    if let Some(sc) = shadow {
        let off = (font.size / 12.0).clamp(1.5, 4.0);
        for d in ring(12, t) {
            p.text(pos + d + egui::vec2(off, off), anchor, s, font.clone(), sc);
        }
    }
    for d in ring(12, t).into_iter().chain(ring(8, t * 0.5)) {
        p.text(pos + d, anchor, s, font.clone(), ink);
    }
    p.text(pos, anchor, s, font, fill)
}

/// Text with a soft glow around it.
pub(super) fn glow_text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, fill: Color32, glow: Color32, spread: f32) -> Rect {
    let rings: &[(f32, f32)] = if font.size < 16.0 { &[(1.0, 0.06), (0.5, 0.12)] } else { &[(1.0, 0.10), (0.6, 0.16), (0.3, 0.24)] };
    for &(k, a) in rings {
        let r = spread * k;
        for d in ring(10, r) {
            p.text(pos + d, anchor, s, font.clone(), fade(glow, a));
        }
    }
    p.text(pos, anchor, s, font, fill)
}

/// Points round a rounded rectangle (clockwise from the top-left).
pub(super) fn rrect_pts(r: Rect, rad: f32) -> Vec<Pos2> {
    let rad = rad.min(r.width() / 2.0).min(r.height() / 2.0).max(0.0);
    let mut pts = Vec::new();
    let corners = [
        (egui::pos2(r.max.x - rad, r.min.y + rad), -90.0f32),
        (egui::pos2(r.max.x - rad, r.max.y - rad), 0.0),
        (egui::pos2(r.min.x + rad, r.max.y - rad), 90.0),
        (egui::pos2(r.min.x + rad, r.min.y + rad), 180.0),
    ];
    for (c, a0) in corners {
        for k in 0..=6 {
            let a = (a0 + k as f32 * 15.0).to_radians();
            pts.push(c + egui::vec2(a.cos() * rad, a.sin() * rad));
        }
    }
    pts
}

/// A convex shape filled with colour by height (`top` to `bottom`).
pub(super) fn vgrad(p: &dyn Sink, pts: &[Pos2], top: Color32, bottom: Color32) {
    if pts.len() < 3 {
        return;
    }
    let (y0, y1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), q| (a.min(q.y), b.max(q.y)));
    let col = |y: f32| mix(top, bottom, ((y - y0) / (y1 - y0).max(1.0)).clamp(0.0, 1.0));
    let c = pts.iter().fold(egui::Vec2::ZERO, |s, q| s + q.to_vec2()) / pts.len() as f32;
    let c = c.to_pos2();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, col(c.y));
    for q in pts {
        mesh.colored_vertex(*q, col(q.y));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.put(Shape::mesh(mesh));
}

/// A convex shape filled with colour from left to right.
pub(super) fn hgrad(p: &dyn Sink, pts: &[Pos2], left: Color32, right: Color32) {
    if pts.len() < 3 {
        return;
    }
    let (x0, x1) = pts.iter().fold((f32::MAX, f32::MIN), |(a, b), q| (a.min(q.x), b.max(q.x)));
    let col = |x: f32| mix(left, right, ((x - x0) / (x1 - x0).max(1.0)).clamp(0.0, 1.0));
    let c = (pts.iter().fold(egui::Vec2::ZERO, |s, q| s + q.to_vec2()) / pts.len() as f32).to_pos2();
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, col(c.x));
    for q in pts {
        mesh.colored_vertex(*q, col(q.x));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.put(Shape::mesh(mesh));
}

pub(super) fn poly(p: &dyn Sink, pts: Vec<Pos2>, fill: Color32) {
    p.put(Shape::convex_polygon(pts, fill, Stroke::NONE));
}

pub(super) fn outline(p: &dyn Sink, pts: Vec<Pos2>, stroke: Stroke) {
    p.put(Shape::closed_line(pts, stroke));
}

/// A spiky star burst (`n` points) with an outline.
#[allow(clippy::too_many_arguments)]
pub(super) fn burst(p: &dyn Sink, c: Pos2, r_in: f32, r_out: f32, n: usize, fill: Color32, ink: Color32, edge: f32, turn: f32) {
    let spikes = |grow: f32, col: Color32| {
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + turn;
            let half = std::f32::consts::PI / n as f32;
            let (ri, ro) = (r_in + grow, r_out + grow * 1.6);
            poly(
                p,
                vec![
                    c + egui::vec2((a - half).cos() * ri, (a - half).sin() * ri),
                    c + egui::vec2(a.cos() * ro, a.sin() * ro),
                    c + egui::vec2((a + half).cos() * ri, (a + half).sin() * ri),
                    c,
                ],
                col,
            );
        }
        circle(p, c, r_in + grow, col);
    };
    if edge > 0.0 {
        spikes(edge, ink);
    }
    spikes(0.0, fill);
}

/// Dots that grow towards a corner (`fx`, `fy` = 0..1 inside `r`).
pub(super) fn halftone(p: &dyn Sink, r: Rect, step: f32, fx: f32, fy: f32, col: Color32) {
    let o = egui::pos2(r.min.x + r.width() * fx, r.min.y + r.height() * fy);
    let reach = r.size().length() * 0.75;
    let mut y = r.min.y;
    let mut row = 0;
    while y <= r.max.y {
        let mut x = r.min.x + if row % 2 == 1 { step / 2.0 } else { 0.0 };
        while x <= r.max.x {
            let d = (egui::pos2(x, y) - o).length() / reach;
            let rad = step * 0.42 * (1.0 - d).clamp(0.0, 1.0);
            if rad > 0.5 {
                circle(p, egui::pos2(x, y), rad, col);
            }
            x += step;
        }
        y += step;
        row += 1;
    }
}

/// Thin wedges radiating from `c` (clip the painter to keep them inside).
pub(super) fn speed_lines(p: &dyn Sink, c: Pos2, r0: f32, r1: f32, n: usize, col: Color32, seed: u32) {
    let mut s = seed.wrapping_mul(2654435761).max(1);
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s % 10_000) as f32 / 10_000.0
    };
    for i in 0..n {
        let a = (i as f32 + rnd() * 0.6) / n as f32 * std::f32::consts::TAU;
        let half = 0.012 + rnd() * 0.02;
        let inner = r0 * (0.8 + rnd() * 0.4);
        poly(
            p,
            vec![
                c + egui::vec2(a.cos() * inner, a.sin() * inner),
                c + egui::vec2((a - half).cos() * r1, (a - half).sin() * r1),
                c + egui::vec2((a + half).cos() * r1, (a + half).sin() * r1),
            ],
            col,
        );
    }
}

/// Falling confetti over `area` (`el` seconds in, fading).
pub(super) fn confetti(p: &dyn Sink, area: Rect, el: f32, cols: &[Color32], alpha: f32) {
    for i in 0..36u32 {
        let seed = i.wrapping_mul(2654435761);
        let fx = (seed % 1000) as f32 / 1000.0;
        let speed = 40.0 + (seed / 1000 % 60) as f32;
        let fy = ((el * speed + (seed / 7 % 140) as f32) % (area.height() + 20.0)) - 10.0;
        let wob = (el * 3.0 + i as f32).sin() * 6.0;
        let pos = egui::pos2(area.min.x + fx * area.width() + wob, area.min.y + fy);
        let c = fade(cols[i as usize % cols.len()], alpha);
        let sz = 2.0 + (i % 3) as f32;
        rrect(p, Rect::from_center_size(pos, egui::vec2(sz * 1.6, sz)), 1.0, c);
    }
}

/// How visible a celebration is `el` seconds in.
pub(super) fn celebrate_fade(el: f32) -> f32 {
    (el / 0.25).min(1.0) * ((5.0 - el) / 0.6).min(1.0)
}

/// A header control's icon in `col`.
pub(super) fn ctrl_icon(p: &Painter, r: Rect, c: Ctrl, on: bool, col: Color32, size: f32) {
    let cc = r.center();
    match c {
        Ctrl::Pin => {
            // A padlock: body and shackle (open when not pinned).
            let body = Rect::from_center_size(cc + egui::vec2(0.0, size * 0.18), egui::vec2(size * 0.62, size * 0.46));
            p.rect_filled(body, 1.5, col);
            let sh = cc + egui::vec2(0.0, -size * 0.06);
            let w = size * 0.2;
            let pts: Vec<Pos2> = (0..=8)
                .map(|k| {
                    let a = std::f32::consts::PI + k as f32 / 8.0 * std::f32::consts::PI;
                    sh + egui::vec2(a.cos() * w, a.sin() * w * 1.2)
                })
                .collect();
            let lift = if on { 0.0 } else { -size * 0.12 };
            let mut pts: Vec<Pos2> = pts.into_iter().map(|q| q + egui::vec2(0.0, lift)).collect();
            pts.insert(0, egui::pos2(sh.x - w, body.min.y));
            if on {
                pts.push(egui::pos2(sh.x + w, body.min.y));
            }
            p.add(Shape::line(pts, Stroke::new(size * 0.12, col)));
        }
        Ctrl::Setup => {
            // A cog.
            let teeth = 8;
            for k in 0..teeth {
                let a = k as f32 / teeth as f32 * std::f32::consts::TAU;
                let d = egui::vec2(a.cos(), a.sin());
                p.line_segment([cc + d * size * 0.22, cc + d * size * 0.44], Stroke::new(size * 0.16, col));
            }
            p.circle_filled(cc, size * 0.3, col);
            p.circle_filled(cc, size * 0.12, if on { GOLD } else { Color32::TRANSPARENT });
            if !on {
                p.circle_stroke(cc, size * 0.1, Stroke::new(size * 0.06, col));
            }
        }
        Ctrl::Hide => {
            p.line_segment([cc + egui::vec2(-size * 0.3, size * 0.1), cc + egui::vec2(size * 0.3, size * 0.1)], Stroke::new(size * 0.13, col));
        }
        Ctrl::Close => {
            let d = size * 0.26;
            let s = Stroke::new(size * 0.13, col);
            p.line_segment([cc - egui::vec2(d, d), cc + egui::vec2(d, d)], s);
            p.line_segment([cc + egui::vec2(-d, d), cc + egui::vec2(d, -d)], s);
        }
    }
}

/// Tooltip over a rectangle.
pub(super) fn tip(ui: &mut egui::Ui, r: Rect, id: impl std::hash::Hash, text: &str) {
    if !text.is_empty() {
        ui.interact(r, ui.id().with(("ov-tip", id)), Sense::hover()).on_hover_text(text);
    }
}

// ───────────────────────────── bar and badge ─────────────────────────────

/// A theme's colours and fonts, for the slim Bar and the hidden badge.
pub(super) struct Look {
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub radius: f32,
    pub num: fn(f32) -> FontId,
    pub label: fn(f32) -> FontId,
    pub body: fn(f32) -> FontId,
    /// Background and frame (rect, opacity, alert blink).
    pub panel: fn(&dyn Sink, Rect, f32, bool),
    /// Big text (counter) style.
    pub big: fn(&Painter, Pos2, Align2, &str, FontId, Color32) -> Rect,
    pub button: fn(&Painter, Rect, &str, Kind, bool, bool),
    pub gauge: fn(&dyn Sink, Rect, f32, Color32),
    pub divider: fn(&dyn Sink, Pos2, Pos2),
    /// Make a status colour (green / yellow / red) readable on this theme.
    pub tone: fn(Color32) -> Color32,
    /// Extra window shapes beyond the panel (e.g. a hard shadow).
    pub shadow: Option<egui::Vec2>,
}

pub(super) fn look(theme: OverlayTheme) -> Look {
    match theme {
        OverlayTheme::Anime => super::ov_anime::look(),
        OverlayTheme::Glass => super::ov_glass::look(),
        OverlayTheme::Retro => super::ov_retro::look(),
        OverlayTheme::Neon => super::ov_neon::look(),
        OverlayTheme::Poketch => super::ov_poketch::look(),
        OverlayTheme::Journal => super::ov_journal::look(),
        OverlayTheme::Holo => super::ov_holo::look(),
        OverlayTheme::Broadcast => super::ov_broadcast::look(),
        OverlayTheme::Terminal => super::ov_terminal::look(),
        OverlayTheme::Classic => super::ov_anime::look(),
    }
}

impl App {
    /// The slim Bar in a theme: sprite, name + count, odds, stats, buttons,
    /// then the controls.
    pub(super) fn themed_bar(&mut self, ctx: &egui::Context, theme: OverlayTheme) {
        let lk = look(theme);
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let v = self.ov_view();
            let m = v.m.clone();
            let win = ui.max_rect();
            let p = ui.painter().clone();
            let top = win.min.y + 1.0;
            let left = win.min.x + 1.0;
            let mid = top + BAR_H / 2.0;
            let bg_slot = p.add(Shape::Noop);
            let drag_w = self.ov_size.map_or(400.0, |s| s[0]);
            self.ov_drag(ui, ctx, Rect::from_min_size(egui::pos2(left, top), egui::vec2(drag_w, BAR_H)), "tbar-drag");

            let mut x = left + 12.0;
            let div = |x: &mut f32| {
                (lk.divider)(&p, egui::pos2(*x, top + 12.0), egui::pos2(*x, top + BAR_H - 12.0));
                *x += 10.0;
            };
            if m.sprites {
                match v.target.clone() {
                    Some(sp) => {
                        self.draw_sprite(&p, &sp, true, egui::pos2(x + 22.0, top + BAR_H - 5.0), egui::vec2(44.0, 42.0));
                    }
                    None => {
                        p.image(self.icon_tex.id(), Rect::from_center_size(egui::pos2(x + 22.0, mid), egui::vec2(28.0, 28.0)), Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                }
                x += 50.0;
            }
            if let Some((_, text)) = &v.alert {
                let r1 = (lk.big)(&p, egui::pos2(x, top + 17.0), Align2::LEFT_CENTER, "SHINY!", (lk.num)(17.0), if v.blink { GOLD } else { lk.text });
                let r2 = p.text(egui::pos2(x, top + 36.0), Align2::LEFT_CENTER, text, (lk.body)(11.0), lk.text);
                x = r1.max.x.max(r2.max.x) + 10.0;
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(egui::pos2(x, mid - 12.0), egui::vec2(66.0, 24.0)), &b, &lk.button);
                x += 76.0;
            } else if m.target || m.counter {
                let mut w: f32 = 0.0;
                if m.target {
                    let name = v.title.clone().unwrap_or_else(|| "Pick a target".to_string()).to_uppercase();
                    let col = if v.flash && v.blink { GOLD } else if v.target.is_some() { lk.text } else { lk.muted };
                    let y = if m.counter { top + 15.0 } else { mid };
                    w = w.max(p.text(egui::pos2(x, y), Align2::LEFT_CENTER, name, (lk.label)(10.5), col).width());
                }
                if m.counter {
                    let y = if m.target { top + 33.0 } else { mid };
                    let r = (lk.big)(&p, egui::pos2(x, y), Align2::LEFT_CENTER, &theme::thousands(v.count), (lk.num)(20.0), lk.text);
                    let mut ww = r.width();
                    if v.counter_label != "ENCOUNTERS" {
                        ww += 8.0 + p.text(egui::pos2(r.max.x + 8.0, y + 1.0), Align2::LEFT_CENTER, v.counter_label, (lk.label)(9.0), (lk.tone)(v.counter_col)).width();
                    }
                    w = w.max(ww);
                }
                x += w + 12.0;
            }
            if m.odds_bar {
                div(&mut x);
                let col = (lk.tone)(v.odds.color);
                p.text(egui::pos2(x, top + 16.0), Align2::LEFT_CENTER, format!("{} ODDS", v.mult_text()), (lk.label)(10.0), col);
                (lk.gauge)(&p, Rect::from_min_size(egui::pos2(x + 1.0, top + 28.0), egui::vec2(66.0, 7.0)), v.odds.frac, col);
                tip(ui, Rect::from_min_size(egui::pos2(x, top + 6.0), egui::vec2(70.0, BAR_H - 12.0)), "tbar-odds", &v.odds.tip);
                x += 76.0;
            }
            let stat = |ui: &mut egui::Ui, x: &mut f32, label: &str, value: String, col: Color32, t: &str| {
                div(x);
                let a = p.text(egui::pos2(*x, top + 16.0), Align2::LEFT_CENTER, label, (lk.label)(9.0), lk.muted);
                let b = p.text(egui::pos2(*x, top + 33.0), Align2::LEFT_CENTER, value, (lk.body)(12.5), col);
                let w = a.width().max(b.width());
                tip(ui, Rect::from_min_size(egui::pos2(*x, top + 6.0), egui::vec2(w, BAR_H - 12.0)), ("tbar", label.to_string()), t);
                *x += w + 10.0;
            };
            if m.luck {
                if let Some((pct, col, t)) = &v.luck {
                    stat(ui, &mut x, "LUCK", format!("{pct:.0}%"), (lk.tone)(*col), t);
                }
            }
            if m.milestone {
                let (what, when, t) = v.milestone.clone();
                stat(ui, &mut x, "NEXT", format!("{what} {when}"), lk.text, &t);
            }
            if m.session {
                stat(ui, &mut x, "SESSION", theme::thousands(v.session.0), lk.text, "Encounters this session");
            }
            if m.streak && v.st.streak > 0 {
                stat(ui, &mut x, "STREAK", format!("{} day{}", v.st.streak, if v.st.streak == 1 { "" } else { "s" }), (lk.tone)(FLAME), "Days in a row you've reached your daily goal");
            }
            if m.sparkline {
                div(&mut x);
                p.text(egui::pos2(x, top + 14.0), Align2::LEFT_CENTER, format!("{}/H", theme::thousands(v.pace_hr)), (lk.label)(9.0), lk.muted);
                let g = Rect::from_min_size(egui::pos2(x, top + 21.0), egui::vec2(60.0, 18.0));
                sparkline(&p, g, &v.pace[5..], lk.accent, fade(lk.muted, 0.6));
                tip(ui, g.expand(4.0), "tbar-pace", "Encounters each minute, last 15 minutes");
                x += 70.0;
            }
            if v.alert.is_none() {
                let acts = self.ov_actions(m.buttons);
                if !acts.btns.is_empty() {
                    div(&mut x);
                    if let Some(q) = &acts.prompt {
                        x += p.text(egui::pos2(x, mid), Align2::LEFT_CENTER, q, (lk.body)(11.5), lk.text).width() + 8.0;
                    }
                    for b in &acts.btns {
                        let w = if acts.normal { if b.act == Act::Caught { 66.0 } else { 30.0 } } else { (b.weight * 34.0).max(30.0) };
                        self.ov_button(ui, &p, Rect::from_min_size(egui::pos2(x, mid - 13.0), egui::vec2(w, 26.0)), b, &lk.button);
                        x += w + 5.0;
                    }
                    x += 5.0;
                }
            }
            if m.status_dot {
                let c = egui::pos2(x + 2.0, mid);
                p.circle_filled(c, 3.5, (lk.tone)(v.status.0));
                tip(ui, Rect::from_center_size(c, egui::vec2(14.0, 14.0)), "tbar-dot", &v.status.1);
                x += 12.0;
            }
            for c in [Ctrl::Pin, Ctrl::Setup, Ctrl::Hide, Ctrl::Close] {
                let r = Rect::from_min_size(egui::pos2(x, mid - 11.0), egui::vec2(20.0, 22.0));
                let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
                if hov {
                    p.rect_filled(r, 5.0, if c == Ctrl::Close { Color32::from_rgb(196, 43, 28) } else { fade(lk.muted, 0.25) });
                }
                ctrl_icon(&p, r, c, on, if on { GOLD } else if hov && c == Ctrl::Close { Color32::WHITE } else { lk.text }, 15.0);
                x += 21.0;
            }
            x += 6.0;
            let width = (x - win.min.x).ceil().max(160.0);
            let bar = Rect::from_min_size(egui::pos2(left, top), egui::vec2(width - 2.0, BAR_H));
            let alert = (v.alert.is_some() || v.flash) && v.blink;
            panel_into(&p, bg_slot, bar, self.cfg.opacity, alert, lk.panel);
            if v.ghost {
                p.text(egui::pos2(bar.max.x - 12.0, bar.max.y - 6.0), Align2::RIGHT_CENTER, "CLICK-THROUGH", (lk.label)(8.0), GOLD);
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                confetti(&p, bar, *el, &[GOLD, lk.accent, Color32::WHITE], a);
                let r = Rect::from_center_size(bar.center(), egui::vec2(bar.width() - 10.0, BAR_H - 10.0));
                p.rect_filled(r, lk.radius, fade(Color32::from_rgb(20, 16, 30), 0.9 * a));
                (lk.big)(&p, r.center(), Align2::CENTER_CENTER, text, (lk.num)(16.0), fade(GOLD, a));
            }
            let mut shapes = vec![[bar.min.x, bar.min.y, bar.width(), bar.height(), lk.radius]];
            if let Some(o) = lk.shadow {
                shapes.push([bar.min.x + o.x, bar.min.y + o.y, bar.width(), bar.height(), lk.radius]);
            }
            let mut h = BAR_H + 2.0 + lk.shadow.map_or(0.0, |o| o.y);
            self.ov_shape = shapes;
            if !self.notices.is_empty() || self.toasts_pending() {
                let right = bar.max.x.max(left + BAR_POPUP_W);
                let area = Rect::from_min_max(egui::pos2(right - BAR_POPUP_W, bar.max.y + 8.0), egui::pos2(right, bar.max.y + 102.0));
                let shape = self.popup_shape(area, self.cfg.overlay.theme);
                self.ov_shape.push(shape);
                if !self.notices.is_empty() {
                    self.draw_notice(ui, ctx, area, self.cfg.overlay.theme);
                }
                if !self.toasts.is_empty() {
                    let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("toasts")));
                    self.draw_toast(ctx, &fg, area, self.cfg.overlay.theme);
                }
                h = BAR_H + 106.0;
            }
            let w = if h > BAR_H + 12.0 { width.max(BAR_POPUP_W + 2.0) } else { width + lk.shadow.map_or(0.0, |o| o.x) };
            self.ov_size = Some([w, h]);
            if v.celebrate.is_some() || v.alert.is_some() {
                ctx.request_repaint();
            }
        });
    }

    /// The hidden badge in a theme: the sprite and the count.
    pub(super) fn themed_mini(&mut self, ctx: &egui::Context, theme: OverlayTheme) -> bool {
        if theme == OverlayTheme::Classic {
            return false;
        }
        let lk = look(theme);
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            let win = ui.max_rect();
            let off = lk.shadow.unwrap_or(egui::Vec2::ZERO);
            let full = Rect::from_min_max(win.min + egui::vec2(2.0, 2.0), win.max - egui::vec2(2.0, 2.0) - off);
            self.ov_shape.clear();
            self.ov_shape.push([full.min.x, full.min.y, full.width(), full.height(), lk.radius]);
            if lk.shadow.is_some() {
                self.ov_shape.push([full.min.x + off.x, full.min.y + off.y, full.width(), full.height(), lk.radius]);
            }
            let p = ui.painter().clone();
            let blink = (self.t_ms() / 350) % 2 == 0;
            let alert = (self.shiny_alert.is_some() || self.celebrate.is_some() || self.toasts_pending()) && blink;
            let slot = p.add(Shape::Noop);
            panel_into(&p, slot, full, self.cfg.opacity, alert, lk.panel);
            let area = Rect::from_min_size(full.min, egui::vec2(full.width(), full.height() - 22.0));
            let drew = match self.target_idx() {
                Some(i) => {
                    let sp = self.species[i].clone();
                    self.draw_sprite(&p, &sp, true, area.center_bottom() - egui::vec2(0.0, 2.0), egui::vec2(60.0, 58.0))
                }
                None => false,
            };
            if !drew {
                p.image(self.icon_tex.id(), Rect::from_center_size(area.center(), egui::vec2(42.0, 42.0)), Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            (lk.big)(&p, egui::pos2(full.center().x, full.max.y - 12.0), Align2::CENTER_CENTER, &theme::thousands(self.cfg.count), (lk.num)(14.0), lk.text);
            let r = ui.interact(full, ui.id().with("mini"), Sense::click_and_drag());
            if r.drag_started() && !self.cfg.pinned {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if r.clicked() {
                self.cfg.mini = false;
                self.dirty = true;
            }
            r.on_hover_text(format!("Click to open{}", self.cfg.keybinds.hint(HotAction::ToggleHide)));
            if ctx.input(|i| i.pointer.hover_pos()).is_some_and(|h| full.expand(4.0).contains(h)) {
                let rr = Rect::from_center_size(full.right_top() + egui::vec2(-12.0, 12.0), egui::vec2(18.0, 18.0));
                let (_, hov) = self.ov_ctrl(ui, ctx, rr, Ctrl::Close);
                p.circle_filled(rr.center(), 9.0, if hov { Color32::from_rgb(196, 43, 28) } else { fade(Color32::BLACK, 0.55) });
                ctrl_icon(&p, rr, Ctrl::Close, false, Color32::WHITE, 14.0);
            }
        });
        true
    }
}

/// Draw `panel` into `slot` (so it sits under what's already drawn).
pub(super) fn panel_into(p: &Painter, slot: egui::layers::ShapeIdx, r: Rect, opacity: f32, alert: bool, panel: fn(&dyn Sink, Rect, f32, bool)) {
    let rec = Rec::default();
    panel(&rec, r, opacity, alert);
    p.set(slot, rec.shape());
}

/// Encounters per minute as little bars.
pub(super) fn sparkline(p: &dyn Sink, r: Rect, bars: &[u64], hot: Color32, cold: Color32) {
    let max = bars.iter().copied().max().unwrap_or(0).max(1) as f32;
    let n = bars.len().max(1) as f32;
    let w = r.width() / n;
    line(p, r.left_bottom(), r.right_bottom(), Stroke::new(1.0f32, fade(cold, 0.6)));
    for (i, &v) in bars.iter().enumerate() {
        if v == 0 {
            continue;
        }
        let h = (v as f32 / max * r.height()).max(2.0);
        let x = r.min.x + i as f32 * w;
        rrect(p, Rect::from_min_max(egui::pos2(x + 1.0, r.max.y - h), egui::pos2(x + w - 1.0, r.max.y)), 1.0, if i + 1 == bars.len() { hot } else { cold });
    }
}

/// Screenshot rig only: a stand-in game picture behind the overlay.
#[cfg(not(windows))]
pub(crate) fn shot_backdrop(r: Rect) -> Shape {
    let mut mesh = egui::Mesh::default();
    let (a, b) = (Color32::from_rgb(70, 120, 90), Color32::from_rgb(40, 60, 110));
    mesh.colored_vertex(r.left_top(), a);
    mesh.colored_vertex(r.right_top(), b);
    mesh.colored_vertex(r.right_bottom(), a);
    mesh.colored_vertex(r.left_bottom(), b);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}

// ───────────────────────────── theme picker ─────────────────────────────

/// A little drawing of a theme, for the picker.
fn theme_preview(p: &Painter, r: Rect, theme: OverlayTheme) {
    let c = r.center();
    let card = Rect::from_center_size(c, egui::vec2(64.0, 76.0));
    let acc = theme::pal().accent;
    match theme {
        OverlayTheme::Classic => {
            rrect(p, card, 8.0, Color32::from_rgb(30, 27, 48));
            p.add(Shape::rect_filled(Rect::from_min_size(card.min, egui::vec2(card.width(), 13.0)), egui::Rounding { nw: 8.0, ne: 8.0, sw: 0.0, se: 0.0 }, acc));
            for k in 0..2 {
                let b = Rect::from_min_size(card.min + egui::vec2(6.0 + k as f32 * 27.0, 18.0), egui::vec2(25.0, 22.0));
                rrect(p, b, 4.0, Color32::from_rgb(18, 17, 26));
                rstroke(p, b, 4.0, Stroke::new(1.0f32, if k == 1 { GOLD } else { Color32::from_rgb(92, 88, 116) }));
            }
            rrect(p, Rect::from_center_size(egui::pos2(c.x, card.min.y + 49.0), egui::vec2(26.0, 6.0)), 2.0, Color32::WHITE);
            rrect(p, Rect::from_min_size(card.min + egui::vec2(6.0, 58.0), egui::vec2(52.0, 4.0)), 2.0, theme::GREEN);
            rrect(p, Rect::from_min_size(card.min + egui::vec2(34.0, 66.0), egui::vec2(24.0, 7.0)), 2.0, acc);
            rstroke(p, card, 8.0, Stroke::new(1.0f32, Color32::from_rgb(8, 8, 12)));
        }
        OverlayTheme::Anime => {
            let ink = super::ov_anime::INK;
            rrect(p, card.translate(egui::vec2(3.0, 4.0)), 9.0, ink);
            rrect(p, card, 9.0, ink);
            rrect(p, card.shrink(2.0), 7.0, super::ov_anime::PAPER);
            let hb = Rect::from_min_size(card.min + egui::vec2(2.0, 2.0), egui::vec2(card.width() - 4.0, 13.0));
            p.add(Shape::rect_filled(hb, egui::Rounding { nw: 7.0, ne: 7.0, sw: 0.0, se: 0.0 }, acc));
            poly(p, vec![hb.left_bottom(), hb.right_bottom(), hb.right_bottom() + egui::vec2(0.0, 3.0), hb.left_bottom() + egui::vec2(0.0, 7.0)], Color32::from_rgb(255, 210, 63));
            line(p, hb.left_bottom() + egui::vec2(0.0, 4.0), hb.right_bottom(), Stroke::new(1.5f32, ink));
            burst(p, card.min + egui::vec2(40.0, 34.0), 9.0, 14.0, 12, Color32::from_rgb(255, 210, 63), ink, 1.5, 0.0);
            lettering(p, card.min + egui::vec2(8.0, 52.0), Align2::LEFT_CENTER, "42", crate::fonts::display_font(14.0), Color32::WHITE, ink, 1.4, Some(ink));
            let b = Rect::from_min_size(card.min + egui::vec2(30.0, 62.0), egui::vec2(28.0, 9.0));
            rrect(p, b.expand(1.5), 4.0, ink);
            rrect(p, b, 3.0, Color32::from_rgb(255, 104, 154));
        }
        OverlayTheme::Glass => {
            vgrad(p, &rrect_pts(card, 12.0), rgba(60, 66, 88, 0.75), rgba(12, 14, 22, 0.85));
            rstroke(p, card, 12.0, Stroke::new(1.0f32, fade(Color32::WHITE, 0.25)));
            let dc = card.min + egui::vec2(18.0, 26.0);
            for (k, a) in [(1.0, 0.08), (0.7, 0.14), (0.45, 0.22)] {
                circle(p, dc, 12.0 * k, fade(acc, a));
            }
            p.text(card.min + egui::vec2(34.0, 27.0), Align2::LEFT_CENTER, "42", crate::fonts::light_font(17.0), Color32::WHITE);
            line(p, card.min + egui::vec2(7.0, 48.0), card.min + egui::vec2(57.0, 48.0), Stroke::new(2.0f32, fade(Color32::WHITE, 0.15)));
            line(p, card.min + egui::vec2(7.0, 48.0), card.min + egui::vec2(36.0, 48.0), Stroke::new(2.0f32, acc));
            circle(p, card.min + egui::vec2(36.0, 48.0), 2.5, Color32::WHITE);
            for k in 0..2 {
                p.circle_stroke(card.min + egui::vec2(12.0 + k as f32 * 13.0, 63.0), 5.0, Stroke::new(1.0f32, fade(Color32::WHITE, 0.4)));
            }
            rrect(p, Rect::from_min_size(card.min + egui::vec2(32.0, 58.0), egui::vec2(26.0, 10.0)), 5.0, acc);
        }
        OverlayTheme::Retro => {
            rrect(p, card, 9.0, Color32::from_rgb(48, 50, 60));
            let top = Rect::from_min_size(card.min + egui::vec2(5.0, 5.0), egui::vec2(54.0, 30.0));
            vgrad(p, &[top.left_top(), top.right_top(), top.right_bottom(), top.left_bottom()], Color32::from_rgb(168, 212, 240), Color32::from_rgb(140, 192, 112));
            p.add(Shape::ellipse_filled(top.min + egui::vec2(38.0, 22.0), egui::vec2(11.0, 3.0), Color32::from_rgb(112, 160, 88)));
            rrect(p, Rect::from_min_size(top.min + egui::vec2(3.0, 3.0), egui::vec2(24.0, 8.0)), 1.0, Color32::from_rgb(248, 248, 232));
            rrect(p, Rect::from_min_size(top.min + egui::vec2(6.0, 8.0), egui::vec2(18.0, 2.0)), 0.0, Color32::from_rgb(88, 208, 128));
            rrect(p, Rect::from_min_size(card.min + egui::vec2(4.0, 37.0), egui::vec2(56.0, 4.0)), 2.0, Color32::from_rgb(28, 30, 36));
            let bot = Rect::from_min_size(card.min + egui::vec2(5.0, 43.0), egui::vec2(54.0, 28.0));
            rrect(p, bot, 1.0, Color32::from_rgb(48, 92, 104));
            rrect(p, Rect::from_min_size(bot.min + egui::vec2(4.0, 3.0), egui::vec2(46.0, 11.0)), 3.0, Color32::from_rgb(232, 88, 72));
            for (k, col) in [Color32::from_rgb(232, 184, 64), Color32::from_rgb(88, 136, 216), Color32::from_rgb(96, 184, 88)].into_iter().enumerate() {
                rrect(p, Rect::from_min_size(bot.min + egui::vec2(4.0 + k as f32 * 16.0, 17.0), egui::vec2(14.0, 8.0)), 2.0, col);
            }
        }
        OverlayTheme::Neon => {
            let (m, cy) = (Color32::from_rgb(255, 46, 196), Color32::from_rgb(0, 232, 255));
            rrect(p, card, 8.0, Color32::from_rgb(9, 8, 20));
            rstroke(p, card.shrink(2.0), 7.0, Stroke::new(3.0f32, fade(m, 0.18)));
            let pts = rrect_pts(card, 8.0);
            let n = pts.len();
            for i in 0..n {
                let t = ((pts[i].x - card.min.x) / card.width()).clamp(0.0, 1.0);
                line(p, pts[i], pts[(i + 1) % n], Stroke::new(1.5f32, mix(m, cy, t)));
            }
            let hc = card.min + egui::vec2(18.0, 26.0);
            let hex: Vec<Pos2> = (0..6).map(|i| {
                let a = (i as f32 * 60.0 - 90.0).to_radians();
                hc + egui::vec2(a.cos() * 11.0, a.sin() * 11.0)
            }).collect();
            outline(p, hex, Stroke::new(1.5f32, cy));
            glow_text(p, card.min + egui::vec2(34.0, 27.0), Align2::LEFT_CENTER, "42", crate::fonts::black_font(15.0), Color32::WHITE, m, 2.5);
            rrect(p, Rect::from_min_size(card.min + egui::vec2(7.0, 46.0), egui::vec2(50.0, 4.0)), 2.0, fade(Color32::WHITE, 0.1));
            hgrad(p, &rrect_pts(Rect::from_min_size(card.min + egui::vec2(7.0, 46.0), egui::vec2(30.0, 4.0)), 2.0), m, cy);
            hgrad(p, &rrect_pts(Rect::from_min_size(card.min + egui::vec2(30.0, 58.0), egui::vec2(28.0, 10.0)), 3.0), m, Color32::from_rgb(150, 92, 255));
        }
        OverlayTheme::Poketch => super::ov_poketch::preview(p, card),
        OverlayTheme::Journal => super::ov_journal::preview(p, card),
        OverlayTheme::Holo => super::ov_holo::preview(p, r, 0.0),
        OverlayTheme::Broadcast => super::ov_broadcast::preview(p, r),
        OverlayTheme::Terminal => super::ov_terminal::preview(p, card),
    }
}

/// A clickable tile for a theme: a little drawing, its name and what it is.
pub(super) fn theme_tile(ui: &mut egui::Ui, theme: OverlayTheme, selected: bool, w: f32) -> egui::Response {
    use super::ui_kit as kit;
    let h = 180.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), Sense::click());
    let p = ui.painter_at(rect.expand(3.0));
    let fill = if selected { kit::SELECTED_BG() } else if resp.hovered() { kit::CARD_HOVER() } else { kit::CARD() };
    rrect(&p, rect, 12.0, fill);
    rstroke(&p, rect, 12.0, if selected { Stroke::new(2.0f32, kit::ACCENT()) } else { Stroke::new(1.0f32, kit::LINE()) });
    let pr = Rect::from_min_size(rect.min + egui::vec2(8.0, 8.0), egui::vec2(w - 16.0, 92.0));
    hgrad(&p, &rrect_pts(pr, 8.0), Color32::from_rgb(64, 112, 88), Color32::from_rgb(44, 64, 112));
    theme_preview(&p, pr, theme);
    p.text(egui::pos2(rect.min.x + 12.0, pr.max.y + 15.0), Align2::LEFT_CENTER, theme.name(), crate::fonts::title_font(14.0), kit::TEXT());
    if selected {
        let c = egui::pos2(rect.max.x - 18.0, pr.max.y + 15.0);
        circle(&p, c, 8.0, kit::ACCENT());
        p.add(Shape::line(vec![c + egui::vec2(-3.5, 0.0), c + egui::vec2(-1.0, 2.8), c + egui::vec2(3.8, -2.8)], Stroke::new(1.8f32, Color32::WHITE)));
    }
    let galley = p.layout(theme.about().to_string(), crate::fonts::sans_font(11.5), kit::MUTED(), w - 24.0);
    p.galley(egui::pos2(rect.min.x + 12.0, pr.max.y + 26.0), galley, kit::MUTED());
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
