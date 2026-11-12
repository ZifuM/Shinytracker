//! Overlay theme "Pokétch": a chunky wrist watch. A red casing with the
//! controls on its top edge and rubber buttons under the screen, and a green
//! LCD screen with a faint pixel grid. The count is in dot-matrix digits
//! with the unlit dots still showing, the odds are a segmented bar, and a
//! shiny inverts the screen.

use super::ov_common::*;
use super::App;
use crate::fonts::pixel_font;
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const LCD: Color32 = Color32::from_rgb(158, 188, 108);
pub(super) const LCD_HI: Color32 = Color32::from_rgb(178, 204, 128);
pub(super) const INK: Color32 = Color32::from_rgb(30, 46, 26);
pub(super) const MID: Color32 = Color32::from_rgb(78, 102, 52);
pub(super) const CASE: Color32 = Color32::from_rgb(206, 70, 62);
pub(super) const CASE_HI: Color32 = Color32::from_rgb(234, 112, 98);
pub(super) const CASE_DARK: Color32 = Color32::from_rgb(132, 38, 38);
const RUBBER: Color32 = Color32::from_rgb(60, 56, 66);
const BUTTON: Color32 = Color32::from_rgb(255, 210, 63);
const RAD: f32 = 20.0;

/// Unlit dots: just visible on the screen.
fn ghost() -> Color32 {
    mix(LCD, INK, 0.1)
}

/// Status colours pulled towards the LCD ink so they sit on the screen.
fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        MID
    } else {
        mix(c, INK, 0.5)
    }
}

// ───────────────────────────── dot matrix ─────────────────────────────

/// A 5×7 (or narrower) glyph: width and one row of bits per line, the
/// leftmost dot in the highest bit.
fn glyph(c: char) -> Option<(u8, [u8; 7])> {
    Some(match c {
        '0' => (5, [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
        '1' => (5, [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
        '2' => (5, [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
        '3' => (5, [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110]),
        '4' => (5, [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
        '5' => (5, [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
        '6' => (5, [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
        '7' => (5, [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
        '8' => (5, [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
        '9' => (5, [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
        ',' => (2, [0, 0, 0, 0, 0, 0b01, 0b10]),
        '.' => (1, [0, 0, 0, 0, 0, 0, 1]),
        '!' => (1, [1, 1, 1, 1, 1, 0, 1]),
        '×' | 'x' => (5, [0, 0, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001]),
        '%' => (5, [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011]),
        '-' | '—' => (5, [0, 0, 0, 0b11111, 0, 0, 0]),
        '/' => (5, [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
        ' ' => (3, [0; 7]),
        'S' => (5, [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        'H' => (5, [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        'I' => (3, [0b111, 0b010, 0b010, 0b010, 0b010, 0b010, 0b111]),
        'N' => (5, [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001]),
        'Y' => (5, [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
        _ => return None,
    })
}

/// Size of `s` in dot-matrix at `cell` per dot (None if a character can't be drawn).
pub(super) fn dots_size(s: &str, cell: f32) -> Option<egui::Vec2> {
    let mut cols = 0u32;
    let mut n = 0u32;
    for c in s.chars() {
        cols += glyph(c)?.0 as u32;
        n += 1;
    }
    Some(vec2((cols + n.saturating_sub(1)) as f32 * cell, 7.0 * cell))
}

/// Dot-matrix text with its unlit dots (`ghost`, if any). Returns its rect.
pub(super) fn dots(p: &dyn Sink, r: Rect, s: &str, cell: f32, on: Color32, ghost: Option<Color32>) {
    let mut x = r.min.x;
    let d = (cell * 0.8).max(1.0);
    for c in s.chars() {
        let Some((w, rows)) = glyph(c) else { continue };
        for (ry, bits) in rows.iter().enumerate() {
            for cx in 0..w {
                let lit = bits >> (w - 1 - cx) & 1 == 1;
                let col = if lit {
                    on
                } else if let (Some(g), true) = (ghost, w >= 3) {
                    g
                } else {
                    continue;
                };
                rrect(p, Rect::from_min_size(pos2(x + cx as f32 * cell, r.min.y + ry as f32 * cell), vec2(d, d)), 0.0, col);
            }
        }
        x += (w as f32 + 1.0) * cell;
    }
}

/// Dot-matrix where it can be drawn, the pixel font otherwise.
fn lcd_big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    let cell = (font.size / 7.5).max(1.5);
    match dots_size(s, cell) {
        Some(size) => {
            let r = anchor.anchor_size(pos, size);
            dots(p, r, s, cell, col, Some(fade(col, 0.1)));
            r
        }
        None => p.text(pos, anchor, s, pixel_font(font.size * 0.55), col),
    }
}

fn label(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    p.text(pos, anchor, s, pixel_font(size), col)
}

// ───────────────────────────── parts ─────────────────────────────

/// The casing: a red shell with a lighter top edge and a dark rim.
fn casing(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.55 + 0.45 * opacity;
    vgrad(s, &rrect_pts(r, RAD), fade(CASE_HI, a), fade(CASE, a));
    rstroke(s, r.shrink(1.0), RAD, Stroke::new(2.0f32, fade(if alert { GOLD } else { CASE_DARK }, a)));
    line(s, pos2(r.min.x + RAD, r.min.y + 3.0), pos2(r.max.x - RAD, r.min.y + 3.0), Stroke::new(1.5f32, fade(Color32::WHITE, 0.3 * a)));
}

/// The LCD screen: inset, faint pixel grid, a soft shine.
fn screen(s: &dyn Sink, r: Rect, inverted: bool) {
    rrect(s, r.expand(3.0), 9.0, CASE_DARK);
    rrect(s, r.expand(1.0), 7.0, Color32::from_rgb(40, 44, 34));
    let bg = if inverted { INK } else { LCD };
    vgrad(s, &rrect_pts(r, 6.0), if inverted { INK } else { LCD_HI }, bg);
    let grid = fade(if inverted { LCD } else { INK }, 0.05);
    let mut x = r.min.x + 3.0;
    while x < r.max.x {
        line(s, pos2(x, r.min.y + 1.0), pos2(x, r.max.y - 1.0), Stroke::new(0.6f32, grid));
        x += 3.0;
    }
    let mut y = r.min.y + 3.0;
    while y < r.max.y {
        line(s, pos2(r.min.x + 1.0, y), pos2(r.max.x - 1.0, y), Stroke::new(0.6f32, grid));
        y += 3.0;
    }
}

/// A row of square segments, `frac` lit.
fn segments(s: &dyn Sink, r: Rect, frac: f32, n: usize, on: Color32) {
    let gap = 2.0;
    let w = (r.width() - gap * (n as f32 - 1.0)) / n as f32;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    for i in 0..n {
        let x = r.min.x + i as f32 * (w + gap);
        let c = if i < lit { on } else { ghost() };
        rrect(s, Rect::from_min_size(pos2(x, r.min.y), vec2(w, r.height())), 0.0, c);
    }
}

fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    rstroke(s, r.expand(2.0), 0.0, Stroke::new(1.0f32, INK));
    segments(s, r, frac, ((r.width() / 7.0) as usize).clamp(6, 30), col);
}

/// Rubber buttons under the screen; the main one is yellow.
fn button(p: &Painter, r: Rect, label_s: &str, kind: Kind, hovered: bool, down: bool) {
    let round = label_s.chars().count() <= 2 && kind == Kind::Plain;
    let r = if round { Rect::from_center_size(r.center(), vec2(r.height(), r.height())) } else { r };
    let rad = r.height() / 2.0;
    let (fill, text) = match kind {
        Kind::Main => (BUTTON, INK),
        Kind::Good => (Color32::from_rgb(110, 196, 92), INK),
        Kind::Gold => (GOLD, INK),
        Kind::Plain => (RUBBER, Color32::WHITE),
    };
    let fill = if hovered { mix(fill, Color32::WHITE, 0.15) } else { fill };
    let dy = if down { 2.0 } else { 0.0 };
    rrect(p, r.translate(vec2(0.0, 3.0)), rad, CASE_DARK);
    rrect(p, r.translate(vec2(0.0, dy)), rad, fill);
    line(p, pos2(r.min.x + rad * 0.7, r.min.y + 3.0 + dy), pos2(r.max.x - rad * 0.7, r.min.y + 3.0 + dy), Stroke::new(1.5f32, fade(Color32::WHITE, 0.3)));
    let c = r.center() + vec2(0.0, dy);
    if label_s == "⏸" {
        for dx in [-3.0f32, 3.0] {
            rrect(p, Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.0, 11.0)), 0.0, text);
        }
    } else if label_s == "▶" {
        poly(p, vec![c + vec2(-3.5, -6.0), c + vec2(6.0, 0.0), c + vec2(-3.5, 6.0)], text);
    } else {
        let size = if kind == Kind::Main { 9.0 } else { 8.0 };
        let f = pixel_font(size);
        let s = clip_text(p, &label_s.to_uppercase(), &f, r.width() - 10.0);
        p.text(c + vec2(0.0, 1.0), Align2::CENTER_CENTER, s, f, text);
    }
}

/// The Bar's background: a casing rim round an LCD strip.
fn bar_panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    casing(s, r, opacity, alert);
    screen(s, r.shrink(5.0), false);
}

pub(super) fn look() -> Look {
    Look {
        text: INK,
        muted: MID,
        accent: INK,
        radius: 12.0,
        num: |s| pixel_font(s),
        label: |s| pixel_font(s * 0.72),
        body: |s| pixel_font(s * 0.7),
        panel: bar_panel,
        big: lcd_big,
        button,
        gauge,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, fade(INK, 0.35))),
        tone,
        shadow: None,
    }
}

// ───────────────────────────── the overlay ─────────────────────────────

impl App {
    pub(super) fn poketch_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let bg = p.add(Shape::Noop);
        let scr_slot = p.add(Shape::Noop);

        // ── Casing top: engraving and the controls
        let mid = top + 13.0;
        label(&p, pos2(left + 18.0, mid + 1.0), Align2::LEFT_CENTER, if v.ghost { "CLICK-THROUGH" } else { "POKETCH" }, 7.0, if v.ghost { GOLD } else { fade(Color32::WHITE, 0.8) });
        let mut cx = left + w - 12.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, mid), vec2(19.0, 19.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            circle(&p, r.center(), 8.5, if hov && c == Ctrl::Close { Color32::from_rgb(120, 20, 20) } else if hov { CASE_DARK } else { mix(CASE, CASE_DARK, 0.45) });
            ctrl_icon(&p, r, c, false, if on { BUTTON } else { Color32::WHITE }, 11.0);
            cx -= 21.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 4.0, top + 26.0)), "pk-drag");

        // ── The screen
        let (x0, x1) = (left + 12.0, left + w - 12.0);
        let sy0 = top + 27.0;
        let (sx0, sx1) = (x0 + 10.0, x1 - 10.0);
        let sw = sx1 - sx0;
        let mut y = sy0 + 9.0;
        let alerting = v.alert.is_some();
        let ink = if alerting { LCD } else { INK };

        // status line
        let mut x = sx0;
        if m.status_dot {
            let c = tone(v.status.0);
            let r = Rect::from_center_size(pos2(x + 4.0, y + 4.0), vec2(8.0, 8.0));
            rrect(&p, r, 0.0, if v.blink || v.status.0 == theme::GREEN { c } else { ghost() });
            tip(ui, r.expand(3.0), "pk-dot", &v.status.1);
            x += 13.0;
        }
        label(&p, pos2(x, y + 4.5), Align2::LEFT_CENTER, if alerting { "!! ALERT !!" } else { "SHINY HUNT" }, 7.0, ink);
        if let Some(lv) = v.level {
            label(&p, pos2(sx1, y + 4.5), Align2::RIGHT_CENTER, &format!("LV{lv}"), 7.0, ink);
        }
        y += 14.0;
        line(&p, pos2(sx0, y), pos2(sx1, y), Stroke::new(1.0f32, fade(ink, 0.5)));
        y += 8.0;

        // ── Hero: sprite window and the count
        let art = v.sprites();
        if art || m.target || m.counter {
            let bs: f32 = if !art { 0.0 } else if v.compact { 62.0 } else { 78.0 };
            let hero_h = bs.max(if m.counter { 70.0 } else { 30.0 });
            if art {
                let br = Rect::from_min_size(pos2(sx0, y), vec2(bs, bs));
                rrect(&p, br, 0.0, if alerting { mix(INK, LCD, 0.15) } else { LCD_HI });
                rstroke(&p, br, 0.0, Stroke::new(2.0f32, ink));
                for (dx, dy) in [(0.0, 0.0), (bs - 4.0, 0.0), (0.0, bs - 4.0), (bs - 4.0, bs - 4.0)] {
                    rrect(&p, Rect::from_min_size(br.min + vec2(dx, dy), vec2(4.0, 4.0)), 0.0, ink);
                }
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = br.center().x + (k as f32 - (n - 1.0) / 2.0) * bs * 0.3;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, br.center().y), vec2(bs * 0.5, bs * 0.7));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, br.center(), vec2(bs - 8.0, bs - 8.0));
                        if !m.single_sprite && !v.compact && !alerting {
                            let nr = Rect::from_min_size(pos2(br.max.x - 24.0, br.max.y - 24.0), vec2(30.0, 30.0));
                            rrect(&p, nr, 0.0, LCD);
                            rstroke(&p, nr, 0.0, Stroke::new(1.5f32, INK));
                            self.draw_sprite_centered(&p, &sp, false, nr.center(), vec2(26.0, 26.0));
                        }
                    }
                }
            }
            let rx = if art { sx0 + bs + 12.0 } else { sx0 };
            let room = sx1 - rx;
            if let Some((_, text)) = &v.alert {
                let size = dots_size("SHINY!", 4.0).unwrap_or(vec2(100.0, 28.0));
                let cell = (room / size.x * 4.0).min(4.0);
                let r = Rect::from_min_size(pos2(rx, y + 4.0), dots_size("SHINY!", cell).unwrap_or(size));
                dots(&p, r, "SHINY!", cell, if v.blink { LCD } else { mix(LCD, GOLD, 0.6) }, Some(fade(LCD, 0.08)));
                let f = pixel_font(7.0);
                label(&p, pos2(rx, r.max.y + 10.0), Align2::LEFT_CENTER, &clip_text(&p, &text.to_uppercase(), &f, room), 7.0, LCD);
                let b = ActBtn { act: Act::Dismiss, label: "OK".into(), tip: "Stop the alert".into(), kind: Kind::Main, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, r.max.y + 20.0), vec2(64.0, 22.0)), &b, &button);
            } else {
                let mut ry = y;
                if m.target {
                    let name = v.title.clone().unwrap_or_else(|| "PICK A TARGET".into()).to_uppercase();
                    let f = pixel_font(if name.chars().count() > 12 { 7.0 } else { 9.0 });
                    let col = if v.flash && v.blink { MID } else if v.title.is_some() { INK } else { MID };
                    let nr = label(&p, pos2(rx, ry + 5.0), Align2::LEFT_CENTER, &clip_text(&p, &name, &f, room), f.size, col);
                    if v.multi {
                        tip(ui, nr, "pk-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                    }
                    ry += 16.0;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let cell = dots_size(&num, 1.0).map_or(4.0, |s| (room / s.x).min(if v.compact { 4.5 } else { 5.0 })).max(2.0);
                    // the unlit 8s behind, as on a real LCD
                    let digits = num.chars().filter(|c| c.is_ascii_digit()).count().max(4);
                    let ghost_s: String = "8".repeat(digits);
                    let gsize = dots_size(&ghost_s, cell).unwrap_or_default();
                    let gx = (rx + room - gsize.x).max(rx);
                    if gsize.x <= room {
                        dots(&p, Rect::from_min_size(pos2(gx, ry + 2.0), gsize), &ghost_s, cell, ghost(), None);
                    }
                    let size = dots_size(&num, cell).unwrap_or_default();
                    let nr = Rect::from_min_size(pos2(if gsize.x <= room { rx + room - size.x } else { rx }, ry + 2.0), size);
                    dots(&p, nr, &num, cell, INK, None);
                    let mut txt = v.counter_label.to_string();
                    if let (Some((t, _, _)), false) = (&v.timer, v.compact) {
                        txt = format!("{txt} {t}");
                    }
                    let f = pixel_font(6.5);
                    let lr = label(&p, pos2(rx, nr.max.y + 9.0), Align2::LEFT_CENTER, &clip_text(&p, &txt, &f, room), 6.5, if v.counter_label == "ENCOUNTERS" { MID } else { tone(v.counter_col) });
                    if let Some((_, _, tp)) = &v.timer {
                        tip(ui, lr, "pk-timer", tp);
                    }
                }
                if let Some((text, dry, t)) = &v.watch {
                    let f = pixel_font(6.0);
                    let wr = label(&p, pos2(sx1, y + hero_h - 2.0), Align2::RIGHT_CENTER, &clip_text(&p, text, &f, room), 6.0, if *dry { INK } else { MID });
                    tip(ui, wr, "pk-watch", t);
                }
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let area = Rect::from_min_max(pos2(sx0 - 4.0, y - 4.0), pos2(sx1 + 4.0, y + hero_h + 4.0));
                rrect(&p, area, 0.0, fade(INK, 0.92 * a));
                confetti(&p, area, *el, &[LCD, LCD_HI, GOLD], a);
                let f = pixel_font(8.0);
                label(&p, area.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "MILESTONE!", 9.0, fade(LCD, a));
                label(&p, area.center() + vec2(0.0, 10.0), Align2::CENTER_CENTER, &clip_text(&p, &text.to_uppercase(), &f, area.width() - 12.0), 8.0, fade(LCD_HI, a));
            }
            y += hero_h + 10.0;
        }

        let rule = |y: &mut f32| {
            let mut x = sx0;
            while x < sx1 {
                rrect(&p, Rect::from_min_size(pos2(x, *y), vec2(2.0, 2.0)), 0.0, fade(ink, 0.45));
                x += 5.0;
            }
            *y += 8.0;
        };

        // ── Odds: a segmented bar
        if m.odds_bar && !alerting {
            rule(&mut y);
            let col = tone(v.odds.color);
            let top_y = y;
            label(&p, pos2(sx0, y + 5.0), Align2::LEFT_CENTER, "ODDS", 7.0, INK);
            let mt = v.mult_text().replace('×', "x");
            lcd_big(&p, pos2(sx1, y + 5.0), Align2::RIGHT_CENTER, &mt, FontId::proportional(12.0), INK);
            y += 15.0;
            gauge(&p, Rect::from_min_max(pos2(sx0 + 2.0, y), pos2(sx1 - 2.0, y + 9.0)), v.odds.frac, col);
            y += 16.0;
            label(&p, pos2(sx0, y + 3.0), Align2::LEFT_CENTER, v.odds_word(), 6.0, col);
            let f = pixel_font(6.0);
            label(&p, pos2(sx1, y + 3.0), Align2::RIGHT_CENTER, &clip_text(&p, &v.odds.right.to_uppercase(), &f, sw * 0.5), 6.0, MID);
            y += 12.0;
            tip(ui, Rect::from_min_max(pos2(sx0, top_y), pos2(sx1, y)), "pk-odds", &v.odds.tip);
        }
        if m.method_line && !alerting {
            let f = pixel_font(6.0);
            label(&p, pos2(sx0, y + 4.0), Align2::LEFT_CENTER, &clip_text(&p, &v.method.to_uppercase(), &f, sw), 6.0, MID);
            y += 13.0;
        }

        // ── Readouts: label …… value
        let mut readouts: Vec<(&str, String, Color32, String)> = Vec::new();
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => readouts.push(("LUCK", format!("{pct:.0}%"), tone(*col), t.clone())),
                None => readouts.push(("LUCK", "AFTER 1ST BATTLE".into(), MID, "Needs the odds first (start a battle).".into())),
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            readouts.push(("NEXT", format!("{what} {when}").to_uppercase(), INK, t));
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("{} {a}", v.last.0),
                None => v.last.0.clone(),
            };
            readouts.push(("LAST", txt.to_uppercase(), INK, String::new()));
        }
        if !readouts.is_empty() && !alerting {
            rule(&mut y);
            for (lab, value, col, t) in readouts {
                let lr = label(&p, pos2(sx0, y + 5.0), Align2::LEFT_CENTER, lab, 7.0, MID);
                let f = pixel_font(7.0);
                let vtxt = clip_text(&p, &value, &f, sx1 - lr.max.x - 14.0);
                let vr = label(&p, pos2(sx1, y + 5.0), Align2::RIGHT_CENTER, &vtxt, 7.0, col);
                let mut dx = lr.max.x + 5.0;
                while dx < vr.min.x - 5.0 {
                    rrect(&p, Rect::from_min_size(pos2(dx, y + 7.0), vec2(1.5, 1.5)), 0.0, MID);
                    dx += 4.0;
                }
                tip(ui, Rect::from_min_max(pos2(sx0, y), pos2(sx1, y + 11.0)), ("pk-r", lab.to_string()), &t);
                y += 15.0;
            }
        }
        if m.session && !alerting {
            rule(&mut y);
            let cells = [("SESSION", theme::thousands(v.session.0), "Encounters this session"), ("TIME", v.session_time(), "Hunting time this session"), ("PACE", v.session_rate(), "Encounters per hour this session")];
            let gap = 5.0;
            let cw = (sw - gap * 2.0) / 3.0;
            for (i, (lab, val, t)) in cells.iter().enumerate() {
                let r = Rect::from_min_size(pos2(sx0 + i as f32 * (cw + gap), y), vec2(cw, 30.0));
                rstroke(&p, r, 0.0, Stroke::new(1.0f32, INK));
                rrect(&p, Rect::from_min_size(r.min, vec2(cw, 10.0)), 0.0, INK);
                label(&p, pos2(r.center().x, r.min.y + 5.5), Align2::CENTER_CENTER, lab, 5.5, LCD);
                let f = pixel_font(7.5);
                label(&p, pos2(r.center().x, r.min.y + 20.5), Align2::CENTER_CENTER, &clip_text(&p, &val.to_uppercase(), &f, cw - 6.0), 7.5, INK);
                tip(ui, r, ("pk-s", *lab), t);
            }
            y += 38.0;
        }
        if m.sparkline && !alerting {
            label(&p, pos2(sx0, y + 4.0), Align2::LEFT_CENTER, "PACE 20 MIN", 6.0, MID);
            label(&p, pos2(sx1, y + 4.0), Align2::RIGHT_CENTER, &format!("{}/H", theme::thousands(v.pace_hr)), 6.5, INK);
            let g = Rect::from_min_max(pos2(sx0, y + 11.0), pos2(sx1, y + 33.0));
            let max = v.pace.iter().copied().max().unwrap_or(0).max(1) as f32;
            let n = v.pace.len().max(1) as f32;
            let bw = g.width() / n;
            for (i, &val) in v.pace.iter().enumerate() {
                let blocks = ((val as f32 / max) * 6.0).ceil() as usize;
                for b in 0..6 {
                    let r = Rect::from_min_size(pos2(g.min.x + i as f32 * bw + 0.5, g.max.y - (b + 1) as f32 * 3.6), vec2(bw - 1.5, 2.6));
                    rrect(&p, r, 0.0, if b < blocks { INK } else { ghost() });
                }
            }
            tip(ui, g, "pk-pace", "Encounters each minute for the last 20 minutes.");
            y += 40.0;
        }
        if !v.chips.is_empty() && !alerting {
            let mut x = sx0;
            for (text, _col, _f, t) in &v.chips {
                let f = pixel_font(6.0);
                let txt = text.to_uppercase();
                let cw = text_w(&p, &txt, f.clone()) + 10.0;
                if x + cw > sx1 && x > sx0 {
                    x = sx0;
                    y += 16.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 13.0));
                rrect(&p, r, 0.0, INK);
                p.text(r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, txt, f, LCD);
                tip(ui, r, ("pk-chip", text.clone()), t);
                x += cw + 4.0;
            }
            y += 20.0;
        }
        let bar = |ui: &mut egui::Ui, y: &mut f32, lab: String, right: String, frac: f32, t: &str| {
            label(&p, pos2(sx0, *y + 4.0), Align2::LEFT_CENTER, &lab, 6.5, INK);
            label(&p, pos2(sx1, *y + 4.0), Align2::RIGHT_CENTER, &right, 6.0, MID);
            gauge(&p, Rect::from_min_max(pos2(sx0 + 2.0, *y + 12.0), pos2(sx1 - 2.0, *y + 18.0)), frac, INK);
            tip(ui, Rect::from_min_max(pos2(sx0, *y), pos2(sx1, *y + 20.0)), ("pk-bar", lab.clone()), t);
            *y += 26.0;
        };
        if m.xp && !alerting {
            bar(ui, &mut y, format!("TRAINER LV{}", v.st.level.max(1)), format!("{}/{}", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, "Trainer level progress");
        }
        if m.daily_goal && !alerting {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{}/{}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "OFF".into() };
            bar(ui, &mut y, if done { "GOAL DONE!".into() } else { "DAILY GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, "Encounters today across every run");
        }
        let scr = Rect::from_min_max(pos2(x0, sy0), pos2(x1, y + 2.0));
        let rec = Rec::default();
        // a shiny turns the screen negative
        screen(&rec, scr, alerting);
        p.set(scr_slot, rec.shape());
        y = scr.max.y + 12.0;

        // ── Rubber buttons under the screen
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 30.0;
            let mut x = x0 + 4.0;
            if let Some(q) = &acts.prompt {
                let f = pixel_font(7.0);
                x += p.text(pos2(x, y + h / 2.0), Align2::LEFT_CENTER, q.to_uppercase(), f, Color32::WHITE).width() + 8.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h, h)), b, &button);
                    x += h + 8.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x + 4.0, y), pos2(x1 - 4.0, y + h)), &acts.btns[3], &button);
            } else {
                let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                let gap = 7.0;
                let room = x1 - 4.0 - x - gap * (acts.btns.len() as f32 - 1.0);
                for b in &acts.btns {
                    let bw = room * b.weight / total;
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &button);
                    x += bw + gap;
                }
            }
            y += h + 14.0;
        } else {
            y += 4.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, casing);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, BUTTON);
    }
}

/// For the theme picker.
pub(super) fn preview(p: &Painter, card: Rect) {
    casing(p, card, 1.0, false);
    let scr = Rect::from_min_max(card.min + vec2(7.0, 12.0), pos2(card.max.x - 7.0, card.max.y - 20.0));
    screen(p, scr, false);
    let b = Rect::from_min_size(scr.min + vec2(4.0, 5.0), vec2(18.0, 18.0));
    rstroke(p, b, 0.0, Stroke::new(1.5f32, INK));
    let num = Rect::from_min_size(pos2(scr.min.x + 25.0, scr.min.y + 7.0), dots_size("42", 2.0).unwrap_or_default());
    dots(p, num, "42", 2.0, INK, Some(ghost()));
    segments(p, Rect::from_min_size(pos2(scr.min.x + 4.0, scr.min.y + 30.0), vec2(scr.width() - 8.0, 5.0)), 0.6, 8, INK);
    for k in 0..2 {
        circle(p, pos2(card.min.x + 14.0 + k as f32 * 12.0, card.max.y - 10.0), 4.5, RUBBER);
    }
    rrect(p, Rect::from_min_size(pos2(card.min.x + 36.0, card.max.y - 15.0), vec2(22.0, 9.0)), 4.5, BUTTON);
}

// ───────────────────────────── pop-up cards ─────────────────────────────

const TOAST_RAD: f32 = 14.0;

/// The watch casing with an LCD strip.
fn toast_card(p: &Painter, r: Rect, _tint: Color32) {
    let rad = TOAST_RAD.min(r.height() / 2.0);
    vgrad(p, &rrect_pts(r, rad), CASE_HI, CASE);
    rstroke(p, r.shrink(1.0), (rad - 1.0).max(0.0), Stroke::new(2.0f32, CASE_DARK));
    let lcd = toast_lcd(r);
    if lcd.height() > 4.0 {
        screen(p, lcd, false);
    }
}

fn toast_lcd(r: Rect) -> Rect {
    r.shrink2(vec2(10.0, 8.0))
}

/// A star in 9×9 dots.
const STAR: [u16; 9] = [0b000010000, 0b000111000, 0b000111000, 0b111111111, 0b011111110, 0b001111100, 0b011101110, 0b011000110, 0b100000001];
/// A medal in 9×9 dots.
const MEDAL: [u16; 9] = [0b010000010, 0b001000100, 0b000101000, 0b001111100, 0b011101110, 0b011000110, 0b011101110, 0b001111100, 0b000000000];
/// An up arrow in 9×9 dots.
const UP: [u16; 9] = [0b000010000, 0b000111000, 0b001111100, 0b011111110, 0b111111111, 0b000111000, 0b000111000, 0b000111000, 0b000111000];

fn pixel_icon(p: &Painter, c: Pos2, cell: f32, rows: &[u16; 9], on: Color32) {
    let o = c - vec2(cell * 4.5, cell * 4.5);
    for (y, bits) in rows.iter().enumerate() {
        for x in 0..9 {
            let lit = bits >> (8 - x) & 1 == 1;
            let col = if lit { on } else { ghost() };
            rrect(p, Rect::from_min_size(o + vec2(x as f32 * cell, y as f32 * cell), vec2(cell * 0.82, cell * 0.82)), 0.0, col);
        }
    }
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    use super::ov_toast::What;
    let lcd = toast_lcd(r);
    let ic = pos2(lcd.min.x + 20.0, lcd.center().y);
    let rows = match v.what {
        What::Achievement | What::Goal => &MEDAL,
        What::Milestone => &STAR,
        What::LevelUp | What::Welcome => &UP,
    };
    // the icon blinks as it lands
    let lit = v.t > 0.9 || (v.t * 6.0) as u32 % 2 == 0;
    pixel_icon(p, ic, 3.4, rows, if lit { INK } else { ghost() });
    let tx = lcd.min.x + 40.0;
    let room = lcd.max.x - tx - 6.0;
    // caption on an inverted strip
    let cap = v.caption.to_uppercase();
    let cf = pixel_font(6.5);
    let cap = clip_text(p, &cap, &cf, room - 8.0);
    let cw = text_w(p, &cap, cf.clone()) + 8.0;
    let cr = Rect::from_min_size(pos2(tx, lcd.min.y + 4.0), vec2(cw, 11.0));
    rrect(p, cr, 0.0, INK);
    p.text(cr.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, cap, cf, LCD);
    // the title in dot matrix when it can be, the pixel font otherwise
    let title = v.title.to_uppercase();
    let ty = lcd.min.y + 26.0;
    match dots_size(&title, 1.8).filter(|s| s.x <= room) {
        Some(sz) => dots(p, Rect::from_min_size(pos2(tx, ty - sz.y / 2.0), sz), &title, 1.8, INK, Some(ghost())),
        None => {
            let f = fit(p, &title, pixel_font, 8.5, 6.5, room);
            let s = clip_text(p, &title, &f, room);
            p.text(pos2(tx, ty), Align2::LEFT_CENTER, s, f, INK);
        }
    }
    let lf = pixel_font(6.0);
    let line_s = clip_text(p, &v.line.to_uppercase(), &lf, room);
    p.text(pos2(tx, lcd.max.y - 7.0), Align2::LEFT_CENTER, line_s, lf, MID);
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: TOAST_RAD,
        card: toast_card,
        inner: |r| toast_lcd(r).shrink2(vec2(2.0, 1.0)),
        toast: toast_body,
        title_font: pixel_font(8.0),
        body_font: pixel_font(6.5),
    }
}
