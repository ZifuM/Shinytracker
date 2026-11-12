//! Overlay theme "Holo card": a collectible card in a clear sleeve. A gold
//! border, a name bar with the odds in the corner, the shiny in an art
//! window with a foil shimmer that sweeps across, the stats listed like
//! moves with energy-dot costs, and the count as the card number
//! ("6,642 / 4,096") along the bottom. An original card design.

use super::ov_common::*;
use super::App;
use crate::fonts::{black_font, bold_font, sans_bold_font, sans_font};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

pub(super) const GOLD_HI: Color32 = Color32::from_rgb(255, 232, 140);
pub(super) const GOLD_LO: Color32 = Color32::from_rgb(196, 146, 42);
pub(super) const TEXT: Color32 = Color32::from_rgb(28, 26, 34);
const SOFT: Color32 = Color32::from_rgb(92, 88, 104);
const RAD: f32 = 14.0;
/// The sleeve's lip above the card (holds the controls).
const LIP: f32 = 22.0;

/// The card's colour: the theme accent, light.
fn body() -> Color32 {
    mix(theme::pal().accent, Color32::WHITE, 0.72)
}

fn deep() -> Color32 {
    mix(theme::pal().accent, Color32::BLACK, 0.25)
}

fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        SOFT
    } else {
        mix(c, Color32::BLACK, 0.3)
    }
}

/// A rainbow colour, `h` 0..1 round the wheel.
pub(super) fn rainbow(h: f32) -> Color32 {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let soft = |v: f32| (150.0 + v * 105.0) as u8;
    Color32::from_rgb(soft(r), soft(g), soft(b))
}

/// Foil: rainbow bands sliding diagonally across `r` (clip the painter).
pub(super) fn foil(p: &Painter, r: Rect, t: f32, strength: f32) {
    let band = 26.0;
    let span = r.width() + r.height();
    let shift = (t * 40.0) % (band * 6.0);
    let mut k = -6.0;
    while k * band < span + band * 6.0 {
        let x = r.min.x - r.height() + k * band + shift;
        let col = fade(rainbow(k / 6.0), strength);
        poly(p, vec![pos2(x, r.max.y), pos2(x + band, r.max.y), pos2(x + band + r.height(), r.min.y), pos2(x + r.height(), r.min.y)], col);
        k += 1.0;
    }
    // a bright glint that passes now and then
    let gx = r.min.x - 60.0 + ((t * 0.35) % 1.6) * (r.width() + 120.0);
    poly(p, vec![pos2(gx, r.max.y), pos2(gx + 16.0, r.max.y), pos2(gx + 16.0 + r.height() * 0.6, r.min.y), pos2(gx + r.height() * 0.6, r.min.y)], fade(Color32::WHITE, 0.35 * strength / 0.3));
}

/// An energy dot: a coloured circle with a little star.
fn energy(s: &dyn Sink, c: Pos2, r: f32, col: Color32) {
    circle(s, c, r + 1.2, fade(Color32::BLACK, 0.45));
    circle(s, c, r, col);
    circle(s, c - vec2(r * 0.3, r * 0.3), r * 0.35, fade(Color32::WHITE, 0.5));
}

/// The clear sleeve and the card in it (the lip on top).
fn sleeve(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.35 + 0.5 * opacity;
    rrect(s, r, RAD + 4.0, rgba(232, 238, 250, a * 0.55));
    rstroke(s, r.shrink(0.5), RAD + 4.0, Stroke::new(1.0f32, fade(Color32::WHITE, 0.8)));
    line(s, pos2(r.min.x + 10.0, r.min.y + LIP - 3.0), pos2(r.max.x - 10.0, r.min.y + LIP - 3.0), Stroke::new(1.0f32, fade(Color32::WHITE, 0.6)));
    let card = Rect::from_min_max(pos2(r.min.x + 5.0, r.min.y + LIP), pos2(r.max.x - 5.0, r.max.y - 5.0));
    border(s, card, alert);
}

/// The gold border and the card's coloured body.
fn border(s: &dyn Sink, card: Rect, alert: bool) {
    vgrad(s, &rrect_pts(card, RAD), GOLD_HI, GOLD_LO);
    rstroke(s, card, RAD, Stroke::new(1.5f32, if alert { Color32::WHITE } else { mix(GOLD_LO, Color32::BLACK, 0.3) }));
    let inner = card.shrink(7.0);
    vgrad(s, &rrect_pts(inner, RAD - 5.0), mix(body(), Color32::WHITE, 0.25), body());
}

fn gauge(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    rrect(s, r, r.height() / 2.0, fade(TEXT, 0.15));
    let w = r.width() * frac.clamp(0.0, 1.0);
    if w > 1.0 {
        hgrad(s, &rrect_pts(Rect::from_min_size(r.min, vec2(w.max(r.height()), r.height())), r.height() / 2.0), mix(col, Color32::WHITE, 0.3), col);
    }
    rstroke(s, r, r.height() / 2.0, Stroke::new(1.0f32, fade(TEXT, 0.4)));
}

/// Gold-edged tabs; Caught! is foil gold.
fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let round = label.chars().count() <= 2 && kind == Kind::Plain;
    let r = if round { Rect::from_center_size(r.center(), vec2(r.height(), r.height())) } else { r };
    let r = if down { r.translate(vec2(0.0, 1.0)) } else { r };
    let rad = r.height() / 2.0;
    match kind {
        Kind::Main | Kind::Gold => {
            vgrad(p, &rrect_pts(r, rad), if hovered { Color32::WHITE } else { GOLD_HI }, GOLD_LO);
        }
        Kind::Good => vgrad(p, &rrect_pts(r, rad), mix(theme::GREEN, Color32::WHITE, 0.3), mix(theme::GREEN, Color32::BLACK, 0.15)),
        Kind::Plain => vgrad(p, &rrect_pts(r, rad), if hovered { Color32::WHITE } else { mix(body(), Color32::WHITE, 0.5) }, body()),
    }
    rstroke(p, r, rad, Stroke::new(1.5f32, GOLD_LO));
    let c = r.center();
    if label == "⏸" {
        for dx in [-3.0f32, 3.0] {
            rrect(p, Rect::from_center_size(c + vec2(dx, 0.0), vec2(3.0, 11.0)), 1.0, TEXT);
        }
    } else if label == "▶" {
        poly(p, vec![c + vec2(-3.5, -6.0), c + vec2(6.0, 0.0), c + vec2(-3.5, 6.0)], TEXT);
    } else {
        let f = if kind == Kind::Main { black_font(13.0) } else { sans_bold_font(if round { 14.0 } else { 12.0 }) };
        let s = clip_text(p, label, &f, r.width() - 8.0);
        p.text(c, Align2::CENTER_CENTER, s, f, TEXT);
    }
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    p.text(pos, anchor, s, black_font(font.size), col)
}

pub(super) fn look() -> Look {
    Look {
        text: TEXT,
        muted: SOFT,
        accent: deep(),
        radius: RAD,
        num: black_font,
        label: bold_font,
        body: sans_bold_font,
        panel: |s, r, _o, a| border(s, r, a),
        big,
        button,
        gauge,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, fade(GOLD_LO, 0.6))),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn holo_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let bg = p.add(Shape::Noop);

        // ── The sleeve's lip: status, title, controls
        let mid = top + LIP / 2.0 - 1.0;
        let mut x = left + 14.0;
        if m.status_dot {
            circle(&p, pos2(x + 3.0, mid), 3.5, v.status.0);
            tip(ui, Rect::from_center_size(pos2(x + 3.0, mid), vec2(14.0, 14.0)), "ho-dot", &v.status.1);
            x += 12.0;
        }
        let lip = if v.ghost { "CLICK-THROUGH" } else if w > 300.0 { "SHINY HUNT · COLLECTOR'S EDITION" } else { "SHINY HUNT" };
        let f = bold_font(8.0);
        p.text(pos2(x, mid), Align2::LEFT_CENTER, clip_text(&p, lip, &f, left + w - 10.0 - 4.0 * 19.0 - x - 6.0), f, if v.ghost { GOLD } else { fade(TEXT, 0.6) });
        let mut cx = left + w - 10.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 8.0, mid), vec2(18.0, 18.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                circle(&p, r.center(), 9.0, if c == Ctrl::Close { Color32::from_rgb(196, 43, 28) } else { fade(Color32::WHITE, 0.7) });
            }
            ctrl_icon(&p, r, c, on, if on { GOLD_LO } else if hov && c == Ctrl::Close { Color32::WHITE } else { fade(TEXT, 0.6) }, 11.0);
            cx -= 19.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 4.0, top + LIP)), "ho-drag");

        let card_x0 = left + 5.0;
        let card_x1 = left + w - 5.0;
        let (x0, x1) = (card_x0 + 13.0, card_x1 - 13.0);
        let mut y = top + LIP + 13.0;

        // ── Name bar: stage tag, name, odds as "HP" with an energy dot
        if m.target || m.odds_bar {
            let mut nx = x0;
            if let Some(lv) = v.level {
                let t = format!("LV {lv}");
                let tw = text_w(&p, &t, bold_font(8.0)) + 10.0;
                let r = Rect::from_min_size(pos2(nx, y + 2.0), vec2(tw, 14.0));
                rrect(&p, r, 3.0, deep());
                p.text(r.center(), Align2::CENTER_CENTER, t, bold_font(8.0), Color32::WHITE);
                nx += tw + 6.0;
            }
            let mut right_w = 0.0;
            if m.odds_bar {
                let col = v.odds.color;
                energy(&p, pos2(x1 - 8.0, y + 10.0), 8.0, col);
                theme::sparkle(&p, pos2(x1 - 8.0, y + 10.0), 4.0, Color32::WHITE);
                let mr = p.text(pos2(x1 - 22.0, y + 11.0), Align2::RIGHT_CENTER, v.mult_text(), black_font(18.0), TEXT);
                let lr = p.text(pos2(mr.min.x - 3.0, y + 14.0), Align2::RIGHT_CENTER, "ODDS", bold_font(7.5), fade(TEXT, 0.75));
                right_w = x1 - lr.min.x + 6.0;
                tip(ui, Rect::from_min_max(lr.min - vec2(0.0, 8.0), pos2(x1, y + 22.0)), "ho-odds-hp", &v.odds.tip);
            }
            if m.target {
                let name = v.title.clone().unwrap_or_else(|| "Pick a target".into());
                let room = x1 - right_w - nx;
                let f = fit(&p, &name, black_font, 17.0, 11.0, room);
                let col = if v.flash && v.blink { GOLD_LO } else { TEXT };
                let nr = p.text(pos2(nx, y + 10.0), Align2::LEFT_CENTER, clip_text(&p, &name, &f, room), f, col);
                if v.multi {
                    tip(ui, nr, "ho-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                }
            }
            y += 24.0;
        }

        // ── Art window with the foil
        let art = v.sprites();
        if art {
            let ah = if v.compact { 100.0 } else { 132.0 };
            let ar = Rect::from_min_max(pos2(x0, y), pos2(x1, y + ah));
            rrect(&p, ar.expand(3.0), 3.0, GOLD_LO);
            rrect(&p, ar.expand(1.5), 2.0, GOLD_HI);
            let alerting = v.alert.is_some();
            vgrad(&p, &rrect_pts(ar, 1.0), mix(theme::pal().accent, Color32::WHITE, 0.55), mix(theme::pal().accent, Color32::from_rgb(20, 18, 40), 0.35));
            let clip = p.with_clip_rect(ar);
            for (k, a) in [(1.0, 0.10), (0.7, 0.14), (0.45, 0.2)] {
                circle(&clip, ar.center(), ah * 0.5 * k, fade(Color32::WHITE, a));
            }
            foil(&clip, ar, v.t, if alerting { 0.55 } else { 0.22 });
            if alerting {
                burst(&clip, ar.center(), ah * 0.28, ah * 0.6, 16, fade(GOLD_HI, 0.85), Color32::TRANSPARENT, 0.0, v.t * 0.6);
            }
            if m.sprites {
                if v.multi {
                    let n = v.all.len().min(3) as f32;
                    for (k, sp) in v.all.iter().take(3).enumerate() {
                        let fx = ar.center().x + (k as f32 - (n - 1.0) / 2.0) * ar.width() * 0.28;
                        self.draw_sprite_centered(&clip, sp, true, pos2(fx, ar.center().y), vec2(ar.width() * 0.4, ah * 0.85));
                    }
                } else if let Some(sp) = v.target.clone() {
                    self.draw_sprite_centered(&clip, &sp, true, ar.center(), vec2(ah * 1.1, ah - 10.0));
                    if !m.single_sprite && !v.compact && !alerting {
                        let nr = Rect::from_min_size(pos2(ar.max.x - 46.0, ar.max.y - 46.0), vec2(40.0, 40.0));
                        circle(&clip, nr.center(), 20.0, fade(Color32::WHITE, 0.55));
                        p.circle_stroke(nr.center(), 20.0, Stroke::new(1.5f32, GOLD_LO));
                        self.draw_sprite_centered(&clip, &sp, false, nr.center(), vec2(34.0, 34.0));
                    }
                }
            }
            if let Some((_, text)) = &v.alert {
                let c = pos2(ar.center().x, ar.min.y + 24.0);
                for d in [vec2(2.0, 2.0), vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(0.0, -1.0), vec2(0.0, 1.0)] {
                    p.text(c + d, Align2::CENTER_CENTER, "SHINY!", black_font(26.0), if d.x > 1.0 { fade(Color32::BLACK, 0.5) } else { GOLD_LO });
                }
                p.text(c, Align2::CENTER_CENTER, "SHINY!", black_font(26.0), if v.blink { GOLD_HI } else { Color32::WHITE });
                let f = sans_bold_font(11.0);
                let tr = Rect::from_center_size(pos2(ar.center().x, ar.max.y - 14.0), vec2(ar.width() - 20.0, 18.0));
                rrect(&clip, tr, 9.0, fade(Color32::BLACK, 0.45));
                p.text(tr.center(), Align2::CENTER_CENTER, clip_text(&p, text, &f, tr.width() - 10.0), f, Color32::WHITE);
            }
            if let Some((text, dry, t)) = &v.watch {
                let f = bold_font(8.0);
                let tw = text_w(&p, text, f.clone()) + 10.0;
                let r = Rect::from_min_size(pos2(ar.min.x + 5.0, ar.min.y + 5.0), vec2(tw, 14.0));
                rrect(&clip, r, 7.0, fade(Color32::BLACK, 0.4));
                p.text(r.center(), Align2::CENTER_CENTER, text, f, if *dry { GOLD_HI } else { Color32::WHITE });
                tip(ui, r, "ho-watch", t);
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                confetti(&clip, ar, *el, &[GOLD_HI, Color32::WHITE, theme::pal().accent], a);
                let br = Rect::from_center_size(ar.center(), vec2(ar.width() - 16.0, 40.0));
                vgrad(&clip, &rrect_pts(br, 8.0), fade(GOLD_HI, a), fade(GOLD_LO, a));
                p.text(br.center(), Align2::CENTER_CENTER, text, black_font(16.0), fade(TEXT, a));
            }
            y += ah + 6.0;
            if v.alert.is_some() {
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Main, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_center_size(pos2((x0 + x1) / 2.0, y + 13.0), vec2(110.0, 24.0)), &b, &button);
                y += 30.0;
            }
        }
        // the info strip under the art
        if m.method_line {
            let r = Rect::from_min_max(pos2(x0 + 8.0, y), pos2(x1 - 8.0, y + 16.0));
            vgrad(&p, &rrect_pts(r, 2.0), GOLD_HI, mix(GOLD_HI, GOLD_LO, 0.5));
            let f = egui::FontId::new(9.0, egui::FontFamily::Proportional);
            p.text(r.center(), Align2::CENTER_CENTER, clip_text(&p, &v.method, &f, r.width() - 8.0), f, TEXT);
            y += 22.0;
        } else {
            y += 4.0;
        }

        // ── "Moves": label with energy cost, text, value on the right
        let accent = theme::pal().accent;
        let mut moves: Vec<(usize, &str, String, String, Color32, String)> = Vec::new();
        if m.odds_bar {
            moves.push((1, "Odds Check", v.odds_word().to_lowercase(), v.mult_text(), tone(v.odds.color), v.odds.tip.clone()));
        }
        if m.luck {
            match &v.luck {
                Some((pct, col, t)) => moves.push((2, "Lucky Streak", "would have it by now".into(), format!("{pct:.0}%"), tone(*col), t.clone())),
                None => moves.push((2, "Lucky Streak", "after the 1st battle".into(), "—".into(), SOFT, "Needs the odds first (start a battle).".into())),
            }
        }
        if m.milestone {
            let (what, when, t) = v.milestone.clone();
            moves.push((3, "Milestone", when, what, TEXT, t));
        }
        for (cost, name, sub, val, col, t) in &moves {
            let top_y = y;
            for k in 0..*cost {
                energy(&p, pos2(x0 + 7.0 + k as f32 * 13.0, y + 11.0), 5.5, if k == 0 { accent } else { mix(accent, Color32::WHITE, 0.3) });
            }
            let tx = x0 + 46.0;
            p.text(pos2(tx, y + 6.0), Align2::LEFT_CENTER, *name, black_font(12.5), TEXT);
            let vr = p.text(pos2(x1, y + 11.0), Align2::RIGHT_CENTER, val, black_font(16.0), *col);
            let f = sans_font(9.5);
            p.text(pos2(tx, y + 19.0), Align2::LEFT_CENTER, clip_text(&p, sub, &f, vr.min.x - tx - 6.0), f, SOFT);
            y += 28.0;
            if *name == "Odds Check" {
                gauge(&p, Rect::from_min_max(pos2(tx, y - 2.0), pos2(x1, y + 4.0)), v.odds.frac, *col);
                y += 9.0;
            }
            line(&p, pos2(x0, y), pos2(x1, y), Stroke::new(1.0f32, fade(TEXT, 0.15)));
            y += 4.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), ("ho-mv", *name), t);
        }

        // ── The weakness / resistance / retreat row: session numbers
        if m.session {
            let cells = [("SESSION", theme::thousands(v.session.0)), ("TIME", v.session_time()), ("PACE", v.session_rate())];
            let cw = (x1 - x0) / 3.0;
            for (i, (lab, val)) in cells.iter().enumerate() {
                let cx = x0 + cw * (i as f32 + 0.5);
                p.text(pos2(cx, y + 6.0), Align2::CENTER_CENTER, *lab, bold_font(7.0), SOFT);
                p.text(pos2(cx, y + 19.0), Align2::CENTER_CENTER, val, sans_bold_font(12.0), TEXT);
                if i > 0 {
                    line(&p, pos2(x0 + cw * i as f32, y + 2.0), pos2(x0 + cw * i as f32, y + 26.0), Stroke::new(1.0f32, fade(TEXT, 0.15)));
                }
            }
            tip(ui, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 28.0)), "ho-session", "This session: encounters, hunting time, pace");
            y += 32.0;
        }
        if m.sparkline {
            let g = Rect::from_min_max(pos2(x0, y + 2.0), pos2(x1, y + 22.0));
            sparkline(&p, g, &v.pace, deep(), fade(deep(), 0.4));
            p.text(pos2(x1, y), Align2::RIGHT_TOP, format!("{}/h", theme::thousands(v.pace_hr)), bold_font(8.0), SOFT);
            tip(ui, g, "ho-pace", "Encounters each minute for the last 20 minutes.");
            y += 28.0;
        }
        // flavour text: the last battle
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("Last met: {} ({a}).", v.last.0),
                None => format!("Last met: {}.", v.last.0),
            };
            let r = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 20.0));
            rstroke(&p, r, 3.0, Stroke::new(1.0f32, fade(GOLD_LO, 0.7)));
            let f = egui::FontId::new(10.0, egui::FontFamily::Proportional);
            p.text(r.center(), Align2::CENTER_CENTER, clip_text(&p, &txt, &f, r.width() - 10.0), f, SOFT);
            y += 26.0;
        }
        // ability tags
        if !v.chips.is_empty() {
            let mut x = x0;
            for (text, col, _f, t) in &v.chips {
                let f = bold_font(8.5);
                let cw = text_w(&p, text, f.clone()) + 16.0;
                if x + cw > x1 && x > x0 {
                    x = x0;
                    y += 22.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 17.0));
                rrect(&p, r, 3.0, if *col == theme::MUTED() { Color32::from_rgb(196, 56, 48) } else { tone(*col) });
                p.text(r.center(), Align2::CENTER_CENTER, text, f, Color32::WHITE);
                tip(ui, r, ("ho-chip", text.clone()), t);
                x += cw + 5.0;
            }
            y += 23.0;
        }
        let bar = |ui: &mut egui::Ui, y: &mut f32, lab: String, right: String, frac: f32, col: Color32, t: &str| {
            p.text(pos2(x0, *y + 5.0), Align2::LEFT_CENTER, &lab, bold_font(8.5), TEXT);
            p.text(pos2(x1, *y + 5.0), Align2::RIGHT_CENTER, &right, sans_font(9.5), SOFT);
            gauge(&p, Rect::from_min_max(pos2(x0, *y + 12.0), pos2(x1, *y + 18.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 20.0)), ("ho-bar", lab.clone()), t);
            *y += 24.0;
        };
        if m.xp {
            bar(ui, &mut y, format!("TRAINER LV {}", v.st.level.max(1)), format!("{} / {} XP", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, Color32::from_rgb(70, 140, 230), "Trainer level progress");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            bar(ui, &mut y, if done { "DAILY GOAL ✔".into() } else { "DAILY GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { GOLD_LO } else { tone(theme::GREEN) }, "Encounters today across every run");
        }

        // ── The card number: count / odds
        if m.counter {
            let r = Rect::from_min_max(pos2(x0 - 2.0, y), pos2(x1 + 2.0, y + 34.0));
            vgrad(&p, &rrect_pts(r, 5.0), mix(deep(), Color32::BLACK, 0.1), deep());
            let num = theme::thousands(v.count);
            let total = v.odds.right.strip_prefix("1/").and_then(|s| s.split(' ').next()).filter(|s| !s.is_empty()).unwrap_or("?").to_string();
            let nr = p.text(pos2(r.min.x + 10.0, r.center().y), Align2::LEFT_CENTER, &num, black_font(22.0), if v.flash && v.blink { GOLD_HI } else { Color32::WHITE });
            p.text(pos2(nr.max.x + 4.0, r.center().y + 3.0), Align2::LEFT_CENTER, format!("/ {total}"), bold_font(11.0), fade(Color32::WHITE, 0.7));
            theme::sparkle(&p, pos2(r.max.x - 14.0, r.center().y), 6.0, GOLD_HI);
            let mut lab = v.counter_label.to_string();
            if let (Some((t, _, _)), false) = (&v.timer, v.compact) {
                lab = format!("{lab} · {t}");
            }
            let lr = p.text(pos2(r.max.x - 26.0, r.center().y), Align2::RIGHT_CENTER, &lab, bold_font(7.5), if v.counter_label == "ENCOUNTERS" { fade(Color32::WHITE, 0.75) } else { mix(v.counter_col, Color32::WHITE, 0.3) });
            if let Some((_, _, tp)) = &v.timer {
                tip(ui, lr, "ho-timer", tp);
            }
            y += 40.0;
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() && v.alert.is_none() {
            let h = 28.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += p.text(pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q, sans_bold_font(12.0), TEXT).width() + 8.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h, h)), b, &button);
                    x += h + 7.0;
                }
                self.ov_button(ui, &p, Rect::from_min_max(pos2(x + 3.0, y), pos2(x1, y + h)), &acts.btns[3], &button);
            } else {
                let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
                let gap = 7.0;
                let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
                for b in &acts.btns {
                    let bw = room * b.weight / total;
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &button);
                    x += bw + gap;
                }
            }
            y += h + 8.0;
        }
        y += 14.0;

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, sleeve);
        if v.alert.is_some() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD + 4.0]], full.max.y, GOLD_LO);
    }
}

/// For the theme picker.
pub(super) fn preview(p: &Painter, card: Rect, t: f32) {
    let card = Rect::from_center_size(card.center(), vec2(58.0, 78.0));
    border(p, card, false);
    p.text(card.min + vec2(9.0, 12.0), Align2::LEFT_CENTER, "Eevee", black_font(8.5), TEXT);
    energy(p, pos2(card.max.x - 11.0, card.min.y + 12.0), 4.0, theme::GREEN);
    let ar = Rect::from_min_max(card.min + vec2(8.0, 19.0), pos2(card.max.x - 8.0, card.min.y + 46.0));
    rrect(p, ar.expand(1.5), 1.0, GOLD_LO);
    rrect(p, ar, 1.0, mix(theme::pal().accent, Color32::WHITE, 0.4));
    foil(&p.with_clip_rect(ar), ar, t, 0.35);
    for k in 0..2 {
        energy(p, pos2(card.min.x + 13.0, card.min.y + 53.0 + k as f32 * 8.0), 2.5, theme::pal().accent);
        line(p, pos2(card.min.x + 19.0, card.min.y + 53.0 + k as f32 * 8.0), pos2(card.max.x - 10.0, card.min.y + 53.0 + k as f32 * 8.0), Stroke::new(1.5f32, fade(TEXT, 0.4)));
    }
    rrect(p, Rect::from_min_max(pos2(card.min.x + 7.0, card.max.y - 14.0), pos2(card.max.x - 7.0, card.max.y - 7.0)), 2.0, deep());
}

// ───────────────────────────── pop-up cards ─────────────────────────────

const TOAST_RAD: f32 = 10.0;

/// A gold-edged card with the foil under its art.
fn toast_card(p: &Painter, r: Rect, _tint: Color32) {
    let rad = TOAST_RAD.min(r.height() / 2.0);
    vgrad(p, &rrect_pts(r, rad), GOLD_HI, GOLD_LO);
    rstroke(p, r.shrink(0.5), rad, Stroke::new(1.0f32, mix(GOLD_LO, Color32::BLACK, 0.3)));
    let inner = r.shrink(4.0);
    if inner.height() > 4.0 {
        vgrad(p, &rrect_pts(inner, (rad - 4.0).max(0.0)), mix(body(), Color32::WHITE, 0.3), body());
    }
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let inner = r.shrink(4.0);
    // the art window: foil behind the icon
    let art = Rect::from_min_size(inner.min + vec2(4.0, 4.0), vec2(48.0, inner.height() - 8.0));
    rrect(p, art.expand(1.5), 4.0, GOLD_LO);
    rrect(p, art, 3.0, mix(deep(), Color32::WHITE, 0.55));
    foil(&p.with_clip_rect(art), art, v.t * 4.0, 0.3);
    energy(p, art.center(), 15.0, fade(v.tint(), 0.9));
    super::ov_toast::badge(p, art.center(), 11.0, v);
    let tx = art.max.x + 10.0;
    let room = inner.max.x - tx - 8.0;
    let cap = v.caption.to_uppercase();
    p.text(pos2(tx, inner.min.y + 10.0), Align2::LEFT_CENTER, clip_text(p, &cap, &sans_bold_font(8.0), room - 14.0), sans_bold_font(8.0), v.a(SOFT));
    // a rarity star in the corner, like a card's set mark
    theme::sparkle(p, pos2(inner.max.x - 9.0, inner.min.y + 10.0), 5.0, v.a(TEXT));
    let f = fit(p, &v.title, black_font, 15.0, 10.0, room);
    let title = clip_text(p, &v.title, &f, room);
    p.text(pos2(tx, inner.min.y + 27.0), Align2::LEFT_CENTER, title, f, v.a(TEXT));
    line(p, pos2(tx, inner.min.y + 37.0), pos2(inner.max.x - 8.0, inner.min.y + 37.0), Stroke::new(1.0f32, fade(GOLD_LO, 0.6)));
    let lf = sans_font(10.0);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, inner.min.y + 46.0), Align2::LEFT_CENTER, line_s, lf, v.a(SOFT));
    // a glint across the whole card once
    let sweep = ((v.t - 0.4) / 1.0).clamp(0.0, 1.0);
    if sweep > 0.0 && sweep < 1.0 {
        let x = r.min.x - 40.0 + sweep * (r.width() + 80.0);
        poly(p, vec![pos2(x, r.max.y), pos2(x + 14.0, r.max.y), pos2(x + 34.0, r.min.y), pos2(x + 20.0, r.min.y)], fade(Color32::WHITE, 0.35));
    }
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: TOAST_RAD,
        card: toast_card,
        inner: |r| r.shrink2(vec2(8.0, 5.0)),
        toast: toast_body,
        title_font: black_font(12.5),
        body_font: sans_font(10.5),
    }
}
