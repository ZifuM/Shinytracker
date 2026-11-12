//! Overlay theme "Sticker": cream paper with thick ink outlines and a
//! hard shadow, a colour-block header with speed lines, the shiny on a
//! spinning star burst, comic lettering for the count, a segmented luck
//! gauge, stats as coloured stickers and chunky sticker buttons.

use super::ov_common::*;
use super::overlay_mod::{FLAME, XP_BLUE};
use super::App;
use crate::fonts::{bold_font, display_font};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const PAPER: Color32 = Color32::from_rgb(255, 247, 232);
pub(super) const INK: Color32 = Color32::from_rgb(28, 24, 48);
const YELLOW: Color32 = Color32::from_rgb(255, 210, 63);
const PINK: Color32 = Color32::from_rgb(255, 104, 154);
const CYAN: Color32 = Color32::from_rgb(60, 203, 232);
const MINT: Color32 = Color32::from_rgb(91, 224, 160);
const ORANGE: Color32 = Color32::from_rgb(255, 146, 72);
const SH: egui::Vec2 = egui::Vec2::new(5.0, 6.0);
const POPS: [Color32; 5] = [CYAN, MINT, YELLOW, ORANGE, Color32::from_rgb(255, 170, 200)];

/// Status colours that read on paper.
fn tone(c: Color32) -> Color32 {
    if c == theme::GREEN {
        Color32::from_rgb(28, 150, 72)
    } else if c == theme::HP_YELLOW || c == GOLD {
        Color32::from_rgb(186, 128, 0)
    } else if c == theme::HP_RED {
        Color32::from_rgb(214, 48, 52)
    } else if c == FLAME {
        Color32::from_rgb(222, 98, 18)
    } else if c == XP_BLUE {
        Color32::from_rgb(36, 108, 214)
    } else if c == theme::MUTED() {
        mix(INK, PAPER, 0.45)
    } else if c == Color32::WHITE {
        INK
    } else {
        c
    }
}

/// A flat fill for a status colour (odds state, luck).
fn pop_for(c: Color32) -> Color32 {
    if c == theme::GREEN {
        MINT
    } else if c == theme::HP_YELLOW || c == GOLD {
        YELLOW
    } else if c == theme::HP_RED {
        PINK
    } else {
        Color32::WHITE
    }
}

/// A sticker: ink outline, flat fill and a hard ink shadow.
fn sticker(s: &dyn Sink, r: Rect, rad: f32, fill: Color32, shadow: f32) {
    if shadow > 0.0 {
        rrect(s, r.translate(vec2(shadow, shadow + 1.0)).expand(1.5), rad + 1.5, INK);
    }
    rrect(s, r.expand(2.0), rad + 2.0, INK);
    rrect(s, r, rad, fill);
}

/// A small label: ink outline round a flat pill. Returns its rect.
fn tag(p: &Painter, at: Pos2, anchor: Align2, text: &str, size: f32, fill: Color32, ink: Color32) -> Rect {
    let w = text_w(p, text, bold_font(size)) + size * 1.4;
    let h = size * 1.75;
    let r = anchor.anchor_size(at, vec2(w, h));
    rrect(p, r.expand(1.8), h / 2.0 + 1.8, INK);
    rrect(p, r, h / 2.0, fill);
    p.text(r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, text, bold_font(size), ink);
    r
}

fn letter(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, fill: Color32, shadow: Option<Color32>) -> Rect {
    let t = (size / 11.0).clamp(1.6, 3.2);
    lettering(p, pos, anchor, s, display_font(size), fill, INK, t, shadow)
}

fn twinkle(p: &Painter, c: Pos2, size: f32, fill: Color32) {
    theme::sparkle(p, c, size + 3.0, INK);
    theme::sparkle(p, c, size, fill);
}

fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let fill = match kind {
        Kind::Main => PINK,
        Kind::Good => MINT,
        Kind::Gold => YELLOW,
        Kind::Plain => match label {
            "−1" => CYAN,
            "+1" => MINT,
            "⏸" | "▶" => YELLOW,
            _ => Color32::WHITE,
        },
    };
    let (r, sh) = if down {
        (r.translate(vec2(2.0, 2.0)), 0.0)
    } else if hovered {
        (r.translate(vec2(-1.0, -1.0)), 4.0)
    } else {
        (r, 3.0)
    };
    sticker(p, r, 9.0, if hovered { mix(fill, Color32::WHITE, 0.2) } else { fill }, sh);
    if kind == Kind::Main {
        let size = (r.height() * 0.5).min(17.0);
        lettering(p, r.center(), Align2::CENTER_CENTER, &label.to_uppercase(), display_font(size), Color32::WHITE, INK, 1.8, None);
    } else if label == "⏸" {
        for dx in [-3.5f32, 3.5] {
            rrect(p, Rect::from_center_size(r.center() + vec2(dx, 0.0), vec2(4.0, 12.0)), 1.0, INK);
        }
    } else if label == "▶" {
        let c = r.center();
        poly(p, vec![c + vec2(-4.0, -7.0), c + vec2(7.0, 0.0), c + vec2(-4.0, 7.0)], INK);
    } else {
        let size = if label.chars().count() <= 2 { 15.0 } else { 12.0 };
        p.text(r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, label, bold_font(size), INK);
    }
}

/// Chunky gauge in three coloured sections (1×, 2×, 3× the odds) with a
/// pointer, or one colour when `col` isn't an odds colour.
fn chunky(p: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    rrect(p, r.expand(2.2), r.height() / 2.0 + 2.2, INK);
    rrect(p, r, r.height() / 2.0, Color32::WHITE);
    let w = r.width() * frac.clamp(0.0, 1.0);
    if w > 1.0 {
        rrect(p, Rect::from_min_size(r.min, vec2(w.max(r.height()), r.height())), r.height() / 2.0, col);
        rrect(p, Rect::from_min_size(r.min + vec2(3.0, 2.0), vec2((w - 6.0).max(0.0), 2.0)), 1.0, fade(Color32::WHITE, 0.6));
    }
}

fn odds_gauge(p: &Painter, r: Rect, mult: Option<f64>) {
    rrect(p, r.expand(2.5), r.height() / 2.0 + 2.5, INK);
    let seg = r.width() / 3.0;
    for (i, c) in [MINT, YELLOW, PINK].into_iter().enumerate() {
        let sr = Rect::from_min_size(pos2(r.min.x + seg * i as f32, r.min.y), vec2(seg, r.height()));
        let rounding = egui::Rounding {
            nw: if i == 0 { r.height() / 2.0 } else { 0.0 },
            sw: if i == 0 { r.height() / 2.0 } else { 0.0 },
            ne: if i == 2 { r.height() / 2.0 } else { 0.0 },
            se: if i == 2 { r.height() / 2.0 } else { 0.0 },
        };
        p.add(Shape::rect_filled(sr, rounding, c));
        if i > 0 {
            line(p, pos2(sr.min.x, r.min.y), pos2(sr.min.x, r.max.y), Stroke::new(2.0f32, INK));
        }
        p.text(pos2(sr.max.x - 5.0, r.center().y + 0.5), Align2::RIGHT_CENTER, format!("{}×", i + 1), bold_font(8.5), fade(INK, 0.55));
    }
    rrect(p, Rect::from_min_size(r.min + vec2(4.0, 2.0), vec2(r.width() - 8.0, 2.0)), 1.0, fade(Color32::WHITE, 0.55));
    if let Some(m) = mult {
        let x = r.min.x + (m.min(3.0) as f32 / 3.0) * r.width();
        let x = x.clamp(r.min.x + 2.0, r.max.x - 2.0);
        line(p, pos2(x, r.min.y - 3.0), pos2(x, r.max.y + 3.0), Stroke::new(3.0f32, INK));
        poly(p, vec![pos2(x - 6.0, r.max.y + 9.0), pos2(x + 6.0, r.max.y + 9.0), pos2(x, r.max.y + 2.0)], INK);
    }
}

fn panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let rad = 14.0;
    rrect(s, r.translate(SH), rad, INK);
    rrect(s, r, rad, if alert { YELLOW } else { INK });
    let inner = r.shrink(2.8);
    rrect(s, inner, rad - 2.8, fade(PAPER, opacity.max(0.55)));
    halftone(s, Rect::from_min_max(pos2(inner.max.x - 90.0, inner.max.y - 60.0), inner.max - vec2(6.0, 6.0)), 9.0, 1.0, 1.0, fade(PINK, 0.25));
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: egui::FontId, col: Color32) -> Rect {
    let fill = if col == INK || col == Color32::WHITE { Color32::WHITE } else { col };
    let t = (font.size / 11.0).clamp(1.5, 3.0);
    lettering(p, pos, anchor, s, display_font(font.size), fill, INK, t, Some(INK))
}

pub(super) fn look() -> Look {
    Look {
        text: INK,
        muted: mix(INK, PAPER, 0.45),
        accent: PINK,
        radius: 14.0,
        num: display_font,
        label: bold_font,
        body: bold_font,
        panel,
        big,
        button,
        gauge: chunky,
        divider: |s, a, b| line(s, a, b, Stroke::new(2.0f32, fade(INK, 0.25))),
        tone,
        shadow: Some(SH),
    }
}

impl App {
    pub(super) fn anime_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - SH.x - 2.0;
        let (x0, x1) = (left + 16.0, left + w - 16.0);
        let accent = theme::pal().accent;
        let bg = p.add(Shape::Noop);

        // ── Header: colour block, speed lines, lettering, round controls
        let head_h = if v.compact { 34.0 } else { 40.0 };
        let hb = Rect::from_min_size(pos2(left + 2.8, top + 2.8), vec2(w - 5.6, head_h));
        p.add(Shape::rect_filled(hb, egui::Rounding { nw: 11.2, ne: 11.2, sw: 0.0, se: 0.0 }, accent));
        poly(&p, vec![hb.left_bottom(), hb.right_bottom(), pos2(hb.min.x, hb.max.y + 9.0)], accent);
        speed_lines(&p.with_clip_rect(hb.shrink2(vec2(10.0, 0.0))), pos2(hb.min.x + 40.0, hb.center().y), 18.0, 420.0, 40, fade(mix(accent, Color32::WHITE, 0.45), 0.5), 7);
        poly(
            &p,
            vec![pos2(hb.min.x, hb.max.y + 9.0), pos2(hb.max.x, hb.max.y), pos2(hb.max.x, hb.max.y + 5.0), pos2(hb.min.x, hb.max.y + 14.0)],
            YELLOW,
        );
        line(&p, pos2(hb.min.x, hb.max.y + 9.0), pos2(hb.max.x, hb.max.y), Stroke::new(2.5f32, INK));
        line(&p, pos2(hb.min.x, hb.max.y + 14.0), pos2(hb.max.x, hb.max.y + 5.0), Stroke::new(2.0f32, INK));
        let title = if v.ghost { "CLICK-THROUGH" } else { "SHINY HUNT!" };
        let tr = letter(&p, pos2(x0 - 2.0, hb.center().y), Align2::LEFT_CENTER, title, if v.compact { 13.0 } else { 16.0 }, if v.ghost { YELLOW } else { Color32::WHITE }, Some(INK));
        if let (Some(lv), false) = (v.level, v.ghost || v.compact) {
            tag(&p, pos2(tr.max.x + 8.0, hb.center().y), Align2::LEFT_CENTER, &format!("LV {lv}"), 9.0, YELLOW, INK);
        }
        let ctrls = [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin];
        let cr = 10.0;
        let mut cx = hb.max.x - 14.0;
        for c in ctrls {
            let r = Rect::from_center_size(pos2(cx, hb.center().y), vec2(cr * 2.0 + 2.0, cr * 2.0 + 2.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            let fill = if hov && c == Ctrl::Close { PINK } else if hov || on { YELLOW } else { Color32::WHITE };
            circle(&p, r.center(), cr + 2.0, INK);
            circle(&p, r.center(), cr, fill);
            ctrl_icon(&p, r, c, false, INK, 12.0);
            cx -= cr * 2.0 + 5.0;
        }
        if m.status_dot {
            let c = pos2(cx + 4.0, hb.center().y);
            circle(&p, c, 6.0, INK);
            circle(&p, c, 4.2, tone(v.status.0));
            tip(ui, Rect::from_center_size(c, vec2(14.0, 14.0)), "an-dot", &v.status.1);
            cx -= 14.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(hb.min, pos2(cx - 4.0, hb.max.y)), "an-drag");
        let mut y = hb.max.y + 18.0;

        // ── Hero: star burst with the shiny, the name on the left
        if v.sprites() || m.target {
            let with_art = v.sprites();
            let hero_h = if !with_art { 36.0 } else if v.compact { 100.0 } else { 122.0 };
            let hero = Rect::from_min_max(pos2(x0, y), pos2(x1, y + hero_h));
            if with_art {
                let named = m.target && !v.compact && v.alert.is_none();
                let cxb = if named { hero.max.x - hero.width() * 0.3 } else { hero.center().x };
                let cy = hero.center().y;
                let (ri, ro) = (hero_h * 0.36, hero_h * 0.5);
                halftone(&p, hero.expand2(vec2(4.0, 0.0)), 8.0, if named { 0.0 } else { 0.5 }, 1.0, fade(PINK, 0.22));
                burst(&p, pos2(cxb, cy), ri, ro, 14, if v.alert.is_some() { PINK } else { YELLOW }, INK, 2.5, v.t * 0.25);
                circle(&p, pos2(cxb, cy), ri * 0.95, fade(Color32::WHITE, 0.85));
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        let step = (ro * 1.3).min(hero.width() / n);
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = cxb + (k as f32 - (n - 1.0) / 2.0) * step * 0.7;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, cy), vec2(ri * 1.1, ri * 1.5));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, pos2(cxb, cy), vec2(ri * 1.75, ri * 1.75));
                        if !m.single_sprite && v.alert.is_none() {
                            let c = pos2(cxb - ro * 0.95, cy + hero_h * 0.26);
                            let r = (hero_h * 0.17).max(15.0);
                            circle(&p, c, r + 2.5, INK);
                            circle(&p, c, r, Color32::WHITE);
                            self.draw_sprite_centered(&p, &sp, false, c, vec2(r * 1.5, r * 1.5));
                            tag(&p, c + vec2(0.0, r + 4.0), Align2::CENTER_TOP, "NORMAL", 7.5, Color32::WHITE, INK);
                        }
                    } else {
                        letter(&p, pos2(cxb, cy), Align2::CENTER_CENTER, "?", ri * 1.2, Color32::WHITE, Some(INK));
                    }
                    let tw = 0.65 + 0.35 * (v.t * 3.0).sin().abs();
                    twinkle(&p, pos2(cxb + ro * 0.8, cy - ro * 0.6), 8.0 * tw, Color32::WHITE);
                    twinkle(&p, pos2(cxb - ro * 0.55, cy - ro * 0.75), 5.5 * (1.6 - tw), PINK);
                }
                if named && v.alert.is_none() {
                    let nx1 = cxb - ro - 6.0;
                    self.anime_name(ui, &p, v, x0, nx1, hero.min.y + 22.0);
                }
            } else {
                self.anime_name(ui, &p, v, x0, x1, hero.min.y + 14.0);
            }
            // Shiny alert / milestone over the hero.
            if let Some((_, text)) = &v.alert {
                let c = hero.center();
                letter(&p, pos2(c.x, hero.min.y + 12.0), Align2::CENTER_CENTER, "SHINY!", 28.0, if v.blink { YELLOW } else { Color32::WHITE }, Some(INK));
                tag(&p, pos2(c.x, hero.max.y + 2.0), Align2::CENTER_BOTTOM, text, 9.5, Color32::WHITE, INK);
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                confetti(&p, hero, *el, &[PINK, YELLOW, CYAN, MINT, ORANGE], a);
                let br = Rect::from_center_size(hero.center(), vec2(hero.width() - 20.0, 54.0));
                sticker(&p, br, 12.0, fade(YELLOW, a), 3.0);
                let size = if text.chars().count() > 16 { 13.0 } else { 17.0 };
                letter(&p, br.center() - vec2(0.0, 7.0), Align2::CENTER_CENTER, &text.to_uppercase(), size, fade(Color32::WHITE, a), None);
                p.text(br.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "MILESTONE! KEEP GOING", bold_font(9.5), fade(INK, a));
            }
            y = hero.max.y + 10.0;
            if v.alert.is_some() {
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_center_size(pos2((x0 + x1) / 2.0, y + 14.0), vec2(96.0, 26.0)), &b, &button);
                y += 36.0;
            }
        }

        // ── Count: big lettering, a state tag and today's time
        if m.counter {
            let size = if v.compact { 30.0 } else { 38.0 };
            let num = theme::thousands(v.count);
            let room = x1 - x0 - 104.0;
            let font = fit(&p, &num, display_font, size, 18.0, room);
            let h = font.size * 1.15;
            lettering(&p, pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, &num, font.clone(), Color32::WHITE, INK, (font.size / 11.0).clamp(1.8, 3.2), Some(CYAN));
            let label_fill = if v.counter_label == "ENCOUNTERS" { INK } else { pop_for(v.counter_col) };
            let label_ink = if v.counter_label == "ENCOUNTERS" { Color32::WHITE } else { INK };
            let tr = tag(&p, pos2(x1, y + 6.0), Align2::RIGHT_TOP, v.counter_label, 8.5, label_fill, label_ink);
            if let Some((t, idle, tp)) = &v.timer {
                let r = p.text(pos2(x1, tr.max.y + 12.0), Align2::RIGHT_CENTER, format!("{t} today"), bold_font(10.5), if *idle { mix(INK, PAPER, 0.55) } else { INK });
                tip(ui, r.expand(3.0), "an-timer", tp);
            }
            y += h.max(40.0) + 8.0;
        }

        // ── Odds: state tag, the multiple, a segmented gauge
        if m.odds_bar {
            let col = pop_for(v.odds.color);
            let top_y = y;
            let tr = tag(&p, pos2(x0, y + 8.0), Align2::LEFT_CENTER, v.odds_word(), 8.5, col, INK);
            p.text(pos2(tr.max.x + 8.0, y + 8.0), Align2::LEFT_CENTER, format!("{} the odds", v.mult_text()), bold_font(12.0), INK);
            p.text(pos2(x1, y + 8.0), Align2::RIGHT_CENTER, &v.odds.right, bold_font(10.0), mix(INK, PAPER, 0.4));
            y += 22.0;
            odds_gauge(&p, Rect::from_min_max(pos2(x0 + 2.0, y), pos2(x1 - 2.0, y + 12.0)), v.odds.mult);
            y += 26.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y - 4.0)), "an-odds", &v.odds.tip);
        }
        if m.method_line {
            p.text(pos2((x0 + x1) / 2.0, y + 6.0), Align2::CENTER_CENTER, &v.method, bold_font(10.0), mix(INK, PAPER, 0.4));
            y += 20.0;
        }

        // ── Stats as stickers
        let mut k = 0usize;
        let mut row = |ui: &mut egui::Ui, y: &mut f32, cells: Vec<(&str, String, Color32, String)>, h: f32| {
            let n = cells.len().max(1) as f32;
            let gap = 10.0;
            let cw = (x1 - x0 - gap * (n - 1.0)) / n;
            for (i, (label, value, col, t)) in cells.into_iter().enumerate() {
                let r = Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), *y), vec2(cw, h));
                sticker(&p, r, 9.0, POPS[k % POPS.len()], 3.0);
                k += 1;
                p.text(r.min + vec2(8.0, 9.0), Align2::LEFT_CENTER, label, bold_font(8.5), fade(INK, 0.75));
                let f = bold_font(13.0);
                let vtxt = clip_text(&p, &value, &f, r.width() - 14.0);
                p.text(pos2(r.min.x + 8.0, r.max.y - 11.0), Align2::LEFT_CENTER, vtxt, f, tone(col));
                tip(ui, r, ("an-cell", label.to_string()), &t);
            }
            *y += h + 12.0;
        };
        let mut pair: Vec<(&str, String, Color32, String)> = Vec::new();
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => pair.push(("LUCK", format!("{pct:.0}% would have it"), *col, t.clone())),
                None => pair.push(("LUCK", "after 1st battle".into(), theme::MUTED(), "Needs the odds first (start a battle).".into())),
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            pair.push(("NEXT", format!("{what} {when}"), INK, t));
        }
        if !pair.is_empty() {
            row(ui, &mut y, pair, 38.0);
        }
        if m.session {
            row(
                ui,
                &mut y,
                vec![
                    ("SESSION", theme::thousands(v.session.0), INK, "Encounters this session (a session ends after a long break)".into()),
                    ("TIME", v.session_time(), INK, "Hunting time this session".into()),
                    ("PACE", v.session_rate(), INK, "Encounters per hour this session".into()),
                ],
                38.0,
            );
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("{} · {a}", v.last.0),
                None => v.last.0.clone(),
            };
            row(ui, &mut y, vec![("LAST BATTLE", txt, INK, String::new())], 38.0);
        }
        if m.sparkline {
            let r = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 46.0));
            sticker(&p, r, 9.0, Color32::WHITE, 3.0);
            p.text(r.min + vec2(8.0, 9.0), Align2::LEFT_CENTER, "PACE · LAST 20 MIN", bold_font(8.5), fade(INK, 0.75));
            p.text(pos2(r.max.x - 8.0, r.min.y + 9.0), Align2::RIGHT_CENTER, format!("{}/h", theme::thousands(v.pace_hr)), bold_font(10.5), INK);
            sparkline(&p, Rect::from_min_max(pos2(r.min.x + 8.0, r.min.y + 18.0), pos2(r.max.x - 8.0, r.max.y - 6.0)), &v.pace, PINK, INK);
            tip(ui, r, "an-pace", "Encounters each minute for the last 20 minutes (pink is this minute).");
            y = r.max.y + 12.0;
        }

        // ── Chips
        if !v.chips.is_empty() {
            let mut x = x0;
            for (text, col, _fire, t) in &v.chips {
                let fill = if *col == FLAME { ORANGE } else if *col == GOLD { YELLOW } else { Color32::WHITE };
                let wdt = text_w(&p, text, bold_font(8.5)) + 8.5 * 1.4;
                if x + wdt > x1 && x > x0 {
                    x = x0;
                    y += 22.0;
                }
                let r = tag(&p, pos2(x, y + 8.0), Align2::LEFT_CENTER, text, 8.5, fill, INK);
                tip(ui, r, ("an-chip", text.clone()), t);
                x = r.max.x + 8.0;
            }
            y += 24.0;
        }

        // ── Trainer XP and daily goal
        let bar = |ui: &mut egui::Ui, y: &mut f32, label: String, right: String, frac: f32, col: Color32, fill: Color32, t: &str| {
            let tr = tag(&p, pos2(x0, *y + 8.0), Align2::LEFT_CENTER, &label, 8.0, fill, INK);
            p.text(pos2(x1, *y + 8.0), Align2::RIGHT_CENTER, right, bold_font(10.0), mix(INK, PAPER, 0.35));
            let br = Rect::from_min_max(pos2(tr.max.x + 10.0, *y + 4.0), pos2(x1 - 74.0, *y + 12.0));
            chunky(&p, br, frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 16.0)), ("an-bar", label.clone()), t);
            *y += 24.0;
        };
        if m.xp {
            bar(ui, &mut y, format!("LV {}", v.st.level.max(1)), format!("{} XP", theme::thousands(v.st.xp_need.saturating_sub(v.st.xp_into))), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, CYAN, CYAN, "Trainer level progress (XP to the next level on the right)");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            bar(ui, &mut y, if done { "GOAL ✔".into() } else { "GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { YELLOW } else { MINT }, if done { YELLOW } else { MINT }, "Encounters today across every run, towards your daily goal");
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 32.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += p.text(pos2(x0, y + h / 2.0 + 2.0), Align2::LEFT_CENTER, q, bold_font(11.5), INK).width() + 10.0;
            }
            let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
            let gap = 9.0;
            let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
            for b in &acts.btns {
                let bw = room * b.weight / total;
                self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y + 2.0), vec2(bw, h)), b, &button);
                x += bw + gap;
            }
            y += h + 14.0;
        } else {
            y += 4.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, panel);
        let shapes = vec![[full.min.x, full.min.y, full.width(), full.height(), 14.0], [full.min.x + SH.x, full.min.y + SH.y, full.width(), full.height(), 14.0]];
        self.ov_finish(ui, ctx, v, full, shapes, full.max.y + SH.y, INK);
    }

    /// The target's name in lettering with its "seen" tag, between x0..x1.
    fn anime_name(&mut self, ui: &mut egui::Ui, p: &Painter, v: &OvView, x0: f32, x1: f32, y: f32) {
        let room = (x1 - x0).max(60.0);
        match &v.title {
            Some(t) => {
                let name = t.to_uppercase();
                let font = fit(p, &name, display_font, 22.0, 11.0, room - 6.0);
                let name = clip_text(p, &name, &font, room - 6.0);
                let r = lettering(p, pos2(x0, y), Align2::LEFT_CENTER, &name, font.clone(), Color32::WHITE, INK, (font.size / 11.0).clamp(1.5, 2.6), Some(PINK));
                if v.multi {
                    tip(ui, r, "an-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                }
                let ty = r.max.y + 12.0;
                if v.flash {
                    if v.blink {
                        tag(p, pos2(x0, ty), Align2::LEFT_CENTER, "SPOTTED!", 8.5, YELLOW, INK);
                    }
                } else if let Some((text, dry, t)) = &v.watch {
                    let tr = tag(p, pos2(x0, ty), Align2::LEFT_CENTER, text, 8.0, if *dry { YELLOW } else { Color32::WHITE }, INK);
                    tip(ui, tr, "an-watch", t);
                }
            }
            None => {
                tag(p, pos2(x0, y), Align2::LEFT_CENTER, "PICK A TARGET ⚙", 9.0, Color32::WHITE, INK);
            }
        }
    }
}

// ───────────────────────────── pop-up cards ─────────────────────────────

/// A sticker card with its hard shadow inside `r`.
fn toast_card(p: &Painter, r: Rect, tint: Color32) {
    let rad = 12.0f32.min(r.height() / 2.0);
    let body = Rect::from_min_max(r.min, r.max - vec2(4.0, 5.0).min(r.size() * 0.1));
    rrect(p, body.translate(vec2(4.0, 5.0)), rad, INK);
    rrect(p, body, rad, INK);
    let inner = body.shrink(2.5);
    if inner.height() > 2.0 {
        rrect(p, inner, (rad - 2.5).max(0.0), PAPER);
        let c = pop_for_rgb(tint);
        halftone(p, Rect::from_min_max(pos2(inner.max.x - 80.0, inner.min.y + 4.0), inner.max - vec2(4.0, 4.0)), 8.0, 1.0, 1.0, fade(c, 0.35));
    }
}

/// A flat sticker colour close to `c`.
fn pop_for_rgb(c: Color32) -> Color32 {
    let pick = [PINK, CYAN, MINT, YELLOW, ORANGE];
    let d = |a: Color32| {
        let (r, g, b) = (a.r() as i32 - c.r() as i32, a.g() as i32 - c.g() as i32, a.b() as i32 - c.b() as i32);
        r * r + g * g + b * b
    };
    *pick.iter().min_by_key(|a| d(**a)).unwrap()
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let body = Rect::from_min_max(r.min, r.max - vec2(4.0, 5.0)).shrink(2.5);
    let pop = if v.what == super::ov_toast::What::Milestone { CYAN } else { pop_for_rgb(v.tint()) };
    // the icon on a spinning star burst
    let ic = pos2(body.min.x + 32.0, body.center().y);
    burst(p, ic, 15.0, 21.5, 12, pop, INK, 2.0, v.t * 0.5);
    super::ov_toast::badge(p, ic, 13.0, v);
    for (i, (dx, dy)) in [(-20.0, -17.0), (19.0, -18.0), (21.0, 16.0)].into_iter().enumerate() {
        let tw = 0.5 + 0.5 * (v.t * 6.0 + i as f32 * 2.1).sin();
        twinkle(p, ic + vec2(dx, dy), 2.5 + 2.5 * tw, Color32::WHITE);
    }
    // caption tag, comic title, small line
    let tx = body.min.x + 64.0;
    let room = body.max.x - tx - 8.0;
    let cap = v.caption.to_uppercase();
    tag(p, pos2(tx, body.min.y + 11.0), Align2::LEFT_CENTER, &cap, 7.5, if pop == YELLOW { PINK } else { YELLOW }, INK);
    let title = v.title.to_uppercase();
    let f = fit(p, &title, display_font, 15.0, 10.0, room - 6.0);
    let title = clip_text(p, &title, &f, room - 6.0);
    letter(p, pos2(tx + 1.0, body.min.y + 31.0), Align2::LEFT_CENTER, &title, f.size, Color32::WHITE, Some(INK));
    let lf = bold_font(10.0);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, body.min.y + 48.5), Align2::LEFT_CENTER, line_s, lf, v.a(mix(INK, PAPER, 0.3)));
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: 12.0,
        card: toast_card,
        inner: |r| Rect::from_min_max(r.min + vec2(4.0, 4.0), r.max - vec2(8.0, 9.0)),
        toast: toast_body,
        title_font: display_font(13.0),
        body_font: bold_font(10.5),
    }
}
