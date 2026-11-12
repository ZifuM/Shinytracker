//! Overlay theme "Glass HUD": a see-through frosted panel with no header
//! bar, the shiny in a soft glowing disc, a huge light count, a hairline
//! odds track with a glowing tip, metrics split by hairlines instead of
//! boxes, and round ghost buttons.

use super::ov_common::*;
use super::App;
use crate::fonts::{bold_font, light_font, sans_bold_font, sans_font};
use crate::theme::{self, GOLD};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

const WHITE: Color32 = Color32::WHITE;
const RAD: f32 = 20.0;

fn soft() -> Color32 {
    fade(WHITE, 0.58)
}

fn accent() -> Color32 {
    mix(theme::pal().accent, WHITE, 0.2)
}

/// Status colours, a touch lighter so they glow on the dark glass.
fn tone(c: Color32) -> Color32 {
    if c == theme::MUTED() {
        soft()
    } else {
        mix(c, WHITE, 0.15)
    }
}

/// Letter-spaced small capitals.
fn tracked(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, col: Color32) -> Rect {
    let font = bold_font(size);
    let gap = size * 0.18;
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

fn glow_dot(s: &dyn Sink, c: Pos2, r: f32, col: Color32) {
    for (k, a) in [(3.2, 0.06), (2.3, 0.12), (1.6, 0.22)] {
        circle(s, c, r * k, fade(col, a));
    }
    circle(s, c, r, col);
}

fn panel(s: &dyn Sink, r: Rect, opacity: f32, alert: bool) {
    let a = 0.25 + 0.6 * opacity;
    vgrad(s, &rrect_pts(r, RAD), rgba(52, 58, 78, a * 0.85), rgba(12, 14, 22, a));
    // a faint sheen across the top-left
    poly(s, vec![r.min + vec2(RAD, 0.0), r.min + vec2(r.width() * 0.55, 0.0), r.min + vec2(r.width() * 0.25, r.height() * 0.45), r.min + vec2(0.0, r.height() * 0.45), r.min + vec2(0.0, RAD)], fade(WHITE, 0.025));
    rstroke(s, r.shrink(0.5), RAD, Stroke::new(1.0f32, if alert { GOLD } else { fade(WHITE, 0.18) }));
    line(s, pos2(r.min.x + RAD, r.min.y + 1.5), pos2(r.max.x - RAD, r.min.y + 1.5), Stroke::new(1.0f32, fade(WHITE, 0.32)));
}

fn track(s: &dyn Sink, r: Rect, frac: f32, col: Color32) {
    let y = r.center().y;
    line(s, pos2(r.min.x, y), pos2(r.max.x, y), Stroke::new(3.0f32, fade(WHITE, 0.12)));
    let x = r.min.x + r.width() * frac.clamp(0.0, 1.0);
    if x > r.min.x + 1.0 {
        hgrad(s, &[pos2(r.min.x, y - 1.5), pos2(x, y - 1.5), pos2(x, y + 1.5), pos2(r.min.x, y + 1.5)], fade(col, 0.25), col);
    }
    glow_dot(s, pos2(x.max(r.min.x + 2.0), y), 3.2, col);
}

fn button(p: &Painter, r: Rect, label: &str, kind: Kind, hovered: bool, down: bool) {
    let round = label.chars().count() <= 2 && kind == Kind::Plain;
    let rad = r.height() / 2.0;
    let (fill, edge) = match kind {
        Kind::Main | Kind::Good => (None, None),
        Kind::Gold => (Some(fade(GOLD, if hovered { 0.35 } else { 0.22 })), Some(fade(GOLD, 0.6))),
        Kind::Plain => (Some(fade(WHITE, if down { 0.2 } else if hovered { 0.13 } else { 0.05 })), Some(fade(WHITE, if hovered { 0.45 } else { 0.25 }))),
    };
    if kind == Kind::Main || kind == Kind::Good {
        let a = if kind == Kind::Main { accent() } else { theme::GREEN };
        let pts = rrect_pts(r, rad);
        hgrad(p, &pts, mix(a, WHITE, if hovered { 0.25 } else { 0.1 }), mix(a, Color32::BLACK, 0.1));
        rstroke(p, r, rad, Stroke::new(1.0f32, fade(WHITE, 0.35)));
    } else {
        let r = if round { Rect::from_center_size(r.center(), vec2(r.height(), r.height())) } else { r };
        rrect(p, r, rad, fill.unwrap_or(Color32::TRANSPARENT));
        rstroke(p, r, rad, Stroke::new(1.0f32, edge.unwrap_or(WHITE)));
    }
    let font = if kind == Kind::Main { sans_bold_font(13.5) } else { sans_bold_font(if round { 14.0 } else { 12.0 }) };
    if label == "⏸" {
        for dx in [-3.0f32, 3.0] {
            rrect(p, Rect::from_center_size(r.center() + vec2(dx, 0.0), vec2(2.5, 11.0)), 1.0, WHITE);
        }
    } else if label == "▶" {
        let c = r.center();
        poly(p, vec![c + vec2(-3.5, -6.0), c + vec2(6.0, 0.0), c + vec2(-3.5, 6.0)], WHITE);
    } else {
        p.text(r.center(), Align2::CENTER_CENTER, label, font, WHITE);
    }
}

fn big(p: &Painter, pos: Pos2, anchor: Align2, s: &str, font: FontId, col: Color32) -> Rect {
    p.text(pos, anchor, s, light_font(font.size * 1.15), col)
}

pub(super) fn look() -> Look {
    Look {
        text: WHITE,
        muted: soft(),
        accent: accent(),
        radius: RAD - 4.0,
        num: light_font,
        label: bold_font,
        body: sans_font,
        panel,
        big,
        button,
        gauge: track,
        divider: |s, a, b| line(s, a, b, Stroke::new(1.0f32, fade(WHITE, 0.12))),
        tone,
        shadow: None,
    }
}

impl App {
    pub(super) fn glass_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, v: &OvView) {
        let p = ui.painter().clone();
        let m = &v.m;
        let win = ui.max_rect();
        let (left, top) = (win.min.x + 1.0, win.min.y + 1.0);
        let w = v.width - 2.0;
        let (x0, x1) = (left + 18.0, left + w - 18.0);
        let acc = accent();
        let bg = p.add(Shape::Noop);

        // ── Top line: status, title, level; quiet controls on the right
        let mut y = top + 8.0;
        let mid = y + 11.0;
        let mut x = x0;
        if m.status_dot {
            glow_dot(&p, pos2(x + 3.0, mid), 3.0, tone(v.status.0));
            tip(ui, Rect::from_center_size(pos2(x + 3.0, mid), vec2(14.0, 14.0)), "gl-dot", &v.status.1);
            x += 13.0;
        }
        let tr = tracked(&p, pos2(x, mid), Align2::LEFT_CENTER, if v.ghost { "CLICK-THROUGH" } else { "SHINY HUNT" }, 9.0, if v.ghost { GOLD } else { fade(WHITE, 0.75) });
        if let (Some(lv), false) = (v.level, v.ghost) {
            tracked(&p, pos2(tr.max.x + 10.0, mid), Align2::LEFT_CENTER, &format!("LV {lv}"), 8.0, acc);
        }
        let mut cx = x1 + 4.0;
        for c in [Ctrl::Close, Ctrl::Hide, Ctrl::Setup, Ctrl::Pin] {
            let r = Rect::from_center_size(pos2(cx - 9.0, mid), vec2(20.0, 20.0));
            let (on, hov) = self.ov_ctrl(ui, ctx, r, c);
            if hov {
                circle(&p, r.center(), 10.0, if c == Ctrl::Close { rgba(220, 60, 60, 0.85) } else { fade(WHITE, 0.14) });
            }
            ctrl_icon(&p, r, c, false, if on { acc } else if hov { WHITE } else { fade(WHITE, 0.5) }, 12.0);
            cx -= 22.0;
        }
        self.ov_drag(ui, ctx, Rect::from_min_max(pos2(left, top), pos2(cx - 6.0, mid + 12.0)), "gl-drag");
        y = mid + 14.0;

        // ── Hero: glowing disc with the shiny, name and a huge count
        let art = v.sprites();
        if art || m.target || m.counter {
            let disc_r = if !art { 0.0 } else if v.compact { 38.0 } else { 48.0 };
            let hero_h = if art { disc_r * 2.0 + 8.0 } else if m.counter { 70.0 } else { 34.0 };
            let c = pos2(x0 + disc_r, y + hero_h / 2.0);
            if art {
                for (k, a) in [(1.0, 0.05), (0.85, 0.07), (0.7, 0.09), (0.55, 0.12)] {
                    circle(&p, c, disc_r * k, fade(acc, a));
                }
                p.circle_stroke(c, disc_r, Stroke::new(1.0f32, fade(WHITE, 0.14)));
                if m.sprites {
                    if v.multi {
                        let n = v.all.len().min(3) as f32;
                        for (k, sp) in v.all.iter().take(3).enumerate() {
                            let fx = c.x + (k as f32 - (n - 1.0) / 2.0) * disc_r * 0.55;
                            self.draw_sprite_centered(&p, sp, true, pos2(fx, c.y), vec2(disc_r * 0.9, disc_r * 1.2));
                        }
                    } else if let Some(sp) = v.target.clone() {
                        self.draw_sprite_centered(&p, &sp, true, c, vec2(disc_r * 1.6, disc_r * 1.6));
                        if !m.single_sprite && !v.compact {
                            let nc = c + vec2(-disc_r * 0.82, disc_r * 0.62);
                            circle(&p, nc, 17.0, rgba(18, 20, 30, 0.85));
                            p.circle_stroke(nc, 17.0, Stroke::new(1.0f32, fade(WHITE, 0.25)));
                            self.draw_sprite_tinted(&p, &sp, false, nc + vec2(0.0, 12.0), vec2(28.0, 26.0), fade(WHITE, 0.8));
                        }
                    }
                }
            }
            let rx = if art { x0 + disc_r * 2.0 + 16.0 } else { x0 };
            let mut ry = y + if art { 8.0 } else { 2.0 };
            if let Some((_, text)) = &v.alert {
                theme::sparkle(&p, pos2(rx + 5.0, ry + 6.0), 5.5, GOLD);
                tracked(&p, pos2(rx + 14.0, ry + 6.0), Align2::LEFT_CENTER, "SHINY", 8.5, GOLD);
                p.text(pos2(rx, ry + 34.0), Align2::LEFT_CENTER, "Found it!", light_font(30.0), if v.blink { GOLD } else { WHITE });
                let f = sans_font(11.5);
                p.text(pos2(rx, ry + 60.0), Align2::LEFT_CENTER, clip_text(&p, text, &f, x1 - rx), f, soft());
                let b = ActBtn { act: Act::Dismiss, label: "Dismiss".into(), tip: "Stop the alert".into(), kind: Kind::Plain, weight: 1.0 };
                self.ov_button(ui, &p, Rect::from_min_size(pos2(rx, ry + 74.0), vec2(84.0, 24.0)), &b, &button);
            } else {
                if m.target {
                    tracked(&p, pos2(rx, ry + 4.0), Align2::LEFT_CENTER, if v.flash && v.blink { "SPOTTED!" } else { "HUNTING" }, 7.5, if v.flash { GOLD } else { soft() });
                    let name = v.title.clone().unwrap_or_else(|| "Pick a target ⚙".into());
                    let f = fit(&p, &name, sans_bold_font, 18.0, 12.0, x1 - rx);
                    let nr = p.text(pos2(rx, ry + 21.0), Align2::LEFT_CENTER, clip_text(&p, &name, &f, x1 - rx), f, if v.title.is_some() { WHITE } else { soft() });
                    if v.multi {
                        tip(ui, nr, "gl-names", &format!("Hunting {}", v.all.iter().map(|s| s.name).collect::<Vec<_>>().join(", ")));
                    }
                    ry += 34.0;
                }
                if m.counter {
                    let num = theme::thousands(v.count);
                    let size = if v.compact { 38.0 } else if m.target { 40.0 } else { 48.0 };
                    let f = fit(&p, &num, light_font, size, 20.0, x1 - rx);
                    let nr = p.text(pos2(rx - 2.0, ry + f.size * 0.55), Align2::LEFT_CENTER, &num, f.clone(), WHITE);
                    let mut line_txt = v.counter_label.to_lowercase();
                    if let Some((t, _, _)) = &v.timer {
                        if !v.compact {
                            line_txt.push_str(&format!(" · {t} today"));
                        }
                    }
                    let lr = p.text(pos2(rx, nr.max.y + 6.0), Align2::LEFT_CENTER, &line_txt, sans_font(11.0), if v.counter_label == "ENCOUNTERS" { soft() } else { tone(v.counter_col) });
                    if let Some((_, _, tp)) = &v.timer {
                        tip(ui, lr, "gl-timer", tp);
                    }
                }
                if let Some((text, dry, t)) = &v.watch {
                    let wr = p.text(pos2(x1, y + 12.0), Align2::RIGHT_CENTER, text.to_lowercase(), sans_font(10.0), if *dry { GOLD } else { fade(WHITE, 0.4) });
                    tip(ui, wr, "gl-watch", t);
                }
            }
            if let Some((el, text)) = &v.celebrate {
                let a = celebrate_fade(*el);
                let area = Rect::from_min_max(pos2(x0 - 6.0, y), pos2(x1 + 6.0, y + hero_h));
                confetti(&p, area, *el, &[GOLD, acc, WHITE], a * 0.8);
                let br = Rect::from_center_size(area.center(), vec2(area.width(), 52.0));
                rrect(&p, br, 14.0, rgba(14, 16, 26, 0.88 * a));
                rstroke(&p, br, 14.0, Stroke::new(1.0f32, fade(GOLD, 0.6 * a)));
                p.text(br.center() - vec2(0.0, 7.0), Align2::CENTER_CENTER, text, light_font(20.0), fade(GOLD, a));
                tracked(&p, br.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, "MILESTONE", 7.5, fade(WHITE, 0.6 * a));
            }
            y += hero_h + 10.0;
        }

        // ── Odds: the multiple, a hairline track with a glowing tip
        if m.odds_bar {
            let col = tone(v.odds.color);
            let top_y = y;
            let mr = p.text(pos2(x0, y + 11.0), Align2::LEFT_CENTER, v.mult_text(), light_font(24.0), WHITE);
            p.text(pos2(mr.max.x + 6.0, y + 15.0), Align2::LEFT_CENTER, "the odds", sans_font(11.0), soft());
            tracked(&p, pos2(x1, y + 6.0), Align2::RIGHT_CENTER, v.odds_word(), 7.5, col);
            p.text(pos2(x1, y + 20.0), Align2::RIGHT_CENTER, &v.odds.right, sans_font(10.5), soft());
            y += 32.0;
            track(&p, Rect::from_min_max(pos2(x0, y), pos2(x1, y + 6.0)), v.odds.frac, col);
            y += 14.0;
            tip(ui, Rect::from_min_max(pos2(x0, top_y), pos2(x1, y)), "gl-odds", &v.odds.tip);
        }
        if m.method_line {
            let f = sans_font(10.5);
            p.text(pos2(x0, y + 5.0), Align2::LEFT_CENTER, clip_text(&p, &v.method, &f, x1 - x0), f, fade(WHITE, 0.45));
            y += 18.0;
        }

        // ── Metrics split by hairlines
        let mut first = true;
        let mut metrics = |ui: &mut egui::Ui, y: &mut f32, cells: Vec<(&str, String, Color32, String)>| {
            if first {
                line(&p, pos2(x0, *y), pos2(x1, *y), Stroke::new(1.0f32, fade(WHITE, 0.1)));
                *y += 8.0;
                first = false;
            }
            let n = cells.len().max(1) as f32;
            let cw = (x1 - x0) / n;
            for (i, (label, value, col, t)) in cells.into_iter().enumerate() {
                let cxl = x0 + i as f32 * cw + if i > 0 { 12.0 } else { 0.0 };
                if i > 0 {
                    line(&p, pos2(x0 + i as f32 * cw, *y + 2.0), pos2(x0 + i as f32 * cw, *y + 32.0), Stroke::new(1.0f32, fade(WHITE, 0.1)));
                }
                tracked(&p, pos2(cxl, *y + 7.0), Align2::LEFT_CENTER, label, 7.0, fade(WHITE, 0.45));
                let f = sans_font(14.5);
                let room = cw - if i > 0 { 16.0 } else { 4.0 };
                p.text(pos2(cxl, *y + 24.0), Align2::LEFT_CENTER, clip_text(&p, &value, &f, room), f, tone(col));
                tip(ui, Rect::from_min_size(pos2(x0 + i as f32 * cw, *y), vec2(cw, 34.0)), ("gl-m", label.to_string()), &t);
            }
            *y += 40.0;
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
            metrics(ui, &mut y, pair);
        }
        if m.session {
            metrics(
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
            metrics(ui, &mut y, vec![("LAST BATTLE", txt, WHITE, String::new())]);
        }
        if m.sparkline {
            tracked(&p, pos2(x0, y + 5.0), Align2::LEFT_CENTER, "PACE · 20 MIN", 7.0, fade(WHITE, 0.45));
            p.text(pos2(x1, y + 5.0), Align2::RIGHT_CENTER, format!("{}/h", theme::thousands(v.pace_hr)), sans_font(11.0), WHITE);
            let g = Rect::from_min_max(pos2(x0, y + 14.0), pos2(x1, y + 38.0));
            sparkline(&p, g, &v.pace, WHITE, fade(acc, 0.7));
            tip(ui, g, "gl-pace", "Encounters each minute for the last 20 minutes.");
            y += 46.0;
        }

        // ── Chips
        if !v.chips.is_empty() {
            let mut x = x0;
            for (text, col, _f, t) in &v.chips {
                let f = bold_font(8.5);
                let cw = text_w(&p, text, f.clone()) + 18.0;
                if x + cw > x1 && x > x0 {
                    x = x0;
                    y += 24.0;
                }
                let r = Rect::from_min_size(pos2(x, y), vec2(cw, 19.0));
                rrect(&p, r, 9.5, fade(WHITE, 0.05));
                rstroke(&p, r, 9.5, Stroke::new(1.0f32, fade(if *col == theme::MUTED() { WHITE } else { *col }, 0.35)));
                p.text(r.center(), Align2::CENTER_CENTER, text, f, tone(*col));
                tip(ui, r, ("gl-chip", text.clone()), t);
                x += cw + 6.0;
            }
            y += 28.0;
        }

        // ── XP and daily goal
        let bar = |ui: &mut egui::Ui, y: &mut f32, label: String, right: String, frac: f32, col: Color32, t: &str| {
            tracked(&p, pos2(x0, *y + 5.0), Align2::LEFT_CENTER, &label, 7.0, col);
            p.text(pos2(x1, *y + 5.0), Align2::RIGHT_CENTER, right, sans_font(10.5), soft());
            track(&p, Rect::from_min_max(pos2(x0, *y + 13.0), pos2(x1, *y + 19.0)), frac, col);
            tip(ui, Rect::from_min_max(pos2(x0, *y), pos2(x1, *y + 20.0)), ("gl-bar", label.clone()), t);
            *y += 28.0;
        };
        if m.xp {
            bar(ui, &mut y, format!("TRAINER LV {}", v.st.level.max(1)), format!("{} / {} XP", theme::thousands(v.st.xp_into), theme::thousands(v.st.xp_need)), v.st.xp_into as f32 / v.st.xp_need.max(1) as f32, Color32::from_rgb(120, 190, 255), "Trainer level progress");
        }
        if m.daily_goal {
            let done = v.goal > 0 && v.st.today >= v.goal;
            let right = if v.goal > 0 { format!("{} / {}", theme::thousands(v.st.today), theme::thousands(v.goal)) } else { "off".into() };
            bar(ui, &mut y, if done { "DAILY GOAL ✔".into() } else { "DAILY GOAL".into() }, right, if v.goal > 0 { v.st.today as f32 / v.goal as f32 } else { 0.0 }, if done { GOLD } else { tone(theme::GREEN) }, "Encounters today across every run");
        }

        // ── Buttons: round ghosts and a filled Caught pill
        let acts = self.ov_actions(m.buttons);
        if !acts.btns.is_empty() {
            let h = 32.0;
            let mut x = x0;
            if let Some(q) = &acts.prompt {
                x += p.text(pos2(x0, y + h / 2.0), Align2::LEFT_CENTER, q, sans_font(12.0), WHITE).width() + 10.0;
            }
            if acts.normal {
                for b in &acts.btns[..3] {
                    self.ov_button(ui, &p, Rect::from_min_size(pos2(x, y), vec2(h, h)), b, &button);
                    x += h + 8.0;
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
        panel_into(&p, bg, full, self.cfg.opacity, alert, panel);
        self.ov_finish(ui, ctx, v, full, vec![[full.min.x, full.min.y, full.width(), full.height(), RAD]], full.max.y, acc);
    }
}

// ───────────────────────────── pop-up cards ─────────────────────────────

const TOAST_RAD: f32 = 16.0;

/// Frosted glass with a soft glow of the toast's colour on the left.
fn toast_card(p: &Painter, r: Rect, tint: Color32) {
    let rad = TOAST_RAD.min(r.height() / 2.0);
    vgrad(p, &rrect_pts(r, rad), rgba(46, 52, 72, 0.96), rgba(14, 16, 26, 0.98));
    let clip = p.with_clip_rect(r.shrink(1.0));
    for (k, a) in [(1.0, 0.10), (0.6, 0.12)] {
        circle(&clip, pos2(r.min.x + 26.0, r.center().y), 46.0 * k, fade(tint, a));
    }
    rstroke(p, r.shrink(0.5), rad, Stroke::new(1.0f32, fade(WHITE, 0.2)));
    if r.height() > rad * 2.0 {
        line(p, pos2(r.min.x + rad, r.min.y + 1.5), pos2(r.max.x - rad, r.min.y + 1.5), Stroke::new(1.0f32, fade(WHITE, 0.35)));
    }
}

fn toast_body(p: &Painter, r: Rect, v: &super::ov_toast::ToastView) {
    let ic = pos2(r.min.x + 32.0, r.center().y);
    glow_dot(p, ic, 15.0, fade(v.tint(), 0.25));
    rstroke(p, Rect::from_center_size(ic, vec2(34.0, 34.0)), 17.0, Stroke::new(1.0f32, fade(WHITE, 0.3)));
    super::ov_toast::badge(p, ic, 12.0, v);
    let tx = r.min.x + 62.0;
    let room = r.max.x - tx - 12.0;
    tracked(p, pos2(tx, r.min.y + 14.0), Align2::LEFT_CENTER, &v.caption.to_uppercase(), 7.0, v.a(mix(v.tint(), WHITE, 0.35)));
    let f = fit(p, &v.title, light_font, 19.0, 12.0, room);
    let title = clip_text(p, &v.title, &f, room);
    p.text(pos2(tx - 1.0, r.min.y + 32.0), Align2::LEFT_CENTER, title, f, v.a(WHITE));
    let lf = sans_font(10.5);
    let line_s = clip_text(p, &v.line, &lf, room);
    p.text(pos2(tx, r.min.y + 49.0), Align2::LEFT_CENTER, line_s, lf, v.a(soft()));
    // a sheen sliding across once
    let sweep = ((v.t - 0.35) / 1.1).clamp(0.0, 1.0);
    if sweep > 0.0 && sweep < 1.0 {
        let x = r.min.x - 60.0 + sweep * (r.width() + 120.0);
        hgrad(p, &[pos2(x - 30.0, r.max.y), pos2(x + 10.0, r.min.y), pos2(x + 40.0, r.min.y), pos2(x, r.max.y)], fade(WHITE, 0.0), fade(WHITE, 0.08));
    }
}

pub(super) fn toast_skin() -> super::ov_toast::ToastSkin {
    super::ov_toast::ToastSkin {
        radius: TOAST_RAD,
        card: toast_card,
        inner: |r| r.shrink2(vec2(6.0, 4.0)),
        toast: toast_body,
        title_font: sans_bold_font(13.0),
        body_font: sans_font(10.5),
    }
}
