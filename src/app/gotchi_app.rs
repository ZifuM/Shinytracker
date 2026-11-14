//! Pokégotchi's page: your partner in its device in the middle, the Box of
//! pets on the left, its needs on the right and the care buttons under it.
//! The device and its screen are drawn here too (the overlay reuses them).

use super::store::pill_button;
use super::ui_kit::{self as kit, LINE, MUTED, TEXT};
use super::{App, Tab};
use crate::apps::AppId;
use super::gotchi_words as words;
use crate::gotchi::{self, Care, Frame, GotchiFile, HiLo, Kind, Mood, Need, OvStyle, Tint};
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Shape, Stroke};
use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct GotchiState {
    pub file: Option<GotchiFile>,
    pub load_error: Option<String>,
    dirty: Option<Instant>,
    saved_once: bool,
    pub save_error: Option<String>,
    /// The game's logs were found and are being read for berries.
    pub reads_logs: bool,
    last_log_read: Option<Instant>,
    last_tick: Option<Instant>,
    last_autosave: Option<Instant>,
    /// The adopt popup, and its search.
    pub picker: bool,
    pick_query: String,
    /// The settings popup.
    pub settings: bool,
    /// Renaming the partner (the text being typed).
    renaming: Option<String>,
    /// Higher or Lower, while it's being played; and the score just
    /// finished, shown for a moment.
    pub game: Option<HiLo>,
    pub game_end: Option<(u32, Instant)>,
    /// What it just said (care, a new berry…), for the speech bubble.
    pub said: Option<(String, Instant)>,
    /// How long the bubble stays (thoughts stay longer).
    said_secs: f32,
    /// Time away found when the save was read: (seconds, from), to write
    /// up in the journal.
    away: Option<(u64, u64)>,
    /// When it next has a thought, and its next little moment.
    next_thought: Option<Instant>,
    next_fun: Option<Instant>,
    /// How many journal lines show.
    journal_show: usize,
    /// Levels and unlocked frames last seen (to note level-ups once).
    seen: Option<(Vec<(String, u32)>, usize)>,
    /// A little hop (care, an encounter), and a shiny party.
    pub hop: Option<Instant>,
    pub party: Option<Instant>,
    /// The last encounter (it naps on the overlay when you've been idle).
    pub last_enc: Option<Instant>,
    /// The device's menu: which care icon is picked.
    sel: usize,
    /// The overlay window, and whether it's open (Start overlay → Go).
    pub bar_ui: super::gotchi_bar::BarState,
    pub shown: bool,
}

impl GotchiState {
    pub fn file(&mut self) -> &mut GotchiFile {
        if self.file.is_none() {
            let (mut f, note) = gotchi::load_from(&crate::config::data_dir());
            // the time the app was closed: gently (and written up later)
            let now = crate::config::now_unix();
            if f.last > 0 && now > f.last + 20 * 60 && !f.pets.is_empty() {
                self.away = Some((now - f.last, f.last));
            }
            f.decay(now, false);
            f.check_unlocks();
            self.load_error = note;
            self.file = Some(f);
        }
        self.file.as_mut().expect("loaded")
    }

    pub fn touch(&mut self) {
        self.dirty.get_or_insert_with(Instant::now);
    }

    pub fn save_tick(&mut self, force: bool) {
        // closing: keep the time it was last looked after
        if force && self.file.as_ref().is_some_and(|f| f.has_data()) {
            self.touch();
        }
        let Some(t) = self.dirty else { return };
        if !force && t.elapsed() < Duration::from_millis(1500) {
            return;
        }
        let Some(f) = self.file.as_ref() else { return };
        match gotchi::save_to(&crate::config::data_dir(), f, !self.saved_once) {
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

    pub fn say(&mut self, s: impl Into<String>) {
        self.say_for(s, 2.6);
    }

    /// Say something for `secs` seconds.
    pub fn say_for(&mut self, s: impl Into<String>, secs: f32) {
        let s = s.into();
        if !s.is_empty() {
            self.said = Some((s, Instant::now()));
            self.said_secs = secs;
        }
    }

    /// The bubble's text while something was just said.
    pub fn saying(&self) -> Option<&str> {
        self.said.as_ref().filter(|(_, at)| at.elapsed().as_secs_f32() < self.said_secs.max(1.0)).map(|(s, _)| s.as_str())
    }

    /// The speech bubble's text right now.
    pub fn bubble(&self, mood: Mood) -> String {
        if let Some(s) = self.saying() {
            return s.to_string();
        }
        if self.party.is_some_and(|t| t.elapsed() < Duration::from_secs(8)) {
            return "A shiny!!".into();
        }
        mood.says().into()
    }

    fn care(&mut self, c: Care) -> bool {
        match c {
            Care::Done(s) => {
                self.say(s);
                self.hop = Some(Instant::now());
                self.touch();
                true
            }
            Care::No(s) => {
                self.say(s);
                false
            }
        }
    }
}

fn today() -> i64 {
    crate::stats::local_day(crate::config::now_unix())
}

/// The device's menu, in order (its icons along the top of the screen).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Act {
    Feed,
    Treat,
    Play,
    Clean,
    Pat,
    Lights,
}

impl Act {
    pub const ALL: [Act; 6] = [Act::Feed, Act::Treat, Act::Play, Act::Clean, Act::Pat, Act::Lights];
    pub fn name(self) -> &'static str {
        match self {
            Act::Feed => "Feed",
            Act::Treat => "Treat",
            Act::Play => "Play",
            Act::Clean => "Clean",
            Act::Pat => "Pat",
            Act::Lights => "Lights",
        }
    }
}

/// The colours of a frame: shell light, shell dark, accent, screen, ink.
pub(super) struct Look {
    pub light: Color32,
    pub dark: Color32,
    pub accent: Color32,
    pub screen: Color32,
    pub ink: Color32,
    pub stage: Color32,
}

/// The Pearl colour in use (set from the save each frame, so every
/// drawing of the device agrees).
static TINT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(super) fn set_tint(t: Tint) {
    TINT.store(Tint::ALL.iter().position(|x| *x == t).unwrap_or(0) as u8, std::sync::atomic::Ordering::Relaxed);
}

fn tint() -> Tint {
    Tint::ALL.get(TINT.load(std::sync::atomic::Ordering::Relaxed) as usize).copied().unwrap_or_default()
}

/// A Pearl colour: shell light and dark, accent, strap, bezel,
/// buttons (light, dark, edge).
pub(super) struct Pearl {
    pub light: Color32,
    pub dark: Color32,
    pub accent: Color32,
    pub strap: Color32,
    pub bezel: Color32,
    pub btn: (Color32, Color32, Color32),
}

pub(super) fn pearl(t: Tint) -> Pearl {
    let c = Color32::from_rgb;
    match t {
        Tint::Pink => Pearl { light: c(255, 252, 255), dark: c(196, 168, 208), accent: c(255, 140, 190), strap: c(228, 207, 227), bezel: c(185, 150, 184), btn: (Color32::WHITE, c(200, 100, 154), c(143, 63, 108)) },
        Tint::Sky => Pearl { light: c(252, 254, 255), dark: c(150, 186, 228), accent: c(110, 180, 255), strap: c(205, 222, 242), bezel: c(128, 160, 205), btn: (Color32::WHITE, c(80, 140, 220), c(40, 90, 160)) },
        Tint::Mint => Pearl { light: c(252, 255, 253), dark: c(140, 206, 180), accent: c(60, 200, 160), strap: c(200, 236, 222), bezel: c(110, 170, 150), btn: (Color32::WHITE, c(50, 170, 130), c(25, 110, 85)) },
        Tint::Platinum => Pearl { light: c(255, 255, 255), dark: c(178, 184, 198), accent: c(214, 180, 96), strap: c(222, 226, 234), bezel: c(146, 152, 168), btn: (Color32::WHITE, c(176, 182, 200), c(108, 114, 132)) },
        Tint::Midnight => Pearl { light: c(124, 130, 178), dark: c(26, 28, 54), accent: c(190, 170, 255), strap: c(90, 96, 140), bezel: c(62, 64, 104), btn: (c(235, 230, 255), c(140, 120, 220), c(70, 56, 140)) },
    }
}

pub(super) fn look(f: Frame) -> Look {
    match f {
        Frame::Pearl => {
            let pl = pearl(tint());
            Look { light: pl.light, dark: pl.dark, accent: pl.accent, screen: Color32::from_rgb(223, 233, 212), ink: Color32::from_rgb(70, 90, 60), stage: pl.accent }
        }
        Frame::Distortion => Look {
            light: Color32::from_rgb(70, 62, 78),
            dark: Color32::from_rgb(12, 10, 14),
            accent: Color32::from_rgb(241, 200, 96),
            screen: Color32::from_rgb(30, 16, 20),
            ink: Color32::from_rgb(255, 143, 122),
            stage: Color32::from_rgb(225, 70, 70),
        },
        Frame::Grass => Look {
            light: Color32::from_rgb(170, 232, 150),
            dark: Color32::from_rgb(38, 116, 56),
            accent: Color32::from_rgb(255, 211, 110),
            screen: Color32::from_rgb(233, 246, 214),
            ink: Color32::from_rgb(52, 120, 60),
            stage: Color32::from_rgb(110, 210, 120),
        },
    }
}

/// Where the parts of a drawn device are.
pub(super) struct Parts {
    pub screen: Rect,
    pub buttons: [Rect; 3],
    /// Its outline (the window on the overlay is cut to this).
    pub outline: Vec<Vec<Pos2>>,
}

/// A polygon filled with a radial shade from `light_at` (convex shapes).
fn shade(p: &egui::Painter, pts: &[Pos2], light_at: Pos2, inner: Color32, outer: Color32, edge: Stroke) {
    if pts.len() < 3 {
        return;
    }
    let far = pts.iter().map(|q| q.distance(light_at)).fold(1.0f32, f32::max);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(light_at, inner);
    for q in pts {
        let k = (q.distance(light_at) / far).powf(0.85);
        mesh.colored_vertex(*q, theme::blend(inner, outer, k));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(mesh));
    p.add(Shape::closed_line(pts.to_vec(), edge));
}

/// An egg: narrower at the top. `squash` < 1 makes it rounder.
fn egg(r: Rect, squash: f32) -> Vec<Pos2> {
    let (c, a, b) = (r.center(), r.width() / 2.0, r.height() / 2.0);
    (0..72)
        .map(|i| {
            let th = i as f32 / 72.0 * TAU;
            let y = -th.cos();
            let k = 1.0 + 0.09 * squash * y;
            pos2(c.x + a * th.sin() * k / 1.09, c.y + b * y)
        })
        .collect()
}

fn scaled(pts: &[Pos2], c: Pos2, k: f32) -> Vec<Pos2> {
    pts.iter().map(|q| c + (*q - c) * k).collect()
}

/// A soft glow.
fn glow(p: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, col);
    let n = 40u32;
    for i in 0..n {
        let a = i as f32 / n as f32 * TAU;
        mesh.colored_vertex(c + vec2(a.cos(), a.sin()) * r, Color32::TRANSPARENT);
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    p.add(Shape::mesh(mesh));
}

/// A four-point sparkle.
pub(super) fn spark(p: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let pts: Vec<Pos2> = (0..8)
        .map(|i| {
            let a = i as f32 * PI / 4.0 - PI / 2.0;
            let rr = if i % 2 == 0 { r } else { r * 0.3 };
            c + vec2(a.cos(), a.sin()) * rr
        })
        .collect();
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
}

/// A leaf blade from `base`, leaning by `lean` radians.
fn blade(p: &egui::Painter, base: Pos2, len: f32, lean: f32, light: Color32, dark: Color32) -> Vec<Pos2> {
    let dir = vec2(lean.sin(), -lean.cos());
    let side = vec2(dir.y, -dir.x);
    let w = len * 0.22;
    let mut pts = Vec::new();
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        pts.push(base + dir * len * t + side * w * (t * PI).sin());
    }
    for i in (1..10).rev() {
        let t = i as f32 / 10.0;
        pts.push(base + dir * len * t - side * w * 0.35 * (t * PI).sin());
    }
    shade(p, &pts, base + dir * len * 0.7, light, dark, Stroke::new(1.0f32, dark));
    pts
}

/// Draw the device in `r` (any size; it keeps its shape) and say where its
/// screen and buttons are. `pressed`: a button held down (0..3).
pub(super) fn paint_device(p: &egui::Painter, r: Rect, frame: Frame, t: f32, pressed: Option<usize>, hovered: Option<usize>, still: bool) -> Parts {
    // fit a 236 × 290 device
    let k = (r.width() / 236.0).min(r.height() / 290.0);
    let body = Rect::from_center_size(r.center(), vec2(236.0 * k, 290.0 * k));
    let lk = look(frame);
    let light_at = pos2(body.min.x + body.width() * 0.36, body.min.y + body.height() * 0.24);
    let mut outline: Vec<Vec<Pos2>> = Vec::new();
    match frame {
        Frame::Pearl => {
            // the strap loop
            let lc = pos2(body.center().x, body.min.y + 4.0 * k);
            let loop_pts: Vec<Pos2> = (0..=16).map(|i| lc + vec2((PI + i as f32 / 16.0 * PI).cos() * 17.0 * k, (PI + i as f32 / 16.0 * PI).sin() * 15.0 * k)).collect();
            p.add(Shape::line(loop_pts, Stroke::new(5.0 * k, pearl(tint()).strap)));
            let ring: Vec<Pos2> = (0..=16)
                .map(|i| lc + vec2((PI + i as f32 / 16.0 * PI).cos() * 19.5 * k, (PI + i as f32 / 16.0 * PI).sin() * 17.5 * k))
                .chain((0..=16).rev().map(|i| lc + vec2((PI + i as f32 / 16.0 * PI).cos() * 14.5 * k, (PI + i as f32 / 16.0 * PI).sin() * 12.5 * k)))
                .collect();
            outline.push(ring);
            let pts = egg(body, 1.0);
            outline.push(pts.clone());
            p.add(Shape::convex_polygon(pts.iter().map(|q| *q + vec2(0.0, 10.0 * k)).collect(), Color32::from_black_alpha(70), Stroke::NONE));
            shade(p, &pts, light_at, lk.light, lk.dark, Stroke::new(1.5f32, theme::blend(lk.dark, Color32::BLACK, 0.2)));
            p.add(Shape::closed_line(scaled(&pts, body.center(), 0.93), Stroke::new(2.0 * k, lk.accent.gamma_multiply(0.55))));
            // the lustre moving round slowly
            let a = t * 0.4;
            glow(p, light_at + vec2(a.cos(), a.sin()) * 12.0 * k, 34.0 * k, Color32::from_white_alpha(120));
        }
        Frame::Distortion => {
            let pulse = 0.75 + 0.25 * (t * 1.4).sin();
            glow(p, body.center() + vec2(0.0, -6.0 * k), 150.0 * k, Color32::from_rgba_unmultiplied(225, 60, 60, (80.0 * pulse) as u8));
            let shape = [(0.5, 0.0), (0.92, 0.14), (1.0, 0.52), (0.86, 0.92), (0.5, 1.0), (0.14, 0.92), (0.0, 0.52), (0.08, 0.14)];
            let pts: Vec<Pos2> = shape.iter().map(|(x, y)| pos2(body.min.x + body.width() * x, body.min.y + body.height() * y)).collect();
            outline.push(pts.clone());
            p.add(Shape::convex_polygon(pts.iter().map(|q| *q + vec2(0.0, 10.0 * k)).collect(), Color32::from_black_alpha(90), Stroke::NONE));
            shade(p, &pts, pos2(body.center().x, body.min.y), Color32::from_rgb(255, 226, 130), Color32::from_rgb(150, 104, 30), Stroke::new(1.0f32, Color32::from_rgb(110, 76, 20)));
            let inner = scaled(&pts, body.center(), 0.93);
            shade(p, &inner, light_at, lk.light, lk.dark, Stroke::new(1.0f32, Color32::from_rgb(90, 60, 20)));
            // gold seams to the corners
            for q in inner.iter().step_by(2) {
                let from = body.center() + (*q - body.center()) * 0.8;
                p.line_segment([from, *q], Stroke::new(1.0f32, lk.accent.gamma_multiply(0.35)));
            }
        }
        Frame::Grass => {
            let sway = if still { 0.0 } else { (t * 1.3).sin() * 0.08 };
            let top = body.min.y + 18.0 * k;
            for (dx, len, lean) in [(-52.0, 50.0, -0.5), (-24.0, 62.0, -0.15), (10.0, 56.0, 0.12), (40.0, 46.0, 0.45)] {
                outline.push(blade(p, pos2(body.center().x + dx * k, top + 12.0 * k), len * k, lean + sway, Color32::from_rgb(190, 244, 160), Color32::from_rgb(60, 150, 70)));
            }
            let pts = egg(body, 0.6);
            outline.push(pts.clone());
            p.add(Shape::convex_polygon(pts.iter().map(|q| *q + vec2(0.0, 10.0 * k)).collect(), Color32::from_black_alpha(70), Stroke::NONE));
            shade(p, &pts, light_at, lk.light, lk.dark, Stroke::new(1.5f32, Color32::from_rgb(28, 90, 44)));
            // sparkles twinkling round it
            for (i, (x, y, s)) in [(0.04, 0.32, 6.0), (0.95, 0.55, 5.0), (0.82, 0.12, 4.0), (0.12, 0.86, 4.5)].iter().enumerate() {
                let tw = (0.5 + 0.5 * (t * 2.2 + i as f32 * 1.7).sin()).powf(2.0);
                spark(p, pos2(body.min.x + body.width() * x, body.min.y + body.height() * y), s * k * (0.6 + 0.6 * tw), lk.accent.gamma_multiply(0.35 + 0.65 * tw));
            }
        }
    }
    // the screen
    let screen = Rect::from_center_size(pos2(body.center().x, body.min.y + body.height() * 0.43), vec2(150.0 * k, 128.0 * k));
    let bezel = screen.expand(5.0 * k);
    let bz = match frame {
        Frame::Pearl => pearl(tint()).bezel,
        Frame::Distortion => Color32::from_rgb(183, 138, 44),
        Frame::Grass => Color32::from_rgb(45, 122, 60),
    };
    p.rect_filled(bezel, 13.0 * k, bz);
    p.rect_filled(screen, 10.0 * k, lk.screen);
    if frame == Frame::Distortion {
        p.rect_stroke(bezel.expand(2.0 * k), 15.0 * k, Stroke::new(2.0 * k, Color32::from_rgba_unmultiplied(255, 90, 70, 60)));
    }
    // the buttons
    let by = body.min.y + body.height() * 0.82;
    let gap = 44.0 * k;
    let mut buttons = [Rect::NOTHING; 3];
    for (i, b) in buttons.iter_mut().enumerate() {
        let c = pos2(body.center().x + (i as f32 - 1.0) * gap, by + if i == 1 { 6.0 * k } else { 0.0 });
        let size = 26.0 * k;
        *b = Rect::from_center_size(c, vec2(size, size));
        let down = pressed == Some(i);
        let hov = hovered == Some(i);
        let c2 = c + vec2(0.0, if down { 2.5 * k } else { 0.0 });
        match frame {
            Frame::Distortion => {
                let s = size * 0.5;
                let dia = |c: Pos2, s: f32| vec![c + vec2(0.0, -s), c + vec2(s, 0.0), c + vec2(0.0, s), c + vec2(-s, 0.0)];
                if !down {
                    p.add(Shape::convex_polygon(dia(c + vec2(0.0, 3.0 * k), s), Color32::from_rgb(110, 76, 20), Stroke::NONE));
                }
                glow(p, c2, s * 1.8, Color32::from_rgba_unmultiplied(255, 180, 60, if hov { 90 } else { 45 }));
                shade(p, &dia(c2, s), c2 + vec2(-s * 0.3, -s * 0.4), Color32::from_rgb(255, 236, 160), Color32::from_rgb(176, 125, 34), Stroke::new(1.0f32, Color32::from_rgb(120, 80, 20)));
            }
            _ => {
                let (hi, lo, base) = match frame {
                    Frame::Pearl => pearl(tint()).btn,
                    _ => (Color32::from_rgb(255, 248, 201), Color32::from_rgb(201, 154, 42), Color32::from_rgb(138, 102, 20)),
                };
                if !down {
                    p.circle_filled(c + vec2(0.0, 3.0 * k), size / 2.0, base);
                }
                let pts: Vec<Pos2> = (0..28).map(|j| c2 + vec2((j as f32 / 28.0 * TAU).cos(), (j as f32 / 28.0 * TAU).sin()) * size / 2.0).collect();
                shade(p, &pts, c2 + vec2(-size * 0.15, -size * 0.2), if hov { hi } else { theme::blend(hi, lk.accent, 0.2) }, lo, Stroke::new(1.0f32, base));
            }
        }
    }
    Parts { screen, buttons, outline }
}

/// A care icon, drawn (so it looks the same everywhere).
pub(super) fn act_icon(p: &egui::Painter, c: Pos2, s: f32, a: Act, col: Color32) {
    let st = Stroke::new((s * 0.12).max(1.0), col);
    match a {
        Act::Feed => {
            // a berry with a leaf
            p.circle_filled(c + vec2(0.0, s * 0.12), s * 0.36, col);
            p.add(Shape::convex_polygon(vec![c + vec2(0.0, -s * 0.22), c + vec2(s * 0.34, -s * 0.48), c + vec2(s * 0.12, -s * 0.18)], col, Stroke::NONE));
        }
        Act::Treat => {
            // a wrapped sweet
            p.circle_filled(c, s * 0.26, col);
            for d in [-1.0f32, 1.0] {
                p.add(Shape::convex_polygon(vec![c + vec2(d * s * 0.2, 0.0), c + vec2(d * s * 0.5, -s * 0.22), c + vec2(d * s * 0.5, s * 0.22)], col, Stroke::NONE));
            }
        }
        Act::Play => {
            // a ball
            p.circle_stroke(c, s * 0.38, st);
            p.line_segment([c + vec2(-s * 0.38, 0.0), c + vec2(s * 0.38, 0.0)], st);
            p.circle_filled(c, s * 0.12, col);
        }
        Act::Clean => {
            // bubbles
            p.circle_stroke(c + vec2(-s * 0.12, s * 0.1), s * 0.26, st);
            p.circle_stroke(c + vec2(s * 0.26, -s * 0.22), s * 0.15, st);
            p.circle_filled(c + vec2(s * 0.3, s * 0.3), s * 0.07, col);
        }
        Act::Pat => {
            // a heart
            let h = s * 0.42;
            let pts: Vec<Pos2> = (0..40)
                .map(|i| {
                    let t = i as f32 / 40.0 * TAU;
                    let x = 16.0 * t.sin().powi(3);
                    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
                    c + vec2(x, -y - 2.0) * (h / 16.0)
                })
                .collect();
            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Act::Lights => {
            // a moon
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = PI * 0.25 + i as f32 / 24.0 * PI * 1.5;
                    c + vec2(a.cos(), a.sin()) * s * 0.4
                })
                .chain((0..=24).rev().map(|i| {
                    let a = PI * 0.25 + i as f32 / 24.0 * PI * 1.5;
                    c + vec2(s * 0.16, -s * 0.1) + vec2(a.cos(), a.sin()) * s * 0.3
                }))
                .collect();
            p.add(Shape::Path(egui::epaint::PathShape { points: pts, closed: true, fill: col, stroke: egui::epaint::PathStroke::NONE }));
        }
    }
}

/// The colour of a need's meter.
pub(super) fn need_col(n: Need) -> Color32 {
    match n {
        Need::Full => Color32::from_rgb(127, 224, 164),
        Need::Happy => Color32::from_rgb(255, 156, 199),
        Need::Energy => Color32::from_rgb(255, 211, 110),
        Need::Clean => Color32::from_rgb(120, 200, 255),
    }
}

/// A rounded meter.
pub(super) fn meter(p: &egui::Painter, r: Rect, frac: f32, col: Color32, track: Color32) {
    p.rect_filled(r, r.height() / 2.0, track);
    let frac = frac.clamp(0.0, 1.0);
    if frac > 0.0 {
        let w = (r.width() * frac).max(r.height());
        p.rect_filled(Rect::from_min_size(r.min, vec2(w, r.height())), r.height() / 2.0, col);
    }
}

/// A speech bubble pointing down at `tip` (wrapping at `max_w`).
pub(super) fn bubble(p: &egui::Painter, tip: Pos2, text: &str, size: f32, ink: Color32, fill: Color32, max_w: f32) -> Rect {
    let g = p.layout(text.to_string(), crate::fonts::sans_bold_font(size), ink, (max_w - size * 1.2).max(30.0));
    let r = Rect::from_min_size(tip - vec2(g.size().x / 2.0 + size * 0.6, g.size().y + size * 1.3), g.size() + vec2(size * 1.2, size * 0.6));
    p.rect_filled(r, size * 0.7, fill);
    p.add(Shape::convex_polygon(vec![pos2(tip.x - size * 0.35, r.max.y - 0.5), pos2(tip.x + size * 0.35, r.max.y - 0.5), tip], fill, Stroke::NONE));
    p.galley(r.min + vec2(size * 0.6, size * 0.3), g, ink);
    r
}

impl App {
    fn gotchi_on(&self) -> bool {
        crate::apps::live(AppId::Gotchi)
    }

    /// The shinies you've caught, as (species index, slug, name), newest first.
    pub(super) fn gotchi_caught(&self) -> Vec<(usize, String, String)> {
        let mut v: Vec<(u64, usize, String, String)> = self
            .dex_caught()
            .into_iter()
            .filter_map(|(slug, src)| {
                let i = self.species.iter().position(|s| s.slug == slug)?;
                let when = match src {
                    super::extras::DexSource::Hunt(t, _) => t,
                    _ => 0,
                };
                Some((when, i, slug, self.species[i].name.to_string()))
            })
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.3.cmp(&b.3)));
        v.into_iter().map(|(_, i, s, n)| (i, s, n)).collect()
    }

    /// The partner's species index.
    pub(super) fn gotchi_species(&self, slug: &str) -> Option<usize> {
        self.species.iter().position(|s| s.slug == slug)
    }

    /// The partner's name (its nickname, else the species).
    pub(super) fn gotchi_name(&self, pet: &gotchi::Pet) -> String {
        if !pet.nick.trim().is_empty() {
            return pet.nick.trim().to_string();
        }
        self.gotchi_species(&pet.slug).map(|i| self.species[i].name.to_string()).unwrap_or_else(|| pet.slug.clone())
    }

    /// Every frame: needs (each second), the logs (every few seconds), saving.
    pub(super) fn gotchi_tick(&mut self, _ctx: &egui::Context) {
        if !self.gotchi_on() {
            return;
        }
        set_tint(self.setup.gotchi.file().tint);
        let due = self.setup.gotchi.last_tick.map_or(true, |t| t.elapsed() >= Duration::from_secs(1));
        if due {
            self.setup.gotchi.last_tick = Some(Instant::now());
            let now = crate::config::now_unix();
            let f = self.setup.gotchi.file();
            let before = f.partner().map(|p| p.needs);
            let slept = f.asleep;
            f.decay(now, true);
            let woke = slept && !f.asleep;
            let moved = before != f.partner().map(|p| p.needs);
            let new_day = f.new_day(today());
            let name = self.setup.gotchi.file().partner().cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
            if woke {
                self.setup.gotchi.say("Good morning!");
                self.gotchi_log(Kind::Care, format!("{name} woke up feeling rested."));
            }
            if new_day {
                self.setup.gotchi.say(format!("+{} berries today", gotchi::DAILY_BERRIES));
                self.gotchi_log(Kind::Care, format!("A new day! {} berries grew overnight.", gotchi::DAILY_BERRIES));
            }
            // needs move a little every second: saved once a minute (and on exit)
            let minute = self.setup.gotchi.last_autosave.map_or(true, |t| t.elapsed() >= Duration::from_secs(60));
            if new_day || woke || (moved && minute) {
                self.setup.gotchi.last_autosave = Some(Instant::now());
                self.setup.gotchi.touch();
            }
        }
        if self.setup.gotchi.last_log_read.map_or(true, |t| t.elapsed() > Duration::from_secs(8)) {
            self.setup.gotchi.last_log_read = Some(Instant::now());
            self.gotchi_read_logs();
        }
        self.gotchi_life();
        if let Some((score, at)) = self.setup.gotchi.game_end {
            if at.elapsed() > Duration::from_millis(2600) {
                self.setup.gotchi.game_end = None;
                let _ = score;
            }
        }
        self.setup.gotchi.save_tick(false);
    }

    /// Add a line to the journal, now.
    pub(super) fn gotchi_log(&mut self, kind: Kind, text: impl Into<String>) {
        let now = crate::config::now_unix();
        self.setup.gotchi.file().note(now, kind, text);
        self.setup.gotchi.touch();
    }

    /// Its own life: what it did while you were away, thoughts every few
    /// minutes, little moments, and level-ups (noted once).
    fn gotchi_life(&mut self) {
        let Some(pet) = self.setup.gotchi.file().partner().cloned() else { return };
        let name = self.gotchi_name(&pet);
        let now = crate::config::now_unix();
        let mut rng = crate::cards::Rng::seeded_now(now);
        // while you were away
        if let Some((secs, from)) = self.setup.gotchi.away.take() {
            let lines = words::away(&mut rng, &name, secs);
            let n = lines.len() as u64;
            let f = self.setup.gotchi.file();
            f.note(from + 60, Kind::Away, format!("You were away for {}.", words::span(secs)));
            for (i, l) in lines.into_iter().enumerate() {
                f.note(from + secs * (i as u64 + 1) / (n + 1), Kind::Away, l);
            }
            let hungry = f.partner().is_some_and(|p| p.need(Need::Full) < 40.0);
            let welcome = words::fill(words::WELCOME[rng.below(words::WELCOME.len())], &name);
            f.note(now, Kind::Away, if hungry { format!("{welcome} Its tummy is rumbling, though.") } else { welcome.clone() });
            self.setup.gotchi.say_for(if hungry { "You're back! I'm hungry…" } else { "You're back!" }, 4.0);
            self.setup.gotchi.touch();
        }
        // level-ups and new frames, noted once
        let f = self.setup.gotchi.file();
        let levels: Vec<(String, u32)> = f.pets.iter().map(|p| (p.slug.clone(), p.level())).collect();
        let unlocked = f.unlocked.clone();
        match self.setup.gotchi.seen.take() {
            Some((before, n)) => {
                for (slug, lv) in &levels {
                    if before.iter().any(|(s, b)| s == slug && lv > b) {
                        let who = self.setup.gotchi.file().pets.iter().find(|p| &p.slug == slug).cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
                        self.gotchi_log(Kind::Big, format!("{who} reached friendship level {lv}!"));
                    }
                }
                for fr in unlocked.iter().skip(n) {
                    self.gotchi_log(Kind::Big, format!("New frame unlocked: {}.", fr.name()));
                }
            }
            None => {}
        }
        self.setup.gotchi.seen = Some((levels, unlocked.len()));
        // thoughts, every few minutes (dreams while asleep)
        let asleep = self.setup.gotchi.file().asleep;
        let g = &mut self.setup.gotchi;
        let due = g.next_thought.get_or_insert_with(|| Instant::now() + Duration::from_secs(45 + (now % 60)));
        if Instant::now() >= *due && g.game.is_none() {
            let wait = if asleep { 600 + rng.below(300) } else { 240 + rng.below(300) };
            g.next_thought = Some(Instant::now() + Duration::from_secs(wait as u64));
            if asleep {
                let d = words::DREAMS[rng.below(words::DREAMS.len())];
                self.setup.gotchi.say_for("zzz…", 4.0);
                self.gotchi_log(Kind::Thought, format!("{name} {d}."));
            } else {
                let hour = (((now as i64 + crate::stats::utc_offset_secs()).rem_euclid(86_400)) / 3600) as u32;
                let target = self.target_label();
                let line = words::thought(&mut rng, pet.mood(false), hour, Some(&target), self.cfg.count);
                self.setup.gotchi.say_for(line.clone(), 7.0);
                self.gotchi_log(Kind::Thought, format!("“{line}”"));
            }
        }
        // a little moment of its own now and then
        let g = &mut self.setup.gotchi;
        let due = g.next_fun.get_or_insert_with(|| Instant::now() + Duration::from_secs(600 + (now % 300)));
        if Instant::now() >= *due && !asleep && g.game.is_none() {
            g.next_fun = Some(Instant::now() + Duration::from_secs(720 + rng.below(900) as u64));
            g.hop = Some(Instant::now());
            if rng.below(4) == 0 {
                let l = words::fill(words::FUN_BERRY[rng.below(words::FUN_BERRY.len())], &name);
                self.setup.gotchi.file().berries += 1;
                self.setup.gotchi.say_for("+1 berry!", 3.0);
                self.gotchi_log(Kind::Fun, format!("{l} (+1 berry)"));
            } else {
                let l = words::fill(words::FUN[rng.below(words::FUN.len())], &name);
                self.gotchi_log(Kind::Fun, l);
            }
        }
    }

    fn gotchi_read_logs(&mut self) {
        let Some(dir) = self.game_log_dir() else {
            self.setup.gotchi.reads_logs = false;
            return;
        };
        let mut cursors = std::mem::take(&mut self.setup.gotchi.file().logs);
        let read = crate::gamelog::read_new_wild(&dir, &mut cursors);
        self.setup.gotchi.file().logs = cursors;
        let Some((found, moved)) = read else {
            self.setup.gotchi.reads_logs = false;
            return;
        };
        self.setup.gotchi.reads_logs = true;
        if moved {
            self.setup.gotchi.touch();
        }
        if !found.is_empty() {
            let rare = found.iter().filter(|f| f.1).count() as u32;
            self.gotchi_earn(found.len() as u32, rare);
        }
    }

    /// Pokémon met while hunting: berries and treats.
    fn gotchi_earn(&mut self, n: u32, rare: u32) {
        let f = self.setup.gotchi.file();
        let had = f.unlocked.len();
        let (b, t) = f.met(n, rare, today());
        f.note_hunt(crate::config::now_unix(), b, t);
        let unlocked = f.unlocked.len() > had;
        if b > 0 || t > 0 {
            let mut s = Vec::new();
            if b > 0 {
                s.push(format!("+{b} berr{}", if b == 1 { "y" } else { "ies" }));
            }
            if t > 0 {
                s.push(format!("+{t} treat{}", if t == 1 { "" } else { "s" }));
            }
            self.setup.gotchi.say(s.join("  "));
        }
        if unlocked {
            self.setup.gotchi.say("New frame unlocked!");
            self.play(crate::sound::Sfx::Achievement);
        }
        self.setup.gotchi.touch();
    }

    /// An encounter counted by the overlay: a hop; and berries when the
    /// game's log isn't being read (screen reading).
    pub(super) fn gotchi_encounter(&mut self, added: u32, rare: bool, from_log: bool) {
        if !self.gotchi_on() || added == 0 {
            return;
        }
        self.setup.gotchi.hop = Some(Instant::now());
        self.setup.gotchi.last_enc = Some(Instant::now());
        if !from_log || !self.setup.gotchi.reads_logs {
            self.gotchi_earn(added, if rare { added } else { 0 });
        }
    }

    /// A shiny was logged (once each).
    pub(super) fn gotchi_shiny(&mut self, key: String) {
        if !self.gotchi_on() {
            return;
        }
        let f = self.setup.gotchi.file();
        if f.pets.is_empty() || f.shiny_keys.contains(&key) {
            return;
        }
        f.shiny_keys.push(key);
        if f.shiny_keys.len() > 60 {
            f.shiny_keys.remove(0);
        }
        f.shiny_found();
        let name = f.partner().cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
        self.gotchi_log(Kind::Big, format!("You found a shiny! {name} did a happy dance."));
        self.setup.gotchi.party = Some(Instant::now());
        self.setup.gotchi.touch();
    }

    /// Do a care action from the page, the device or the overlay.
    pub(super) fn gotchi_act(&mut self, a: Act) {
        let now = crate::config::now_unix();
        let f = self.setup.gotchi.file();
        let had = f.unlocked.len();
        let c = match a {
            Act::Feed => f.feed(false),
            Act::Treat => f.feed(true),
            Act::Clean => f.clean(),
            Act::Pat => f.pat(now),
            Act::Lights => f.lights(),
            Act::Play => match f.can_play() {
                Ok(()) => {
                    let seed = crate::cards::Rng::seeded_now(now).next();
                    let g = &mut self.setup.gotchi;
                    g.game = Some(HiLo::new(seed));
                    g.game_end = None;
                    g.say("Higher or lower?");
                    return;
                }
                Err(e) => Care::No(e),
            },
        };
        let unlocked = self.setup.gotchi.file().unlocked.len() > had;
        if matches!(c, Care::Done(_)) {
            let name = self.setup.gotchi.file().partner().cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
            let asleep = self.setup.gotchi.file().asleep;
            let line = match a {
                Act::Feed => Some(format!("You fed {name} a berry.")),
                Act::Treat => Some(format!("You gave {name} a treat. Yum!")),
                Act::Clean => Some(format!("You gave {name} a good clean.")),
                Act::Lights => Some(if asleep { format!("Lights off. Sweet dreams, {name}.") } else { format!("Lights on. {name} is up!") }),
                // pats are many and small: one line every so often
                Act::Pat => {
                    let recent = self.setup.gotchi.file().journal.iter().rev().take(6).any(|e| e.kind == Kind::Care && e.text.contains("pat") && now.saturating_sub(e.at) < 30 * 60);
                    (!recent).then(|| format!("You gave {name} a pat on the head."))
                }
                Act::Play => None,
            };
            if let Some(l) = line {
                self.gotchi_log(Kind::Care, l);
            }
        }
        self.setup.gotchi.care(c);
        if unlocked {
            self.setup.gotchi.say("New frame unlocked!");
            self.play(crate::sound::Sfx::Achievement);
        }
    }

    /// One of the device's three buttons (on the page or the overlay): pick
    /// the previous / next care icon or do it; in a game, lower / stop / higher.
    pub(super) fn gotchi_press(&mut self, i: usize) {
        if self.setup.gotchi.game.is_some() {
            match i {
                0 => self.gotchi_guess(false),
                2 => self.gotchi_guess(true),
                _ => {
                    self.setup.gotchi.game = None;
                    self.setup.gotchi.say("Maybe later");
                }
            }
        } else {
            let n = Act::ALL.len();
            match i {
                0 => self.setup.gotchi.sel = (self.setup.gotchi.sel + n - 1) % n,
                2 => self.setup.gotchi.sel = (self.setup.gotchi.sel + 1) % n,
                _ => self.gotchi_act(Act::ALL[self.setup.gotchi.sel]),
            }
        }
    }

    /// The tooltip of a device button right now.
    pub(super) fn gotchi_button_tip(&self, i: usize) -> String {
        if self.setup.gotchi.game.is_some() {
            return ["Lower", "Stop the game", "Higher"][i].to_string();
        }
        match i {
            1 => format!("Do it: {}", Act::ALL[self.setup.gotchi.sel.min(Act::ALL.len() - 1)].name()),
            0 => "Previous".to_string(),
            _ => "Next".to_string(),
        }
    }

    /// A guess in Higher or Lower.
    pub(super) fn gotchi_guess(&mut self, higher: bool) {
        let Some(g) = self.setup.gotchi.game.as_mut() else { return };
        let right = g.guess(higher);
        let done = g.done().then_some(g.score);
        self.setup.gotchi.say(if right { "Yes!" } else { "Nope!" });
        if right {
            self.setup.gotchi.hop = Some(Instant::now());
        }
        if let Some(score) = done {
            self.setup.gotchi.game = None;
            self.setup.gotchi.game_end = Some((score, Instant::now()));
            self.setup.gotchi.file().played(score, today());
            let name = self.setup.gotchi.file().partner().cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
            let how = match score {
                5 => "a perfect 5 out of 5!",
                3 | 4 => "and got {s} out of 5.",
                _ => "and got {s} out of 5. Next time!",
            }
            .replace("{s}", &score.to_string());
            self.gotchi_log(Kind::Care, format!("You played Higher or Lower with {name} {how}").replace(" a perfect", ": a perfect"));
            self.setup.gotchi.say(match score {
                5 => "Perfect!!",
                3 | 4 => "That was fun!",
                _ => "Again sometime!",
            });
            self.setup.gotchi.touch();
        }
    }

    // ───────────────────────────── the screen ─────────────────────────────

    /// What's on the device's screen: the menu icons, your pet, its needs.
    /// `small`: the overlay's mini device (no menu, bigger pet).
    pub(super) fn gotchi_screen(&mut self, p: &egui::Painter, s: Rect, frame: Frame, t: f32, small: bool) {
        let lk = look(frame);
        let f = self.setup.gotchi.file().clone();
        let asleep = f.asleep;
        let p = p.with_clip_rect(s);
        let k = s.width() / 150.0;
        let ink = lk.ink;
        let faint = theme::blend(lk.screen, ink, 0.22);
        let Some(pet) = f.partner().cloned() else {
            // an egg waiting
            let wob = (t * 3.0).sin() * 0.08;
            let c = s.center() + vec2(0.0, 6.0 * k);
            let pts: Vec<Pos2> = egg(Rect::from_center_size(c, vec2(34.0 * k, 42.0 * k)), 1.0).into_iter().map(|q| c + egui::emath::Rot2::from_angle(wob) * (q - c)).collect();
            p.add(Shape::convex_polygon(pts, theme::blend(lk.screen, ink, 0.12), Stroke::new(2.0 * k, ink)));
            p.text(c + vec2(0.0, 2.0 * k), Align2::CENTER_CENTER, "?", crate::fonts::pixel_font((12.0 * k).round().max(7.0)), ink);
            return;
        };
        // lights off: a dark screen
        let dark = theme::blend(lk.screen, Color32::from_rgb(10, 12, 24), 0.82);
        if asleep {
            p.rect_filled(s, 10.0 * k, dark);
        }
        // the pixel grid, faintly
        if !small {
            let step = 4.0 * k;
            let gc = Color32::from_black_alpha(if asleep { 0 } else { 10 });
            let mut x = s.min.x + step;
            while x < s.max.x {
                p.vline(x, s.y_range(), Stroke::new(1.0f32, gc));
                x += step;
            }
        }
        // the menu along the top
        let menu_h = if small { 0.0 } else { 20.0 * k };
        if !small {
            let n = Act::ALL.len() as f32;
            let cw = (s.width() - 12.0 * k) / n;
            for (i, a) in Act::ALL.iter().enumerate() {
                let c = pos2(s.min.x + 6.0 * k + cw * (i as f32 + 0.5), s.min.y + 11.0 * k);
                let on = self.setup.gotchi.sel == i && self.setup.gotchi.game.is_none();
                let ic = if asleep { theme::blend(dark, lk.screen, 0.35) } else { ink };
                if on {
                    p.rect_filled(Rect::from_center_size(c, vec2(cw - 2.0 * k, 16.0 * k)), 3.0 * k, ic);
                }
                act_icon(&p, c, 11.0 * k, *a, if on { if asleep { dark } else { lk.screen } } else if asleep { ic } else { theme::blend(lk.screen, ink, 0.55) });
            }
        }
        // the needs along the bottom
        let bars_h = 9.0 * k;
        let bars = Rect::from_min_max(pos2(s.min.x + 8.0 * k, s.max.y - bars_h - 4.0 * k), pos2(s.max.x - 8.0 * k, s.max.y - 4.0 * k));
        {
            let gap = 4.0 * k;
            let w = (bars.width() - gap * 3.0) / 4.0;
            for (i, n) in Need::ALL.iter().enumerate() {
                let r = Rect::from_min_size(pos2(bars.min.x + i as f32 * (w + gap), bars.center().y - 2.5 * k), vec2(w, 5.0 * k));
                let fill = if asleep { theme::blend(dark, lk.screen, 0.45) } else { ink };
                let track = if asleep { theme::blend(dark, lk.screen, 0.12) } else { faint };
                meter(&p, r, pet.need(*n) / 100.0, fill, track);
            }
        }
        let mid = Rect::from_min_max(pos2(s.min.x, s.min.y + menu_h), pos2(s.max.x, bars.min.y));
        // Higher or Lower
        if let Some(g) = self.setup.gotchi.game.clone() {
            let big = crate::fonts::pixel_font((22.0 * k).round());
            let c = mid.center() + vec2(-14.0 * k, 4.0 * k);
            p.text(c, Align2::CENTER_CENTER, g.shown.to_string(), big, ink);
            p.text(c + vec2(30.0 * k, 0.0), Align2::CENTER_CENTER, "?", crate::fonts::pixel_font((14.0 * k).round()), theme::blend(lk.screen, ink, 0.6));
            p.text(pos2(mid.min.x + 8.0 * k, mid.min.y + 6.0 * k), Align2::LEFT_TOP, format!("{}/{}", g.round + 1, HiLo::ROUNDS), crate::fonts::pixel_font((7.0 * k).round().max(6.0)), ink);
            p.text(pos2(mid.max.x - 8.0 * k, mid.min.y + 6.0 * k), Align2::RIGHT_TOP, format!("★{}", g.score), crate::fonts::sans_bold_font(9.0 * k), ink);
            p.text(pos2(mid.min.x + 10.0 * k, mid.max.y - 6.0 * k), Align2::LEFT_BOTTOM, "◀ LO", crate::fonts::sans_bold_font(8.5 * k), ink);
            p.text(pos2(mid.max.x - 10.0 * k, mid.max.y - 6.0 * k), Align2::RIGHT_BOTTOM, "HI ▶", crate::fonts::sans_bold_font(8.5 * k), ink);
            // the pet watching from the corner
            if let Some(i) = self.gotchi_species(&pet.slug) {
                let sp = self.species[i].clone();
                let hop = self.setup.gotchi.hop.map_or(0.0, |h| hop_y(h.elapsed().as_secs_f32()));
                self.draw_sprite(&p, &sp, true, pos2(mid.max.x - 22.0 * k, mid.center().y + 16.0 * k - hop * 8.0 * k), vec2(30.0 * k, 30.0 * k));
            }
            return;
        }
        if let Some((score, _)) = self.setup.gotchi.game_end {
            p.text(mid.center() + vec2(0.0, -8.0 * k), Align2::CENTER_CENTER, format!("{score}/{}", HiLo::ROUNDS), crate::fonts::pixel_font((16.0 * k).round()), ink);
            p.text(mid.center() + vec2(0.0, 14.0 * k), Align2::CENTER_CENTER, "SCORE", crate::fonts::pixel_font((7.0 * k).round().max(6.0)), ink);
            return;
        }
        // your pet
        let mood = pet.mood(asleep);
        let bob = if asleep { 0.0 } else { (t * 2.4).sin() * 1.5 * k };
        let hop = self.setup.gotchi.hop.map_or(0.0, |h| hop_y(h.elapsed().as_secs_f32()));
        let party = self.setup.gotchi.party.is_some_and(|h| h.elapsed() < Duration::from_secs(8));
        let party_hop = if party { ((t * 7.0).sin().abs()) * 6.0 * k } else { 0.0 };
        let foot = pos2(mid.center().x, mid.max.y - 4.0 * k - bob - hop * 12.0 * k - party_hop);
        let size = if small { vec2(mid.width() * 0.8, mid.height() * 0.86) } else { vec2(mid.width() * 0.62, mid.height() - 8.0 * k) };
        // its shadow
        p.add(Shape::ellipse_filled(pos2(mid.center().x, mid.max.y - 3.0 * k), vec2(size.x * 0.3, 2.5 * k), Color32::from_black_alpha(if asleep { 40 } else { 28 })));
        let drawn = match self.gotchi_species(&pet.slug) {
            Some(i) => {
                let sp = self.species[i].clone();
                if asleep {
                    self.draw_sprite_tinted(&p, &sp, true, foot, size, Color32::from_rgb(110, 120, 160))
                } else {
                    self.draw_sprite(&p, &sp, true, foot, size)
                }
            }
            None => false,
        };
        if !drawn {
            p.circle_filled(foot - vec2(0.0, 14.0 * k), 12.0 * k, faint);
        }
        // mess on the floor: little piles with wavy lines over them
        let clean = pet.need(Need::Clean);
        let marks = if clean < 25.0 { 3 } else if clean < 45.0 { 2 } else if clean < 65.0 { 1 } else { 0 };
        for m in 0..marks {
            let x = [mid.max.x - 18.0 * k, mid.min.x + 18.0 * k, mid.max.x - 36.0 * k][m];
            let base = pos2(x, mid.max.y - 3.0 * k);
            let mc = if asleep { theme::blend(dark, lk.screen, 0.3) } else { theme::blend(lk.screen, ink, 0.7) };
            for (j, (w, h)) in [(10.0, 4.4), (7.6, 3.8), (5.0, 3.2)].iter().enumerate() {
                let c = base - vec2(0.0, (2.0 + j as f32 * 2.3) * k);
                p.add(Shape::ellipse_filled(c, vec2(w * k * 0.5, h * k * 0.5), mc));
            }
            p.circle_filled(base - vec2(-0.6 * k, 8.6 * k), 1.1 * k, mc);
            for d in [-1.0f32, 1.0] {
                let wv = (t * 3.0 + m as f32 + d).sin() * 1.2 * k;
                let x0 = base.x + d * 4.0 * k;
                let pts: Vec<Pos2> = (0..=6).map(|i| pos2(x0 + ((i as f32 * 1.4).sin()) * 1.2 * k + wv * (i as f32 / 6.0), base.y - 13.0 * k - i as f32 * 1.2 * k)).collect();
                p.add(Shape::line(pts, Stroke::new(0.9 * k, mc.gamma_multiply(0.8))));
            }
        }
        // sleeping: floating z's
        if asleep {
            for i in 0..3 {
                let ph = (t * 0.6 + i as f32 / 3.0).fract();
                let c = pos2(mid.center().x + 16.0 * k + ph * 14.0 * k, mid.min.y + mid.height() * 0.55 - ph * mid.height() * 0.45);
                p.text(c, Align2::CENTER_CENTER, "z", crate::fonts::sans_bold_font((7.0 + 5.0 * ph) * k), Color32::from_white_alpha((200.0 * (1.0 - ph)) as u8));
            }
        }
        // party sparkles
        if party {
            for i in 0..6 {
                let a = t * 1.5 + i as f32 * TAU / 6.0;
                let c = mid.center() + vec2(a.cos() * mid.width() * 0.36, a.sin() * mid.height() * 0.34);
                spark(&p, c, 4.0 * k * (0.6 + 0.4 * (t * 5.0 + i as f32).sin().abs()), Color32::from_rgb(255, 205, 80));
            }
        }
        // what it's saying
        if !small {
            let text = self.setup.gotchi.bubble(mood);
            let size = (8.0 * k).max(7.0);
            let max_w = mid.width() - 12.0 * k;
            let lines = p.layout(text.clone(), crate::fonts::sans_bold_font(size), ink, (max_w - size * 1.2).max(30.0)).size();
            let tip = pos2(mid.center().x + if lines.x > mid.width() * 0.5 { 0.0 } else { 22.0 * k }, mid.min.y + 6.0 * k + lines.y + size * 1.3);
            let fill = if asleep { theme::blend(dark, lk.screen, 0.2) } else { theme::blend(lk.screen, Color32::WHITE, 0.6) };
            let _ = bubble(&p, tip, &text, size, if asleep { lk.screen } else { ink }, fill, max_w);
        }
    }

    // ───────────────────────────── the page ─────────────────────────────

    pub(super) fn page_gotchi(&mut self, ui: &mut egui::Ui) {
        let now_t = ui.input(|i| i.time) as f32;
        ui.ctx().request_repaint_after(Duration::from_millis(50));
        let f = self.setup.gotchi.file().clone();
        // header with the settings button on the right
        let top = ui.cursor().min;
        kit::page_header(ui, "POKÉGOTCHI", "A shiny you caught, as a pocket pet. Hunting earns berries; it keeps you company on the overlay.");
        let gear = Rect::from_min_size(pos2(ui.max_rect().max.x - 128.0, top.y + 6.0), vec2(120.0, 34.0));
        if pill_button(ui, gear, "⚙  Settings", false, "gotchi-gear").clicked() {
            self.setup.gotchi.settings = true;
        }
        if let Some(e) = self.setup.gotchi.load_error.clone() {
            kit::pill(ui, &e, kit::PillKind::Warn);
            ui.add_space(8.0);
        }
        if f.pets.is_empty() {
            self.gotchi_empty(ui, now_t);
        } else {
            self.gotchi_main(ui, now_t);
        }
        self.gotchi_picker(ui.ctx());
        self.gotchi_settings(ui.ctx());
    }

    /// No pet yet: an egg waiting, and a way to adopt.
    fn gotchi_empty(&mut self, ui: &mut egui::Ui, t: f32) {
        let caught = self.gotchi_caught();
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 470.0), Sense::hover());
        let p = ui.painter().clone();
        panel(&p, r);
        let lk = look(Frame::Pearl);
        glow(&p, pos2(r.center().x, r.min.y + 190.0), 260.0, lk.stage.gamma_multiply(0.14));
        let dev = Rect::from_center_size(pos2(r.center().x, r.min.y + 180.0), vec2(220.0, 270.0));
        let parts = paint_device(&p, dev, Frame::Pearl, t, None, None, false);
        self.gotchi_screen(&p, parts.screen, Frame::Pearl, t, false);
        let y = dev.max.y + 26.0;
        if caught.is_empty() {
            p.text(pos2(r.center().x, y), Align2::CENTER_CENTER, "Catch a shiny first", crate::fonts::sans_bold_font(18.0), TEXT());
            p.text(pos2(r.center().x, y + 26.0), Align2::CENTER_CENTER, "Your Pokégotchi is a shiny you've caught. Hunt one with Shiny Hunter, or mark one you caught before in the Shiny Dex.", crate::fonts::sans_font(13.0), MUTED());
            let b = Rect::from_center_size(pos2(r.center().x, y + 66.0), vec2(190.0, 38.0));
            if pill_button(ui, b, "✨  Go to Shiny Hunter", true, "gotchi-hunt").clicked() {
                self.go_to(Tab::Shiny);
            }
        } else {
            p.text(pos2(r.center().x, y), Align2::CENTER_CENTER, "Pick your first Pokégotchi", crate::fonts::sans_bold_font(18.0), TEXT());
            p.text(
                pos2(r.center().x, y + 26.0),
                Align2::CENTER_CENTER,
                format!("Any of the {} shin{} you've caught can come home with you. You get {} berries to start.", caught.len(), if caught.len() == 1 { "y" } else { "ies" }, gotchi::START_BERRIES),
                crate::fonts::sans_font(13.0),
                MUTED(),
            );
            let b = Rect::from_center_size(pos2(r.center().x, y + 66.0), vec2(170.0, 38.0));
            if pill_button(ui, b, "🐾  Choose a shiny", true, "gotchi-first").clicked() {
                self.setup.gotchi.picker = true;
            }
        }
    }

    fn gotchi_main(&mut self, ui: &mut egui::Ui, t: f32) {
        let w = ui.available_width();
        let narrow = w < 940.0;
        let box_w = 190.0;
        let side_w = 300.0;
        let gap = 16.0;
        let h = 548.0;
        let (all, _) = ui.allocate_exact_size(vec2(w, if narrow { h + 16.0 + 330.0 } else { h }), Sense::hover());
        let (box_r, dev_r, side_r) = if narrow {
            let dev = Rect::from_min_size(all.min, vec2(w, h));
            let below = Rect::from_min_size(pos2(all.min.x, dev.max.y + 16.0), vec2(w, 330.0));
            let half = (w - gap) / 2.0;
            (Rect::from_min_size(below.min, vec2(half, below.height())), dev, Rect::from_min_size(pos2(below.min.x + half + gap, below.min.y), vec2(half, below.height())))
        } else {
            let box_r = Rect::from_min_size(all.min, vec2(box_w, h));
            let side_r = Rect::from_min_size(pos2(all.max.x - side_w, all.min.y), vec2(side_w, h));
            let dev_r = Rect::from_min_max(pos2(box_r.max.x + gap, all.min.y), pos2(side_r.min.x - gap, all.max.y));
            (box_r, dev_r, side_r)
        };
        self.gotchi_box(ui, box_r);
        self.gotchi_device_card(ui, dev_r, t);
        self.gotchi_side(ui, side_r);
        ui.add_space(16.0);
        self.gotchi_journal(ui);
    }

    /// The Box: your pets, the partner first in line; adopt more.
    fn gotchi_box(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        panel(&p, r);
        let f = self.setup.gotchi.file().clone();
        let inner = r.shrink(14.0);
        p.text(inner.min, Align2::LEFT_TOP, "Box", crate::fonts::sans_bold_font(15.0), TEXT());
        p.text(pos2(inner.max.x, inner.min.y + 2.0), Align2::RIGHT_TOP, format!("{} / {}", f.pets.len(), gotchi::MAX_BOX), crate::fonts::sans_font(12.0), MUTED());
        let mut y = inner.min.y + 30.0;
        let row_h = 64.0;
        let mut swap = None;
        let mut release = None;
        let mut rename = None;
        for (i, pet) in f.pets.iter().enumerate() {
            let row = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), row_h));
            let resp = ui.interact(row, ui.id().with(("gotchi-box", i)), Sense::click());
            let out = i == f.partner;
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let fill = if out { theme::blend(kit::CARD(), kit::C_PINK, 0.16) } else { theme::blend(kit::sunken(), TEXT(), 0.03 * ht) };
            p.rect_filled(row, 12.0, fill);
            if out {
                p.rect_stroke(row, 12.0, Stroke::new(1.0f32, kit::C_PINK.gamma_multiply(0.6)));
            }
            if let Some(si) = self.gotchi_species(&pet.slug) {
                let sp = self.species[si].clone();
                self.draw_sprite_centered(&p, &sp, true, pos2(row.min.x + 30.0, row.center().y), vec2(50.0, 50.0));
            }
            let name = self.gotchi_name(pet);
            p.with_clip_rect(row.shrink(2.0)).text(pos2(row.min.x + 62.0, row.min.y + 20.0), Align2::LEFT_CENTER, &name, crate::fonts::sans_bold_font(13.0), TEXT());
            let sub = if out { format!("Lv {} · out", pet.level()) } else { format!("Lv {} · resting", pet.level()) };
            p.text(pos2(row.min.x + 62.0, row.min.y + 40.0), Align2::LEFT_CENTER, sub, crate::fonts::sans_font(11.5), if out { kit::C_PINK } else { MUTED() });
            let resp = resp.on_hover_text(if out { "Out with you now. Right-click to rename or send back." } else { "Click to bring it out. Right-click to rename or send back." });
            if resp.clicked() && !out {
                swap = Some(i);
            }
            resp.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    rename = Some(i);
                    ui.close_menu();
                }
                if ui.button("Send back").on_hover_text("It's still your shiny; only the Pokégotchi goes.").clicked() {
                    release = Some(i);
                    ui.close_menu();
                }
            });
            y += row_h + 8.0;
        }
        if f.pets.len() < gotchi::MAX_BOX {
            let row = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 48.0));
            let resp = ui.interact(row, ui.id().with("gotchi-adopt"), Sense::click());
            let col = if resp.hovered() { TEXT() } else { MUTED() };
            dashed(&p, row, 12.0, Stroke::new(1.2f32, if resp.hovered() { kit::C_PINK } else { LINE() }));
            p.text(row.center(), Align2::CENTER_CENTER, "+  Adopt a shiny", crate::fonts::sans_bold_font(12.5), col);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.setup.gotchi.picker = true;
            }
        }
        if let Some(i) = swap {
            self.setup.gotchi.file().swap(i);
            self.setup.gotchi.game = None;
            self.setup.gotchi.say("Hi again!");
            if let Some(p) = self.setup.gotchi.file().partner().cloned() {
                let name = self.gotchi_name(&p);
                self.gotchi_log(Kind::Care, format!("{name} came out of the Box to play."));
            }
            self.setup.gotchi.hop = Some(Instant::now());
            self.setup.gotchi.touch();
        }
        if let Some(i) = rename {
            self.setup.gotchi.file().swap(i);
            let n = self.setup.gotchi.file().pets[i].nick.clone();
            self.setup.gotchi.renaming = Some(n);
            self.setup.gotchi.settings = true;
        }
        if let Some(i) = release {
            if let Some(p) = self.setup.gotchi.file().pets.get(i).cloned() {
                let name = self.gotchi_name(&p);
                self.gotchi_log(Kind::Care, format!("{name} went back to being a regular shiny. Goodbye for now!"));
            }
            self.setup.gotchi.file().release(i);
            self.setup.gotchi.game = None;
            self.setup.gotchi.touch();
        }
    }

    /// The middle: the device on a stage, and the care buttons under it.
    fn gotchi_device_card(&mut self, ui: &mut egui::Ui, r: Rect, t: f32) {
        let p = ui.painter().clone();
        panel(&p, r);
        let frame = self.setup.gotchi.file().frame;
        let lk = look(frame);
        let stage = Rect::from_min_max(r.min, pos2(r.max.x, r.max.y - 74.0)).shrink(1.0);
        {
            let sp = p.with_clip_rect(stage);
            glow(&sp, pos2(stage.center().x, stage.center().y + 10.0), stage.width().max(stage.height()) * 0.55, lk.stage.gamma_multiply(0.16));
            // a little desk shadow
            sp.add(Shape::ellipse_filled(pos2(stage.center().x, stage.max.y - 26.0), vec2(120.0, 10.0), Color32::from_black_alpha(50)));
        }
        let dev = Rect::from_center_size(pos2(stage.center().x, stage.center().y + 2.0), vec2(330.0, stage.height() - 40.0));
        // the device's buttons
        let k = (dev.width() / 236.0).min(dev.height() / 290.0);
        let body = Rect::from_center_size(dev.center(), vec2(236.0 * k, 290.0 * k));
        let by = body.min.y + body.height() * 0.82;
        let mut hovered = None;
        let mut pressed = None;
        let mut clicked = None;
        for i in 0..3 {
            let c = pos2(body.center().x + (i as f32 - 1.0) * 44.0 * k, by + if i == 1 { 6.0 * k } else { 0.0 });
            let br = Rect::from_center_size(c, vec2(30.0 * k, 30.0 * k));
            let resp = ui.interact(br, ui.id().with(("gotchi-btn", i)), Sense::click());
            if resp.hovered() {
                hovered = Some(i);
            }
            if resp.is_pointer_button_down_on() {
                pressed = Some(i);
            }
            let tip = self.gotchi_button_tip(i);
            if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                clicked = Some(i);
            }
        }
        let parts = paint_device(&p, dev, frame, t, pressed, hovered, false);
        self.gotchi_screen(&p, parts.screen, frame, t, false);
        // click the screen: pat it
        let sresp = ui.interact(parts.screen, ui.id().with("gotchi-screen"), Sense::click());
        if sresp.on_hover_text("Click to pat").clicked() && self.setup.gotchi.game.is_none() {
            self.gotchi_act(Act::Pat);
        }
        if let Some(i) = clicked {
            self.gotchi_press(i);
        }
        // the care buttons
        let f = self.setup.gotchi.file().clone();
        let row = Rect::from_min_max(pos2(r.min.x + 16.0, r.max.y - 64.0), pos2(r.max.x - 16.0, r.max.y - 16.0));
        p.hline(r.x_range().shrink(1.0), row.min.y - 10.0, Stroke::new(1.0f32, LINE()));
        let n = Act::ALL.len() as f32;
        let bw = ((row.width() - 8.0 * (n - 1.0)) / n).min(110.0);
        let x0 = row.center().x - (bw * n + 8.0 * (n - 1.0)) / 2.0;
        let in_game = self.setup.gotchi.game.is_some();
        for (i, a) in Act::ALL.iter().enumerate() {
            let br = Rect::from_min_size(pos2(x0 + i as f32 * (bw + 8.0), row.min.y), vec2(bw, row.height()));
            let label = match a {
                Act::Feed => format!("Feed · {}", f.berries),
                Act::Treat => format!("Treat · {}", f.treats),
                Act::Lights => if f.asleep { "Lights on".into() } else { "Lights off".into() },
                _ => a.name().to_string(),
            };
            let resp = ui.interact(br, ui.id().with(("gotchi-care", i)), Sense::click());
            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered(), kit::QUICK);
            let sel = self.setup.gotchi.sel == i && !in_game;
            p.rect_filled(br, 12.0, theme::blend(kit::sunken(), need_tint(*a), 0.08 + 0.12 * ht));
            p.rect_stroke(br, 12.0, Stroke::new(1.0f32, if sel { need_tint(*a).gamma_multiply(0.8) } else { theme::blend(LINE(), need_tint(*a), 0.3 * ht) }));
            act_icon(&p, pos2(br.center().x, br.min.y + 16.0), 16.0, *a, need_tint(*a));
            p.text(pos2(br.center().x, br.max.y - 12.0), Align2::CENTER_CENTER, label, crate::fonts::sans_bold_font(11.5), TEXT());
            let tip = match a {
                Act::Feed => "A berry: +20 Full. Every 10 Pokémon you meet earns one.",
                Act::Treat => "A treat: +40 Full and +10 Happy. Rare grass drops them.",
                Act::Play => "Higher or Lower: guess if the next number is higher or lower. 5 rounds.",
                Act::Clean => "Tidy up: Clean back to full.",
                Act::Pat => "A little happy bounce.",
                Act::Lights => "Lights off: it sleeps, Energy fills up and its other needs wait.",
            };
            if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !in_game {
                self.setup.gotchi.sel = i;
                self.gotchi_act(*a);
            }
        }
    }

    /// The right: name, mood, friendship, needs, pantry.
    fn gotchi_side(&mut self, ui: &mut egui::Ui, r: Rect) {
        let p = ui.painter().clone();
        panel(&p, r);
        let f = self.setup.gotchi.file().clone();
        let Some(pet) = f.partner().cloned() else { return };
        let inner = r.shrink2(vec2(18.0, 16.0));
        let name = self.gotchi_name(&pet);
        let mood = pet.mood(f.asleep);
        let mut y = inner.min.y;
        p.text(pos2(inner.min.x, y), Align2::LEFT_TOP, &name, crate::fonts::sans_bold_font(20.0), TEXT());
        y += 30.0;
        let species = self.gotchi_species(&pet.slug).map(|i| self.species[i].name.to_string()).unwrap_or_default();
        let days = (crate::config::now_unix().saturating_sub(pet.adopted) / 86_400) as u32;
        let sub = if pet.nick.trim().is_empty() { format!("Shiny · {}", together(days)) } else { format!("Shiny {species} · {}", together(days)) };
        p.text(pos2(inner.min.x, y), Align2::LEFT_TOP, sub, crate::fonts::sans_font(12.0), MUTED());
        y += 30.0;
        // mood
        let mc = mood_col(mood);
        let g = p.layout(mood.hint().to_string(), crate::fonts::sans_font(11.5), MUTED(), inner.width() - 24.0);
        let mr = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 40.0 + g.size().y));
        p.rect_filled(mr, 12.0, theme::blend(kit::CARD(), mc, 0.12));
        p.circle_filled(pos2(mr.min.x + 18.0, mr.min.y + 18.0), 5.0, mc);
        p.text(pos2(mr.min.x + 30.0, mr.min.y + 18.0), Align2::LEFT_CENTER, mood.name(), crate::fonts::sans_bold_font(13.5), TEXT());
        p.galley(pos2(mr.min.x + 12.0, mr.min.y + 30.0), g, MUTED());
        y += mr.height() + 14.0;
        // friendship
        p.text(pos2(inner.min.x, y), Align2::LEFT_TOP, "FRIENDSHIP", crate::fonts::sans_bold_font(10.5), MUTED());
        p.text(pos2(inner.max.x, y), Align2::RIGHT_TOP, format!("Level {}", pet.level()), crate::fonts::sans_bold_font(12.0), kit::C_GOLD);
        y += 20.0;
        meter(&p, Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 8.0)), pet.level_frac(), kit::C_GOLD, kit::sunken());
        y += 26.0;
        // needs
        p.text(pos2(inner.min.x, y), Align2::LEFT_TOP, "NEEDS", crate::fonts::sans_bold_font(10.5), MUTED());
        y += 22.0;
        for n in Need::ALL {
            let v = pet.need(n);
            p.text(pos2(inner.min.x, y + 4.0), Align2::LEFT_CENTER, n.name(), crate::fonts::sans_bold_font(12.0), TEXT());
            p.text(pos2(inner.max.x, y + 4.0), Align2::RIGHT_CENTER, format!("{:.0}", v), crate::fonts::sans_font(12.0), if v < 30.0 { Color32::from_rgb(255, 150, 120) } else { MUTED() });
            meter(&p, Rect::from_min_size(pos2(inner.min.x + 64.0, y), vec2(inner.width() - 64.0 - 34.0, 8.0)), v / 100.0, need_col(n), kit::sunken());
            y += 26.0;
        }
        y += 8.0;
        // pantry and hunting
        p.text(pos2(inner.min.x, y), Align2::LEFT_TOP, "PANTRY", crate::fonts::sans_bold_font(10.5), MUTED());
        y += 22.0;
        let half = (inner.width() - 10.0) / 2.0;
        for (i, (a, n, what)) in [(Act::Feed, f.berries, "berries"), (Act::Treat, f.treats, "treats")].iter().enumerate() {
            let tr = Rect::from_min_size(pos2(inner.min.x + i as f32 * (half + 10.0), y), vec2(half, 46.0));
            p.rect_filled(tr, 10.0, kit::sunken());
            act_icon(&p, pos2(tr.min.x + 20.0, tr.center().y), 16.0, *a, need_tint(*a));
            p.text(pos2(tr.min.x + 38.0, tr.center().y - 7.0), Align2::LEFT_CENTER, n.to_string(), crate::fonts::sans_bold_font(15.0), TEXT());
            p.text(pos2(tr.min.x + 38.0, tr.center().y + 10.0), Align2::LEFT_CENTER, *what, crate::fonts::sans_font(11.0), MUTED());
        }
        y += 58.0;
        let left = gotchi::ENC_PER_BERRY - f.enc_part;
        let line = if self.setup.gotchi.reads_logs || self.cfg.count > 0 {
            format!("{left} more Pokémon to meet for the next berry")
        } else {
            "Berries come from hunting: 1 for every 10 Pokémon you meet".to_string()
        };
        let g = p.layout(line, crate::fonts::sans_font(11.5), MUTED(), inner.width());
        p.galley(pos2(inner.min.x, y), g, MUTED());
        y += 34.0;
        let today = if f.today.0 == today() { f.today } else { (0, 0, 0, 0) };
        if y + 14.0 < inner.max.y {
            p.text(
                pos2(inner.min.x, y),
                Align2::LEFT_TOP,
                format!("Today: {} berr{} · {} treat{} · {} game{}", today.1, if today.1 == 1 { "y" } else { "ies" }, today.2, if today.2 == 1 { "" } else { "s" }, today.3, if today.3 == 1 { "" } else { "s" }),
                crate::fonts::sans_font(11.5),
                MUTED(),
            );
        }
    }

    /// The journal: what happened, newest first, by day.
    fn gotchi_journal(&mut self, ui: &mut egui::Ui) {
        let f = self.setup.gotchi.file().clone();
        let name = f.partner().cloned().map(|p| self.gotchi_name(&p)).unwrap_or_default();
        let show = self.setup.gotchi.journal_show.max(40);
        let mut more = false;
        let w = ui.available_width();
        egui::Frame::none()
            .fill(kit::CARD())
            .stroke(Stroke::new(1.0f32, LINE()))
            .rounding(16.0)
            .inner_margin(egui::Margin::symmetric(20.0, 18.0))
            .show(ui, |ui| {
                ui.set_width(w - 40.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Journal").font(crate::fonts::sans_bold_font(17.0)).color(TEXT()));
                    ui.label(egui::RichText::new(format!("What {name} got up to, what it was thinking, and your time together")).size(kit::fs(12.5)).color(MUTED()));
                });
                ui.add_space(10.0);
                if f.journal.is_empty() {
                    ui.label(egui::RichText::new("Nothing yet. Thoughts and little moments show up here as the day goes on.").size(kit::fs(12.5)).color(MUTED()));
                    return;
                }
                let off = crate::stats::utc_offset_secs();
                let day_of = |at: u64| (at as i64 + off).div_euclid(86_400);
                let today = day_of(crate::config::now_unix());
                let mut last_day: Option<i64> = None;
                for e in f.journal.iter().rev().take(show) {
                    let d = day_of(e.at);
                    if last_day != Some(d) {
                        last_day = Some(d);
                        let label = match today - d {
                            0 => "TODAY".to_string(),
                            1 => "YESTERDAY".to_string(),
                            n if n < 7 => format!("{n} DAYS AGO"),
                            _ => crate::stats::fmt_day(d).to_uppercase(),
                        };
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(label).font(crate::fonts::sans_bold_font(10.5)).color(MUTED()));
                        ui.add_space(4.0);
                    }
                    let secs = ((e.at as i64 + off).rem_euclid(86_400)) as u32;
                    let (h, m) = (secs / 3600, (secs % 3600) / 60);
                    let time = format!("{}:{:02} {}", if h % 12 == 0 { 12 } else { h % 12 }, m, if h < 12 { "am" } else { "pm" });
                    let col = journal_col(e.kind);
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        let (tr, _) = ui.allocate_exact_size(vec2(62.0, 18.0), Sense::hover());
                        ui.painter().text(pos2(tr.max.x, tr.center().y), Align2::RIGHT_CENTER, time, crate::fonts::sans_font(11.5), MUTED());
                        let (dr, _) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::hover());
                        ui.painter().circle_filled(dr.center(), 4.0, col);
                        let text = egui::RichText::new(&e.text).size(kit::fs(13.0));
                        let text = match e.kind {
                            Kind::Thought => text.italics().color(theme::blend(TEXT(), col, 0.35)),
                            Kind::Big => text.strong().color(TEXT()),
                            _ => text.color(TEXT()),
                        };
                        ui.add(egui::Label::new(text).wrap());
                    });
                    ui.add_space(3.0);
                }
                if f.journal.len() > show {
                    ui.add_space(6.0);
                    if ui.add(egui::Button::new(egui::RichText::new("Show older").size(kit::fs(12.5))).fill(kit::btn_fill())).clicked() {
                        more = true;
                    }
                }
            });
        if more {
            self.setup.gotchi.journal_show = show + 60;
        }
    }

    // ───────────────────────────── popups ─────────────────────────────

    /// Choose a caught shiny to adopt.
    fn gotchi_picker(&mut self, ctx: &egui::Context) {
        if !self.setup.gotchi.picker {
            return;
        }
        let caught = self.gotchi_caught();
        let have: Vec<String> = self.setup.gotchi.file().pets.iter().map(|p| p.slug.clone()).collect();
        let full = have.len() >= gotchi::MAX_BOX;
        let screen = ctx.screen_rect();
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let mut adopt: Option<(String, String)> = None;
        egui::Area::new(egui::Id::new("gotchi-picker")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(170));
            let w = 700.0f32.min(sr.width() - 40.0);
            let h = 560.0f32.min(sr.height() - 40.0);
            let panel_r = Rect::from_center_size(sr.center(), vec2(w, h));
            let _ = ui.interact(panel_r, ui.id().with("gotchi-picker-panel"), Sense::click());
            let p = ui.painter().clone();
            p.add(kit::shadow_tier(4).as_shape(panel_r, 16.0));
            p.rect_filled(panel_r, 16.0, kit::CARD());
            p.rect_stroke(panel_r, 16.0, Stroke::new(1.0f32, LINE()));
            let inner = panel_r.shrink2(vec2(24.0, 22.0));
            p.text(inner.min, Align2::LEFT_TOP, "Adopt a shiny", crate::fonts::sans_bold_font(19.0), TEXT());
            p.text(
                inner.min + vec2(0.0, 28.0),
                Align2::LEFT_TOP,
                if full { "The Box is full (6). Send one back to make room." } else { "Any shiny you've caught can be a Pokégotchi. It's still in your Shiny Dex either way." },
                crate::fonts::sans_font(12.5),
                MUTED(),
            );
            if close_x(ui, &p, pos2(inner.max.x - 10.0, inner.min.y + 10.0), "gotchi-picker-x") {
                close = true;
            }
            // search
            let sr2 = Rect::from_min_size(inner.min + vec2(0.0, 60.0), vec2(inner.width(), 34.0));
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(sr2).layout(egui::Layout::left_to_right(egui::Align::Center)));
            kit::search_box(&mut cu, &mut self.setup.gotchi.pick_query, "Search your shinies", sr2.width());
            let q = self.setup.gotchi.pick_query.trim().to_lowercase();
            let list: Vec<&(usize, String, String)> = caught.iter().filter(|c| q.is_empty() || c.2.to_lowercase().contains(&q)).collect();
            let area = Rect::from_min_max(pos2(inner.min.x, sr2.max.y + 14.0), inner.max);
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(area).layout(egui::Layout::top_down(egui::Align::Min)));
            egui::ScrollArea::vertical().id_salt("gotchi-picker-scroll").auto_shrink([false, false]).show(&mut cu, |ui| {
                let cols = ((area.width() + 10.0) / 120.0).floor().max(2.0) as usize;
                let cw = (area.width() - 10.0 * (cols as f32 - 1.0) - 12.0) / cols as f32;
                if list.is_empty() {
                    ui.label(egui::RichText::new("No shinies match.").color(MUTED()));
                }
                for chunk in list.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for (i, slug, name) in chunk {
                            let (r, resp) = ui.allocate_exact_size(vec2(cw, 120.0), Sense::click());
                            let p = ui.painter_at(r);
                            let in_box = have.contains(slug);
                            let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && !in_box && !full, kit::QUICK);
                            p.rect_filled(r, 12.0, theme::blend(kit::sunken(), kit::C_PINK, 0.04 + 0.12 * ht));
                            if ht > 0.0 {
                                p.rect_stroke(r, 12.0, Stroke::new(1.0f32, kit::C_PINK.gamma_multiply(0.6 * ht)));
                            }
                            let sp = self.species[*i].clone();
                            self.draw_sprite_centered(&p, &sp, true, pos2(r.center().x, r.min.y + 48.0), vec2(cw - 20.0, 70.0));
                            theme::sparkle(&p, r.right_top() + vec2(-12.0, 12.0), 4.0, kit::C_GOLD);
                            p.text(pos2(r.center().x, r.max.y - 24.0), Align2::CENTER_CENTER, name.as_str(), crate::fonts::sans_bold_font(12.5), TEXT());
                            if in_box {
                                p.text(pos2(r.center().x, r.max.y - 9.0), Align2::CENTER_CENTER, "In your Box", crate::fonts::sans_font(10.5), kit::C_PINK);
                                p.rect_filled(r, 12.0, Color32::from_black_alpha(60));
                            }
                            if !in_box && !full && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                adopt = Some((slug.clone(), name.clone()));
                            }
                        }
                    });
                    ui.add_space(10.0);
                }
            });
            if scrim.clicked() && !panel_r.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
                close = true;
            }
        });
        if let Some((slug, name)) = adopt {
            let now = crate::config::now_unix();
            match self.setup.gotchi.file().adopt(&slug, now) {
                Ok(()) => {
                    self.setup.gotchi.game = None;
                    self.setup.gotchi.say(format!("Hi! I'm {name}!"));
                    self.gotchi_log(Kind::Big, format!("{name} came home with you!"));
                    self.setup.gotchi.hop = Some(Instant::now());
                    self.setup.gotchi.touch();
                    self.setup.gotchi.save_tick(true);
                    self.play(crate::sound::Sfx::Caught);
                }
                Err(e) => self.setup.gotchi.say(e),
            }
            close = true;
        }
        if close {
            self.setup.gotchi.picker = false;
            self.setup.gotchi.pick_query.clear();
        }
    }

    /// Frames, the overlay and the partner's name.
    fn gotchi_settings(&mut self, ctx: &egui::Context) {
        if !self.setup.gotchi.settings {
            return;
        }
        let f = self.setup.gotchi.file().clone();
        let screen = ctx.screen_rect();
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let t = ctx.input(|i| i.time) as f32;
        let mut pick: Option<Frame> = None;
        let mut tint_pick: Option<Tint> = None;
        let mut ov = f.overlay.clone();
        egui::Area::new(egui::Id::new("gotchi-settings")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
            kit::settings_style(ui);
            let (sr, scrim) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(sr, 0.0, Color32::from_black_alpha(170));
            let w = 640.0f32.min(sr.width() - 40.0);
            let h = 750.0f32.min(sr.height() - 40.0);
            let panel_r = Rect::from_center_size(sr.center(), vec2(w, h));
            let _ = ui.interact(panel_r, ui.id().with("gotchi-settings-panel"), Sense::click());
            let p = ui.painter().clone();
            p.add(kit::shadow_tier(4).as_shape(panel_r, 16.0));
            p.rect_filled(panel_r, 16.0, kit::CARD());
            p.rect_stroke(panel_r, 16.0, Stroke::new(1.0f32, LINE()));
            let inner = panel_r.shrink2(vec2(24.0, 22.0));
            p.text(inner.min, Align2::LEFT_TOP, "Pokégotchi settings", crate::fonts::sans_bold_font(19.0), TEXT());
            p.text(inner.min + vec2(0.0, 28.0), Align2::LEFT_TOP, "Your device's frame, and how your pet shows on the overlay.", crate::fonts::sans_font(12.5), MUTED());
            if close_x(ui, &p, pos2(inner.max.x - 10.0, inner.min.y + 10.0), "gotchi-settings-x") {
                close = true;
            }
            let label = |y: f32, s: &str| {
                p.text(pos2(inner.min.x, y), Align2::LEFT_CENTER, s, crate::fonts::sans_bold_font(11.0), MUTED());
            };
            // ── name
            let mut y = inner.min.y + 66.0;
            if let Some(pet) = f.partner() {
                label(y, "NAME");
                y += 14.0;
                let row = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 34.0));
                let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
                let species = self.gotchi_species(&pet.slug).map(|i| self.species[i].name.to_string()).unwrap_or_default();
                let text = self.setup.gotchi.renaming.get_or_insert_with(|| pet.nick.clone());
                let resp = cu.add(egui::TextEdit::singleline(text).hint_text(species).char_limit(16).desired_width(260.0));
                if resp.changed() {
                    let nick = text.trim().to_string();
                    if let Some(pp) = self.setup.gotchi.file().partner_mut() {
                        pp.nick = nick;
                    }
                    self.setup.gotchi.touch();
                }
                cu.label(egui::RichText::new("Leave empty for its species' name").size(kit::fs(12.0)).color(MUTED()));
                y += 50.0;
            }
            // ── frames
            label(y, "FRAME");
            y += 14.0;
            let tw = (inner.width() - 20.0) / 3.0;
            for (i, fr) in Frame::ALL.iter().enumerate() {
                let r = Rect::from_min_size(pos2(inner.min.x + i as f32 * (tw + 10.0), y), vec2(tw, 196.0));
                let resp = ui.interact(r, ui.id().with(("gotchi-frame", i)), Sense::click());
                let on = f.frame == *fr;
                let open = f.is_unlocked(*fr);
                let ht = kit::anim(ui.ctx(), resp.id.with("h"), resp.hovered() && open, kit::QUICK);
                p.rect_filled(r, 14.0, theme::blend(kit::sunken(), look(*fr).stage, 0.06 + 0.08 * ht));
                p.rect_stroke(r, 14.0, Stroke::new(if on { 2.0f32 } else { 1.0 }, if on { kit::primary() } else { LINE() }));
                let dr = Rect::from_center_size(pos2(r.center().x, r.min.y + 74.0), vec2(100.0, 124.0));
                paint_device(&p, dr, *fr, t, None, None, false);
                let title = if *fr == Frame::Pearl { f.tint.name() } else { fr.name() };
                p.text(pos2(r.center().x, r.max.y - 44.0), Align2::CENTER_CENTER, title, crate::fonts::sans_bold_font(13.0), TEXT());
                if *fr == Frame::Pearl {
                    // its five colours
                    let n = Tint::ALL.len() as f32;
                    let (d, gap) = (16.0, 9.0);
                    let x0 = r.center().x - (n * d + (n - 1.0) * gap) / 2.0 + d / 2.0;
                    for (j, tn) in Tint::ALL.iter().enumerate() {
                        let c = pos2(x0 + j as f32 * (d + gap), r.max.y - 22.0);
                        let dresp = ui.interact(Rect::from_center_size(c, vec2(d + 4.0, d + 4.0)), ui.id().with(("gotchi-tint", j)), Sense::click());
                        let pl = pearl(*tn);
                        let pts: Vec<Pos2> = (0..20).map(|q| c + vec2((q as f32 / 20.0 * TAU).cos(), (q as f32 / 20.0 * TAU).sin()) * d / 2.0).collect();
                        shade(&p, &pts, c + vec2(-3.0, -3.0), pl.light, pl.dark, Stroke::new(1.0f32, pl.accent));
                        let owned = self.tint_owned(*tn);
                        if !owned {
                            p.circle_filled(c, d / 2.0, Color32::from_black_alpha(140));
                            lock(&p, c, 6.0, Color32::from_gray(235));
                        }
                        if f.tint == *tn {
                            p.circle_stroke(c, d / 2.0 + 3.0, Stroke::new(2.0f32, kit::primary()));
                        } else if dresp.hovered() && owned {
                            p.circle_stroke(c, d / 2.0 + 3.0, Stroke::new(1.0f32, MUTED()));
                        }
                        let tip = match crate::pass::tint_reward(*tn) {
                            Some(rw) if !owned => format!("{} ({})\nBattle pass tier {}", tn.name(), tn.colour(), rw.tier),
                            _ => format!("{} ({})", tn.name(), tn.colour()),
                        };
                        let dresp = dresp.on_hover_text(tip);
                        if owned && dresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            tint_pick = Some(*tn);
                        }
                    }
                } else {
                    let sub = if open { if on { "In use" } else { fr.blurb() } } else { fr.unlock() };
                    let g = p.layout(sub.to_string(), crate::fonts::sans_font(10.5), if open { MUTED() } else { kit::C_GOLD }, tw - 16.0);
                    p.galley(pos2(r.center().x - g.size().x / 2.0, r.max.y - 32.0), g, MUTED());
                }
                if !open {
                    p.rect_filled(dr.expand(4.0), 12.0, Color32::from_black_alpha(110));
                    lock(&p, dr.center(), 14.0, Color32::from_gray(230));
                }
                if open && !on && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    pick = Some(*fr);
                }
            }
            y += 214.0;
            // ── overlay
            label(y, "ON THE OVERLAY");
            y += 14.0;
            let row = Rect::from_min_size(pos2(inner.min.x, y), vec2(inner.width(), 36.0));
            let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)));
            cu.spacing_mut().item_spacing.x = 12.0;
            kit::segmented(&mut cu, &mut ov.style, &[(OvStyle::Device, OvStyle::Device.name()), (OvStyle::Perch, OvStyle::Perch.name()), (OvStyle::Strip, OvStyle::Strip.name())]);
            use crate::gotchi::OvScale;
            kit::segmented(&mut cu, &mut ov.scale, &[(OvScale::Small, OvScale::Small.name()), (OvScale::Medium, OvScale::Medium.name()), (OvScale::Large, OvScale::Large.name())]);
            y += 46.0;
            // a preview over a game
            let pv = Rect::from_min_max(pos2(inner.min.x, y), pos2(inner.max.x, inner.max.y));
            mock_game(&p, pv);
            let size = self.gotchi_ov_size(&ov);
            // too big for the preview: shown smaller (said below)
            let fit = ((pv.height() - 40.0) / size.y).min((pv.width() - 16.0) / size.x).min(1.0);
            let shrunk = fit < 1.0 && ov.style != OvStyle::Perch;
            let spot = Rect::from_center_size(pv.center() - vec2(0.0, 8.0), if shrunk { size * fit } else { size });
            let pc = p.with_clip_rect(pv);
            self.gotchi_overlay_paint(&pc, spot, &ov, f.frame, t, false);
            let note = if shrunk { format!("Shown at {:.0}% here. Open it with Start overlay, and drag it anywhere.", fit * 100.0) } else { "Open it with Start overlay, and drag it anywhere.".to_string() };
            pc.text(pos2(pv.min.x + 12.0, pv.max.y - 12.0), Align2::LEFT_BOTTOM, note, crate::fonts::sans_font(11.5), Color32::from_white_alpha(200));
            if scrim.clicked() && !panel_r.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
                close = true;
            }
        });
        if let Some(fr) = pick {
            self.setup.gotchi.file().frame = fr;
            self.setup.gotchi.touch();
        }
        if let Some(tn) = tint_pick {
            // picking a colour puts the Pearl frame on too
            let g = self.setup.gotchi.file();
            g.tint = tn;
            g.frame = Frame::Pearl;
            set_tint(tn);
            self.setup.gotchi.touch();
        }
        if ov != f.overlay {
            self.setup.gotchi.file().overlay = ov;
            self.setup.gotchi.touch();
        }
        if close {
            self.setup.gotchi.settings = false;
            self.setup.gotchi.renaming = None;
        }
    }
}

/// A stand-in for the game behind an overlay preview.
pub(super) fn mock_game(p: &egui::Painter, pv: Rect) {
    p.rect_filled(pv, 12.0, Color32::from_rgb(58, 92, 58));
    let pc = p.with_clip_rect(pv.shrink(1.0));
    let mut x = pv.min.x - pv.height();
    while x < pv.max.x {
        pc.line_segment([pos2(x, pv.max.y), pos2(x + pv.height(), pv.min.y)], Stroke::new(10.0f32, Color32::from_rgb(64, 100, 62)));
        x += 26.0;
    }
}

/// A hop's height (0..1) `e` seconds after it started.
pub(super) fn hop_y(e: f32) -> f32 {
    if e > 0.5 {
        0.0
    } else {
        (e / 0.5 * PI).sin()
    }
}

fn together(days: u32) -> String {
    match days {
        0 => "together since today".into(),
        1 => "1 day together".into(),
        d => format!("{d} days together"),
    }
}

pub(super) fn mood_col(m: Mood) -> Color32 {
    match m {
        Mood::Happy => Color32::from_rgb(127, 224, 164),
        Mood::Fine => Color32::from_rgb(150, 190, 255),
        Mood::Asleep => Color32::from_rgb(150, 150, 230),
        Mood::Grumpy => Color32::from_rgb(255, 120, 110),
        _ => Color32::from_rgb(255, 190, 110),
    }
}

/// A journal line's dot.
fn journal_col(k: Kind) -> Color32 {
    match k {
        Kind::Care => Color32::from_rgb(255, 156, 199),
        Kind::Hunt => Color32::from_rgb(127, 224, 164),
        Kind::Thought => Color32::from_rgb(170, 160, 255),
        Kind::Away => Color32::from_rgb(120, 200, 255),
        Kind::Fun => Color32::from_rgb(255, 211, 110),
        Kind::Big => Color32::from_rgb(255, 120, 110),
    }
}

/// Each care button's colour.
pub(super) fn need_tint(a: Act) -> Color32 {
    match a {
        Act::Feed => Color32::from_rgb(240, 96, 120),
        Act::Treat => Color32::from_rgb(255, 170, 90),
        Act::Play => Color32::from_rgb(255, 211, 110),
        Act::Clean => Color32::from_rgb(120, 200, 255),
        Act::Pat => Color32::from_rgb(255, 156, 199),
        Act::Lights => Color32::from_rgb(160, 160, 255),
    }
}

fn panel(p: &egui::Painter, r: Rect) {
    p.add(kit::card_shadow().as_shape(r, 16.0));
    p.rect_filled(r, 16.0, kit::CARD());
    p.rect_stroke(r, 16.0, Stroke::new(1.0f32, LINE()));
}

fn dashed(p: &egui::Painter, r: Rect, round: f32, st: Stroke) {
    let r = r.shrink(0.5);
    let pts: Vec<Pos2> = {
        let mut v = Vec::new();
        let corners = [(r.right_top(), -PI / 2.0), (r.right_bottom(), 0.0), (r.left_bottom(), PI / 2.0), (r.left_top(), PI)];
        for (c, a0) in corners {
            let cc = c + vec2(if c.x > r.center().x { -round } else { round }, if c.y > r.center().y { -round } else { round });
            for j in 0..=6 {
                let a = a0 + j as f32 / 6.0 * PI / 2.0;
                v.push(cc + vec2(a.cos(), a.sin()) * round);
            }
        }
        v.push(v[0]);
        v
    };
    p.extend(Shape::dashed_line(&pts, st, 6.0, 4.0));
}

fn lock(p: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, s * 0.3), vec2(s * 1.3, s));
    p.rect_filled(body, s * 0.2, col);
    let pts: Vec<Pos2> = (0..=12).map(|i| c + vec2(0.0, -s * 0.2) + vec2((PI + i as f32 / 12.0 * PI).cos(), (PI + i as f32 / 12.0 * PI).sin()) * s * 0.42).collect();
    p.add(Shape::line(pts, Stroke::new(s * 0.18, col)));
}

/// A close (×) button; true when clicked.
fn close_x(ui: &mut egui::Ui, p: &egui::Painter, c: Pos2, id: &str) -> bool {
    let xr = Rect::from_center_size(c, vec2(28.0, 28.0));
    let xresp = ui.interact(xr, ui.id().with(id), Sense::click());
    if xresp.hovered() {
        p.rect_filled(xr, 8.0, kit::CARD_HOVER());
    }
    for (a, b) in [(vec2(-5.0, -5.0), vec2(5.0, 5.0)), (vec2(-5.0, 5.0), vec2(5.0, -5.0))] {
        p.line_segment([c + a, c + b], Stroke::new(1.6f32, if xresp.hovered() { TEXT() } else { MUTED() }));
    }
    xresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
