//! Poké Balls left, on the overlay (Full, Compact and Bar, every theme):
//! the Bag's Poké Balls pocket from the save, less each throw the game's
//! log shows since that save (the save only catches up every few minutes).
//! Read only.

use super::ov_common::{look, Look};
use super::App;
use crate::config::OverlayTheme;
use crate::sim::Ball;
use crate::theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Rect, Stroke};

/// One kind of ball in the Bag.
#[derive(Clone, Debug)]
pub(super) struct BallNow {
    pub item: u16,
    pub ball: Option<Ball>,
    pub name: String,
    pub count: u32,
    /// Thrown since the save (the newest throw, unix ms), for the order.
    pub last: u64,
}

/// The Full / Compact strip heights.
pub(super) const STRIP_FULL_H: f32 = 40.0;
pub(super) const STRIP_COMPACT_H: f32 = 36.0;
/// Fewer than this of the ball you're using shows red.
const LOW: u32 = 10;

/// The game's ball item numbers.
pub(super) fn ball_of(item: u16) -> Option<Ball> {
    Some(match item {
        1 => Ball::Master,
        2 => Ball::Ultra,
        3 => Ball::Great,
        4 => Ball::Poke,
        5 => Ball::Safari,
        6 => Ball::Net,
        7 => Ball::Dive,
        8 => Ball::Nest,
        9 => Ball::Repeat,
        10 => Ball::Timer,
        11 => Ball::Luxury,
        12 => Ball::Premier,
        13 => Ball::Dusk,
        14 => Ball::Heal,
        15 => Ball::Quick,
        16 => Ball::Cherish,
        _ => return None,
    })
}

impl App {
    /// The log saw a ball thrown (`at`: seconds into the day).
    pub(super) fn ball_thrown(&mut self, item: u16, at: u32) {
        let now = crate::config::now_unix() as i64;
        let into_day = (now + crate::stats::utc_offset_secs()).rem_euclid(86_400);
        let mut t = now - into_day + at as i64;
        if t > now + 60 {
            t -= 86_400;
        }
        self.setup.boxes.throws.push(((t.max(0) as u64) * 1000, item));
    }

    /// The balls in your Bag now (the most recently thrown first, then the
    /// most). Empty when the save hasn't been read.
    pub(super) fn balls_now(&self) -> Vec<BallNow> {
        let st = &self.setup.boxes;
        let Some(s) = st.save.as_ref() else { return Vec::new() };
        let text = st.text.as_ref();
        let mut v: Vec<BallNow> = s
            .bag
            .iter()
            .filter(|(pocket, _, n)| *pocket == "Poké Balls" && *n > 0)
            .map(|(_, item, n)| BallNow {
                item: *item,
                ball: ball_of(*item),
                name: text.map_or_else(|| format!("Ball #{item}"), |t| t.item_name(*item)),
                count: *n as u32,
                last: 0,
            })
            .collect();
        for &(ms, item) in st.throws.iter().filter(|(ms, _)| *ms > st.save_ms) {
            if let Some(b) = v.iter_mut().find(|b| b.item == item) {
                b.count = b.count.saturating_sub(1);
                b.last = b.last.max(ms);
            }
        }
        v.sort_by(|a, b| b.last.cmp(&a.last).then(b.count.cmp(&a.count)));
        v
    }

    /// The balls for the overlay (its Balls section on, and some to show).
    pub(super) fn ov_balls(&self) -> Vec<BallNow> {
        if !self.cfg.overlay.balls {
            return Vec::new();
        }
        self.balls_now()
    }

    /// The strips under the Full / Compact overlay, in its theme, from `y`
    /// down: your Eggs, then your Poké Balls. Returns the new bottom and
    /// the window shapes they need.
    pub(super) fn under_strips(&mut self, ui: &mut egui::Ui, p: &Painter, x0: f32, y: f32, w: f32, theme: OverlayTheme, compact: bool) -> (f32, Vec<[f32; 5]>) {
        let radius = if theme == OverlayTheme::Classic { 12.0 } else { look(theme).radius };
        let mut bottom = y;
        let mut shapes = Vec::new();
        if let Some(eggs) = self.ov_eggs() {
            let h = if compact { super::eggs::STRIP_COMPACT_H } else { super::eggs::STRIP_FULL_H };
            let r = Rect::from_min_size(pos2(x0, bottom + super::eggs::STRIP_GAP), vec2(w, h));
            self.egg_strip(ui, p, r, theme, compact, &eggs);
            shapes.push([r.min.x, r.min.y, r.width(), r.height(), radius]);
            bottom = r.max.y;
        }
        let balls = self.ov_balls();
        if !balls.is_empty() {
            let h = if compact { STRIP_COMPACT_H } else { STRIP_FULL_H };
            let r = Rect::from_min_size(pos2(x0, bottom + super::eggs::STRIP_GAP), vec2(w, h));
            self.ball_strip(ui, p, r, theme, compact, &balls);
            shapes.push([r.min.x, r.min.y, r.width(), r.height(), radius]);
            bottom = r.max.y;
        }
        (bottom, shapes)
    }

    /// The Poké Balls strip: each kind's ball and how many are left.
    fn ball_strip(&self, ui: &mut egui::Ui, p: &Painter, r: Rect, theme: OverlayTheme, compact: bool, balls: &[BallNow]) {
        let classic = theme == OverlayTheme::Classic;
        let lk: Look = look(theme);
        let (text, low) = if classic { (Color32::WHITE, theme::HP_RED) } else { (lk.text, (lk.tone)(theme::HP_RED)) };
        if classic {
            p.rect_filled(r, 12.0, theme::BG().gamma_multiply(self.cfg.opacity));
            p.rect_stroke(r, 12.0, Stroke::new(1.5f32, theme::OUTLINE));
        } else {
            (lk.panel)(p, r, self.cfg.opacity, false);
        }
        let font = if classic { crate::fonts::sans_bold_font(if compact { 11.0 } else { 12.0 }) } else { (lk.body)(if compact { 11.0 } else { 12.5 }) };
        let max = if compact { 4 } else { 5 };
        let shown = &balls[..balls.len().min(max)];
        let pad = 10.0;
        let cw = (r.width() - 2.0 * pad) / shown.len().max(1) as f32;
        let br = if compact { 7.0 } else { 8.5 };
        for (i, b) in shown.iter().enumerate() {
            let cx = r.min.x + pad + cw * i as f32;
            let c = pos2(cx + br + 4.0, r.center().y);
            draw_any_ball(p, c, br, b.ball);
            let col = if i == 0 && b.count < LOW { low } else { text };
            let label = b.count.to_string();
            let max_w = cw - 2.0 * br - 10.0;
            let label = super::charts::clip(ui, &label, &font, max_w);
            p.text(pos2(c.x + br + 5.0, c.y), Align2::LEFT_CENTER, label, font.clone(), col);
        }
        super::ov_common::tip(ui, r, "ball-strip", &balls_tip(balls));
    }

    /// The Balls cell in the Bar: a label over each kind's ball and count.
    /// Returns how wide it was.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bar_balls(&self, ui: &mut egui::Ui, p: &Painter, x: f32, top: f32, bar_h: f32, balls: &[BallNow], label: FontId, body: FontId, text: Color32, muted: Color32, low: Color32) -> f32 {
        let a = p.text(pos2(x, top + 16.0), Align2::LEFT_CENTER, "BALLS", label, muted);
        let mut cx = x;
        for (i, b) in balls.iter().take(3).enumerate() {
            draw_any_ball(p, pos2(cx + 6.0, top + 32.0), 6.0, b.ball);
            let col = if i == 0 && b.count < LOW { low } else { text };
            let g = p.text(pos2(cx + 15.0, top + 32.0), Align2::LEFT_CENTER, b.count.to_string(), body.clone(), col);
            cx = g.max.x + 8.0;
        }
        let w = a.width().max(cx - x - 8.0);
        super::ov_common::tip(ui, Rect::from_min_size(pos2(x, top + 6.0), vec2(w, bar_h - 12.0)), "bar-balls", &balls_tip(balls));
        w
    }
}

fn balls_tip(balls: &[BallNow]) -> String {
    let mut t: String = balls.iter().map(|b| format!("{} × {}\n", b.name, b.count)).collect();
    t.push_str("From your save, less the throws the game's log has shown since.");
    t
}

/// A ball's picture, or a plain red and white one for a ball the app doesn't know.
fn draw_any_ball(p: &Painter, c: egui::Pos2, r: f32, b: Option<Ball>) {
    match b {
        Some(b) => super::sim_catch::draw_ball(p, c, r, 0.0, b),
        None => super::sim_catch::draw_ball_color(p, c, r * 0.9, Color32::from_rgb(220, 60, 60), false),
    }
}
