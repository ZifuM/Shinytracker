//! Profile → Battle Pass: the season's daily and weekly missions, the
//! 30-tier free reward track, claiming and wearing rewards. Five layouts
//! while one is being chosen; the parts inside are shared.

use super::charts as ch;
use super::team_home::{caps, small};
use super::ui_kit::{self as kit, CARD, CARD_HOVER, LINE, MUTED, TEXT};
use super::{App, RunRef};
use crate::card::CardStyle;
use crate::pass::{self, Kind2, Mission, Reward, Theme};
use crate::stats::{local_day, now_unix};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions};
use std::collections::HashMap;

// the season's colours
const VIOLET: Color32 = Color32::from_rgb(120, 62, 190);
const CRIMSON: Color32 = Color32::from_rgb(222, 44, 78);
const GOLD: Color32 = Color32::from_rgb(236, 186, 80);
const TEAL: Color32 = Color32::from_rgb(46, 210, 194);
const LILAC: Color32 = Color32::from_rgb(160, 130, 255);
const GREEN: Color32 = Color32::from_rgb(70, 200, 120);

/// A battle pass title's colour (its look's), or `other` for any other title.
pub(super) fn title_col(title: &str, other: Color32) -> Color32 {
    pass::title_theme(title).map_or(other, |t| ch::ink(theme_col(t)))
}

fn theme_col(t: Theme) -> Color32 {
    match t {
        Theme::Distortion => CRIMSON,
        Theme::Aurora => TEAL,
        Theme::Both => LILAC,
    }
}

/// Pictures of the rewards, made once.
#[derive(Default)]
pub struct PassCache {
    tex: HashMap<String, TextureHandle>,
    /// The week picked above the weekly missions (None = this week).
    pub week: Option<i64>,
    /// Rewards (the track) or Locker, above the missions.
    pub tab: u8,
    /// The season whose Locker page is open (None = the one on now).
    pub locker: Option<u32>,
}

/// Where a reward stands.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Locked,
    Ready,
    Claimed,
}

fn kind_label(k: Kind2) -> &'static str {
    match k {
        Kind2::Plate => "Name plate",
        Kind2::Frame => "Picture frame",
        Kind2::Title => "Title",
        Kind2::Style(s) if s.live() => "Animated hunt image",
        Kind2::Style(_) => "Hunt image",
        Kind2::Colour(_) => "Pokégotchi colour",
    }
}

/// A small padlock centred on `c`.
fn padlock(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, s * 0.35), vec2(s * 1.8, s * 1.3));
    let pts: Vec<egui::Pos2> = (0..=16).map(|i| {
        let a = std::f32::consts::PI + std::f32::consts::PI * i as f32 / 16.0;
        pos2(c.x + a.cos() * s * 0.6, body.min.y + a.sin() * s * 0.75)
    }).collect();
    p.add(egui::Shape::line(pts, Stroke::new((s * 0.28).max(1.4), col)));
    p.rect_filled(body, s * 0.25, col);
}

/// The season theme's little mark: a crimson rift for Distortion World, a
/// snowflake for Snowpoint Aurora, both for Two Worlds.
fn theme_mark(p: &egui::Painter, c: egui::Pos2, s: f32, t: Theme) {
    let rift = |p: &egui::Painter, c: egui::Pos2, s: f32| {
        let pts = vec![c + vec2(-s * 0.3, -s), c + vec2(s * 0.35, -s * 0.2), c + vec2(-s * 0.3, s * 0.2), c + vec2(s * 0.3, s)];
        p.add(egui::Shape::line(pts, Stroke::new(2.0f32, CRIMSON)));
    };
    let flake = |p: &egui::Painter, c: egui::Pos2, s: f32| {
        for k in 0..3 {
            let a = k as f32 * std::f32::consts::PI / 3.0;
            let d = vec2(a.cos(), a.sin()) * s;
            p.line_segment([c - d, c + d], Stroke::new(1.6f32, TEAL));
        }
    };
    match t {
        Theme::Distortion => rift(p, c, s),
        Theme::Aurora => flake(p, c, s),
        Theme::Both => {
            rift(p, c - vec2(s * 0.5, 0.0), s * 0.8);
            flake(p, c + vec2(s * 0.6, 0.0), s * 0.7);
        }
    }
}

fn countdown(secs: i64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    if h >= 24 {
        format!("{}d {}h", h / 24, h % 24)
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

impl App {
    // ───────────────────────────── state ─────────────────────────────

    /// Whether reward `id` is yours (claimed).
    pub(super) fn owns_reward(&self, id: &str) -> bool {
        self.cfg.pass_claimed.iter().any(|c| c == id)
    }

    fn reward_state(&self, r: &Reward, tier: u32) -> State {
        if self.owns_reward(r.id) {
            State::Claimed
        } else if tier >= r.tier {
            State::Ready
        } else {
            State::Locked
        }
    }

    /// The titles claimed from the pass.
    pub(super) fn pass_titles(&self) -> Vec<&'static str> {
        pass::all_rewards().filter(|r| r.kind == Kind2::Title && self.owns_reward(r.id)).map(|r| r.name).collect()
    }

    /// Whether a share image style can be used (the pass ones once claimed).
    pub(super) fn style_owned(&self, s: CardStyle) -> bool {
        match pass::style_reward(s) {
            Some(r) => self.owns_reward(r.id),
            None => true,
        }
    }

    /// Whether a Pokégotchi colour can be used (Pearl always; the others
    /// once claimed from the battle pass).
    pub(super) fn tint_owned(&self, t: crate::gotchi::Tint) -> bool {
        match pass::tint_reward(t) {
            Some(r) => self.owns_reward(r.id),
            None => true,
        }
    }

    fn claim(&mut self, r: &Reward) {
        if !self.cfg.pass_claimed.iter().any(|c| c == r.id) {
            self.cfg.pass_claimed.push(r.id.to_string());
            self.dirty = true;
        }
    }

    /// Take off anything worn that isn't really claimed (a plate, a frame, a
    /// pass title, a pass share image style): what the old debug unlock let
    /// you wear goes back to normal. Runs every frame; a few string compares.
    pub(super) fn pass_tidy(&mut self) {
        let mut changed = false;
        if !self.cfg.plate.is_empty() && !self.owns_reward(&self.cfg.plate.clone()) {
            self.cfg.plate.clear();
            changed = true;
        }
        if !self.cfg.frame.is_empty() && !self.owns_reward(&self.cfg.frame.clone()) {
            self.cfg.frame.clear();
            changed = true;
        }
        let pass_title = pass::all_rewards().find(|r| r.kind == Kind2::Title && r.name == self.cfg.title_choice).map(|r| r.id);
        if let Some(id) = pass_title {
            if !self.owns_reward(id) {
                self.cfg.title_choice.clear();
                changed = true;
            }
        }
        if let Some(r) = pass::style_reward(self.cfg.card.style) {
            if !self.owns_reward(r.id) {
                self.cfg.card.style = crate::card::CardOptions::default().style;
                changed = true;
            }
        }
        // a Pokégotchi colour that isn't claimed goes back to Sinnoh Pearl
        if let Some(t) = self.setup.gotchi.file.as_ref().map(|f| f.tint) {
            if !self.tint_owned(t) {
                if let Some(f) = self.setup.gotchi.file.as_mut() {
                    f.tint = crate::gotchi::Tint::Pink;
                }
                self.setup.gotchi.touch();
            }
        }
        if changed {
            self.dirty = true;
        }
    }

    /// Wear / use a claimed reward (again to take a plate or frame off).
    fn equip(&mut self, r: &Reward) {
        match r.kind {
            Kind2::Plate => self.cfg.plate = if self.cfg.plate == r.id { String::new() } else { r.id.to_string() },
            Kind2::Frame => self.cfg.frame = if self.cfg.frame == r.id { String::new() } else { r.id.to_string() },
            Kind2::Title => self.cfg.title_choice = if self.cfg.title_choice == r.name { String::new() } else { r.name.to_string() },
            Kind2::Style(_) => return,
            Kind2::Colour(t) => {
                // put it on your Pokégotchi (the Sinnoh frame, in this colour)
                if self.has_app(crate::apps::AppId::Gotchi) {
                    let f = self.setup.gotchi.file();
                    f.tint = t;
                    f.frame = crate::gotchi::Frame::Pearl;
                    super::gotchi_app::set_tint(t);
                    self.setup.gotchi.touch();
                }
                return;
            }
        }
        self.dirty = true;
    }

    fn equipped(&self, r: &Reward) -> bool {
        match r.kind {
            Kind2::Plate => self.cfg.plate == r.id,
            Kind2::Frame => self.cfg.frame == r.id,
            Kind2::Title => self.cfg.title_choice == r.name,
            Kind2::Style(_) => false,
            Kind2::Colour(t) => {
                self.has_app(crate::apps::AppId::Gotchi)
                    && self.setup.gotchi.file.as_ref().is_some_and(|f| f.tint == t && f.frame == crate::gotchi::Frame::Pearl)
            }
        }
    }

    // ───────────────────────────── pictures ─────────────────────────────


    fn canvas_tex(&mut self, ctx: &egui::Context, key: String, make: impl FnOnce() -> crate::canvas::Canvas) -> TextureHandle {
        if let Some(t) = self.setup.pass.tex.get(&key) {
            return t.clone();
        }
        let c = make();
        // premultiply in plain 0-255 terms: egui's own unmultiplied path goes through linear
        // light, which leaves faint glows far too bright where they fade out (a visible box)
        let mut pm = c.px.clone();
        for q in pm.chunks_exact_mut(4) {
            let a = q[3] as u32;
            for ch in &mut q[..3] {
                *ch = ((*ch as u32 * a + 127) / 255) as u8;
            }
        }
        let img = egui::ColorImage::from_rgba_premultiplied([c.w as usize, c.h as usize], &pm);
        let t = ctx.load_texture(key.clone(), img, TextureOptions::LINEAR);
        self.setup.pass.tex.insert(key, t.clone());
        t
    }

    /// The plate picture for `id` filling `r` (the picture reaches a little
    /// above `r` for crowns and snow), as a shape to paint.
    pub(super) fn plate_shape(&mut self, ctx: &egui::Context, id: &str, r: Rect) -> egui::Shape {
        let ppp = ctx.pixels_per_point();
        let total_h = r.height() / 0.78;
        let (w, h) = (((r.width() + 6.0) * ppp).round() as u32, (total_h * ppp).round() as u32);
        let key = format!("plate|{id}|{w}x{h}");
        let idc = id.to_string();
        let t = self.canvas_tex(ctx, key, move || crate::cosmetics::plate_image(&idc, w, h));
        let size = vec2(w as f32 / ppp, h as f32 / ppp);
        // the plate itself is the bottom 78% of the picture (less 4 px), starting 2 px in
        let img_r = Rect::from_min_size(pos2(r.min.x - 2.0 / ppp, r.max.y + 4.0 / ppp - size.y), size);
        egui::Shape::image(t.id(), img_r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE)
    }

    /// Draw name plate `id` in `r` with `name` centred on it (app side).
    pub(super) fn draw_plate_ui(&mut self, ctx: &egui::Context, p: &egui::Painter, id: &str, r: Rect, name: &str, size: f32) {
        let shape = self.plate_shape(ctx, id, r);
        p.add(shape);
        if !name.is_empty() {
            plate_text(p, r.center(), name, size, plate_ink(id));
        }
    }

    /// The rectangle of a plate sized to fit text `text_w` wide starting at
    /// `text_x`, centred on `cy`, `h` tall.
    pub(super) fn plate_rect(text_x: f32, cy: f32, text_w: f32, h: f32) -> Rect {
        let pad = h * crate::cosmetics::PLATE_PAD;
        Rect::from_min_size(pos2(text_x - pad, cy - h / 2.0), vec2(text_w + 2.0 * pad, h))
    }

    /// Draw frame `id` around a picture of radius `rad` at `c`.
    pub(super) fn draw_frame_ui(&mut self, ctx: &egui::Context, p: &egui::Painter, id: &str, c: egui::Pos2, rad: f32) {
        let ppp = ctx.pixels_per_point();
        let reach = crate::cosmetics::FRAME_REACH;
        let side = rad * 2.0 * (1.0 + reach);
        let px = (side * ppp).round() as u32;
        let key = format!("frame|{id}|{px}");
        let idc = id.to_string();
        let t = self.canvas_tex(ctx, key, move || crate::cosmetics::frame_image(&idc, px));
        p.image(t.id(), Rect::from_center_size(c, vec2(side, side)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }

    /// Your picture (or a placeholder) with `frame` round it.
    fn framed_avatar(&mut self, ctx: &egui::Context, p: &egui::Painter, frame: &str, c: egui::Pos2, rad: f32) {
        match self.avatar_tex(ctx) {
            Some(tex) => super::avatar::circle_image(p, tex, c, rad, super::avatar::full_uv()),
            None => {
                p.circle_filled(c, rad, theme::blend(CARD(), TEXT(), 0.12));
                p.text(c, Align2::CENTER_CENTER, "?", crate::fonts::sans_bold_font(rad * 0.8), MUTED());
            }
        }
        if !frame.is_empty() {
            self.draw_frame_ui(ctx, p, frame, c, rad);
        }
    }

    /// A picture of share image style `s` (made once, from your hunt).
    fn style_tex(&mut self, ctx: &egui::Context, s: CardStyle) -> TextureHandle {
        let key = format!("style|{s:?}");
        if let Some(t) = self.setup.pass.tex.get(&key) {
            return t.clone();
        }
        let mut data = self.run_info(RunRef::Current).map(|i| self.pass_card_data(&i)).unwrap_or_default();
        if data.name.is_empty() {
            data.name = "Giratina".into();
        }
        let mut o = self.cfg.card.clone();
        o.style = s;
        o.layout = crate::card::CardLayout::Wide;
        o.res = 480;
        o.animated = false;
        if o.trainer.trim().is_empty() {
            o.trainer = self.cfg.trainer_name.clone();
        }
        let c = crate::card::render_frame(&data, &o, self.card_fonts_pub(), if s.live() { Some(0.25) } else { None });
        let img = egui::ColorImage::from_rgba_unmultiplied([c.w as usize, c.h as usize], &c.px);
        let t = ctx.load_texture(key.clone(), img, TextureOptions::LINEAR);
        self.setup.pass.tex.insert(key, t.clone());
        t
    }

    /// A reward's picture filling `r`.
    fn reward_preview(&mut self, ctx: &egui::Context, p: &egui::Painter, rw: &Reward, r: Rect, state: State) {
        let col = theme_col(rw.theme);
        p.rect_filled(r, 12.0, theme::blend(kit::sunken(), col, 0.14));
        let name = if self.cfg.trainer_name.trim().is_empty() { "Trainer".to_string() } else { self.cfg.trainer_name.trim().to_string() };
        match rw.kind {
            Kind2::Plate => {
                let ph = (r.height() * 0.34).clamp(22.0, 44.0);
                let pr = Rect::from_center_size(r.center() + vec2(0.0, ph * 0.12), vec2(r.width() - 18.0, ph));
                self.draw_plate_ui(ctx, p, rw.id, pr, &name, ph * 0.34);
            }
            Kind2::Frame => {
                let rad = (r.height().min(r.width()) * 0.27).max(14.0);
                self.framed_avatar(ctx, p, rw.id, r.center(), rad);
            }
            Kind2::Title => {
                let f = crate::fonts::sans_bold_font((r.height() * 0.13).clamp(11.0, 18.0));
                let g = p.layout(rw.name.to_string(), f, Color32::WHITE, r.width() - 24.0);
                let pr = Rect::from_center_size(r.center(), g.size() + vec2(22.0, 14.0));
                p.rect_filled(pr, pr.height() / 2.0, theme::blend(Color32::from_rgb(16, 10, 30), col, 0.45));
                p.rect_stroke(pr, pr.height() / 2.0, Stroke::new(1.5f32, col));
                p.galley(pr.center() - g.size() / 2.0, g, Color32::WHITE);
            }
            Kind2::Style(s) => {
                let t = self.style_tex(ctx, s);
                let aspect = 1200.0 / 675.0;
                let w = (r.width() - 12.0).min((r.height() - 12.0) * aspect);
                let ir = Rect::from_center_size(r.center(), vec2(w, w / aspect));
                p.image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                p.rect_stroke(ir, 4.0, Stroke::new(1.0f32, col));
                if s.live() {
                    let f = crate::fonts::sans_bold_font(10.0);
                    let g = p.layout_no_wrap("▶ ANIMATED".into(), f, Color32::WHITE);
                    let br = Rect::from_min_size(ir.min + vec2(6.0, 6.0), g.size() + vec2(10.0, 6.0));
                    p.rect_filled(br, 5.0, CRIMSON.gamma_multiply(0.9));
                    p.galley(br.min + vec2(5.0, 3.0), g, Color32::WHITE);
                }
            }
            Kind2::Colour(t) => {
                // the device in that colour (drawn in it just for this)
                let now = self.setup.gotchi.file.as_ref().map(|f| f.tint).unwrap_or_default();
                super::gotchi_app::set_tint(t);
                let dh = r.height() - 10.0;
                let dr = Rect::from_center_size(r.center(), vec2(dh * 236.0 / 290.0, dh));
                let time = ctx.input(|i| i.time) as f32;
                super::gotchi_app::paint_device(p, dr, crate::gotchi::Frame::Pearl, time, None, None, false);
                super::gotchi_app::set_tint(now);
            }
        }
        if state == State::Locked {
            p.rect_filled(r, 12.0, Color32::from_black_alpha(110));
            padlock(p, r.right_top() + vec2(-14.0, 14.0), 7.0, Color32::from_white_alpha(210));
        }
    }

    /// The button under a reward: Claim / Wear / Worn… Returns true if
    /// something changed.
    fn reward_button(&mut self, ui: &mut egui::Ui, rw: &Reward, state: State, w: f32) {
        match state {
            State::Locked => {
                ui.add_enabled(false, egui::Button::new(RichText::new(format!("Tier {}", rw.tier)).size(12.0)).min_size(vec2(w, 28.0)));
            }
            State::Ready => {
                if kit::accent_button_w(ui, egui::Button::new(RichText::new("Claim").size(12.5)), 6.0, w).clicked() {
                    self.claim(rw);
                }
            }
            // an image style isn't worn: once claimed it's simply one of the
            // looks in the share image style picker
            State::Claimed if matches!(rw.kind, Kind2::Colour(_)) && !self.has_app(crate::apps::AppId::Gotchi) => {
                let b = egui::Button::new(RichText::new("✔ Waiting in Pokégotchi").size(12.0).color(theme::blend(TEXT(), GREEN, 0.6))).fill(theme::blend(CARD(), GREEN, 0.14)).min_size(vec2(w, 28.0));
                ui.add_enabled(false, b).on_disabled_hover_text("Get Pokégotchi in the App Store to use it");
            }
            State::Claimed if matches!(rw.kind, Kind2::Style(_)) => {
                let b = egui::Button::new(RichText::new("✔ In your image styles").size(12.0).color(theme::blend(TEXT(), GREEN, 0.6))).fill(theme::blend(CARD(), GREEN, 0.14)).min_size(vec2(w, 28.0));
                ui.add_enabled(false, b).on_disabled_hover_text("Pick it in Share image › Style for any hunt");
            }
            State::Claimed => {
                let on = self.equipped(rw);
                let colour = matches!(rw.kind, Kind2::Colour(_));
                let (l, tip) = match (on, colour) {
                    (true, true) => ("✔ On your Pokégotchi", "Your Pokégotchi is in this colour"),
                    (false, true) => ("Use", "Put your Pokégotchi in this colour"),
                    (true, false) => ("✔ Wearing", "Click to take it off"),
                    (false, false) => ("Wear", "Show it on your profile and share images"),
                };
                let b = egui::Button::new(RichText::new(l).size(12.5).color(if on { Color32::WHITE } else { TEXT() }))
                    .fill(if on { theme::blend(CARD(), GREEN, 0.55) } else { kit::btn_fill() })
                    .min_size(vec2(w, 28.0));
                if ui.add(b).on_hover_text(tip).clicked() && !(on && colour) {
                    self.equip(rw);
                }
            }
        }
    }

    /// A reward as a card: picture, name, what it is and its button.
    fn reward_tile(&mut self, ui: &mut egui::Ui, rw: &Reward, tier: u32, w: f32, h: f32) {
        let state = self.reward_state(rw, tier);
        let col = theme_col(rw.theme);
        let next = state == State::Locked && rw.tier == tier + 1;
        egui::Frame::none()
            .fill(if state == State::Ready { theme::blend(CARD(), GOLD, 0.12) } else { CARD() })
            .stroke(Stroke::new(if next || state == State::Ready { 1.5f32 } else { 1.0 }, if state == State::Ready { GOLD } else if next { col } else { LINE() }))
            .rounding(14.0)
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.set_width(w - 16.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("TIER {}", rw.tier)).font(crate::fonts::sans_bold_font(10.5)).color(ch::ink(if state == State::Ready { GOLD } else { col })));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (mr, resp) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        theme_mark(ui.painter(), mr.center(), 6.0, rw.theme);
                        resp.on_hover_text(rw.theme.name());
                    });
                });
                let (pr, _) = ui.allocate_exact_size(vec2(w - 16.0, h), Sense::hover());
                let ctx = ui.ctx().clone();
                let p = ui.painter().clone();
                self.reward_preview(&ctx, &p, rw, pr, state);
                ui.add_space(4.0);
                ui.add(egui::Label::new(RichText::new(rw.name).font(crate::fonts::sans_bold_font(12.5)).color(TEXT())).truncate());
                ui.label(RichText::new(kind_label(rw.kind)).size(11.0).color(MUTED()));
                ui.add_space(4.0);
                self.reward_button(ui, rw, state, w - 16.0);
            });
    }

    // ───────────────────────────── missions ─────────────────────────────

    /// One mission: a tick or a ring, what to do, how far, the XP.
    #[allow(clippy::too_many_arguments)]
    fn mission_row(&self, ui: &mut egui::Ui, m: &Mission, cur: f64, target: f64, done: bool, xp: u64, unit: &str, col: Color32) {
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(r, 10.0, CARD_HOVER());
        }
        let c = pos2(r.min.x + 20.0, r.center().y);
        let f = (cur / target.max(1.0)).clamp(0.0, 1.0) as f32;
        if done {
            p.circle_filled(c, 11.0, GREEN);
            p.text(c, Align2::CENTER_CENTER, "✔", crate::fonts::sans_bold_font(12.0), Color32::WHITE);
        } else {
            p.circle_stroke(c, 10.0, Stroke::new(3.0f32, theme::blend(CARD(), TEXT(), 0.12)));
            if f > 0.0 {
                let pts: Vec<egui::Pos2> = (0..=40).map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * f * i as f32 / 40.0;
                    c + vec2(a.cos(), a.sin()) * 10.0
                }).collect();
                p.add(egui::Shape::line(pts, Stroke::new(3.0f32, col)));
            }
        }
        let x = r.min.x + 42.0;
        let tc = if done { MUTED() } else { TEXT() };
        p.text(pos2(x, r.min.y + 14.0), Align2::LEFT_CENTER, m.title, crate::fonts::sans_bold_font(13.0), tc);
        let prog = if done {
            "Done".to_string()
        } else if unit == "h" {
            format!("{cur:.1} / {target:.0} h")
        } else {
            format!("{} / {}{}", theme::thousands(cur as u64), theme::thousands(target as u64), if unit.is_empty() { String::new() } else { format!(" {unit}") })
        };
        let bar = Rect::from_min_size(pos2(x, r.min.y + 30.0), vec2((r.width() - 42.0 - 150.0).max(60.0), 5.0));
        p.rect_filled(bar, 2.5, theme::blend(CARD(), TEXT(), 0.09));
        p.rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * f, 5.0)), 2.5, if done { GREEN } else { col });
        p.text(pos2(bar.max.x + 10.0, bar.center().y), Align2::LEFT_CENTER, prog, crate::fonts::sans_font(11.0), MUTED());
        let xr = Rect::from_min_size(pos2(r.max.x - 64.0, r.center().y - 11.0), vec2(60.0, 22.0));
        p.rect_filled(xr, 11.0, if done { theme::blend(CARD(), GREEN, 0.2) } else { theme::blend(CARD(), GOLD, 0.16) });
        p.text(xr.center(), Align2::CENTER_CENTER, format!("+{xp} XP"), crate::fonts::sans_bold_font(11.0), ch::ink(if done { GREEN } else { GOLD }));
    }

    fn mission_unit(m: &Mission) -> &'static str {
        match m.ask {
            pass::Ask::Minutes(_) | pass::Ask::SessionMinutes(_) => "min",
            pass::Ask::Week(crate::goals::Kind::SessionMinutes(_)) => "min",
            pass::Ask::Week(crate::goals::Kind::Hours(_)) => "h",
            pass::Ask::Week(crate::goals::Kind::Days(_)) => "days",
            _ => "",
        }
    }

    /// Today's three missions.
    fn pass_daily(&mut self, ui: &mut egui::Ui) {
        let now = now_unix();
        let today = local_day(now);
        let ds = self.day_stats(today);
        let goal = self.cfg.daily_goal;
        let left = 86_400 - ((now as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400));
        ui.horizontal(|ui| {
            ui.label(RichText::new("TODAY").font(crate::fonts::sans_bold_font(11.0)).color(ch::ink(TEAL)));
            ui.label(small(format!("·  new missions in {}", countdown(left))));
        });
        ui.add_space(4.0);
        let season = pass::current(today);
        if !season.active(today) {
            ui.label(small("No missions: the season isn't on."));
            return;
        }
        for m in season.daily(today) {
            let done = self.goal_done_pub(&season.daily_key(today, &m));
            let (c, t) = pass::daily_progress(&m, &ds, goal);
            let title_goal;
            let m2 = if matches!(m.ask, pass::Ask::DailyGoal) {
                title_goal = format!("Reach your daily goal ({} encounters)", theme::thousands(goal.max(1)));
                Mission { title: Box::leak(title_goal.into_boxed_str()), ..m }
            } else {
                m
            };
            self.mission_row(ui, &m2, c, t, done, pass::DAILY_PASS_XP, Self::mission_unit(&m), TEAL);
        }
        if goal == 0 {
            ui.label(small("Set a daily goal (Hunt settings) to start the first one."));
        }
    }

    /// The weekly missions, with a button for each week of the season above
    /// them. Weeks so far can be opened (missions stay open until the season
    /// ends); later weeks show their missions locked until the week arrives.
    fn pass_weekly(&mut self, ui: &mut egui::Ui) {
        let today = local_day(now_unix());
        let season = pass::current(today);
        let Some(this) = season.week_of(today) else {
            ui.label(small("No missions: the season isn't on."));
            return;
        };
        let wk = self.setup.pass.week.unwrap_or(this).clamp(0, pass::WEEKS - 1);
        // the week buttons, in one row
        egui::ScrollArea::horizontal().id_salt("pass-weeks").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for n in 0..pass::WEEKS {
                    let picked = n == wk;
                    let locked = n > this;
                    let ms = season.weekly(n);
                    let all_done = !locked && ms.iter().all(|m| self.goal_done_pub(&season.weekly_key(n, m)));
                    let label = format!("Week {}", n + 1);
                    let g = ui.painter().layout_no_wrap(label.clone(), crate::fonts::sans_bold_font(11.5), TEXT());
                    let extra = if locked || all_done { 16.0 } else { 0.0 };
                    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 20.0 + extra, 26.0), Sense::click());
                    let p = ui.painter();
                    let fill = if picked {
                        theme::blend(CARD(), VIOLET, 0.45)
                    } else if resp.hovered() {
                        CARD_HOVER()
                    } else {
                        theme::blend(CARD(), TEXT(), 0.05)
                    };
                    p.rect_filled(r, 13.0, fill);
                    p.rect_stroke(r, 13.0, Stroke::new(1.0f32, if picked { VIOLET } else if n == this { theme::blend(LINE(), VIOLET, 0.6) } else { LINE() }));
                    let tc = if picked { Color32::WHITE } else if locked { MUTED() } else { TEXT() };
                    p.text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, &label, crate::fonts::sans_bold_font(11.5), tc);
                    let ic = pos2(r.max.x - 13.0, r.center().y);
                    if locked {
                        padlock(p, ic, 4.5, MUTED());
                    } else if all_done {
                        p.text(ic, Align2::CENTER_CENTER, "✔", crate::fonts::sans_bold_font(11.0), GREEN);
                    }
                    let tip = if locked {
                        let (a, _) = season.week_days(n);
                        format!("Opens in {} day{}", a - today, if a - today == 1 { "" } else { "s" })
                    } else if n == this {
                        "This week".to_string()
                    } else {
                        "Still open until the season ends".to_string()
                    };
                    if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.setup.pass.week = if n == this { None } else { Some(n) };
                    }
                }
            });
        });
        ui.add_space(10.0);
        let (a, b) = season.week_days(wk);
        let ms = season.weekly(wk);
        if wk > this {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("WEEK {}", wk + 1)).font(crate::fonts::sans_bold_font(11.0)).color(MUTED()));
                ui.label(small(format!("·  locked  ·  opens in {} day{}", a - today, if a - today == 1 { "" } else { "s" })));
            });
            ui.add_space(2.0);
            for m in &ms {
                self.mission_locked_row(ui, m);
            }
            return;
        }
        let ws = self.range_stats(a, today.min(b));
        let done_n = ms.iter().filter(|m| self.goal_done_pub(&season.weekly_key(wk, m))).count();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("WEEK {}", wk + 1)).font(crate::fonts::sans_bold_font(11.0)).color(ch::ink(VIOLET)));
            let note = if wk == this {
                format!("·  {done_n} of {} done  ·  {} days left this week", ms.len(), b - today + 1)
            } else {
                format!("·  {done_n} of {} done  ·  still open until the season ends", ms.len())
            };
            ui.label(small(note));
        });
        ui.add_space(2.0);
        for m in &ms {
            let done = self.goal_done_pub(&season.weekly_key(wk, m));
            let (c, t) = pass::weekly_progress(m, &ws);
            self.mission_row(ui, m, c, t, done, pass::WEEKLY_PASS_XP, Self::mission_unit(m), VIOLET);
        }
    }

    /// A mission of a week that hasn't started: greyed, with a padlock.
    fn mission_locked_row(&self, ui: &mut egui::Ui, m: &Mission) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::hover());
        let p = ui.painter();
        let c = pos2(r.min.x + 20.0, r.center().y);
        p.circle_filled(c, 11.0, theme::blend(CARD(), TEXT(), 0.07));
        padlock(p, c, 5.0, MUTED());
        let x = r.min.x + 42.0;
        p.text(pos2(x, r.min.y + 14.0), Align2::LEFT_CENTER, m.title, crate::fonts::sans_bold_font(13.0), MUTED().gamma_multiply(0.8));
        let bar = Rect::from_min_size(pos2(x, r.min.y + 30.0), vec2((r.width() - 42.0 - 150.0).max(60.0), 5.0));
        p.rect_filled(bar, 2.5, theme::blend(CARD(), TEXT(), 0.05));
        let xr = Rect::from_min_size(pos2(r.max.x - 64.0, r.center().y - 11.0), vec2(60.0, 22.0));
        p.rect_filled(xr, 11.0, theme::blend(CARD(), TEXT(), 0.06));
        p.text(xr.center(), Align2::CENTER_CENTER, format!("+{} XP", pass::WEEKLY_PASS_XP), crate::fonts::sans_bold_font(11.0), MUTED());
    }

    // ───────────────────────────── header ─────────────────────────────

    /// The season banner: name, week, days left, tier and XP.
    fn pass_header(&mut self, ui: &mut egui::Ui, h: f32) {
        let today = local_day(now_unix());
        let season = pass::current(today);
        let xp = self.pass_xp_in(season);
        let tier = pass::tier(xp);
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
        let p = ui.painter().clone();
        let half = r.width() / 2.0;
        // the two worlds, side by side
        let l = Rect::from_min_size(r.min, vec2(half, h));
        let rr = Rect::from_min_size(pos2(r.min.x + half, r.min.y), vec2(half, h));
        p.rect_filled(l, egui::Rounding { nw: 18.0, sw: 18.0, ne: 0.0, se: 0.0 }, theme::blend(CARD(), VIOLET, 0.3));
        p.rect_filled(rr, egui::Rounding { nw: 0.0, sw: 0.0, ne: 18.0, se: 18.0 }, theme::blend(CARD(), TEAL, 0.2));
        p.rect_stroke(r, 18.0, Stroke::new(1.0f32, LINE()));
        let x = r.min.x + 26.0;
        p.text(pos2(x, r.min.y + 26.0), Align2::LEFT_CENTER, format!("SEASON {}  ·  FREE BATTLE PASS", season.num), crate::fonts::sans_bold_font(11.5), ch::ink(GOLD));
        p.text(pos2(x, r.min.y + 56.0), Align2::LEFT_CENTER, season.name, crate::fonts::sans_bold_font(26.0), TEXT());
        let when = if season.active(today) {
            let wk = season.week_of(today).unwrap_or(0) + 1;
            format!("Week {wk} of {}  ·  {} days left", pass::WEEKS, season.days_left(today))
        } else if today < season.start_day {
            format!("Starts in {} days", season.start_day - today)
        } else {
            "Season over: rewards you reached can still be claimed".to_string()
        };
        p.text(pos2(x, r.min.y + 86.0), Align2::LEFT_CENTER, when, crate::fonts::sans_font(13.0), MUTED());
        // tier and the bar to the next one
        let bx = r.min.x + half * 0.9;
        let bw = r.max.x - 30.0 - bx;
        let into = xp - tier as u64 * pass::TIER_XP;
        let f = if tier >= pass::TIERS { 1.0 } else { into as f32 / pass::TIER_XP as f32 };
        p.text(pos2(bx, r.min.y + 32.0), Align2::LEFT_CENTER, format!("Tier {tier}"), crate::fonts::sans_bold_font(24.0), TEXT());
        p.text(pos2(bx + 96.0, r.min.y + 34.0), Align2::LEFT_CENTER, format!("of {}", pass::TIERS), crate::fonts::sans_font(13.0), MUTED());
        let bar = Rect::from_min_size(pos2(bx, r.min.y + 58.0), vec2(bw, 12.0));
        p.rect_filled(bar, 6.0, theme::blend(CARD(), TEXT(), 0.1));
        let fw = bar.width() * f;
        if fw > 1.0 {
            let mut mesh = egui::Mesh::default();
            let fr = Rect::from_min_size(bar.min, vec2(fw.max(12.0), 12.0));
            mesh.colored_vertex(fr.left_top(), CRIMSON);
            mesh.colored_vertex(fr.right_top(), GOLD);
            mesh.colored_vertex(fr.right_bottom(), GOLD);
            mesh.colored_vertex(fr.left_bottom(), CRIMSON);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            p.add(mesh);
        }
        let label = if tier >= pass::TIERS { "Every tier reached!".to_string() } else { format!("{} / {} XP to tier {}", theme::thousands(into), theme::thousands(pass::TIER_XP), tier + 1) };
        p.text(pos2(bx, r.min.y + 86.0), Align2::LEFT_CENTER, label, crate::fonts::sans_bold_font(12.0), TEXT());
        p.text(pos2(bar.max.x, r.min.y + 86.0), Align2::RIGHT_CENTER, format!("{} / {} season XP", theme::thousands(xp), theme::thousands(pass::SEASON_XP)), crate::fonts::sans_font(12.0), MUTED());
        let ready = season.all().filter(|rw| self.reward_state(rw, tier) == State::Ready).count();
        if ready > 0 {
            let t = format!("🎁  {ready} reward{} to claim", if ready == 1 { "" } else { "s" });
            let g = p.layout_no_wrap(t, crate::fonts::sans_bold_font(12.0), Color32::BLACK);
            let cr = Rect::from_min_size(pos2(bar.max.x - g.size().x - 20.0, r.min.y + 18.0), g.size() + vec2(20.0, 10.0));
            p.rect_filled(cr, cr.height() / 2.0, GOLD);
            p.galley(cr.min + vec2(10.0, 5.0), g, Color32::BLACK);
        }
    }

    /// How you look with what you're wearing: frame, picture, plate, title.
    fn pass_look(&mut self, ui: &mut egui::Ui, w: f32) {
        let ctx = ui.ctx().clone();
        let (r, _) = ui.allocate_exact_size(vec2(w, 96.0), Sense::hover());
        let p = ui.painter().clone();
        let frame = self.cfg.frame.clone();
        self.framed_avatar(&ctx, &p, &frame, pos2(r.min.x + 46.0, r.center().y), 30.0);
        let name = if self.cfg.trainer_name.trim().is_empty() { "Trainer".to_string() } else { self.cfg.trainer_name.trim().to_string() };
        let plate = self.cfg.plate.clone();
        let pr = Rect::from_min_size(pos2(r.min.x + 96.0, r.center().y - 26.0), vec2((w - 110.0).min(260.0), 34.0));
        if plate.is_empty() {
            p.text(pr.left_center() + vec2(4.0, 0.0), Align2::LEFT_CENTER, &name, crate::fonts::sans_bold_font(17.0), TEXT());
        } else {
            self.draw_plate_ui(&ctx, &p, &plate, pr, &name, 13.0);
        }
        let level = self.current_level();
        p.text(pos2(pr.min.x + 4.0, pr.max.y + 16.0), Align2::LEFT_CENTER, self.player_title(level), crate::fonts::sans_bold_font(12.5), {
            let t = self.player_title(level);
            title_col(&t, ch::ink(GOLD))
        });
    }

    // ───────────────────────────── the page ─────────────────────────────

    /// Profile → Battle Pass.
    pub(super) fn pass_section(&mut self, ui: &mut egui::Ui) {
        self.pass_track(ui);
    }

    fn two_cols(&mut self, ui: &mut egui::Ui, left_w: f32, gap: f32, mut part: impl FnMut(&mut App, &mut egui::Ui, usize)) {
        let full = ui.available_width();
        let lw = if left_w <= 1.0 { ((full - gap) * left_w.max(0.5)).floor() } else { left_w };
        let rw = full - gap - lw;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.allocate_ui_with_layout(vec2(lw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(lw);
                part(self, ui, 0);
            });
            ui.allocate_ui_with_layout(vec2(rw, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(rw);
                part(self, ui, 1);
            });
        });
    }

    /// 1: the banner, the whole track in a row you scroll, then missions.
    fn pass_track(&mut self, ui: &mut egui::Ui) {
        self.pass_header(ui, 112.0);
        ui.add_space(14.0);
        let today = local_day(now_unix());
        let season = pass::current(today);
        let tier = pass::tier(self.pass_xp_in(season));
        // Rewards (the whole track in a row you scroll) or the Locker
        let mut tab = self.setup.pass.tab.min(1);
        kit::tab_bar(ui, &mut tab, &[(0u8, "Rewards"), (1, "Locker")]);
        self.setup.pass.tab = tab;
        ui.add_space(12.0);
        if tab == 1 {
            // the locker on its own: missions belong to the Rewards tab
            self.pass_lockers(ui, today);
            return;
        }
        egui::ScrollArea::horizontal().id_salt("pass-track").show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for t in 1..=pass::TIERS {
                    let list: Vec<&Reward> = season.at(t).collect();
                    if list.len() > 1 {
                        // a milestone tier with a second reward: both together
                        egui::Frame::none()
                            .fill(theme::blend(CARD(), GOLD, 0.05))
                            .stroke(Stroke::new(1.0f32, theme::blend(LINE(), GOLD, 0.35)))
                            .rounding(18.0)
                            .inner_margin(egui::Margin::same(5.0))
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    for rw in list {
                                        ui.vertical(|ui| self.reward_tile(ui, rw, tier, 168.0, 100.0));
                                    }
                                });
                            });
                    } else {
                        for rw in list {
                            ui.vertical(|ui| self.reward_tile(ui, rw, tier, 168.0, 110.0));
                        }
                    }
                }
            });
            ui.add_space(6.0);
        });
        ui.add_space(14.0);
        if season.ended(today) {
            // no missions once the season is over
            kit::card(ui, |ui| {
                ui.label(RichText::new(format!("Season {} · {} has ended", season.num, season.name)).font(crate::fonts::sans_bold_font(15.0)).color(TEXT()));
                ui.add_space(4.0);
                ui.label(small("There are no more missions this season. Every reward you reached can still be claimed above, and everything you've claimed stays in your Locker for good."));
            });
            return;
        }
        self.two_cols(ui, 0.5, 16.0, |app, ui, i| {
            kit::card(ui, |ui| if i == 0 { app.pass_daily(ui) } else { app.pass_weekly(ui) });
        });
    }

    /// The Locker: this season's page, and a page for each earlier season
    /// (picked with the buttons along the top once there's more than one).
    fn pass_lockers(&mut self, ui: &mut egui::Ui, today: i64) {
        let seasons = pass::started(today);
        let mut pick = self.setup.pass.locker.filter(|n| seasons.iter().any(|s| s.num == *n)).unwrap_or(seasons[0].num);
        if seasons.len() > 1 {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for (i, s) in seasons.iter().enumerate() {
                    let label = if i == 0 { format!("Season {} · {}  (now)", s.num, s.name) } else { format!("Season {} · {}", s.num, s.name) };
                    let on = s.num == pick;
                    let b = egui::Button::new(RichText::new(label).size(kit::fs(12.0)).color(if on { kit::on_primary() } else { TEXT() }))
                        .fill(if on { kit::primary() } else { kit::btn_fill() })
                        .min_size(vec2(0.0, 30.0));
                    if ui.add(b).clicked() {
                        pick = s.num;
                    }
                }
            });
            ui.add_space(12.0);
        }
        self.setup.pass.locker = Some(pick);
        let season = seasons.iter().copied().find(|s| s.num == pick).unwrap_or(seasons[0]);
        if season.ended(today) {
            ui.label(small(format!(
                "Season {} · {} has ended. What you claimed stays yours to wear; anything you reached and haven't claimed yet can still be claimed here.",
                season.num, season.name
            )));
            ui.add_space(10.0);
        }
        let tier = pass::tier(self.pass_xp_in(season));
        self.pass_locker(ui, season, tier);
    }

    /// What you've claimed, by kind, with your look on the left.
    fn pass_locker(&mut self, ui: &mut egui::Ui, season: &'static pass::Season, tier: u32) {
        self.two_cols(ui, 330.0, 16.0, |app, ui, i| {
            if i == 0 {
                kit::card(ui, |ui| {
                    ui.label(caps("YOUR LOOK"));
                    ui.add_space(6.0);
                    let w = ui.available_width();
                    app.pass_look(ui, w);
                    ui.add_space(6.0);
                    ui.label(small("This is how you show on your Profile, in the sidebar, on your trainer card and on hunt images."));
                });
            } else {
                for (k, label) in [(Kind2::Plate, "NAME PLATES"), (Kind2::Frame, "PICTURE FRAMES"), (Kind2::Title, "TITLES")] {
                    kit::card(ui, |ui| {
                        ui.label(caps(label));
                        ui.add_space(6.0);
                        let list: Vec<Reward> = season.rewards.iter().filter(|r| r.kind == k).copied().collect();
                        let gap = 10.0;
                        let cols = 3usize;
                        let w = ((ui.available_width() - gap * 2.0) / 3.0).floor();
                        for row in list.chunks(cols) {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for rw in row {
                                    ui.vertical(|ui| app.reward_tile(ui, rw, tier, w, 84.0));
                                }
                            });
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(10.0);
                }
                let colours: Vec<Reward> = season.extras.iter().filter(|r| matches!(r.kind, Kind2::Colour(_))).copied().collect();
                if !colours.is_empty() {
                    kit::card(ui, |ui| {
                        ui.label(caps("POKÉGOTCHI COLOURS"));
                        ui.add_space(6.0);
                        let gap = 10.0;
                        let cols = 4usize;
                        let w = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
                        for row in colours.chunks(cols) {
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for rw in row {
                                    ui.vertical(|ui| app.reward_tile(ui, rw, tier, w, 110.0));
                                }
                            });
                            ui.add_space(gap);
                        }
                    });
                    ui.add_space(10.0);
                }
                kit::card(ui, |ui| {
                    ui.label(caps("HUNT IMAGES"));
                    ui.add_space(6.0);
                    let list: Vec<Reward> = season.rewards.iter().filter(|r| matches!(r.kind, Kind2::Style(_))).copied().collect();
                    let gap = 10.0;
                    let w = ((ui.available_width() - gap) / 2.0).floor();
                    for row in list.chunks(2) {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for rw in row {
                                ui.vertical(|ui| app.reward_tile(ui, rw, tier, w, 150.0));
                            }
                        });
                        ui.add_space(gap);
                    }
                });
            }
        });
    }

}

/// The text colour that reads on plate `id`.
pub(super) fn plate_ink(id: &str) -> Color32 {
    let tc = crate::cosmetics::plate_text(id);
    Color32::from_rgba_unmultiplied(tc.0, tc.1, tc.2, tc.3)
}

/// Where the top-left of `galley` goes so that its capital letters sit in the
/// middle of `c` (text boxes have room for accents and descenders, so centring
/// the box itself leaves names looking high or low).
pub(super) fn cap_centred(galley: &egui::Galley, c: egui::Pos2, size: f32) -> egui::Pos2 {
    let base = galley.rows.first().and_then(|r| r.glyphs.first()).map(|g| g.pos.y).unwrap_or(galley.size().y * 0.78);
    // capitals are about 0.72 of the font size tall (Inter / the app's sans)
    pos2((c.x - galley.size().x / 2.0).round(), (c.y - base + size * 0.36).round())
}

/// A name on a plate: bold, centred on `c` (capitals in the middle), with a
/// soft shadow.
pub(super) fn plate_text(p: &egui::Painter, c: egui::Pos2, name: &str, size: f32, col: Color32) {
    let f = crate::fonts::sans_bold_font(size);
    let shadow = p.layout_no_wrap(name.to_string(), f.clone(), Color32::from_black_alpha(160));
    let g = p.layout_no_wrap(name.to_string(), f, col);
    let at = cap_centred(&g, c, size);
    p.galley(at + vec2(1.0, 1.0), shadow, Color32::from_black_alpha(160));
    p.galley(at, g, col);
}
