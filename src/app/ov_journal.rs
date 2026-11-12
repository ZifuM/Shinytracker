//! Overlay theme "Field journal": a page from a spiral notebook. Ruled
//! lines and a red margin, the shiny in a taped-on polaroid, the count in
//! handwriting with tally marks, a hatched pencil odds bar, sticky-note
//! chips and pen-drawn buttons with a red "Caught!" stamp.

use super::ov_common::*;
use super::App;
use crate::fonts::hand_font;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const PAPER: Color32 = Color32::from_rgb(251, 246, 233);
pub(super) const PEN: Color32 = Color32::from_rgb(32, 44, 96);
pub(super) const PENCIL: Color32 = Color32::from_rgb(98, 98, 106);
pub(super) const RED: Color32 = Color32::from_rgb(206, 52, 52);
const RULE: Color32 = Color32::from_rgb(160, 192, 228);
const MARGIN: Color32 = Color32::from_rgb(232, 128, 128);
const STICKY: Color32 = Color32::from_rgb(255, 232, 128);
const HIGHLIGHT: Color32 = Color32::from_rgb(255, 232, 80);
const RAD: f32 = 6.0;
/// The spiral binding above the page.
const BIND: f32 = 20.0;

/// Status colours as coloured pencil on paper.
fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        PENCIL
    } else {
        mix(c, PEN, 0.35)
    }
}

/// The tape's colour (the theme accent, see-through).
fn tape() -> Color32 {
    fade(mix(theme::pal().accent, Color32::WHITE, 0.35), 0.62)
}

/// A wobble of -1..1 for point `i` of line `seed`.
fn wob(seed: u32, i: u32) -> f32 {
    let mut s = seed.wrapping_mul(2654435761) ^ i.wrapping_mul(40503).wrapping_add(0x9e37);
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    (s % 2001) as f32 / 1000.0 - 1.0
}

fn seed_of(a: Pos2) -> u32 {
    (a.x * 7.0 + a.y * 13.0) as i32 as u32
}

/// A hand-drawn line (a little wobbly).
pub(super) fn scribble(s: &dyn Sink, a: Pos2, b: Pos2, w: f32, col: Color32) {
    let seed = seed_of(a) ^ seed_of(b).rotate_left(7);
    let d = b - a;
    let len = d.length().max(1.0);
    let n = (len / 14.0).ceil().max(2.0) as u32;
    let perp = vec2(-d.y, d.x) / len;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let k = if i == 0 || i == n { 0.3 } else { 1.0 };
            a + d * t + perp * wob(seed, i) * 0.9 * k
        })
        .collect();
    s.put(Shape::line(pts, Stroke::new(w, col)));
}

/// A hand-drawn box: four wobbly lines that overshoot a little.
pub(super) fn scribble_rect(s: &dyn Sink, r: Rect, w: f32, col: Color32) {
    let o = 2.0;
    scribble(s, r.left_top() + vec2(-o, 0.0), r.right_top() + vec2(o * 0.5, 0.0), w, col);
    scribble(s, r.right_top() + vec2(0.0, -o * 0.5), r.right_bottom() + vec2(0.0, o), w, col);
    scribble(s, r.right_bottom() + vec2(o, 0.0), r.left_bottom() + vec2(-o * 0.5, 0.0), w, col);
    scribble(s, r.left_bottom() + vec2(0.0, o * 0.5), r.left_top() + vec2(0.0, -o), w, col);
}

/// A hand-drawn loop round `r` (for circling things).
fn scribble_oval(s: &dyn Sink, r: Rect, w: f32, col: Color32) {
    let seed = seed_of(r.min);
    let c = r.center();
    let n = 28;
    let pts: Vec<Pos2> = (0..=n + 3)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU - 0.6;
            let k = 1.0 + wob(seed, i) * 0.04 + i as f32 * 0.002;
            c + vec2(a.cos() * r.width() / 2.0 * k, a.sin() * r.height() / 2.0 * k)
        })
        .collect();
    s.put(Shape::line(pts, Stroke::new(w, col)));
}

/// Corners of `r` turned by `deg` round its centre.
fn turned(r: Rect, deg: f32) -> Vec<Pos2> {
    let (sn, cs) = deg.to_radians().sin_cos();
    let c = r.center();
    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]
        .iter()
        .map(|q| {
            let d = *q - c;
            c + vec2(d.x * cs - d.y * sn, d.x * sn + d.y * cs)
        })
        .collect()
}

/// Diagonal pencil hatching inside `r` (the part up to `frac`).
fn hatch(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    let x_end = r.min.x + r.width() * frac.clamp(0.0, 1.0);
    let step = 4.0;
    let mut x = r.min.x - r.height();
    while x < x_end {
        let (mut a, mut b) = (pos2(x, r.max.y), pos2(x + r.height(), r.min.y));
        if a.x < r.min.x {
            a = pos2(r.min.x, r.max.y - (r.min.x - a.x));
        }
        if b.x > x_end {
            b = pos2(x_end, r.min.y + (b.x - x_end));
        }
        if b.x > a.x {
            line(s, a, b, Stroke::new(1.3f32, col));
        }
        x += step;
    }
}

/// Tally marks: bundles of five.
fn tally(s: &dyn Sink, pos: Pos2, n: u64, h: f32, col: Color32) -> f32 {
    let mut x = pos.x;
    let gap = h * 0.22;
    for b in 0..n.div_ceil(5) {
        let k = (n - b * 5).min(5);
        let x0 = x;
        for i in 0..k.min(4) {
            let xx = x0 + i as f32 * gap;
            scribble(s, pos2(xx, pos.y), pos2(xx + 0.8, pos.y + h), 1.6, col);
        }
        if k == 5 {
            scribble(s, pos2(x0 - gap * 0.5, pos.y + h * 0.75), pos2(x0 + gap * 3.5, pos.y + h * 0.2), 1.6, col);
        }
        x = x0 + gap * 4.0 + h * 0.4;
    }
    x - pos.x
}

fn hand(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    p.text(pos, anchor, s, hand_font(size), col)
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    p.text(pos, anchor, s, hand_font(font.size * 1.25), col)
}

/// The page: paper, ruled lines, a margin, the spiral and a dog-ear.
fn page(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.6 + 0.4 * opacity;
    let paper = Rect::from_min_max(pos2(r.min.x, r.min.y + BIND * 0.45), r.max);
    rrect(s, paper, RAD, fade(PAPER, a));
    let mut y = paper.min.y + 34.0;
    while y < paper.max.y - 6.0 {
        line(s, pos2(paper.min.x + 2.0, y), pos2(paper.max.x - 2.0, y), Stroke::new(1.0f32, fade(RULE, 0.55 * a)));
        y += 22.0;
    }
    line(s, pos2(paper.min.x + 26.0, paper.min.y + 2.0), pos2(paper.min.x + 26.0, paper.max.y - 2.0), Stroke::new(1.2f32, fade(MARGIN, 0.6 * a)));
    // the spiral: holes and wire loops
    let mut x = r.min.x + 16.0;
    while x < r.max.x - 10.0 {
        circle(s, pos2(x, paper.min.y + 9.0), 3.2, fade(Color32::from_rgb(60, 58, 66), a));
        line(s, pos2(x - 4.0, r.min.y + 1.5), pos2(x + 0.5, paper.min.y + 9.0), Stroke::new(2.6f32, Color32::from_rgb(120, 122, 132)));
        line(s, pos2(x - 4.0, r.min.y + 1.5), pos2(x + 0.5, paper.min.y + 9.0), Stroke::new(1.0f32, Color32::from_rgb(210, 212, 222)));
        x += 16.0;
    }
    // dog-ear
    let k = 14.0;
    let br = paper.right_bottom();
    poly(s, vec![br + vec2(-k, 0.0), br + vec2(0.0, -k), br + vec2(-k, -k)], fade(mix(PAPER, PENCIL, 0.25), a));
    if alert {
        scribble_rect(s, paper.shrink(3.0), 2.5, RED);
    }
}

fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    hatch(s, r.shrink(1.0), frac, col);
    scribble_rect(s, r, 1.3, PEN);
}

/// Pen-drawn buttons; Caught! is a red stamp.
fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let r = if down { r.translate(vec2(0.5, 1.0)) } else { r };
    let round = label.chars().count() <= 2 && kind == Kind::Plain;
    let r = if round { Rect::from_center_size(r.center(), vec2(r.height(), r.height())) } else { r };
    match kind {
        Kind::Main => {
            if hovered {
                rrect(p, r, 5.0, fade(RED, 0.12));
            }
            rstroke(p, r, 5.0, Stroke::new(2.4f32, RED));
            rstroke(p, r.shrink(3.0), 3.0, Stroke::new(1.0f32, fade(RED, 0.7)));
        }
        Kind::Gold => {
            rrect(p, r.shrink(2.0), 2.0, fade(HIGHLIGHT, if hovered { 0.8 } else { 0.55 }));
            scribble_rect(p, r, 1.4, PEN);
        }
        Kind::Good | Kind::Plain => {
            if hovered {
                rrect(p, r.shrink(2.0), 2.0, fade(HIGHLIGHT, 0.45));
            }
            scribble_rect(p, r, 1.4, if kind == Kind::Good { mix(theme::GREEN, PEN, 0.45) } else { PEN });
        }
    }
    let col = match kind {
        Kind::Main => RED,
        Kind::Good => mix(theme::GREEN, PEN, 0.45),
        _ => PEN,
    };
    let c = r.center();
    if label == "⏸" {
        for dx in [-3.0f32, 3.0] {
            scribble(p, c + vec2(dx, -6.0), c + vec2(dx, 6.0), 2.2, col);
        }
    } else if label == "▶" {
        outline(p, vec![c + vec2(-4.0, -6.0), c + vec2(6.0, 0.0), c + vec2(-4.0, 6.0)], Stroke::new(2.0f32, col));
    } else {
        let size = if kind == Kind::Main { 17.0 } else { 15.0 };
        let f = hand_font(size);
        let s = clip_text(p, label, &f, r.width() - 6.0);
        p.text(c, Align2::CENTER_CENTER, s, f, col);
    }
}

pub(super) fn look() -> Look {
    Look {
        text: PEN,
        muted: PENCIL,
        accent: RED,
        radius: RAD,
        num: |s| hand_font(s * 1.1),
        label: |s| hand_font(s * 1.25),
        body: |s| hand_font(s * 1.15),
        panel: |s, r, o, a| {
            rrect(s, r, RAD, fade(PAPER, 0.6 + 0.4 * o));
            let mut y = r.min.y + 13.0;
            while y < r.max.y {
                line(s, pos2(r.min.x + 2.0, y), pos2(r.max.x - 2.0, y), Stroke::new(1.0f32, fade(RULE, 0.45)));
                y += 13.0;
            }
            if a {
                scribble_rect(s, r.shrink(2.0), 2.0, RED);
            }
        },
        big,
        button,
        gauge,
        divider: |s, a, b| scribble(s, a, b, 1.0, fade(PENCIL, 0.6)),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn journal_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let (x0, x1) = (left + 36.0, left + w - 14.0);
        let bg = p.add(Shape::Noop);

        // ── Heading: "Shiny hunt", level circled, pencil controls
        let mut y = top + BIND + 6.0;
        let mid = y + 12.0;
        let mut hx = x0;
        if m.status_dot {
            let c = pos2(left + 13.0, mid);
            circle(&p, c, 4.5, tone(v.status.0));
            scribble_oval(&p, Rect::from_center_size(c, vec2(11.0, 11.0)), 1.0, PEN);
            tip(ui, Rect::from_center_size(c, vec2(14.0, 14.0)), "jn-dot", &v.status.1);
        }
        let head = if v.ghost { "click-through on" } else { "Shiny hunt" };
        let hr = hand(&p, pos2(hx, mid), Align2::LEFT_CENTER, head, 20.0, if v.ghost { RED } else { PEN });
        scribble(&p, pos2(hr.min.x, hr.max.y - 1.0), pos2(hr.max.x + 4.0, hr.max.y - 2.0), 1.4, fade(PEN, 0.7));
        hx = hr.max.x + 12.0;
        let ctrl_left = x1 + 2.0 - 4.0 * 21.0;
        if let (Some(lv), false) = (v.level, v.ghost) {
            let t = format!("Lv {lv}");
            if hx + text_w(&p, &t, hand_font(15.0)) + 10.0 < ctrl_left {
                let lr = hand(&p, pos2(hx, mid), Align2::LEFT_CENTER, &t, 15.0, RED);
                scribble_oval(&p, lr.expand2(vec2(6.0, 2.0)), 1.3, RED);
            }
        }
        let mut cx = x1 + 2.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, mid), vec2(20.0, 20.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                rrect(&p, r.shrink(1.0), 3.0, fade(if c == Ctrl::Close { RED } else { HIGHLIGHT }, 0.35));
            }
            ctrl_icon(&p, r, c, false, if on { RED } else { PENCIL }, 12.0);
            cx -= 21.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 4.0, mid + 12.0)), "jn-drag");
        y = mid + 20.0;

        // ── Hero: the polaroid, the count and tally marks
        let art = v.sprites();
        if art || m.target || m.counter {
            let (pw, ph): (f32, f32) = if !art { (0.0, 0.0) } else if v.compact { (86.0, 100.0) } else { (106.0, 124.0) };
            let hero_h = (ph + 8.0).max(if m.counter { 78.0 } else { 34.0 });
            if art {
                let pr = Rect::from_min_size(pos2(x0 - 8.0, y + 4.0), vec2(pw, ph));
                poly(&p, turned(pr.translate(vec2(3.0, 4.0)), -4.0), fade(Color32::BLACK, 0.16));
                poly(&p, turned(pr, -4.0), Color32::WHITE);
                let photo = Rect::from_min_max(pr.min + vec2(8.0, 8.0), pos2(pr.max.x - 8.0, pr.max.y - 26.0));
                poly(&p, turned(photo, -4.0), if v.alert.is_some() { mix(HIGHLIGHT, Color32::WHITE, 0.4) } else { Color32::from_rgb(226, 234, 242) });
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = photo.center().x + (k as f32 - (n - 1.0) / 2.0) * photo.width() * 0.3;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, photo.center().y), vec2(photo.width() * 0.45, photo.height() * 0.8));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, photo.center(), photo.size() - vec2(6.0, 6.0));
                    }
                }
                let cap = v.target.as_ref().map(|s| s.name.to_string()).unwrap_or_else(|| "???".into());
                let f = hand_font(14.0);
                p.text(pos2(pr.center().x + 1.0, pr.max.y - 12.0), Align2::CENTER_CENTER, clip_text(&p, &cap, &f, pw - 12.0), f, PEN);
                // tape over the top
                let tr = Rect::from_center_size(pos2(pr.center().x + 4.0, pr.min.y + 1.0), vec2(46.0, 14.0));
                poly(&p, turned(tr, 6.0), tape());
                if !m.single_sprite && !v.compact && !v.multi && v.alert.is_none() {
                    if let Some(sp) = v.target.clone() {
                        let nc = pos2(pr.max.x + 16.0, pr.max.y - 16.0);
                        self.draw_sprite_tinted(&p, &sp, false, nc, vec2(30.0, 28.0), fade(Color32::WHITE, 0.75));
                        hand(&p, nc + vec2(0.0, 9.0), Align2::CENTER_CENTER, "normal", 11.0, PENCIL);
                    }
                }
            }
            let rx = if art { x0 + pw + if !m.single_sprite && !v.compact && !v.multi { 34.0 } else { 10.0 } } else { x0 };
            let room = x1 - rx;
            if let Some((_, text)) = &v.alert {
                let r = hand(&p, pos2(rx, y + 26.0), Align2::LEFT_CENTER, "SHINY!!", 34.0, if v.blink { RED } else { mix(RED, GOLD, 0.5) });
                scribble_oval(&p, r.expand2(vec2(8.0, 2.0)), 2.0, RED);
                for (dx, dy, s) in [(r.max.x + 10.0, r.min.y + 4.0, 7.0), (r.max.x + 2.0, r.max.y - 2.0, 4.0)] {
                    theme::sparkle(&p, pos2(dx.min(x1 - 4.0), dy), s, GOLD);
                }
                let f = hand_font(14.0);
                p.text(pos2(rx, y + 58.0), Align2::LEFT_CENTER, clip_text(&p, text, &f, room), f, PEN);
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, y + 74.0), vec2(84.0, 26.0)), &b, &button);
            } else {
                let mut ry = y + 2.0;
                if m.target && !art {
                    let name = v.title.clone().unwrap_or_else(|| "pick a target…".into());
                    let f = hand_font(20.0);
                    hand(&p, pos2(rx, ry + 10.0), Align2::LEFT_CENTER, &clip_text(&p, &name, &f, room), 20.0, PEN);
                    ry += 26.0;
                } else if m.target {
                    let txt = if v.flash && v.blink { "just saw one!" } else { "encounters so far:" };
                    hand(&p, pos2(rx, ry + 8.0), Align2::LEFT_CENTER, txt, 14.0, if v.flash { RED } else { PENCIL });
                    ry += 18.0;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let f = fit(&p, &num, hand_font, if v.compact { 38.0 } else { 46.0 }, 20.0, room);
                    let nr = p.text(pos2(rx, ry + f.size * 0.55), Align2::LEFT_CENTER, &num, f.clone(), PEN);
                    scribble(&p, pos2(nr.min.x, nr.max.y - 4.0), pos2(nr.max.x + 6.0, nr.max.y - 5.0), 2.0, RED);
                    scribble(&p, pos2(nr.min.x + 4.0, nr.max.y - 0.5), pos2(nr.max.x + 2.0, nr.max.y - 1.5), 1.4, RED);
                    ry = nr.max.y + 4.0;
                    if v.counter_label != "ENCOUNTERS" {
                        hand(&p, pos2(nr.max.x + 10.0, nr.center().y), Align2::LEFT_CENTER, &v.counter_label.to_lowercase(), 13.0, tone(v.counter_col));
                    }
                    // this session in tally marks
                    let n = v.session.0;
                    if n > 0 && !v.compact {
                        let shown = n.min(20);
                        let tw = tally(&p, pos2(rx + 2.0, ry + 2.0), shown, 14.0, PENCIL);
                        let extra = if n > shown { format!("+{} today", theme::thousands(n - shown)) } else { "today".into() };
                        hand(&p, pos2(rx + tw + 6.0, ry + 9.0), Align2::LEFT_CENTER, &extra, 12.0, PENCIL);
                        ry += 22.0;
                    }
                    if let Some((t, _, tp)) = &v.timer {
                        let lr = hand(&p, pos2(rx, ry + 6.0), Align2::LEFT_CENTER, &format!("⏱ {t} today"), 12.5, PENCIL);
                        tip(ui, lr, "jn-timer", tp);
                    }
                }
                if let Some((text, dry, t)) = &v.watch {
                    let wr = hand(&p, pos2(x1, y + hero_h - 4.0), Align2::RIGHT_CENTER, &text.to_lowercase(), 12.0, if *dry { RED } else { PENCIL });
                    tip(ui, wr, "jn-watch", t);
                }
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let area = Rect::from_min_max(pos2(x0 - 10.0, y), pos2(x1, y + hero_h));
                confetti(&p, area, *el, &[RED, GOLD, theme::pal().accent], a);
                let br = Rect::from_center_size(area.center(), vec2(area.width() - 10.0, 34.0));
                rrect(&p, br, 2.0, fade(HIGHLIGHT, 0.85 * a));
                hand(&p, br.center(), Align2::CENTER_CENTER, &format!("★ {text}! ★"), 18.0, fade(PEN, a));
            }
            y += hero_h + 8.0;
        }

        // ── Odds: a hatched pencil bar and a note
        if m.odds_bar {
            let col = tone(v.odds.color);
            let top_y = y;
            let lr = hand(&p, pos2(x0, y + 9.0), Align2::LEFT_CENTER, "odds:", 15.0, PEN);
            let mr = hand(&p, pos2(lr.max.x + 8.0, y + 9.0), Align2::LEFT_CENTER, &v.mult_text(), 18.0, col);
            let word = v.odds_word().to_lowercase();
            let wr = hand(&p, pos2(mr.max.x + 10.0, y + 9.0), Align2::LEFT_CENTER, &word, 13.0, col);
            scribble_oval(&p, wr.expand2(vec2(5.0, 1.0)), 1.1, col);
            y += 22.0;
            gauge(&p, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 12.0)), v.odds.frac, col);
            for k in 1..3 {
                let tx = x0 + (x1 - x0) * k as f32 / 3.0;
                scribble(&p, pos2(tx, y - 3.0), pos2(tx, y + 15.0), 1.0, PENCIL);
            }
            y += 16.0;
            hand(&p, pos2(x1, y + 7.0), Align2::RIGHT_CENTER, &v.odds.right.to_lowercase(), 12.0, PENCIL);
            y += 16.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "jn-odds", &v.odds.tip);
        }
        if m.method_line {
            let f = hand_font(12.5);
            p.text(pos2(x0, y + 7.0), Align2::LEFT_CENTER, clip_text(&p, &format!("({})", v.method), &f, x1 - x0), f, PENCIL);
            y += 18.0;
        }

        // ── Notes, one per line
        let note = |ui: &mut egui::Ui, y: &mut f32, lab: &str, val: String, col: Color32, t: &str| {
            let lr = hand(&p, pos2(x0, *y + 9.0), Align2::LEFT_CENTER, &format!("{lab} –"), 14.0, PENCIL);
            let f = hand_font(15.0);
            p.text(pos2(lr.max.x + 6.0, *y + 9.0), Align2::LEFT_CENTER, clip_text(&p, &val, &f, x1 - lr.max.x - 6.0), f, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 20.0)), ("jn-n", lab.to_string()), t);
            *y += 22.0;
        };
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => note(ui, &mut y, "luck", format!("{pct:.0}% would have it by now"), tone(*col), t),
                None => note(ui, &mut y, "luck", "after the 1st battle".into(), PENCIL, "Needs the odds first (start a battle)."),
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            note(ui, &mut y, "next", format!("{what} {when}"), PEN, &t);
        }
        if m.session {
            note(ui, &mut y, "session", format!("{} · {} · {}", theme::thousands(v.session.0), v.session_time(), v.session_rate()), PEN, "Encounters, hunting time and pace this session");
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("{} ({a})", v.last.0),
                None => v.last.0.clone(),
            };
            note(ui, &mut y, "last", txt, PEN, "");
        }
        if m.sparkline {
            hand(&p, pos2(x0, y + 8.0), Align2::LEFT_CENTER, "pace, last 20 min", 13.0, PENCIL);
            hand(&p, pos2(x1, y + 8.0), Align2::RIGHT_CENTER, &format!("{}/h", theme::thousands(v.pace_hr)), 14.0, PEN);
            let g = Rect::from_min_max(pos2(x0, y + 18.0), pos2(x1, y + 44.0));
            scribble(&p, g.left_bottom(), g.right_bottom(), 1.0, PENCIL);
            scribble(&p, g.left_bottom(), g.left_top(), 1.0, PENCIL);
            let max = v.pace.iter().copied().max().unwrap_or(0).max(1) as f32;
            let n = (v.pace.len().max(2) - 1) as f32;
            let pts: Vec<Pos2> = v.pace.iter().enumerate().map(|(i, &val)| pos2(g.min.x + 4.0 + (g.width() - 6.0) * i as f32 / n, g.max.y - 2.0 - (g.height() - 4.0) * val as f32 / max)).collect();
            p.add(Shape::line(pts.clone(), Stroke::new(1.6f32, PEN)));
            if let Some(last) = pts.last() {
                circle(&p, *last, 3.0, RED);
            }
            tip(ui, g, "jn-pace", "Encounters each minute for the last 20 minutes.");
            y += 52.0;
        }

        // ── Chips as sticky notes
        if !v.chips.is_empty() {
            let mut x = x0;
            for (i, (text, col, _f, t)) in v.chips.iter().enumerate() {
                let f = hand_font(13.0);
                let cw = text_w(&p, text, f.clone()) + 16.0;
                if x + cw > x1 && x > x0 {
                    x = x0;
                    y += 28.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 22.0));
                let deg = if i % 2 == 0 { -2.5 } else { 2.0 };
                poly(&p, turned(r.translate(vec2(2.0, 2.5)), deg), fade(Color32::BLACK, 0.12));
                poly(&p, turned(r, deg), if *col == theme::MUTED() { STICKY } else { mix(STICKY, *col, 0.25) });
                p.text(r.center(), Align2::CENTER_CENTER, text, f, PEN);
                tip(ui, r, ("jn-chip", text.clone()), t);
                x += cw + 8.0;
            }
            y += 32.0;
        }

        // ── XP and daily goal
        let bar = |ui: &mut egui::Ui, y: &mut f32, lab: String, right: String, frac: f32, col: Color32, t: &str| {
            hand(&p, pos2(x0, *y + 8.0), Align2::LEFT_CENTER, &lab, 14.0, PEN);
            hand(&p, pos2(x1, *y + 8.0), Align2::RIGHT_CENTER, &right, 13.0, PENCIL);
            gauge(&p, Rect::from_min_max(pos2(x0, *y + 18.0), pos2(x1, *y + 27.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 28.0)), ("jn-bar", lab.clone()), t);
            *y += 34.0;
        };
        if m.xp {
            bar(ui, &mut y, format!("trainer lv {}", v.st.level.max(1)), format!("{} / {} xp", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, mix(Color32::from_rgb(70, 130, 220), PEN, 0.2), "Trainer level progress");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            bar(ui, &mut y, if done { "daily goal ✔".into() } else { "daily goal".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, tone(theme::GREEN), "Encounters today across every run");
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 30.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += hand(&p, pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q, 15.0, PEN).width() + 10.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h, h)), b, &button);
                    x += h + 9.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x + 4.0, y), pos2(x1, y + h)), &acts.btns[3], &button);
            } else {
                let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                let gap = 8.0;
                let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
                for b in &acts.btns {
                    let bw = room * b.weight / total;
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &button);
                    x += bw + gap;
                }
            }
            y += h + 16.0;
        } else {
            y += 8.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, page);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y + BIND * 0.45, full.width(), full.height() - BIND * 0.45, RAD], [full.min.x + 8.0, full.min.y, full.width() - 16.0, 14.0, 0.0]], full.max.y, RED);
    }
}

/// For the theme picker.
pub(super) fn preview(p: &Painter, card: Rect) {
    page(p, card, 1.0, false);
    let ph = Rect::from_min_size(card.min + vec2(7.0, 20.0), vec2(26.0, 30.0));
    poly(p, turned(ph, -5.0), Color32::WHITE);
    poly(p, turned(Rect::from_min_max(ph.min + vec2(3.0, 3.0), ph.max - vec2(3.0, 8.0)), -5.0), Color32::from_rgb(226, 234, 242));
    poly(p, turned(Rect::from_center_size(pos2(ph.center().x, ph.min.y), vec2(16.0, 6.0)), 6.0), tape());
    p.text(pos2(card.min.x + 37.0, card.min.y + 30.0), Align2::LEFT_CENTER, "42", hand_font(18.0), PEN);
    scribble(p, pos2(card.min.x + 37.0, card.min.y + 40.0), pos2(card.min.x + 56.0, card.min.y + 39.0), 1.5, RED);
    tally(p, pos2(card.min.x + 9.0, card.min.y + 55.0), 7, 9.0, PENCIL);
    rstroke(p, Rect::from_min_size(pos2(card.min.x + 36.0, card.max.y - 15.0), vec2(22.0, 10.0)), 2.0, Stroke::new(1.5f32, RED));
}

// ───────────────────────────── pop-up cards ─────────────────────────────

/// A sticky note held on with tape, a little crooked, its corner curling.
fn toast_card(p: &Painter, r: Rect, _tint: Color32) {
    let note = Rect::from_min_max(r.min + vec2(4.0, 6.0), r.max - vec2(4.0, 3.0));
    if note.height() < 4.0 {
        rrect(p, r, 2.0, STICKY);
        return;
    }
    let pts = turned(note, -0.8);
    poly(p, pts.iter().map(|q| *q + vec2(1.5, 2.0)).collect(), fade(Color32::BLACK, 0.18));
    vgrad(p, &pts, mix(STICKY, Color32::WHITE, 0.25), STICKY);
    // the curled corner
    let br = pts[2];
    let k = 12.0;
    poly(p, vec![br + vec2(-k, 0.0), br + vec2(0.0, -k), br + vec2(-k, -k * 0.2)], mix(STICKY, Color32::from_rgb(160, 120, 20), 0.35));
    // tape across the top middle
    let tape_r = Rect::from_center_size(pos2(note.center().x, note.min.y + 1.0), vec2(54.0, 13.0));
    poly(p, turned(tape_r, 3.0), tape());
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let note = Rect::from_min_max(r.min + vec2(4.0, 6.0), r.max - vec2(4.0, 3.0));
    let ic = pos2(note.min.x + 28.0, note.center().y + 1.0);
    super::ov_toast::badge(p, ic, 12.0, v);
    // circled in red pen, the loop drawn as it opens
    let ring = Rect::from_center_size(ic, vec2(40.0, 36.0));
    let k = ((v.t - 0.3) / 0.6).clamp(0.0, 1.0);
    if k > 0.0 {
        let seed = 17.0;
        let n = 28;
        let upto = ((n + 3) as f32 * k) as usize;
        let pts: Vec<Pos2> = (0..=upto)
            .map(|i| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU - 0.6;
                let w = 1.0 + ((i as f32 * 1.7 + seed).sin()) * 0.04 + i as f32 * 0.002;
                ring.center() + vec2(a.cos() * ring.width() / 2.0 * w, a.sin() * ring.height() / 2.0 * w)
            })
            .collect();
        if pts.len() > 1 {
            p.add(Shape::line(pts, Stroke::new(1.8f32, RED)));
        }
    }
    let tx = note.min.x + 56.0;
    let room = note.max.x - tx - 12.0;
    let cap = format!("{}", v.caption);
    let cr = hand(p, pos2(tx, note.min.y + 11.0), Align2::LEFT_CENTER, &clip_text(p, &cap, &hand_font(13.0), room), 13.0, v.a(RED));
    scribble(p, pos2(cr.min.x, cr.max.y - 1.0), pos2(cr.max.x + 2.0, cr.max.y - 2.0), 1.3, fade(RED, 0.7 * v.alpha));
    let f = fit(p, &v.title, hand_font, 20.0, 14.0, room);
    let title = clip_text(p, &v.title, &f, room);
    p.text(pos2(tx, note.min.y + 30.0), Align2::LEFT_CENTER, title, f, v.a(PEN));
    let lf = hand_font(13.0);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, note.min.y + 46.0), Align2::LEFT_CENTER, line_s, lf, v.a(PENCIL));
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: 4.0,
        card: toast_card,
        inner: |r| Rect::from_min_max(r.min + vec2(6.0, 8.0), r.max - vec2(8.0, 4.0)),
        toast: toast_body,
        title_font: hand_font(16.0),
        body_font: hand_font(13.0),
    }
}
