//! Overlay theme "Stream neon": near-black with a glowing magenta-to-cyan
//! edge, corner brackets and faint scan lines. A LIVE badge up top, the
//! shiny in a neon hexagon, a huge glowing count, a gradient odds bar, stat
//! tiles with coloured top edges, and outlined neon buttons.

use super::ov_common::*;
use super::overlay_mod::FLAME;
use super::App;
use crate::fonts::{black_font, bold_font, sans_font};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

const MAGENTA: Color32 = Color32::from_rgb(255, 46, 196);
const CYAN: Color32 = Color32::from_rgb(0, 232, 255);
const VIOLET: Color32 = Color32::from_rgb(150, 92, 255);
const LIME: Color32 = Color32::from_rgb(140, 255, 90);
const BG: Color32 = Color32::from_rgb(9, 8, 20);
const WHITE: Color32 = Color32::WHITE;
const RAD: f32 = 12.0;

fn soft() -> Color32 {
    Color32::from_rgb(150, 150, 190)
}

/// Status colours as neon.
fn tone(c: Color32) -> Color32 {
    if c == theme::GREEN {
        LIME
    } else if c == theme::HP_YELLOW || c == GOLD {
        Color32::from_rgb(255, 214, 64)
    } else if c == theme::HP_RED {
        Color32::from_rgb(255, 70, 110)
    } else if c == FLAME {
        Color32::from_rgb(255, 140, 50)
    } else if c == theme::MUTED() {
        soft()
    } else {
        c
    }
}

/// Letter-spaced capitals.
fn tracked(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    let font = bold_font(size);
    let gap = size * 0.22;
    let widths: Vec<f32> = s.chars().map(|c| text_w(p, &c.to_string(), font.clone())).collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
    let r = anchor.anchor_size(pos, vec2(total, size * 1.2));
    let mut x = r.min.x;
    for (c, w) in s.chars().zip(widths) {
        p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, c, font.clone(), col);
        x += w + gap;
    }
    r
}

/// A neon line: a soft wide glow under a bright core.
fn neon_line(s: &dyn Sink, a: Pos2, b: Pos2, col: Color32, w: f32) {
    line(s, a, b, Stroke::new(w * 4.0, fade(col, 0.08)));
    line(s, a, b, Stroke::new(w * 2.2, fade(col, 0.18)));
    line(s, a, b, Stroke::new(w, mix(col, WHITE, 0.25)));
}

/// A rounded neon outline with an inner glow.
fn neon_rect(s: &dyn Sink, r: Rect, rad: f32, col: Color32, w: f32) {
    for (k, a) in [(3.0, 0.08), (2.0, 0.14)] {
        rstroke(s, r.shrink(k), (rad - k).max(0.0), Stroke::new(w * k, fade(col, a)));
    }
    rstroke(s, r, rad, Stroke::new(w, mix(col, WHITE, 0.2)));
}

fn hexagon(c: Pos2, r: f32) -> Vec<Pos2> {
    (0..6)
        .map(|i| {
            let a = (i as f32 * 60.0 - 90.0).to_radians();
            c + vec2(a.cos() * r, a.sin() * r)
        })
        .collect()
}

fn panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    rrect(s, r, RAD, fade(BG, 0.55 + 0.45 * opacity));
    // scan lines
    let mut y = r.min.y + 3.0;
    while y < r.max.y - 3.0 {
        line(s, pos2(r.min.x + 4.0, y), pos2(r.max.x - 4.0, y), Stroke::new(1.0f32, fade(WHITE, 0.025)));
        y += 3.0;
    }
    // the edge: magenta on the left fading to cyan on the right
    let (a, b) = if alert { (GOLD, GOLD) } else { (MAGENTA, CYAN) };
    for (k, al) in [(5.0, 0.05), (3.0, 0.1), (1.5, 0.2)] {
        let rr = r.shrink(k);
        let pts = rrect_pts(rr, RAD - k);
        let n = pts.len();
        for i in 0..n {
            let (p0, p1) = (pts[i], pts[(i + 1) % n]);
            let t = ((p0.x - r.min.x) / r.width()).clamp(0.0, 1.0);
            line(s, p0, p1, Stroke::new(2.0f32, fade(mix(a, b, t), al)));
        }
    }
    let pts = rrect_pts(r.shrink(0.8), RAD);
    let n = pts.len();
    for i in 0..n {
        let (p0, p1) = (pts[i], pts[(i + 1) % n]);
        let t = ((p0.x - r.min.x) / r.width()).clamp(0.0, 1.0);
        line(s, p0, p1, Stroke::new(1.6f32, mix(mix(a, b, t), WHITE, 0.2)));
    }
    // corner brackets
    let l = 14.0;
    let i = r.shrink(7.0);
    for (c, dx, dy, col) in [(i.left_top(), 1.0, 1.0, a), (i.right_top(), -1.0, 1.0, b), (i.left_bottom(), 1.0, -1.0, a), (i.right_bottom(), -1.0, -1.0, b)] {
        neon_line(s, c, c + vec2(l * dx, 0.0), col, 1.5);
        neon_line(s, c, c + vec2(0.0, l * dy), col, 1.5);
    }
}

fn bar(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    rrect(s, r, r.height() / 2.0, fade(WHITE, 0.08));
    let w = r.width() * frac.clamp(0.0, 1.0);
    if w > 1.0 {
        let f = Rect::from_min_size(r.min, vec2(w.max(r.height()), r.height()));
        rrect(s, f.expand(3.0), r.height() / 2.0 + 3.0, fade(col, 0.15));
        hgrad(s, &rrect_pts(f, r.height() / 2.0), MAGENTA, col);
        circle(s, pos2(f.max.x - r.height() / 2.0, f.center().y), r.height() * 0.9, fade(WHITE, 0.7));
    }
}

fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let col = match kind {
        Kind::Main => MAGENTA,
        Kind::Good => LIME,
        Kind::Gold => Color32::from_rgb(255, 214, 64),
        Kind::Plain => match label {
            "−1" => CYAN,
            "+1" => CYAN,
            "⏸" | "▶" => VIOLET,
            _ => soft(),
        },
    };
    let r = if down { r.shrink(1.0) } else { r };
    if kind == Kind::Main {
        rrect(p, r.expand(3.0), 9.0, fade(MAGENTA, if hovered { 0.3 } else { 0.15 }));
        hgrad(p, &rrect_pts(r, 6.0), if hovered { mix(MAGENTA, WHITE, 0.2) } else { MAGENTA }, VIOLET);
        rstroke(p, r, 6.0, Stroke::new(1.0f32, fade(WHITE, 0.5)));
        let size = (r.height() * 0.42).min(14.0);
        p.text(r.center() + vec2(1.0, 1.0), Align2::CENTER_CENTER, label.to_uppercase(), black_font(size), fade(Color32::BLACK, 0.5));
        p.text(r.center(), Align2::CENTER_CENTER, label.to_uppercase(), black_font(size), WHITE);
        return;
    }
    rrect(p, r, 6.0, fade(col, if hovered { 0.16 } else { 0.06 }));
    neon_rect(p, r, 6.0, col, if hovered { 1.6 } else { 1.2 });
    if label == "⏸" {
        for dx in [-3.5f32, 3.5] {
            rrect(p, Rect::from_center_size(r.center() + vec2(dx, 0.0), vec2(3.0, 12.0)), 1.0, mix(col, WHITE, 0.4));
        }
    } else if label == "▶" {
        let c = r.center();
        poly(p, vec![c + vec2(-4.0, -7.0), c + vec2(7.0, 0.0), c + vec2(-4.0, 7.0)], mix(col, WHITE, 0.4));
    } else {
        p.text(r.center(), Align2::CENTER_CENTER, label, black_font(if label.chars().count() <= 2 { 15.0 } else { 12.0 }), mix(col, WHITE, 0.6));
    }
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    glow_text(p, pos, anchor, s, black_font(font.size * 1.1), if col == WHITE { WHITE } else { col }, MAGENTA, font.size * 0.22)
}

pub(super) fn look() -> Look {
    Look {
        text: WHITE,
        muted: soft(),
        accent: MAGENTA,
        radius: RAD,
        num: black_font,
        label: bold_font,
        body: sans_font,
        panel,
        big,
        button,
        gauge: bar,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, fade(VIOLET, 0.35))),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn neon_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let (x0, x1) = (left + 18.0, left + w - 18.0);
        let bg = p.add(Shape::Noop);

        // ── Header: LIVE badge, title, level, controls
        let mid = top + 25.0;
        let mut x = x0;
        let (live, live_col) = if v.counter_label == "PAUSED" {
            ("PAUSED", soft())
        } else if v.counter_label == "GAME CLOSED" {
            ("OFFLINE", soft())
        } else {
            ("LIVE", Color32::from_rgb(255, 50, 80))
        };
        let lw = text_w(&p, live, bold_font(9.0)) + 22.0;
        let lb = Rect::from_min_size(pos2(x, mid - 9.0), vec2(lw, 18.0));
        rrect(&p, lb, 4.0, fade(live_col, 0.18));
        rstroke(&p, lb, 4.0, Stroke::new(1.0f32, fade(live_col, 0.7)));
        let pulse = if live == "LIVE" { 0.55 + 0.45 * (v.t * 3.0).sin().abs() } else { 1.0 };
        circle(&p, pos2(lb.min.x + 8.0, mid), 3.0, fade(live_col, pulse));
        p.text(pos2(lb.min.x + 14.0, mid), Align2::LEFT_CENTER, live, bold_font(9.0), live_col);
        if m.status_dot {
            tip(ui, lb, "ne-live", &v.status.1);
        }
        x = lb.max.x + 10.0;
        // Title and level only where they fit before the controls.
        let room_end = x1 + 2.0 - 4.0 * 23.0 - 4.0;
        let title = if v.ghost { "CLICK-THROUGH" } else { "SHINY HUNT" };
        let tw = text_w(&p, title, bold_font(9.5)) + title.len() as f32 * 9.5 * 0.22;
        if x + tw <= room_end {
            let tr = tracked(&p, pos2(x, mid), Align2::LEFT_CENTER, title, 9.5, if v.ghost { GOLD } else { WHITE });
            if let (Some(lv), false) = (v.level, v.ghost) {
                let lt = format!("LV{lv}");
                if tr.max.x + 9.0 + text_w(&p, &lt, bold_font(8.5)) + lt.len() as f32 * 2.0 <= room_end {
                    tracked(&p, pos2(tr.max.x + 9.0, mid), Align2::LEFT_CENTER, &lt, 8.5, CYAN);
                }
            }
        }
        let mut cx = x1 + 2.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, mid), vec2(20.0, 20.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            let col = if c == Ctrl::Close && hov { Color32::from_rgb(255, 70, 110) } else if on { CYAN } else if hov { MAGENTA } else { soft() };
            if hov {
                neon_rect(&p, r, 4.0, col, 1.0);
            }
            ctrl_icon(&p, r, c, false, col, 12.0);
            cx -= 23.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 6.0, mid + 14.0)), "ne-drag");
        let mut y = mid + 16.0;
        neon_line(&p, pos2(x0, y), pos2(x1, y), fade(VIOLET, 0.6), 1.0);
        y += 12.0;

        // ── Hero: hexagon with the shiny; name and a glowing count
        let art = v.sprites();
        if art || m.target || m.counter {
            let hr = if !art { 0.0 } else if v.compact { 40.0 } else { 50.0 };
            let hero_h = if art { hr * 2.0 + 4.0 } else if m.counter { 64.0 } else { 30.0 };
            let c = pos2(x0 + hr * 0.95, y + hero_h / 2.0);
            if art {
                let hex = hexagon(c, hr);
                poly(&p, hexagon(c, hr), fade(VIOLET, 0.12));
                let spin = 0.5 + 0.5 * (v.t * 1.2).sin();
                let n = hex.len();
                for i in 0..n {
                    let t = i as f32 / n as f32;
                    neon_line(&p, hex[i], hex[(i + 1) % n], mix(MAGENTA, CYAN, (t + spin * 0.3).fract()), 1.6);
                }
                if m.sprites {
                    if v.multi {
                        let k = v.all.len().min(3) as f32;
                        for (i, sp) in v.all.iter().take(3).enumerate() {
                            let fx = c.x + (i as f32 - (k - 1.0) / 2.0) * hr * 0.55;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, c.y), vec2(hr * 0.8, hr * 1.1));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, c, vec2(hr * 1.45, hr * 1.45));
                        if !m.single_sprite && !v.compact {
                            let sc = c + vec2(hr * 0.78, hr * 0.62);
                            poly(&p, hexagon(sc, 15.0), BG);
                            let sh = hexagon(sc, 15.0);
                            for i in 0..6 {
                                neon_line(&p, sh[i], sh[(i + 1) % 6], CYAN, 1.0);
                            }
                            self.draw_sprite(&p, &sp, false, sc + vec2(0.0, 11.0), vec2(24.0, 22.0));
                        }
                    }
                }
            }
            let rx = if art { x0 + hr * 2.0 + 14.0 } else { x0 };
            let mut ry = y + if art { 6.0 } else { 0.0 };
            if let Some((_, text)) = &v.alert {
                glow_text(&p, pos2(rx, ry + 22.0), Align2::LEFT_CENTER, "SHINY!", black_font(30.0), if v.blink { Color32::from_rgb(255, 214, 64) } else { WHITE }, MAGENTA, 6.0);
                let f = sans_font(11.0);
                p.text(pos2(rx, ry + 48.0), Align2::LEFT_CENTER, clip_text(&p, text, &f, x1 - rx), f, soft());
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, ry + 62.0), vec2(84.0, 24.0)), &b, &button);
            } else {
                if m.target {
                    let name = v.title.clone().map(|t| t.to_uppercase()).unwrap_or_else(|| "PICK A TARGET".into());
                    let f = fit(&p, &name, black_font, 17.0, 11.0, x1 - rx);
                    let nr = glow_text(&p, pos2(rx, ry + 10.0), Align2::LEFT_CENTER, &clip_text(&p, &name, &f, x1 - rx), f, if v.flash && v.blink { Color32::from_rgb(255, 214, 64) } else { mix(CYAN, WHITE, 0.6) }, CYAN, 3.0);
                    if v.multi {
                        tip(ui, nr, "ne-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                    }
                    if let Some((text, dry, t)) = &v.watch {
                        let wr = tracked(&p, pos2(rx, ry + 25.0), Align2::LEFT_CENTER, text, 7.0, if *dry { Color32::from_rgb(255, 214, 64) } else { soft() });
                        tip(ui, wr, "ne-watch", t);
                    }
                    ry += 32.0;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let size = if v.compact { 34.0 } else { 40.0 };
                    let f = fit(&p, &num, black_font, size, 18.0, x1 - rx);
                    let nr = glow_text(&p, pos2(rx - 1.0, ry + f.size * 0.6), Align2::LEFT_CENTER, &num, f.clone(), WHITE, MAGENTA, f.size * 0.2);
                    let mut lab = v.counter_label.to_string();
                    if let Some((t, _, _)) = &v.timer {
                        if !v.compact {
                            lab.push_str(&format!("  ·  {}", t.to_uppercase()));
                        }
                    }
                    let lr = tracked(&p, pos2(rx, nr.max.y + 5.0), Align2::LEFT_CENTER, &lab, 7.5, if v.counter_label == "ENCOUNTERS" { soft() } else { tone(v.counter_col) });
                    if let Some((_, _, tp)) = &v.timer {
                        tip(ui, lr, "ne-timer", tp);
                    }
                }
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let area = Rect::from_min_max(pos2(x0 - 6.0, y), pos2(x1 + 6.0, y + hero_h));
                confetti(&p, area, *el, &[MAGENTA, CYAN, VIOLET, LIME, WHITE], a);
                let br = Rect::from_center_size(area.center(), vec2(area.width(), 48.0));
                rrect(&p, br, 8.0, fade(BG, 0.92 * a));
                neon_rect(&p, br, 8.0, fade(MAGENTA, a), 1.5);
                glow_text(&p, br.center() - vec2(0.0, 6.0), Align2::CENTER_CENTER, &text.to_uppercase(), black_font(16.0), fade(WHITE, a), fade(MAGENTA, a), 4.0);
                tracked(&p, br.center() + vec2(0.0, 13.0), Align2::CENTER_CENTER, "MILESTONE", 7.0, fade(CYAN, a));
            }
            y += hero_h + 10.0;
        }

        // ── Odds: label, gradient bar
        if m.odds_bar {
            let col = tone(v.odds.color);
            let top_y = y;
            let lr = glow_text(&p, pos2(x0, y + 8.0), Align2::LEFT_CENTER, &v.mult_text(), black_font(17.0), WHITE, col, 3.0);
            tracked(&p, pos2(lr.max.x + 8.0, y + 9.0), Align2::LEFT_CENTER, v.odds_word(), 7.5, col);
            p.text(pos2(x1, y + 9.0), Align2::RIGHT_CENTER, &v.odds.right, sans_font(10.5), soft());
            y += 22.0;
            bar(&p, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 7.0)), v.odds.frac, col);
            y += 16.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "ne-odds", &v.odds.tip);
        }
        if m.method_line {
            tracked(&p, pos2(x0, y + 4.0), Align2::LEFT_CENTER, &v.method.to_uppercase(), 7.0, fade(soft(), 0.85));
            y += 18.0;
        }

        // ── Stat tiles with coloured top edges
        let cols = [MAGENTA, CYAN, VIOLET, LIME];
        let mut k = 0usize;
        let mut tiles = |ui: &mut egui::Ui, y: &mut f32, cells: Vec<(&str, String, Color32, String)>| {
            let n = cells.len().max(1) as f32;
            let gap = 8.0;
            let cw = (x1 - x0 - gap * (n - 1.0)) / n;
            for (i, (label, value, col, t)) in cells.into_iter().enumerate() {
                let r = Rect::from_min_size(pos2(x0 + i as f32 * (cw + gap), *y), vec2(cw, 40.0));
                let edge = cols[k % cols.len()];
                k += 1;
                rrect(&p, r, 6.0, fade(WHITE, 0.045));
                neon_line(&p, pos2(r.min.x + 6.0, r.min.y + 0.5), pos2(r.max.x - 6.0, r.min.y + 0.5), edge, 1.5);
                tracked(&p, pos2(r.min.x + 8.0, r.min.y + 12.0), Align2::LEFT_CENTER, label, 6.5, fade(edge, 0.9));
                let f = bold_font(13.0);
                p.text(pos2(r.min.x + 8.0, r.max.y - 11.0), Align2::LEFT_CENTER, clip_text(&p, &value, &f, r.width() - 14.0), f, tone(col));
                tip(ui, r, ("ne-tile", label.to_string()), &t);
            }
            *y += 48.0;
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
            pair.push(("NEXT", format!("{what} {when}"), WHITE, t));
        }
        if !pair.is_empty() {
            tiles(ui, &mut y, pair);
        }
        if m.session {
            tiles(
                ui,
                &mut y,
                vec![
                    ("SESSION", theme::thousands(v.session.0), WHITE, "Encounters this session".into()),
                    ("TIME", v.session_time(), WHITE, "Hunting time this session".into()),
                    ("PACE", v.session_rate(), WHITE, "Encounters per hour this session".into()),
                ],
            );
        }
        if m.last_seen {
            let txt = match &v.last.1 {
                Some(a) => format!("{} · {a}", v.last.0),
                None => v.last.0.clone(),
            };
            tiles(ui, &mut y, vec![("LAST BATTLE", txt, WHITE, String::new())]);
        }
        if m.sparkline {
            let r = Rect::from_min_max(pos2(x0, y), pos2(x1, y + 46.0));
            rrect(&p, r, 6.0, fade(WHITE, 0.045));
            tracked(&p, pos2(r.min.x + 8.0, r.min.y + 10.0), Align2::LEFT_CENTER, "PACE · 20 MIN", 6.5, CYAN);
            p.text(pos2(r.max.x - 8.0, r.min.y + 10.0), Align2::RIGHT_CENTER, format!("{}/h", theme::thousands(v.pace_hr)), bold_font(11.0), WHITE);
            sparkline(&p, Rect::from_min_max(pos2(r.min.x + 8.0, r.min.y + 18.0), pos2(r.max.x - 8.0, r.max.y - 6.0)), &v.pace, MAGENTA, fade(CYAN, 0.6));
            tip(ui, r, "ne-pace", "Encounters each minute for the last 20 minutes.");
            y = r.max.y + 8.0;
        }

        // ── Chips
        if !v.chips.is_empty() {
            let mut x = x0;
            for (text, col, _f, t) in &v.chips {
                let c = tone(*col);
                let cw = text_w(&p, text, bold_font(8.5)) + 20.0;
                if x + cw > x1 && x > x0 {
                    x = x0;
                    y += 25.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 19.0));
                neon_rect(&p, r, 9.5, c, 1.0);
                p.text(r.center(), Align2::CENTER_CENTER, text, bold_font(8.5), mix(c, WHITE, 0.3));
                tip(ui, r, ("ne-chip", text.clone()), t);
                x += cw + 7.0;
            }
            y += 29.0;
        }

        // ── XP and daily goal
        let gbar = |ui: &mut egui::Ui, y: &mut f32, label: String, right: String, frac: f32, col: Color32, t: &str| {
            tracked(&p, pos2(x0, *y + 5.0), Align2::LEFT_CENTER, &label, 7.0, col);
            p.text(pos2(x1, *y + 5.0), Align2::RIGHT_CENTER, right, sans_font(10.5), soft());
            bar(&p, Rect::from_min_max(pos2(x0, *y + 13.0), pos2(x1, *y + 18.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 20.0)), ("ne-bar", label.clone()), t);
            *y += 27.0;
        };
        if m.xp {
            gbar(ui, &mut y, format!("TRAINER LV {}", v.st.level.max(1)), format!("{} / {} XP", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, CYAN, "Trainer level progress");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            gbar(ui, &mut y, if done { "DAILY GOAL ✔".into() } else { "DAILY GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { Color32::from_rgb(255, 214, 64) } else { LIME }, "Encounters today across every run");
        }

        // ── Buttons
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 32.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += glow_text(&p, pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q, bold_font(12.0), WHITE, MAGENTA, 2.0).width() + 10.0;
            }
            let total: f32 = acts.btns.iter().map(|b| b.weight).sum();
            let gap = 8.0;
            let room = x1 - x - gap * (acts.btns.len() as f32 - 1.0);
            for b in &acts.btns {
                let bw = room * b.weight / total;
                self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(bw, h)), b, &button);
                x += bw + gap;
            }
            y += h + 16.0;
        } else {
            y += 8.0;
        }

        let full = Rect::from_min_max(pos2(left, top), pos2(left + w, y));
        let alert = (v.alert.is_some() || v.flash) && v.blink;
        panel_into(&p, bg, full, self.cfg.opacity, alert, panel);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, MAGENTA);
    }
}

// ───────────────────────────── pop-up cards ─────────────────────────────

/// A neon sign flickering on: 0 or 1 for the first half second, then 1.
fn flicker(t: f32) -> f32 {
    if t > 0.75 {
        return 1.0;
    }
    match (t * 20.0) as u32 {
        3 | 4 | 7 | 10 | 11 => 0.25,
        _ => 1.0,
    }
}

/// Near-black with scan lines and a glowing edge in the toast's colour.
fn toast_card(p: &Painter, r: Rect, tint: Color32) {
    let rad = RAD.min(r.height() / 2.0);
    rrect(p, r, rad, BG);
    let mut y = r.min.y + 3.0;
    while y < r.max.y - 2.0 {
        line(p, pos2(r.min.x + 3.0, y), pos2(r.max.x - 3.0, y), Stroke::new(1.0f32, fade(WHITE, 0.025)));
        y += 3.0;
    }
    let col = tone(tint);
    let pts = rrect_pts(r.shrink(1.0), (rad - 1.0).max(0.0));
    let n = pts.len();
    for (w, a) in [(5.0f32, 0.08f32), (2.6, 0.2)] {
        for i in 0..n {
            let (p0, p1) = (pts[i], pts[(i + 1) % n]);
            let k = ((p0.x - r.min.x) / r.width()).clamp(0.0, 1.0);
            line(p, p0, p1, Stroke::new(w, fade(mix(col, CYAN, k), a)));
        }
    }
    for i in 0..n {
        let (p0, p1) = (pts[i], pts[(i + 1) % n]);
        let k = ((p0.x - r.min.x) / r.width()).clamp(0.0, 1.0);
        line(p, p0, p1, Stroke::new(1.4f32, mix(mix(col, CYAN, k), WHITE, 0.25)));
    }
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let on = flicker(v.t);
    let col = fade(tone(v.tint()), on);
    let ic = pos2(r.min.x + 32.0, r.center().y);
    let hex = hexagon(ic, 22.0);
    poly(p, hex.clone(), fade(col, 0.1));
    for i in 0..6 {
        neon_line(p, hex[i], hex[(i + 1) % 6], col, 1.3);
    }
    super::ov_toast::badge(p, ic, 11.5, v);
    let tx = r.min.x + 64.0;
    let room = r.max.x - tx - 12.0;
    tracked(p, pos2(tx, r.min.y + 14.0), Align2::LEFT_CENTER, &v.caption.to_uppercase(), 7.0, v.a(col));
    let title = v.title.to_uppercase();
    let f = fit(p, &title, black_font, 15.0, 10.0, room);
    let title = clip_text(p, &title, &f, room);
    glow_text(p, pos2(tx, r.min.y + 32.0), Align2::LEFT_CENTER, &title, f, v.a(fade(WHITE, on)), fade(col, on), 3.0);
    let lf = sans_font(10.0);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, r.min.y + 49.0), Align2::LEFT_CENTER, line_s, lf, v.a(soft()));
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: RAD,
        card: toast_card,
        inner: |r| r.shrink2(vec2(6.0, 4.0)),
        toast: toast_body,
        title_font: black_font(12.5),
        body_font: sans_font(10.5),
    }
}
